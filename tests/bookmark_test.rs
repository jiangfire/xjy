mod common;

use serde_json::Value;

#[tokio::test]
async fn toggle_bookmark() {
    let app = common::spawn_app().await;
    let (user_id, token) = common::create_test_user(&app, "bookmarkuser").await;
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

    // Create post
    let resp = app
        .client
        .post(app.url("/posts"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "forum_id": forum_id,
            "title": "Bookmark Test",
            "content": "Content"
        }))
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    let post_id = body["data"]["id"].as_i64().unwrap();

    // Bookmark on
    let resp = app
        .client
        .post(app.url(&format!("/posts/{}/bookmark", post_id)))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["data"], "Bookmarked");

    // List bookmarks
    let resp = app
        .client
        .get(app.url("/bookmarks"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    let items = body["data"].as_array().unwrap();
    assert_eq!(items.len(), 1);

    // Bookmark off (toggle)
    let resp = app
        .client
        .post(app.url(&format!("/posts/{}/bookmark", post_id)))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["data"], "Unbookmarked");
}
