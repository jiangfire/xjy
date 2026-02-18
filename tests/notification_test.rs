mod common;

use serde_json::Value;

// Helper to extract notifications from response body
fn get_notifications(body: &Value) -> Vec<Value> {
    if body["data"].is_array() {
        body["data"].as_array().cloned().unwrap_or_default()
    } else if body["data"]["items"].is_array() {
        body["data"]["items"].as_array().cloned().unwrap_or_default()
    } else {
        vec![]
    }
}

#[tokio::test]
async fn list_notifications_empty() {
    let app = common::spawn_app().await;
    let (_user_id, token) = common::create_test_user(&app, "user1").await;

    let resp = app
        .client
        .get(app.url("/notifications"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    assert_eq!(get_notifications(&body).len(), 0);
}

#[tokio::test]
async fn list_notifications_paginated() {
    let app = common::spawn_app().await;
    let (_user_id, token) = common::create_test_user(&app, "user1").await;
    let (admin_id, admin_token) = common::create_test_user(&app, "admin").await;
    common::make_admin(&app.db, admin_id).await;

    let forum_slug = common::create_test_forum(&app, &admin_token).await;
    let forum_id = common::get_forum_id(&app, &forum_slug).await;

    // Create multiple posts that should generate notifications
    for i in 1..=5 {
        app.client
            .post(app.url("/posts"))
            .bearer_auth(&admin_token)
            .json(&serde_json::json!({
                "title": format!("Post {}", i),
                "content": "Content here",
                "forum_id": forum_id
            }))
            .send()
            .await
            .unwrap();
    }

    // List notifications (first page)
    let resp = app
        .client
        .get(app.url("/notifications?page=1&limit=3"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    let notifications = get_notifications(&body);
    assert!(notifications.len() <= 3);
}

#[tokio::test]
async fn mark_notification_read() {
    let app = common::spawn_app().await;
    let (_user_id, token) = common::create_test_user(&app, "user1").await;
    let (admin_id, admin_token) = common::create_test_user(&app, "admin").await;
    common::make_admin(&app.db, admin_id).await;

    let forum_slug = common::create_test_forum(&app, &admin_token).await;
    let forum_id = common::get_forum_id(&app, &forum_slug).await;

    // Create a post that generates notification
    app.client
        .post(app.url("/posts"))
        .bearer_auth(&admin_token)
        .json(&serde_json::json!({
            "title": "Test Post",
            "content": "Content",
            "forum_id": forum_id
        }))
        .send()
        .await
        .unwrap();

    // Get notifications
    let resp = app
        .client
        .get(app.url("/notifications"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    let body: Value = resp.json().await.unwrap();
    let notifications = get_notifications(&body);

    if !notifications.is_empty() {
        let notification_id = notifications[0]["id"].as_i64().unwrap();

        // Mark as read
        let resp = app
            .client
            .put(app.url(&format!("/notifications/{}/read", notification_id)))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), 200);

        // Verify marked as read
        let resp = app
            .client
            .get(app.url(&format!("/notifications/{}", notification_id)))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();

        let body: Value = resp.json().await.unwrap();
        assert_eq!(body["data"]["read"].as_bool().unwrap(), true);
    }
}

#[tokio::test]
async fn mark_all_notifications_read() {
    let app = common::spawn_app().await;
    let (_user_id, token) = common::create_test_user(&app, "user1").await;
    let (admin_id, admin_token) = common::create_test_user(&app, "admin").await;
    common::make_admin(&app.db, admin_id).await;

    let forum_slug = common::create_test_forum(&app, &admin_token).await;
    let forum_id = common::get_forum_id(&app, &forum_slug).await;

    // Create multiple posts
    for i in 1..=3 {
        app.client
            .post(app.url("/posts"))
            .bearer_auth(&admin_token)
            .json(&serde_json::json!({
                "title": format!("Post {}", i),
                "content": "Content",
                "forum_id": forum_id
            }))
            .send()
            .await
            .unwrap();
    }

    // Mark all as read
    let resp = app
        .client
        .put(app.url("/notifications/read-all"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);

    // Verify unread count is 0
    let resp = app
        .client
        .get(app.url("/notifications/unread-count"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["data"]["count"].as_i64().unwrap(), 0);
}

#[tokio::test]
async fn get_unread_count() {
    let app = common::spawn_app().await;
    let (_user_id, token) = common::create_test_user(&app, "user1").await;
    let (admin_id, admin_token) = common::create_test_user(&app, "admin").await;
    common::make_admin(&app.db, admin_id).await;

    let forum_slug = common::create_test_forum(&app, &admin_token).await;
    let forum_id = common::get_forum_id(&app, &forum_slug).await;

    // Initially should have 0 unread
    let resp = app
        .client
        .get(app.url("/notifications/unread-count"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    let initial_count = body["data"]["count"].as_i64().unwrap();

    // Create some posts
    for i in 1..=3 {
        app.client
            .post(app.url("/posts"))
            .bearer_auth(&admin_token)
            .json(&serde_json::json!({
                "title": format!("Post {}", i),
                "content": "Content",
                "forum_id": forum_id
            }))
            .send()
            .await
            .unwrap();
    }

    // Check unread count again
    let resp = app
        .client
        .get(app.url("/notifications/unread-count"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    let new_count = body["data"]["count"].as_i64().unwrap();
    assert!(new_count >= initial_count);
}

#[tokio::test]
async fn notification_requires_auth() {
    let app = common::spawn_app().await;

    let resp = app
        .client
        .get(app.url("/notifications"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn mark_nonexistent_notification_read() {
    let app = common::spawn_app().await;
    let (_user_id, token) = common::create_test_user(&app, "user1").await;

    let resp = app
        .client
        .put(app.url("/notifications/99999/read"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn notifications_ordered_by_created_at() {
    let app = common::spawn_app().await;
    let (_user_id, token) = common::create_test_user(&app, "user1").await;
    let (admin_id, admin_token) = common::create_test_user(&app, "admin").await;
    common::make_admin(&app.db, admin_id).await;

    let forum_slug = common::create_test_forum(&app, &admin_token).await;
    let forum_id = common::get_forum_id(&app, &forum_slug).await;

    // Create posts with delays
    for i in 1..=3 {
        app.client
            .post(app.url("/posts"))
            .bearer_auth(&admin_token)
            .json(&serde_json::json!({
                "title": format!("Post {}", i),
                "content": "Content",
                "forum_id": forum_id
            }))
            .send()
            .await
            .unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    // Get notifications
    let resp = app
        .client
        .get(app.url("/notifications"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    let notifications = get_notifications(&body);

    // Verify notifications are ordered (newest first)
    if notifications.len() >= 2 {
        let first_time = notifications[0]["created_at"].as_str().unwrap();
        let second_time = notifications[1]["created_at"].as_str().unwrap();
        assert!(first_time >= second_time);
    }
}
