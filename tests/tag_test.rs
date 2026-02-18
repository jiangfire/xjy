mod common;

use serde_json::Value;

#[tokio::test]
async fn create_post_with_tags_and_list_tags() {
    let app = common::spawn_app().await;
    let (user_id, token) = common::create_test_user(&app, "taguser").await;
    common::make_admin(&app.db, user_id).await;
    common::create_test_forum(&app, &token).await;

    let resp = app
        .client
        .get(app.url("/forums/test-forum"))
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    let forum_id = body["data"]["id"].as_i64().unwrap();

    // Create post with tags
    let resp = app
        .client
        .post(app.url("/posts"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "forum_id": forum_id,
            "title": "Tagged Post",
            "content": "Content",
            "tags": ["rust", "web", "api"]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    let tags = body["data"]["tags"].as_array().unwrap();
    assert_eq!(tags.len(), 3);

    // List all tags
    let resp = app
        .client
        .get(app.url("/tags"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    let tags = body["data"].as_array().unwrap();
    assert!(tags.len() >= 3);

    // Filter by tag
    let resp = app
        .client
        .get(app.url("/tags/rust/posts"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    let items = body["data"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
}

#[tokio::test]
async fn too_many_tags_rejected() {
    let app = common::spawn_app().await;
    let (user_id, token) = common::create_test_user(&app, "taguser2").await;
    common::make_admin(&app.db, user_id).await;
    common::create_test_forum(&app, &token).await;

    let resp = app
        .client
        .get(app.url("/forums/test-forum"))
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    let forum_id = body["data"]["id"].as_i64().unwrap();

    // Try creating post with 6 tags (max is 5)
    let resp = app
        .client
        .post(app.url("/posts"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "forum_id": forum_id,
            "title": "Too Many Tags",
            "content": "Content",
            "tags": ["a", "b", "c", "d", "e", "f"]
        }))
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    assert!(!body["success"].as_bool().unwrap_or(true));
}
