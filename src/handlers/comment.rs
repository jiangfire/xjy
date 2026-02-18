use crate::error::{AppError, AppResult};
use crate::middleware::auth::parse_user_id;
use crate::middleware::AuthUser;
use crate::models::CommentModel;
use crate::response::ApiResponse;
use crate::services::comment::CommentService;
use crate::services::notification::NotificationService;
use crate::services::post::PostService;
use crate::utils::render_markdown;
use crate::websocket::hub::NotificationHub;
use axum::{extract::Path, response::IntoResponse, Extension, Json};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateCommentRequest {
    pub post_id: i32,
    pub parent_id: Option<i32>,
    #[validate(length(min = 1))]
    pub content: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateCommentRequest {
    #[validate(length(min = 1))]
    pub content: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CommentResponse {
    pub id: i32,
    pub post_id: i32,
    pub user_id: i32,
    pub parent_id: Option<i32>,
    pub content: String,
    pub content_html: String,
    pub upvotes: i32,
    pub downvotes: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl From<CommentModel> for CommentResponse {
    fn from(c: CommentModel) -> Self {
        let content_html = render_markdown(&c.content);
        Self {
            id: c.id,
            post_id: c.post_id,
            user_id: c.user_id,
            parent_id: c.parent_id,
            content: c.content,
            content_html,
            upvotes: c.upvotes,
            downvotes: c.downvotes,
            created_at: c.created_at.to_string(),
            updated_at: c.updated_at.to_string(),
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct CommentTreeNode {
    pub id: i32,
    pub post_id: i32,
    pub user_id: i32,
    pub parent_id: Option<i32>,
    pub content: String,
    pub content_html: String,
    pub upvotes: i32,
    pub downvotes: i32,
    pub created_at: String,
    pub updated_at: String,
    pub children: Vec<CommentTreeNode>,
}

impl utoipa::ToSchema for CommentTreeNode {
    fn name() -> std::borrow::Cow<'static, str> {
        "CommentTreeNode".into()
    }
}

impl utoipa::PartialSchema for CommentTreeNode {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        use utoipa::openapi::schema::{ObjectBuilder, Schema, Type};
        utoipa::openapi::RefOr::T(Schema::Object(
            ObjectBuilder::new()
                .schema_type(Type::Object)
                .property("id", i32::schema())
                .property("post_id", i32::schema())
                .property("user_id", i32::schema())
                .property("parent_id", Option::<i32>::schema())
                .property("content", String::schema())
                .property("content_html", String::schema())
                .property("upvotes", i32::schema())
                .property("downvotes", i32::schema())
                .property("created_at", String::schema())
                .property("updated_at", String::schema())
                .property("children", utoipa::openapi::schema::ArrayBuilder::new()
                    .items(utoipa::openapi::Ref::from_schema_name("CommentTreeNode"))
                    .build())
                .required("id")
                .required("post_id")
                .required("user_id")
                .required("content")
                .required("content_html")
                .required("upvotes")
                .required("downvotes")
                .required("created_at")
                .required("updated_at")
                .required("children")
                .build(),
        ))
    }
}

impl From<CommentModel> for CommentTreeNode {
    fn from(c: CommentModel) -> Self {
        let content_html = render_markdown(&c.content);
        Self {
            id: c.id,
            post_id: c.post_id,
            user_id: c.user_id,
            parent_id: c.parent_id,
            content: c.content,
            content_html,
            upvotes: c.upvotes,
            downvotes: c.downvotes,
            created_at: c.created_at.to_string(),
            updated_at: c.updated_at.to_string(),
            children: Vec::new(),
        }
    }
}

fn build_comment_tree(comments: Vec<CommentModel>) -> Vec<CommentTreeNode> {
    let mut nodes: HashMap<i32, CommentTreeNode> = HashMap::new();
    let mut children_map: HashMap<Option<i32>, Vec<i32>> = HashMap::new();

    for comment in &comments {
        children_map
            .entry(comment.parent_id)
            .or_default()
            .push(comment.id);
    }
    for comment in comments {
        let id = comment.id;
        nodes.insert(id, CommentTreeNode::from(comment));
    }

    fn attach_children(
        node_id: i32,
        nodes: &mut HashMap<i32, CommentTreeNode>,
        children_map: &HashMap<Option<i32>, Vec<i32>>,
    ) -> Option<CommentTreeNode> {
        let mut node = nodes.remove(&node_id)?;
        if let Some(child_ids) = children_map.get(&Some(node_id)) {
            for &child_id in child_ids {
                if nodes.contains_key(&child_id) {
                    if let Some(child) = attach_children(child_id, nodes, children_map) {
                        node.children.push(child);
                    }
                }
            }
        }
        Some(node)
    }

    let root_ids = children_map.get(&None).cloned().unwrap_or_default();
    root_ids
        .into_iter()
        .filter_map(|id| attach_children(id, &mut nodes, &children_map))
        .collect()
}

#[utoipa::path(
    get,
    path = "/api/v1/posts/{post_id}/comments",
    params(("post_id" = i32, Path, description = "Post ID")),
    responses(
        (status = 200, description = "Comment tree", body = Vec<CommentTreeNode>),
    ),
    tag = "comments"
)]
pub async fn list_comments(
    Extension(db): Extension<DatabaseConnection>,
    Path(post_id): Path<i32>,
) -> AppResult<impl IntoResponse> {
    let service = CommentService::new(db);
    let comments = service.list_by_post(post_id).await?;
    let tree = build_comment_tree(comments);
    Ok(ApiResponse::ok(tree))
}

#[utoipa::path(
    post,
    path = "/api/v1/comments",
    security(("jwt_token" = [])),
    request_body = CreateCommentRequest,
    responses(
        (status = 200, description = "Comment created", body = CommentResponse),
        (status = 400, description = "Validation error", body = AppError),
        (status = 401, description = "Unauthorized", body = AppError),
    ),
    tag = "comments"
)]
pub async fn create_comment(
    Extension(db): Extension<DatabaseConnection>,
    Extension(hub): Extension<NotificationHub>,
    auth_user: AuthUser,
    Json(payload): Json<CreateCommentRequest>,
) -> AppResult<impl IntoResponse> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let user_id = parse_user_id(&auth_user)?;

    let comment_service = CommentService::new(db.clone());
    let comment = comment_service
        .create(
            payload.post_id,
            user_id,
            payload.parent_id,
            &payload.content,
        )
        .await?;

    // Fire notifications (best-effort, don't fail the request)
    let notif_service = NotificationService::new(db.clone(), hub);
    let post_service = PostService::new(db);

    // Notify post author
    if let Ok(post) = post_service.get_by_id(payload.post_id).await {
        let _ = notif_service
            .notify(
                post.user_id,
                user_id,
                "comment_on_post",
                "post",
                post.id,
                "Someone commented on your post",
            )
            .await;
    }

    // Notify parent comment author (if replying)
    if let Some(parent_id) = payload.parent_id {
        if let Ok(parent) = comment_service.get_by_id(parent_id).await {
            let _ = notif_service
                .notify(
                    parent.user_id,
                    user_id,
                    "reply_to_comment",
                    "comment",
                    parent.id,
                    "Someone replied to your comment",
                )
                .await;
        }
    }

    Ok(ApiResponse::ok(CommentResponse::from(comment)))
}

