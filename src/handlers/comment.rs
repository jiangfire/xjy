use crate::error::{AppError, AppResult};
use crate::middleware::auth::parse_user_id;
use crate::middleware::AuthUser;
use crate::models::CommentModel;
use crate::response::ApiResponse;
use crate::services::comment::CommentService;
use crate::services::notification::NotificationService;
use crate::services::post::PostService;
use crate::websocket::hub::NotificationHub;
use axum::{extract::Path, response::IntoResponse, Extension, Json};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateCommentRequest {
    pub post_id: i32,
    pub parent_id: Option<i32>,
    #[validate(length(min = 1))]
    pub content: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateCommentRequest {
    #[validate(length(min = 1))]
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct CommentResponse {
    pub id: i32,
    pub post_id: i32,
    pub user_id: i32,
    pub parent_id: Option<i32>,
    pub content: String,
    pub upvotes: i32,
    pub downvotes: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl From<CommentModel> for CommentResponse {
    fn from(c: CommentModel) -> Self {
        Self {
            id: c.id,
            post_id: c.post_id,
            user_id: c.user_id,
            parent_id: c.parent_id,
            content: c.content,
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
    pub upvotes: i32,
    pub downvotes: i32,
    pub created_at: String,
    pub updated_at: String,
    pub children: Vec<CommentTreeNode>,
}

impl From<CommentModel> for CommentTreeNode {
    fn from(c: CommentModel) -> Self {
        Self {
            id: c.id,
            post_id: c.post_id,
            user_id: c.user_id,
            parent_id: c.parent_id,
            content: c.content,
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

pub async fn list_comments(
    Extension(db): Extension<DatabaseConnection>,
    Path(post_id): Path<i32>,
) -> AppResult<impl IntoResponse> {
    let service = CommentService::new(db);
    let comments = service.list_by_post(post_id).await?;
    let tree = build_comment_tree(comments);
    Ok(ApiResponse::ok(tree))
}

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
}
