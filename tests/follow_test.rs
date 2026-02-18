mod common;

use serde_json::Value;

#[tokio::test]
async fn toggle_follow() {
    let app = common::spawn_app().await;
    let (_user_id, token) = common::create_test_user(&app, "follower").await;
    let (target_id, _) = common::create_test_user(&app, "target").await;

    // Follow
    let resp = app
        .client
        .post(app.url(&format!("/users/{}/follow", target_id)))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["data"], "Followed");

    // Check followers
    let resp = app
        .client
        .get(app.url(&format!("/users/{}/followers", target_id)))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    let followers = body["data"].as_array().unwrap();
    assert_eq!(followers.len(), 1);

    // Unfollow (toggle)
    let resp = app
        .client
        .post(app.url(&format!("/users/{}/follow", target_id)))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["data"], "Unfollowed");
}

#[tokio::test]
async fn self_follow_error() {
    let app = common::spawn_app().await;
    let (user_id, token) = common::create_test_user(&app, "selffollow").await;

    let resp = app
        .client
        .post(app.url(&format!("/users/{}/follow", user_id)))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    assert!(!body["success"].as_bool().unwrap_or(true));
}
