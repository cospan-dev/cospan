use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("invalid request: {0}")]
    InvalidRequest(String),

    #[error("unauthorized: {0}")]
    Unauthorized(String),

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_name) = match &self {
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, "NotFound"),
            AppError::InvalidRequest(_) => (StatusCode::BAD_REQUEST, "InvalidRequest"),
            AppError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "Unauthorized"),
            AppError::Database(e) => {
                tracing::error!(error = %e, "database error");
                (StatusCode::INTERNAL_SERVER_ERROR, "InternalServerError")
            }
        };

        let message = match &self {
            AppError::Database(_) => "internal error".to_string(),
            other => other.to_string(),
        };

        (
            status,
            Json(serde_json::json!({
                "error": error_name,
                "message": message,
            })),
        )
            .into_response()
    }
}
