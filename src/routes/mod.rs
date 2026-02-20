use crate::config::rate_limit::{RateLimitConfig, RateLimitRule};
use crate::handlers;
use crate::middleware::auth::auth_middleware;
use crate::websocket;
use axum::{middleware, routing, Router};
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};

pub fn create_routes() -> Router {
    Router::new()
        .nest("/api/v1", api_routes())
        // WebSocket route (auth handled inside the handler via query token)
        .route("/ws", routing::get(websocket::notification::ws_handler))
}

fn api_routes() -> Router {
    let rate_limit_config = RateLimitConfig::from_env();

    let auth = auth_routes(&rate_limit_config);
    let public_read = public_read_routes(&rate_limit_config);
    let protected =
        protected_routes(&rate_limit_config).layer(middleware::from_fn(auth_middleware));

    auth.merge(public_read).merge(protected)
}

/// Auth routes: register, login, verify-email.
fn auth_routes(config: &RateLimitConfig) -> Router {
    let router = Router::new()
        .route("/auth/register", routing::post(handlers::register))
        .route("/auth/login", routing::post(handlers::login))
        .route(
            "/auth/refresh",
            routing::post(handlers::auth::refresh_token),
        )
        .route("/auth/verify-email", routing::post(handlers::verify_email))
        .route(
            "/auth/forgot-password",
            routing::post(handlers::auth::forgot_password),
        )
        .route(
            "/auth/reset-password",
            routing::post(handlers::auth::reset_password),
        );

    with_optional_rate_limit(router, config.enabled, config.auth)
}

/// Public read routes: all public GETs + search.
fn public_read_routes(config: &RateLimitConfig) -> Router {
    let router = Router::new()
        // Users
        .route(
            "/users/{username}",
            routing::get(handlers::user::get_user_profile),
        )
        // Forums
        .route("/forums", routing::get(handlers::forum::list_forums))
        .route("/forums/{slug}", routing::get(handlers::forum::get_forum))
        // Posts
        .route(
            "/forums/{forum_id}/posts",
            routing::get(handlers::post::list_posts),
        )
        .route("/posts/{id}", routing::get(handlers::post::get_post))
        // Comments
        .route(
            "/posts/{post_id}/comments",
            routing::get(handlers::comment::list_comments),
        )
        // Search
        .route("/search", routing::get(handlers::post::search_posts))
        // Tags
        .route("/tags", routing::get(handlers::tag::list_tags))
        .route(
            "/tags/{slug}/posts",
            routing::get(handlers::tag::get_posts_by_tag),
        )
        // Follow (public reads)
        .route(
            "/users/{id}/followers",
            routing::get(handlers::follow::list_followers),
        )
        .route(
            "/users/{id}/following",
            routing::get(handlers::follow::list_following),
        );

    with_optional_rate_limit(router, config.enabled, config.public_read)
}

/// Protected routes: all authenticated writes.
fn protected_routes(config: &RateLimitConfig) -> Router {
    let router = Router::new()
        // Auth
        .route("/auth/me", routing::get(handlers::get_current_user))
        .route("/auth/logout", routing::post(handlers::auth::logout))
        .route(
            "/auth/profile",
            routing::put(handlers::user::update_profile),
        )
        .route("/auth/password", routing::put(handlers::change_password))
        .route(
            "/auth/resend-verification",
            routing::post(handlers::resend_verification),
        )
        // PoW
        .route(
            "/pow/challenge",
            routing::post(handlers::pow::create_pow_challenge),
        )
        // Forums (admin only - checked in handler)
        .route("/forums", routing::post(handlers::forum::create_forum))
        .route(
            "/forums/{slug}",
            routing::put(handlers::forum::update_forum).delete(handlers::forum::delete_forum),
        )
        // Posts
        .route("/posts", routing::post(handlers::post::create_post))
        .route(
            "/posts/{id}",
            routing::put(handlers::post::update_post).delete(handlers::post::delete_post),
        )
        .route("/posts/{id}/pin", routing::put(handlers::post::pin_post))
        .route("/posts/{id}/lock", routing::put(handlers::post::lock_post))
        // Votes
        .route("/posts/{id}/vote", routing::post(handlers::vote::vote_post))
        .route(
            "/comments/{id}/vote",
            routing::post(handlers::vote::vote_comment),
        )
        // Comments
        .route(
            "/comments",
            routing::post(handlers::comment::create_comment),
        )
        .route(
            "/comments/{id}",
            routing::put(handlers::comment::update_comment)
                .delete(handlers::comment::delete_comment),
        )
        // Notifications
        .route(
            "/notifications",
            routing::get(handlers::notification::list_notifications),
        )
        .route(
            "/notifications/unread-count",
            routing::get(handlers::notification::unread_count),
        )
        .route(
            "/notifications/read-all",
            routing::put(handlers::notification::mark_all_read),
        )
        .route(
            "/notifications/{id}/read",
            routing::put(handlers::notification::mark_read),
        )
        // Admin
        .route("/admin/stats", routing::get(handlers::admin::get_stats))
        .route("/admin/users", routing::get(handlers::admin::list_users))
        .route(
            "/admin/users/{id}/role",
            routing::put(handlers::admin::update_user_role),
        )
        .route(
            "/admin/posts/{id}",
            routing::delete(handlers::admin::admin_delete_post),
        )
        .route(
            "/admin/comments/{id}",
            routing::delete(handlers::admin::admin_delete_comment),
        )
        // Bookmarks
        .route(
            "/posts/{id}/bookmark",
            routing::post(handlers::bookmark::toggle_bookmark),
        )
        .route(
            "/bookmarks",
            routing::get(handlers::bookmark::list_bookmarks),
        )
        // Follow
        .route(
            "/users/{id}/follow",
            routing::post(handlers::follow::toggle_follow),
        )
        // Upload
        .route(
            "/upload/avatar",
            routing::post(handlers::upload::upload_avatar),
        )
        .route(
            "/upload/image",
            routing::post(handlers::upload::upload_image),
        )
        // Reports
        .route("/reports", routing::post(handlers::report::create_report))
        .route(
            "/admin/reports",
            routing::get(handlers::report::list_reports),
        )
        .route(
            "/admin/reports/{id}/resolve",
            routing::put(handlers::report::resolve_report),
        )
        // Tags (admin)
        .route("/admin/tags", routing::post(handlers::tag::create_tag))
        .route(
            "/admin/tags/{id}",
            routing::put(handlers::tag::update_tag).delete(handlers::tag::delete_tag),
        );

    with_optional_rate_limit(router, config.enabled, config.protected)
}

fn with_optional_rate_limit(router: Router, enabled: bool, rule: RateLimitRule) -> Router {
    if !enabled {
        return router;
    }

    let governor_conf = GovernorConfigBuilder::default()
        .per_second(rule.per_second)
        .burst_size(rule.burst_size)
        .finish()
        .expect("Invalid rate limit configuration");

    router.layer(GovernorLayer::new(governor_conf))
}
