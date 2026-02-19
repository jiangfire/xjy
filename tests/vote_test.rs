mod common;

use serde_json::Value;

fn find_nonce(pow_token: &str) -> String {
    let secret = std::env::var("POW_SECRET").unwrap().into_bytes();
    let ch = xjy::utils::pow::verify_and_decode_challenge(&secret, pow_token).unwrap();
    for i in 0u64..2_000_000 {
        let nonce = format!("{i}");
        if xjy::utils::pow::validate_pow_solution(&ch, &nonce).is_ok() {
            return nonce;
        }
    }
    panic!("nonce not found");
}

async fn vote_post_with_pow(
    app: &common::TestApp,
    token: &str,
    post_id: i64,
    value: i16,
) -> reqwest::Response {
    let ch = app
        .client
        .post(app.url("/pow/challenge"))
        .bearer_auth(token)
        .json(&serde_json::json!({
            "action": "vote",
            "target_type": "post",
            "target_id": post_id
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(ch.status(), 200);
    let ch_body: Value = ch.json().await.unwrap();
    let pow_token = ch_body["data"]["pow_token"].as_str().unwrap();
    let pow_nonce = find_nonce(pow_token);

    app.client
        .post(app.url(&format!("/posts/{}/vote", post_id)))
        .bearer_auth(token)
        .json(&serde_json::json!({
            "value": value,
            "pow_token": pow_token,
            "pow_nonce": pow_nonce
        }))
        .send()
        .await
        .unwrap()
}

async fn setup(app: &common::TestApp) -> (String, i64) {
    let (user_id, token) = common::create_test_user(app, "voteuser").await;
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

    let resp = vote_post_with_pow(&app, &token, post_id, 1).await;
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["data"]["value"], 1);
}

#[tokio::test]
async fn downvote_post() {
    let app = common::spawn_app().await;
    let (token, post_id) = setup(&app).await;

    let resp = vote_post_with_pow(&app, &token, post_id, -1).await;
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["data"]["value"], -1);
}

#[tokio::test]
async fn toggle_vote_off() {
    let app = common::spawn_app().await;
    let (token, post_id) = setup(&app).await;

    // Upvote
    let _ = vote_post_with_pow(&app, &token, post_id, 1).await;

    // Vote same value again -> toggle off
    let resp = vote_post_with_pow(&app, &token, post_id, 1).await;
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["data"]["value"], 0);
}

#[tokio::test]
async fn swing_vote() {
    let app = common::spawn_app().await;
    let (token, post_id) = setup(&app).await;

    // Upvote
    let _ = vote_post_with_pow(&app, &token, post_id, 1).await;

    // Swing to downvote
    let resp = vote_post_with_pow(&app, &token, post_id, -1).await;
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["data"]["value"], -1);
}
