//! Axum extractors for authentication.
//!
//! - `OptionalAuth`: resolves the session if present, but does not require it.
//! - `RequiredAuth`: resolves the session and returns 401 if missing or invalid.

use std::sync::Arc;

use axum::Json;
use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use crate::auth::oauth::extract_session_id;
use crate::state::AppState;

/// Authenticated user info extracted from the session.
#[derive(Debug, Clone)]
pub struct AuthInfo {
    /// The user's DID.
    pub did: String,
    /// The user's handle (may be stale).
    pub handle: Option<String>,
    /// The session ID (for further operations).
    pub session_id: String,
}

/// Extractor that optionally resolves the authenticated user.
/// If no valid session cookie is present, `self.0` is `None`.
#[derive(Debug, Clone)]
pub struct OptionalAuth(pub Option<AuthInfo>);

/// Extractor that requires an authenticated user.
/// Returns 401 Unauthorized if no valid session is found.
#[derive(Debug, Clone)]
pub struct RequiredAuth(pub AuthInfo);

#[derive(Debug)]
pub struct AuthError;

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": "Unauthorized",
                "message": "authentication required",
            })),
        )
            .into_response()
    }
}

impl FromRequestParts<Arc<AppState>> for OptionalAuth {
    type Rejection = std::convert::Infallible;

    fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        let state = state.clone();
        let headers = parts.headers.clone();

        async move {
            let session_id = match extract_session_id(&headers) {
                Some(id) => id,
                None => return Ok(OptionalAuth(None)),
            };

            let session = match state.session_store.get_session(&session_id).await {
                Ok(Some(s)) => s,
                _ => return Ok(OptionalAuth(None)),
            };

            Ok(OptionalAuth(Some(AuthInfo {
                did: session.did,
                handle: session.handle,
                session_id,
            })))
        }
    }
}

impl FromRequestParts<Arc<AppState>> for RequiredAuth {
    type Rejection = AuthError;

    fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        let state = state.clone();
        let headers = parts.headers.clone();

        async move {
            let session_id = extract_session_id(&headers).ok_or(AuthError)?;

            let session = state
                .session_store
                .get_session(&session_id)
                .await
                .map_err(|_| AuthError)?
                .ok_or(AuthError)?;

            Ok(RequiredAuth(AuthInfo {
                did: session.did,
                handle: session.handle,
                session_id,
            }))
        }
    }
}
