mod config;
mod error;
mod handlers;
mod middleware;
mod migration;
mod models;
mod response;
mod routes;
mod services;
mod utils;
mod websocket;

use axum::{extract::Extension, response::IntoResponse, routing::get, Json, Router};
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use sea_orm_migration::MigratorTrait;
use serde_json::json;
use services::cache::CacheService;
use services::upload::UploadConfig;
use std::env;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use websocket::hub::NotificationHub;

#[derive(OpenApi)]
#[openapi(
    paths(
        health_check,
        // Auth routes
        crate::handlers::register,
        crate::handlers::login,
        crate::handlers::auth::refresh_token,
        crate::handlers::get_current_user,
        crate::handlers::change_password,
        crate::handlers::verify_email,
        crate::handlers::resend_verification,
        crate::handlers::auth::forgot_password,
        crate::handlers::auth::reset_password,
        crate::handlers::auth::logout,
        // User routes
        crate::handlers::user::get_user_profile,
        crate::handlers::user::update_profile,
        // Forum routes
        crate::handlers::forum::list_forums,
        crate::handlers::forum::get_forum,
        crate::handlers::forum::create_forum,
        crate::handlers::forum::update_forum,
        crate::handlers::forum::delete_forum,
        // Post routes
        crate::handlers::post::list_posts,
        crate::handlers::post::get_post,
        crate::handlers::post::create_post,
        crate::handlers::post::update_post,
        crate::handlers::post::delete_post,
        crate::handlers::post::pin_post,
        crate::handlers::post::lock_post,
        crate::handlers::post::search_posts,
        // Comment routes
        crate::handlers::comment::list_comments,
        crate::handlers::comment::create_comment,
        crate::handlers::comment::update_comment,
        crate::handlers::comment::delete_comment,
        // Tag routes
        crate::handlers::tag::list_tags,
        crate::handlers::tag::get_posts_by_tag,
        crate::handlers::tag::create_tag,
        crate::handlers::tag::update_tag,
        crate::handlers::tag::delete_tag,
        // Vote routes
        crate::handlers::vote::vote_post,
        crate::handlers::vote::vote_comment,
        // Follow routes
        crate::handlers::follow::list_followers,
        crate::handlers::follow::list_following,
        crate::handlers::follow::toggle_follow,
        // Notification routes
        crate::handlers::notification::list_notifications,
        crate::handlers::notification::unread_count,
        crate::handlers::notification::mark_all_read,
        crate::handlers::notification::mark_read,
        // Bookmark routes
        crate::handlers::bookmark::toggle_bookmark,
        crate::handlers::bookmark::list_bookmarks,
        // Upload routes
        crate::handlers::upload::upload_avatar,
        crate::handlers::upload::upload_image,
        // Report routes
        crate::handlers::report::create_report,
        crate::handlers::report::list_reports,
        crate::handlers::report::resolve_report,
        // Admin routes
        crate::handlers::admin::get_stats,
        crate::handlers::admin::list_users,
        crate::handlers::admin::update_user_role,
        crate::handlers::admin::admin_delete_post,
        crate::handlers::admin::admin_delete_comment,
    ),
    components(
        schemas(
            crate::response::ApiResponse<serde_json::Value>,
            crate::response::PaginatedResponse<serde_json::Value>,
            crate::response::PaginationQuery,
            crate::error::AppError,
            // Auth
            crate::handlers::auth::RegisterRequest,
            crate::handlers::auth::LoginRequest,
            crate::handlers::auth::RefreshTokenRequest,
            crate::handlers::auth::AuthResponse,
            crate::handlers::auth::RegisterResponse,
            crate::handlers::auth::TokenResponse,
            crate::handlers::auth::UserResponse,
            crate::handlers::auth::ChangePasswordRequest,
            crate::handlers::auth::VerifyEmailRequest,
            crate::handlers::auth::ForgotPasswordRequest,
            crate::handlers::auth::ResetPasswordRequest,
            // User
            crate::handlers::user::UserProfileResponse,
            crate::handlers::user::UpdateProfileRequest,
            // Forum
            crate::handlers::forum::ForumResponse,
            crate::handlers::forum::CreateForumRequest,
            crate::handlers::forum::UpdateForumRequest,
            // Post
            crate::handlers::post::PostResponse,
            crate::handlers::post::CreatePostRequest,
            crate::handlers::post::UpdatePostRequest,
            crate::handlers::post::PostListQuery,
            crate::handlers::post::SearchPostsQuery,
            // Comment
            crate::handlers::comment::CommentResponse,
            crate::handlers::comment::CommentTreeNode,
            crate::handlers::comment::CreateCommentRequest,
            crate::handlers::comment::UpdateCommentRequest,
            // Tag
            crate::handlers::tag::TagResponse,
            crate::handlers::tag::CreateTagRequest,
            crate::handlers::tag::UpdateTagRequest,
            // Vote
            crate::handlers::vote::VoteRequest,
            crate::handlers::vote::VoteResponse,
            // Follow
            crate::handlers::follow::FollowToggleResponse,
            // Notification
            crate::handlers::notification::NotificationResponse,
            crate::handlers::notification::UnreadCountResponse,
            // Bookmark
            crate::handlers::bookmark::BookmarkToggleResponse,
            // Upload
            crate::handlers::upload::UploadResponse,
            // Report
            crate::handlers::report::ReportResponse,
            crate::handlers::report::CreateReportRequest,
            crate::handlers::report::ResolveReportRequest,
            // Admin
            crate::handlers::admin::StatsResponse,
            crate::handlers::admin::AdminUserResponse,
            crate::handlers::admin::UpdateRoleRequest,
        )
    ),
    tags(
        (name = "auth", description = "Authentication operations"),
        (name = "users", description = "User profile operations"),
        (name = "forums", description = "Forum management operations"),
        (name = "posts", description = "Post management operations"),
        (name = "comments", description = "Comment management operations"),
        (name = "tags", description = "Tag management operations"),
        (name = "votes", description = "Voting operations"),
        (name = "follows", description = "Follow operations"),
        (name = "notifications", description = "Notification operations"),
        (name = "bookmarks", description = "Bookmark operations"),
        (name = "uploads", description = "File upload operations"),
        (name = "reports", description = "Report management operations"),
        (name = "admin", description = "Administrative operations"),
    )
)]
struct ApiDoc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "xjy=debug,tower_http=debug,axum=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Validate configuration before doing anything else
    let jwt_config = validate_config()?;

    // Initialize JWT config
    utils::jwt::init_jwt_config(jwt_config)?;

    tracing::info!("Starting Forum API v{}...", env!("CARGO_PKG_VERSION"));

    let db = config::database::get_database().await?;
    tracing::info!("Database connected successfully");

    migration::Migrator::up(&db, None).await?;
    tracing::info!("Database migrations applied successfully");

    let hub = NotificationHub::new();

    let upload_dir = env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string());
    let upload_config = UploadConfig {
        upload_dir: upload_dir.clone(),
    };

    // Redis/Cache is optional - graceful degradation if unavailable
    let cache = match config::redis::get_redis().await {
        Ok(conn) => {
            tracing::info!("Redis connected successfully");
            Some(CacheService::new(conn))
        }
        Err(e) => {
            tracing::warn!("Redis unavailable, running without cache: {}", e);
            None
        }
    };

    let email_service = services::email::EmailService::from_env();
    if email_service.is_configured() {
        tracing::info!("SMTP email service configured");
    } else {
        tracing::warn!("SMTP not configured, emails will be skipped");
    }

    let mut app = create_app(&upload_dir)
        .layer(Extension(db))
        .layer(Extension(hub))
        .layer(Extension(upload_config))
        .layer(Extension(email_service));

    if let Some(cache) = cache {
        app = app.layer(Extension(cache));
    }

    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("{}:{}", host, port);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Listening on http://{}", addr);
    tracing::info!("Swagger UI available at http://{}/swagger-ui/", addr);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    tracing::info!("Server shut down gracefully");
    Ok(())
}

