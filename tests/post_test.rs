mod common;

use serde_json::Value;

async fn setup_forum(app: &common::TestApp) -> (String, i32, String) {
    let (user_id, token) = common::create_test_user(app, "postuser").await;
    common::make_admin(&app.db, user_id).await;
    let slug = common::create_test_forum(app, &token).await;
    (token, user_id, slug)
}

#[tokio::test]
async fn create_and_get_post() {
    let app = common::spawn_app().await;
    let (token, _user_id, slug) = setup_forum(&app).await;

    // Get forum_id
    let resp = app
        .client
        .get(app.url(&format!("/forums/{}", slug)))
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
            "title": "Test Post",
            "content": "Hello, world!",
            "tags": ["rust", "axum"]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    let post_id = body["data"]["id"].as_i64().unwrap();
    assert_eq!(body["data"]["title"], "Test Post");
    assert_eq!(body["data"]["tags"].as_array().unwrap().len(), 2);

    // Get post
    let resp = app
        .client
        .get(app.url(&format!("/posts/{}", post_id)))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["data"]["title"], "Test Post");
    assert!(body["data"]["tags"].as_array().unwrap().len() >= 2);
}

#[tokio::test]
async fn list_posts_with_sorting() {
    let app = common::spawn_app().await;
    let (token, _user_id, slug) = setup_forum(&app).await;

    let resp = app
        .client
        .get(app.url(&format!("/forums/{}", slug)))
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    let forum_id = body["data"]["id"].as_i64().unwrap();

    // Create two posts
    for i in 1..=2 {
        app.client
            .post(app.url("/posts"))
            .bearer_auth(&token)
            .json(&serde_json::json!({
                "forum_id": forum_id,
                "title": format!("Post {}", i),
                "content": format!("Content {}", i)
            }))
            .send()
            .await
            .unwrap();
    }

    // List with sort=new
    let resp = app
        .client
        .get(app.url(&format!("/forums/{}/posts?sort=new", forum_id)))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    let items = body["data"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);

    // List with sort=top
    let resp = app
        .client
        .get(app.url(&format!("/forums/{}/posts?sort=top", forum_id)))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // List with sort=hot
    let resp = app
        .client
        .get(app.url(&format!("/forums/{}/posts?sort=hot", forum_id)))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn update_and_delete_post() {
    let app = common::spawn_app().await;
    let (token, _user_id, slug) = setup_forum(&app).await;

    let resp = app
        .client
        .get(app.url(&format!("/forums/{}", slug)))
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
            "title": "Original",
            "content": "Original content"
        }))
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    let post_id = body["data"]["id"].as_i64().unwrap();

    // Update
    let resp = app
        .client
        .put(app.url(&format!("/posts/{}", post_id)))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "title": "Updated",
            "content": "Updated content"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["data"]["title"], "Updated");

    // Delete
    let resp = app
        .client
        .delete(app.url(&format!("/posts/{}", post_id)))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn wrong_user_cannot_update_post() {
    let app = common::spawn_app().await;
    let (token, _user_id, slug) = setup_forum(&app).await;

    let resp = app
        .client
        .get(app.url(&format!("/forums/{}", slug)))
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    let forum_id = body["data"]["id"].as_i64().unwrap();

    // Create post as first user
    let resp = app
        .client
        .post(app.url("/posts"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "forum_id": forum_id,
            "title": "Owner Post",
            "content": "Content"
        }))
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    let post_id = body["data"]["id"].as_i64().unwrap();

    // Create second user
    let (_user_id2, token2) = common::create_test_user(&app, "other_user").await;

    // Try to update as wrong user
    let resp = app
        .client
        .put(app.url(&format!("/posts/{}", post_id)))
        .bearer_auth(&token2)
        .json(&serde_json::json!({
            "title": "Hacked",
            "content": "Hacked content"
        }))
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_client_error());
}