#[utoipa::path(
    put,
    path = "/api/v1/comments/{id}",
    security(("jwt_token" = [])),
    params(("id" = i32, Path, description = "Comment ID")),
    request_body = UpdateCommentRequest,
    responses(
        (status = 200, description = "Comment updated", body = CommentResponse),
        (status = 400, description = "Validation error", body = AppError),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 404, description = "Comment not found", body = AppError),
    ),
    tag = "comments"
)]
pub async fn update_comment(
    Extension(db): Extension<DatabaseConnection>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateCommentRequest>,
) -> AppResult<impl IntoResponse> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let user_id = parse_user_id(&auth_user)?;

    let service = CommentService::new(db);
    let comment = service.update(id, user_id, &payload.content).await?;

    Ok(ApiResponse::ok(CommentResponse::from(comment)))
}

#[utoipa::path(
    delete,
    path = "/api/v1/comments/{id}",
    security(("jwt_token" = [])),
    params(("id" = i32, Path, description = "Comment ID")),
    responses(
        (status = 200, description = "Comment deleted", body = String),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 404, description = "Comment not found", body = AppError),
    ),
    tag = "comments"
)]
pub async fn delete_comment(
    Extension(db): Extension<DatabaseConnection>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> AppResult<impl IntoResponse> {
    let user_id = parse_user_id(&auth_user)?;

    let service = CommentService::new(db);
    service.delete(id, user_id).await?;

    Ok(ApiResponse::ok("Comment deleted"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;

    fn make_comment(id: i32, post_id: i32, parent_id: Option<i32>) -> CommentModel {
        let now = NaiveDateTime::default();
        CommentModel {
            id,
            post_id,
            user_id: 1,
            parent_id,
            content: format!("Comment {}", id),
            upvotes: 0,
            downvotes: 0,
            is_hidden: false,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn flat_comments_become_roots() {
        let comments = vec![
            make_comment(1, 1, None),
            make_comment(2, 1, None),
            make_comment(3, 1, None),
        ];
        let tree = build_comment_tree(comments);
        assert_eq!(tree.len(), 3);
        assert!(tree.iter().all(|n| n.children.is_empty()));
    }

    #[test]
    fn nested_comments_build_tree() {
        let comments = vec![
            make_comment(1, 1, None),
            make_comment(2, 1, Some(1)),
            make_comment(3, 1, Some(2)),
        ];
        let tree = build_comment_tree(comments);
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].id, 1);
        assert_eq!(tree[0].children.len(), 1);
        assert_eq!(tree[0].children[0].id, 2);
        assert_eq!(tree[0].children[0].children.len(), 1);
        assert_eq!(tree[0].children[0].children[0].id, 3);
    }

    #[test]
    fn orphan_comments_are_skipped() {
        let comments = vec![
            make_comment(1, 1, None),
            make_comment(2, 1, Some(999)), // parent doesn't exist
        ];
        let tree = build_comment_tree(comments);
        // Root should be id=1, orphan id=2 is never attached since parent_id 999 isn't a root
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].id, 1);
    }

    #[test]
    fn empty_input_gives_empty_tree() {
        let tree = build_comment_tree(vec![]);
        assert!(tree.is_empty());
    }

    #[test]
    fn multiple_roots_with_children() {
        let comments = vec![
            make_comment(1, 1, None),
            make_comment(2, 1, None),
            make_comment(3, 1, Some(1)),
            make_comment(4, 1, Some(2)),
        ];
        let tree = build_comment_tree(comments);
        assert_eq!(tree.len(), 2);
        assert_eq!(tree[0].children.len(), 1);
        assert_eq!(tree[1].children.len(), 1);
    }

    #[test]
    fn content_html_is_rendered() {
        let mut c = make_comment(1, 1, None);
        c.content = "**bold** text".to_string();
        let node = CommentTreeNode::from(c);
        assert!(node.content_html.contains("<strong>bold</strong>"));
        assert_eq!(node.content, "**bold** text");
    }
}
