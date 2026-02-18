use axum::{response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> axum::response::Response {
        Json(self).into_response()
    }
}

#[allow(dead_code)]
impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
        }
    }

    pub fn with_message(data: T, message: String) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: Some(message),
        }
    }

    pub fn err(message: String) -> Self {
        Self {
            success: false,
            message: Some(message),
            data: None,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedResponse<T: Serialize> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub per_page: u64,
    pub total_pages: u64,
}

impl<T: Serialize> PaginatedResponse<T> {
    pub fn new(items: Vec<T>, total: u64, page: u64, per_page: u64) -> Self {
        let total_pages = if per_page == 0 {
            0
        } else {
            (total + per_page - 1) / per_page
        };
        Self {
            items,
            total,
            page,
            per_page,
            total_pages,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PaginationQuery {
    pub page: Option<u64>,
    pub per_page: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_pages_basic() {
        let resp = PaginatedResponse::<String>::new(vec![], 100, 1, 20);
        assert_eq!(resp.total_pages, 5);
    }

    #[test]
    fn total_pages_with_remainder() {
        let resp = PaginatedResponse::<String>::new(vec![], 101, 1, 20);
        assert_eq!(resp.total_pages, 6);
    }

    #[test]
    fn total_pages_exact_division() {
        let resp = PaginatedResponse::<String>::new(vec![], 60, 1, 20);
        assert_eq!(resp.total_pages, 3);
    }

    #[test]
    fn total_pages_zero_per_page() {
        let resp = PaginatedResponse::<String>::new(vec![], 10, 1, 0);
        assert_eq!(resp.total_pages, 0);
    }

    #[test]
    fn total_pages_zero_total() {
        let resp = PaginatedResponse::<String>::new(vec![], 0, 1, 20);
        assert_eq!(resp.total_pages, 0);
    }

    #[test]
    fn total_pages_single_item() {
        let resp = PaginatedResponse::<String>::new(vec![], 1, 1, 20);
        assert_eq!(resp.total_pages, 1);
    }
}
