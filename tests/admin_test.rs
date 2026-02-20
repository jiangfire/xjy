mod common;

use serde_json::Value;

#[tokio::test]
async fn get_platform_stats() {
    let app = common::spawn_app().await;
    let (admin_id, admin_token) = common::create_test_user(&app, "admin").await;
    common::make_admin(&app.db, admin_id).await;

    let resp = app
        .client
        .get(app.url("/admin/stats"))
        .bearer_auth(&admin_token)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    // API returns total_users, total_posts, total_comments, etc.
    assert!(body["data"]["total_users"].is_number());
    assert!(body["data"]["total_posts"].is_number());
    assert!(body["data"]["total_comments"].is_number());
}

#[tokio::test]
async fn get_stats_as_regular_user_fails() {
    let app = common::spawn_app().await;
    let (_user_id, user_token) = common::create_test_user(&app, "regularuser").await;

    let resp = app
        .client
        .get(app.url("/admin/stats"))
        .bearer_auth(&user_token)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 403);
}

#[tokio::test]
async fn list_all_users() {
    let app = common::spawn_app().await;
    let (admin_id, admin_token) = common::create_test_user(&app, "admin").await;
    common::make_admin(&app.db, admin_id).await;

    // Create multiple users
    for i in 1..=5 {
        common::create_test_user(&app, &format!("user{}", i)).await;
    }

    let resp = app
        .client
        .get(app.url("/admin/users?page=1&limit=10"))
        .bearer_auth(&admin_token)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    // Paginated response has structure: { data: { items: [...], total, page, per_page } }
    let users = body["data"]["items"]
        .as_array()
        .expect("Expected items array in paginated response");
    assert!(users.len() >= 5);
}

#[tokio::test]
async fn list_users_pagination() {
    let app = common::spawn_app().await;
    let (admin_id, admin_token) = common::create_test_user(&app, "admin").await;
    common::make_admin(&app.db, admin_id).await;

    // Create users (reduced to avoid rate limiting)
    for i in 1..=10 {
        common::create_test_user(&app, &format!("user{}", i)).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
    }

    // First page
    let resp = app
        .client
        .get(app.url("/admin/users?page=1&per_page=5"))
        .bearer_auth(&admin_token)
        .send()
        .await
        .unwrap();

    let body: Value = resp.json().await.unwrap();
    let page1 = body["data"]["items"]
        .as_array()
        .expect("Expected items in page 1");
    assert_eq!(page1.len(), 5);

    // Second page
    let resp = app
        .client
        .get(app.url("/admin/users?page=2&per_page=5"))
        .bearer_auth(&admin_token)
        .send()
        .await
        .unwrap();

    let body: Value = resp.json().await.unwrap();
    let page2 = body["data"]["items"]
        .as_array()
        .expect("Expected items in page 2");
    assert!(page2.len() > 0);
}