/// Validate all required configuration at startup (fail-fast).
fn validate_config() -> anyhow::Result<crate::config::jwt::JwtConfig> {
    // JWT config — validated and cached
    let jwt_config = config::jwt::JwtConfig::from_env()?;

    // DATABASE_URL — checked here for early error; actual connection happens later
    if env::var("DATABASE_URL").is_err() {
        return Err(anyhow::anyhow!(
            "DATABASE_URL environment variable must be set"
        ));
    }

    // Upload directory — create if needed
    let upload_dir = env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string());
    std::fs::create_dir_all(&upload_dir).map_err(|e| {
        anyhow::anyhow!("Failed to create upload directory '{}': {}", upload_dir, e)
    })?;

    Ok(jwt_config)
}

fn build_cors_layer() -> CorsLayer {
    use axum::http::{header, HeaderValue, Method};

    let origins_str = env::var("CORS_ORIGINS").unwrap_or_else(|_| "*".to_string());

    let cors = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]);

    if origins_str == "*" {
        cors.allow_origin(tower_http::cors::Any)
    } else {
        let origins: Vec<HeaderValue> = origins_str
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        cors.allow_origin(origins)
    }
}

fn create_app(upload_dir: &str) -> Router {
    Router::new()
        .route("/", get(health_check))
        .merge(routes::create_routes())
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .nest_service("/uploads", ServeDir::new(upload_dir))
        .layer(TraceLayer::new_for_http())
        .layer(build_cors_layer())
}

#[utoipa::path(
    get,
    path = "/",
    responses(
        (status = 200, description = "Health check successful", body = serde_json::Value)
    )
)]
async fn health_check(
    Extension(db): Extension<DatabaseConnection>,
) -> impl IntoResponse {
    let db_ok = db
        .query_one(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT 1".to_string(),
        ))
        .await
        .is_ok();

    let status = if db_ok { "ok" } else { "degraded" };

    Json(json!({
        "status": status,
        "service": "Forum API",
        "version": env!("CARGO_PKG_VERSION"),
        "database": db_ok,
    }))
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install CTRL+C signal handler");
    tracing::info!("Shutdown signal received, gracefully shutting down...");
}
