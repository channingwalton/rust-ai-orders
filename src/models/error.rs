use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use super::order::OrderId;
use super::user::UserId;

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("User not found: {0}")]
    UserNotFound(UserId),

    #[error("Order not found: {0}")]
    OrderNotFound(OrderId),

    #[error("Invalid JSON request: {0}")]
    InvalidJsonRequest(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),
}

impl IntoResponse for ServiceError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ServiceError::UserNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            ServiceError::OrderNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            ServiceError::InvalidJsonRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ServiceError::ValidationError(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ServiceError::DatabaseError(inner) => {
                tracing::error!("Database error: {}", inner);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
        };

        let body = json!({ "error": message });
        (status, axum::Json(body)).into_response()
    }
}

impl From<sqlx::Error> for ServiceError {
    fn from(err: sqlx::Error) -> Self {
        ServiceError::DatabaseError(err.to_string())
    }
}