#[tokio::test]
async fn update_user_role_promote_to_admin() {
    let app = common::spawn_app().await;
    let (admin_id, admin_token) = common::create_test_user(&app, "admin").await;
    common::make_admin(&app.db, admin_id).await;

    let (user_id, _user_token) = common::create_test_user(&app, "regularuser").await;

    let resp = app
        .client
        .put(app.url(&format!("/admin/users/{}/role", user_id)))
        .bearer_auth(&admin_token)
        .json(&serde_json::json!({
            "role": "admin"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    assert_eq!(body["data"]["role"], "admin");
}

#[tokio::test]
async fn update_user_role_demote_to_user() {
    let app = common::spawn_app().await;
    let (admin_id, admin_token) = common::create_test_user(&app, "admin").await;
    common::make_admin(&app.db, admin_id).await;

    let (user_id, _user_token) = common::create_test_user(&app, "otheradmin").await;
    common::make_admin(&app.db, user_id).await;

    // Demote to regular user
    let resp = app
        .client
        .put(app.url(&format!("/admin/users/{}/role", user_id)))
        .bearer_auth(&admin_token)
        .json(&serde_json::json!({
            "role": "user"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["data"]["role"], "user");
}

#[tokio::test]
async fn update_user_role_as_regular_user_fails() {
    let app = common::spawn_app().await;
    let (admin_id, _admin_token) = common::create_test_user(&app, "admin").await;
    common::make_admin(&app.db, admin_id).await;

    let (target_id, _target_token) = common::create_test_user(&app, "target").await;
    let (_attacker_id, attacker_token) = common::create_test_user(&app, "attacker").await;

    let resp = app
        .client
        .put(app.url(&format!("/admin/users/{}/role", target_id)))
        .bearer_auth(&attacker_token)
        .json(&serde_json::json!({
            "role": "admin"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 403);
}

#[tokio::test]
async fn admin_delete_post() {
    let app = common::spawn_app().await;
    let (admin_id, admin_token) = common::create_test_user(&app, "admin").await;
    common::make_admin(&app.db, admin_id).await;

    let (_user_id, user_token) = common::create_test_user(&app, "user").await;
    let forum_slug = common::create_test_forum(&app, &admin_token).await;
    let forum_id = common::get_forum_id(&app, &forum_slug).await;

    // Create a post
    let resp = app
        .client
        .post(app.url("/posts"))
        .bearer_auth(&user_token)
        .json(&serde_json::json!({
            "title": "Test Post",
            "content": "This should be deleted",
            "forum_id": forum_id
        }))
        .send()
        .await
        .unwrap();

    let body: Value = resp.json().await.unwrap();
    let post_id = body["data"]["id"].as_i64().unwrap();

    // Admin deletes post
    let resp = app
        .client
        .delete(app.url(&format!("/admin/posts/{}", post_id)))
        .bearer_auth(&admin_token)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);

    // Verify post deleted
    let resp = app
        .client
        .get(app.url(&format!("/posts/{}", post_id)))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn admin_delete_comment() {
    let app = common::spawn_app().await;
    let (admin_id, admin_token) = common::create_test_user(&app, "admin").await;
    common::make_admin(&app.db, admin_id).await;

    let (_user_id, user_token) = common::create_test_user(&app, "user").await;
    let forum_slug = common::create_test_forum(&app, &admin_token).await;
    let forum_id = common::get_forum_id(&app, &forum_slug).await;

    // Create post and comment
    let resp = app
        .client
        .post(app.url("/posts"))
        .bearer_auth(&user_token)
        .json(&serde_json::json!({
            "title": "Test Post",
            "content": "Post content",
            "forum_id": forum_id
        }))
        .send()
        .await
        .unwrap();

    let body: Value = resp.json().await.unwrap();
    let post_id = body["data"]["id"].as_i64().unwrap();

    let resp = app
        .client
        .post(app.url("/comments"))
        .bearer_auth(&user_token)
        .json(&serde_json::json!({
            "post_id": post_id,
            "content": "This comment should be deleted"
        }))
        .send()
        .await
        .unwrap();

    let body: Value = resp.json().await.unwrap();
    let comment_id = body["data"]["id"].as_i64().unwrap();

    // Admin deletes comment
    let resp = app
        .client
        .delete(app.url(&format!("/admin/comments/{}", comment_id)))
        .bearer_auth(&admin_token)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);

    // Verify comment deleted
    let resp = app
        .client
        .get(app.url(&format!("/posts/{}/comments", post_id)))
        .send()
        .await
        .unwrap();

    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["data"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn admin_delete_nonexistent_post_returns_404() {
    let app = common::spawn_app().await;
    let (admin_id, admin_token) = common::create_test_user(&app, "admin").await;
    common::make_admin(&app.db, admin_id).await;

    let resp = app
        .client
        .delete(app.url("/admin/posts/99999"))
        .bearer_auth(&admin_token)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn admin_delete_nonexistent_comment_returns_404() {
    let app = common::spawn_app().await;
    let (admin_id, admin_token) = common::create_test_user(&app, "admin").await;
    common::make_admin(&app.db, admin_id).await;

    let resp = app
        .client
        .delete(app.url("/admin/comments/99999"))
        .bearer_auth(&admin_token)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn admin_requires_auth() {
    let app = common::spawn_app().await;

    let resp = app
        .client
        .get(app.url("/admin/stats"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn update_nonexistent_user_role_returns_404() {
    let app = common::spawn_app().await;
    let (admin_id, admin_token) = common::create_test_user(&app, "admin").await;
    common::make_admin(&app.db, admin_id).await;

    let resp = app
        .client
        .put(app.url("/admin/users/99999/role"))
        .bearer_auth(&admin_token)
        .json(&serde_json::json!({
            "role": "admin"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 404);
}
