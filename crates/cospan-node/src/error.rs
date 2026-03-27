use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[derive(Debug, thiserror::Error)]
pub enum NodeError {
    #[error("object not found: {0}")]
    ObjectNotFound(String),

    #[error("ref not found: {0}")]
    RefNotFound(String),

    #[error("repo not found: {did}/{name}")]
    RepoNotFound { did: String, name: String },

    #[error("unauthorized: {0}")]
    Unauthorized(String),

    #[error("forbidden: {0}")]
    Forbidden(String),

    #[error("invalid request: {0}")]
    InvalidRequest(String),

    #[error("validation failed: {0}")]
    ValidationFailed(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl IntoResponse for NodeError {
    fn into_response(self) -> Response {
        let (status, error_name) = match &self {
            NodeError::ObjectNotFound(_)
            | NodeError::RefNotFound(_)
            | NodeError::RepoNotFound { .. } => (StatusCode::NOT_FOUND, "NotFound"),
            NodeError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "AuthRequired"),
            NodeError::Forbidden(_) => (StatusCode::FORBIDDEN, "Forbidden"),
            NodeError::InvalidRequest(_) => (StatusCode::BAD_REQUEST, "InvalidRequest"),
            NodeError::ValidationFailed(_) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "ValidationFailed")
            }
            NodeError::Internal(_) => {
                tracing::error!(error = %self, "internal error");
                (StatusCode::INTERNAL_SERVER_ERROR, "InternalServerError")
            }
        };

        let message = match &self {
            NodeError::Internal(_) => "internal error".to_string(),
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
