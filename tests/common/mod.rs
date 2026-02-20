#![allow(dead_code)]

use reqwest::Client;
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use sea_orm_migration::MigratorTrait;
use std::net::SocketAddr;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Once,
};

static INIT: Once = Once::new();
static MIGRATIONS_RAN: AtomicBool = AtomicBool::new(false);
static FORUM_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn init_env() {
    INIT.call_once(|| {
        dotenv::dotenv().ok();
        std::env::set_var(
            "JWT_SECRET",
            "integration_test_secret_that_is_at_least_32_characters_long",
        );
        // PoW: keep low for integration tests and set secret
        std::env::set_var("POW_SECRET", "integration_test_pow_secret");
        std::env::set_var("POW_TTL_SECONDS", "300");
        std::env::set_var("POW_DIFFICULTY", "8");
        let config = xjy::config::jwt::JwtConfig::from_env().unwrap();
        let _ = xjy::utils::jwt::init_jwt_config(config);
    });
}

pub struct TestApp {
    pub addr: String,
    pub db: DatabaseConnection,
    pub client: Client,
}

impl TestApp {
    pub fn url(&self, path: &str) -> String {
        format!("{}/api/v1{}", self.addr, path)
    }
}

pub async fn spawn_app() -> TestApp {
    init_env();

    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"));

    let db = sea_orm::Database::connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    // Run migrations only once globally (using atomic bool for thread safety)
    if !MIGRATIONS_RAN.swap(true, Ordering::SeqCst) {
        // Migrations haven't run yet, run them now
        xjy::migration::Migrator::up(&db, None)
            .await
            .expect("Failed to run migrations");
    }

    // Clean data tables (reverse dependency order)
    cleanup_tables(&db).await;

    let hub = xjy::websocket::hub::NotificationHub::new();
    let upload_config = xjy::services::upload::UploadConfig {
        upload_dir: "./test_uploads".to_string(),
    };
    let email_service = xjy::services::email::EmailService::from_env();

    let app = axum::Router::new()
        .route("/", axum::routing::get(|| async { "ok" }))
        .merge(xjy::routes::create_routes())
        .layer(axum::middleware::from_fn(
            xjy::middleware::security::security_headers_middleware,
        ))
        .layer(axum::extract::Extension(db.clone()))
        .layer(axum::extract::Extension(hub))
        .layer(axum::extract::Extension(upload_config))
        .layer(axum::extract::Extension(email_service));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind random port");
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });

    let addr_str = format!("http://{}", addr);
    let client = Client::new();

    TestApp {
        addr: addr_str,
        db,
        client,
    }
}

async fn cleanup_tables(db: &DatabaseConnection) {
    let tables = [
        "refresh_tokens",
        "post_tags",
        "tags",
        "bookmarks",
        "follows",
        "votes",
        "notifications",
        "reports",
        "comments",
        "posts",
        "forums",
        "users",
    ];

    for table in tables {
        let sql = format!("TRUNCATE TABLE {} CASCADE", table);
        let _ = db
            .execute(Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                sql,
            ))
            .await;
    }
}

/// Register a user and return (user_id, token).
pub async fn create_test_user(app: &TestApp, username_prefix: &str) -> (i32, String) {
    use std::sync::atomic::{AtomicUsize, Ordering};

    static USER_COUNTER: AtomicUsize = AtomicUsize::new(0);
    let counter = USER_COUNTER.fetch_add(1, Ordering::SeqCst);
    let unique_username = format!("{}_{}", username_prefix, counter);

    let resp = app
        .client
        .post(app.url("/auth/register"))
        .json(&serde_json::json!({
            "username": unique_username,
            "email": format!("{}@test.com", unique_username),
            "password": "test_password_123"
        }))
        .send()
        .await
        .expect("Failed to register user");

    let status = resp.status();
    let body: serde_json::Value = resp.json().await.unwrap_or_else(|e| {
        panic!(
            "Failed to parse register response for user '{}': status={}, error={}",
            unique_username, status, e
        );
    });

    if !body["success"].as_bool().unwrap_or(false) {
        panic!(
            "Failed to register user '{}': status={}, body={}",
            unique_username, status, body
        );
    }

    let user_id = body["data"]["user_id"].as_i64().expect(&format!(
        "Response missing user_id for user '{}': {:?}",
        unique_username, body
    )) as i32;
    let token = body["data"]["token"]
        .as_str()
        .expect(&format!(
            "Response missing token for user '{}': {:?}",
            unique_username, body
        ))
        .to_string();
    (user_id, token)
}

/// Create a forum and return its slug.
pub async fn create_test_forum(app: &TestApp, admin_token: &str) -> String {
    let counter = FORUM_COUNTER.fetch_add(1, Ordering::SeqCst);
    let slug = format!("test-forum-{}", counter);

    let resp = app
        .client
        .post(app.url("/forums"))
        .bearer_auth(admin_token)
        .json(&serde_json::json!({
            "name": format!("Test Forum {}", counter),
            "slug": slug,
            "description": "A test forum"
        }))
        .send()
        .await
        .expect("Failed to create forum");

    let status = resp.status();
    let body: serde_json::Value = resp.json().await.expect("Failed to parse response");

    if !body["success"].as_bool().unwrap_or(false) {
        panic!("Failed to create forum: status={}, body={}", status, body);
    }

    body["data"]["slug"]
        .as_str()
        .expect("Response missing slug field")
        .to_string()
}

/// Make a user admin by directly updating the database.
pub async fn make_admin(db: &DatabaseConnection, user_id: i32) {
    db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "UPDATE users SET role = 'admin' WHERE id = $1",
        vec![user_id.into()],
    ))
    .await
    .expect("Failed to make user admin");
}

/// Get forum_id from slug.
pub async fn get_forum_id(app: &TestApp, slug: &str) -> i32 {
    let resp = app
        .client
        .get(app.url(&format!("/forums/{}", slug)))
        .send()
        .await
        .expect("Failed to get forum");

    let body: serde_json::Value = resp.json().await.expect("Failed to parse forum response");
    body["data"]["id"]
        .as_i64()
        .expect("Forum response missing id field") as i32
}
