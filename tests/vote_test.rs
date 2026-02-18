mod common;

use serde_json::Value;

async fn setup(app: &common::TestApp) -> (String, i64) {
    let (user_id, token) = common::create_test_user(app, "voteuser").await;
    common::make_admin(&app.db, user_id).await;
    common::create_test_forum(app, &token).await;

    let resp = app
        .client
        .get(app.url("/forums/test-forum"))
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    let forum_id = body["data"]["id"].as_i64().unwrap();

    let resp = app
        .client
        .post(app.url("/posts"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "forum_id": forum_id,
            "title": "Vote Test",
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
async fn upvote_post() {
    let app = common::spawn_app().await;
    let (token, post_id) = setup(&app).await;

    let resp = app
        .client
        .post(app.url(&format!("/posts/{}/vote", post_id)))
        .bearer_auth(&token)
        .json(&serde_json::json!({ "value": 1 }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["data"]["value"], 1);
}

#[tokio::test]
async fn downvote_post() {
    let app = common::spawn_app().await;
    let (token, post_id) = setup(&app).await;

    let resp = app
        .client
        .post(app.url(&format!("/posts/{}/vote", post_id)))
        .bearer_auth(&token)
        .json(&serde_json::json!({ "value": -1 }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["data"]["value"], -1);
}

#[tokio::test]
async fn toggle_vote_off() {
    let app = common::spawn_app().await;
    let (token, post_id) = setup(&app).await;

    // Upvote
    app.client
        .post(app.url(&format!("/posts/{}/vote", post_id)))
        .bearer_auth(&token)
        .json(&serde_json::json!({ "value": 1 }))
        .send()
        .await
        .unwrap();

    // Vote same value again -> toggle off
    let resp = app
        .client
        .post(app.url(&format!("/posts/{}/vote", post_id)))
        .bearer_auth(&token)
        .json(&serde_json::json!({ "value": 1 }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["data"]["value"], 0);
}

#[tokio::test]
async fn swing_vote() {
    let app = common::spawn_app().await;
    let (token, post_id) = setup(&app).await;

    // Upvote
    app.client
        .post(app.url(&format!("/posts/{}/vote", post_id)))
        .bearer_auth(&token)
        .json(&serde_json::json!({ "value": 1 }))
        .send()
        .await
        .unwrap();

    // Swing to downvote
    let resp = app
        .client
        .post(app.url(&format!("/posts/{}/vote", post_id)))
        .bearer_auth(&token)
        .json(&serde_json::json!({ "value": -1 }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["data"]["value"], -1);
}
