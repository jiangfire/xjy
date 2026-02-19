use crate::error::{AppError, AppResult};
use crate::middleware::auth::{parse_user_id, require_admin, AuthUser};
use crate::models::ReportModel;
use crate::response::{ApiResponse, PaginatedResponse};
use crate::services::report::ReportService;
use axum::{extract::Path, extract::Query, response::IntoResponse, Extension, Json};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateReportRequest {
    /// Target type (post or comment)
    #[validate(length(min = 1, max = 20))]
    pub target_type: String,
    /// Target ID
    pub target_id: i32,
    /// Report reason (1-50 characters)
    #[validate(length(min = 1, max = 50))]
    pub reason: String,
    /// Detailed description
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ListReportsQuery {
    /// Filter by status
    pub status: Option<String>,
    /// Page number
    pub page: Option<u64>,
    /// Items per page
    pub per_page: Option<u64>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ResolveReportRequest {
    /// Action taken (1-20 characters)
    #[validate(length(min = 1, max = 20))]
    pub action: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ReportResponse {
    /// Report ID
    pub id: i32,
    /// Reporter user ID
    pub reporter_id: i32,
    /// Target type
    pub target_type: String,
    /// Target ID
    pub target_id: i32,
    /// Report reason
    pub reason: String,
    /// Detailed description
    pub description: Option<String>,
    /// Report status
    pub status: String,
    /// Admin user ID who resolved
    pub resolved_by: Option<i32>,
    /// Resolution timestamp
    pub resolved_at: Option<String>,
    /// Creation timestamp
    pub created_at: String,
}

impl From<ReportModel> for ReportResponse {
    fn from(r: ReportModel) -> Self {
        Self {
            id: r.id,
            reporter_id: r.reporter_id,
            target_type: r.target_type,
            target_id: r.target_id,
            reason: r.reason,
            description: r.description,
            status: r.status,
            resolved_by: r.resolved_by,
            resolved_at: r.resolved_at.map(|t| t.to_string()),
            created_at: r.created_at.to_string(),
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/reports",
    security(("jwt_token" = [])),
    request_body = CreateReportRequest,
    responses(
        (status = 200, description = "Report created", body = ReportResponse),
        (status = 400, description = "Validation error", body = AppError),
        (status = 401, description = "Unauthorized", body = AppError),
    ),
    tag = "reports"
)]
pub async fn create_report(
    Extension(db): Extension<DatabaseConnection>,
    auth_user: AuthUser,
    Json(payload): Json<CreateReportRequest>,
) -> AppResult<impl IntoResponse> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let user_id = parse_user_id(&auth_user)?;

    let service = ReportService::new(db);
    let report = service
        .create_report(
            user_id,
            &payload.target_type,
            payload.target_id,
            &payload.reason,
            payload.description.as_deref(),
        )
        .await?;

    Ok(ApiResponse::ok(ReportResponse::from(report)))
}

#[utoipa::path(
    get,
    path = "/api/v1/admin/reports",
    security(("jwt_token" = [])),
    params(
        ("status" = Option<String>, Query, description = "Filter by status"),
        ("page" = Option<u64>, Query, description = "Page number"),
        ("per_page" = Option<u64>, Query, description = "Items per page"),
    ),
    responses(
        (status = 200, description = "List of reports", body = PaginatedResponse<ReportResponse>),
        (status = 403, description = "Admin only", body = AppError),
    ),
    tag = "reports"
)]
pub async fn list_reports(
    Extension(db): Extension<DatabaseConnection>,
    auth_user: AuthUser,
    Query(params): Query<ListReportsQuery>,
) -> AppResult<impl IntoResponse> {
    require_admin(&db, &auth_user).await?;

    let page = params.page.unwrap_or(1);
    let per_page = params.per_page.unwrap_or(20).min(100);

    let service = ReportService::new(db);
    let (reports, total) = service
        .list_reports(params.status.as_deref(), page, per_page)
        .await?;
    let items = reports.into_iter().map(ReportResponse::from).collect();

    Ok(ApiResponse::ok(PaginatedResponse::new(
        items, total, page, per_page,
    )))
}

#[utoipa::path(
    put,
    path = "/api/v1/admin/reports/{id}/resolve",
    security(("jwt_token" = [])),
    params(("id" = i32, Path, description = "Report ID")),
    request_body = ResolveReportRequest,
    responses(
        (status = 200, description = "Report resolved", body = ReportResponse),
        (status = 400, description = "Validation error", body = AppError),
        (status = 403, description = "Admin only", body = AppError),
    ),
    tag = "reports"
)]
pub async fn resolve_report(
    Extension(db): Extension<DatabaseConnection>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
    Json(payload): Json<ResolveReportRequest>,
) -> AppResult<impl IntoResponse> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let admin_id = require_admin(&db, &auth_user).await?;

    let service = ReportService::new(db);
    let report = service.resolve(id, admin_id, &payload.action).await?;

    Ok(ApiResponse::ok(ReportResponse::from(report)))
}
