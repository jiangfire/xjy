mod common;

use serde_json::Value;

async fn setup(app: &common::TestApp) -> (String, i64) {
    let (user_id, token) = common::create_test_user(app, "commentuser").await;
    common::make_admin(&app.db, user_id).await;
    let slug = common::create_test_forum(app, &token).await;

    let resp = app
        .client
        .get(app.url(&format!("/forums/{}", slug)))
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    let forum_id = body["data"]["id"].as_i64().unwrap();

    // Create a post
    let resp = app
        .client
        .post(app.url("/posts"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "forum_id": forum_id,
            "title": "Comment Test Post",
            "content": "Content"
        }))
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    let post_id = body["data"]["id"].as_i64().unwrap();

    (token, post_id)
}

#[tokio::test]
async fn create_and_list_comments() {
    let app = common::spawn_app().await;
    let (token, post_id) = setup(&app).await;

    // Create comment
    let resp = app
        .client
        .post(app.url("/comments"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "post_id": post_id,
            "content": "Great post!"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["data"]["content"], "Great post!");

    // List comments (tree)
    let resp = app
        .client
        .get(app.url(&format!("/posts/{}/comments", post_id)))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    let tree = body["data"].as_array().unwrap();
    assert_eq!(tree.len(), 1);
}

#[tokio::test]
async fn nested_comments() {
    let app = common::spawn_app().await;
    let (token, post_id) = setup(&app).await;

    // Root comment
    let resp = app
        .client
        .post(app.url("/comments"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "post_id": post_id,
            "content": "Root"
        }))
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    let root_id = body["data"]["id"].as_i64().unwrap();

    // Reply
    let resp = app
        .client
        .post(app.url("/comments"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "post_id": post_id,
            "parent_id": root_id,
            "content": "Reply"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // List should show nested tree
    let resp = app
        .client
        .get(app.url(&format!("/posts/{}/comments", post_id)))
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    let tree = body["data"].as_array().unwrap();
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0]["children"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn update_and_delete_comment() {
    let app = common::spawn_app().await;
    let (token, post_id) = setup(&app).await;

    // Create
    let resp = app
        .client
        .post(app.url("/comments"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "post_id": post_id,
            "content": "Original"
        }))
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    let comment_id = body["data"]["id"].as_i64().unwrap();

    // Update
    let resp = app
        .client
        .put(app.url(&format!("/comments/{}", comment_id)))
        .bearer_auth(&token)
        .json(&serde_json::json!({ "content": "Edited" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["data"]["content"], "Edited");

    // Delete
    let resp = app
        .client
        .delete(app.url(&format!("/comments/{}", comment_id)))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}
