mod common;

use serde_json::Value;

#[tokio::test]
async fn register_and_login() {
    let app = common::spawn_app().await;

    // Register
    let resp = app
        .client
        .post(app.url("/auth/register"))
        .json(&serde_json::json!({
            "username": "alice",
            "email": "alice@example.com",
            "password": "password_123"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    assert!(body["data"]["token"].as_str().is_some());
    let token = body["data"]["token"].as_str().unwrap();

    // Login
    let resp = app
        .client
        .post(app.url("/auth/login"))
        .json(&serde_json::json!({
            "username": "alice",
            "password": "password_123"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    // Get current user
    let resp = app
        .client
        .get(app.url("/auth/me"))
        .bearer_auth(token)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["data"]["username"], "alice");
}

#[tokio::test]
async fn register_duplicate_email_fails() {
    let app = common::spawn_app().await;

    // Register first user
    let resp = app
        .client
        .post(app.url("/auth/register"))
        .json(&serde_json::json!({
            "username": "bob",
            "email": "bob@example.com",
            "password": "password_123"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // Register duplicate
    let resp = app
        .client
        .post(app.url("/auth/register"))
        .json(&serde_json::json!({
            "username": "bob2",
            "email": "bob@example.com",
            "password": "password_123"
        }))
        .send()
        .await
        .unwrap();
    // Should fail with conflict status (409) or validation error (400)
    assert!(resp.status() == 400 || resp.status() == 409);
    let body: Value = resp.json().await.unwrap();
    // Error response should contain an "error" field
    assert!(body["error"].is_string());
}

#[tokio::test]
async fn login_wrong_password_fails() {
    let app = common::spawn_app().await;

    common::create_test_user(&app, "charlie").await;

    let resp = app
        .client
        .post(app.url("/auth/login"))
        .json(&serde_json::json!({
            "username": "charlie",
            "password": "wrong_password"
        }))
        .send()
        .await
        .unwrap();
    // Should fail with unauthorized status (401)
    assert_eq!(resp.status(), 401);
    let body: Value = resp.json().await.unwrap();
    // Error response should contain an "error" field
    assert!(body["error"].is_string());
}

#[tokio::test]
async fn change_password() {
    let app = common::spawn_app().await;
    let (_user_id, token) = common::create_test_user(&app, "dave").await;

    let resp = app
        .client
        .put(app.url("/auth/password"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "current_password": "test_password_123",
            "new_password": "new_password_456"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // Login with new password
    // Note: Username now has counter suffix, get it from /me first
    let resp = app
        .client
        .get(app.url("/auth/me"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    let actual_username = body["data"]["username"].as_str().unwrap();

    let resp = app
        .client
        .post(app.url("/auth/login"))
        .json(&serde_json::json!({
            "username": actual_username,
            "password": "new_password_456"
        }))
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
}
