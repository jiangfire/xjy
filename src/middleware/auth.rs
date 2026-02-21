use crate::{
    error::AppError,
    models::User,
    utils::{
        cookie::{extract_cookie, ACCESS_TOKEN_COOKIE},
        jwt::decode_jwt,
    },
};
use axum::{extract::Request, http::HeaderMap, middleware::Next, response::Response, Extension};
use sea_orm::{DatabaseConnection, EntityTrait};

/// Extracted user information from JWT token
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: String,
}

/// JWT authentication middleware
///
/// Verifies the JWT token from the Authorization header,
/// checks the user is not banned, and adds user info to request extensions.
pub async fn auth_middleware(
    Extension(db): Extension<DatabaseConnection>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Prefer Authorization: Bearer, fallback to HttpOnly cookie.
    let token = extract_bearer_token(&headers)
        .or_else(|| extract_cookie(&headers, ACCESS_TOKEN_COOKIE))
        .ok_or(AppError::Unauthorized)?;

    // Verify JWT
    let claims = decode_jwt(&token).map_err(|_| AppError::Unauthorized)?;

    // Access routes must use access token (not refresh token).
    if !crate::utils::jwt::is_access_token(&claims) {
        return Err(AppError::Unauthorized);
    }

    // Check user is not banned
    let user_id: i32 = claims
        .sub
        .parse()
        .map_err(|_| AppError::Validation("Invalid user ID in token".to_string()))?;

    let user = User::find_by_id(user_id)
        .one(&db)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if user.role == "banned" {
        return Err(AppError::Forbidden);
    }

    // Add user info to request extensions
    let auth_user = AuthUser {
        user_id: claims.sub,
    };
    request.extensions_mut().insert(auth_user);

    // Continue to next handler
    Ok(next.run(request).await)
}

fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())?;

    let token = auth_header.strip_prefix("Bearer ")?;
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

/// Parse user_id from AuthUser string to i32
pub fn parse_user_id(auth_user: &AuthUser) -> crate::error::AppResult<i32> {
    auth_user
        .user_id
        .parse()
        .map_err(|_| AppError::Validation("Invalid user ID".to_string()))
}

/// Verify the current user has admin role
pub async fn require_admin(
    db: &sea_orm::DatabaseConnection,
    auth_user: &AuthUser,
) -> crate::error::AppResult<i32> {
    let user_id = parse_user_id(auth_user)?;
    let auth_service = crate::services::auth::AuthService::new(db.clone());
    let user = auth_service.get_user_by_id(user_id).await?;
    if user.role != "admin" {
        return Err(AppError::Forbidden);
    }
    Ok(user_id)
}

/// Extractor for AuthUser from request extensions
use axum::extract::FromRequestParts;

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthUser>()
            .cloned()
            .ok_or(AppError::Unauthorized)
    }
}
