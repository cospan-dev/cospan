//! `POST /xrpc/dev.cospan.repo.createPushToken`
//!
//! Issues a short-lived JWT that authorizes `git push` to the cospan
//! node. The token is signed by the appview's DPoP key (verifiable via
//! the JWKS endpoint at `/.well-known/jwks.json`) and contains:
//!
//!   - `sub`: the authenticated user's DID
//!   - `scope`: `"push"`
//!   - `iat` / `exp`: issued-at and expiration (default 1 hour)
//!
//! The user passes this token as their git password (with their DID as
//! username) when pushing via git smart HTTP. The node verifies the JWT
//! against the appview's JWKS before accepting the push.

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use serde::Serialize;

use crate::auth::oauth::extract_session_id;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Serialize)]
struct PushTokenClaims {
    /// Subject: the user's DID.
    sub: String,
    /// Issuer: the appview's public URL.
    iss: String,
    /// Scope: always "push" for git push tokens.
    scope: String,
    /// Issued at (unix seconds).
    iat: i64,
    /// Expires at (unix seconds).
    exp: i64,
    /// Unique token ID.
    jti: String,
}

/// `POST /xrpc/dev.cospan.repo.createPushToken`
///
/// Requires an authenticated session. Returns a push token the user
/// can use as their git password when pushing to the cospan node.
pub async fn handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    // 1. Require an authenticated session.
    let has_cookie = headers.get("cookie").is_some();
    let cookie_preview = headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .map(|s| if s.len() > 40 { format!("{}...", &s[..40]) } else { s.to_string() })
        .unwrap_or_default();
    tracing::info!(has_cookie, cookie_preview, "createPushToken auth check");
    let session_id = extract_session_id(&headers)
        .ok_or_else(|| AppError::Unauthorized("sign in required".to_string()))?;
    let session = state
        .session_store
        .get_session(&session_id)
        .await
        .map_err(|e| AppError::Upstream(format!("session lookup: {e}")))?
        .ok_or_else(|| AppError::Unauthorized("session not found".to_string()))?;

    // 2. Build the push token JWT.
    let now = chrono::Utc::now().timestamp();
    let ttl = 3600; // 1 hour
    let claims = PushTokenClaims {
        sub: session.did.clone(),
        iss: state.config.public_url().to_string(),
        scope: "push".to_string(),
        iat: now,
        exp: now + ttl,
        jti: uuid::Uuid::new_v4().to_string(),
    };

    // 3. Sign with the appview's DPoP key (same key served at JWKS).
    let header = PushTokenHeader {
        alg: "ES256".to_string(),
        typ: "jwt".to_string(),
        kid: state.dpop_key.kid.clone(),
    };
    let token = crate::auth::dpop::encode_es256_jwt_public(&header, &claims, &state.dpop_key)
        .map_err(|e| AppError::Upstream(format!("sign push token: {e}")))?;

    Ok(Json(serde_json::json!({
        "token": token,
        "did": session.did,
        "expiresIn": ttl,
        "usage": "Use as git password when pushing. Your DID is the username.",
        "example": format!("git push https://node.cospan.dev/{}/REPO main", session.did),
    })))
}

#[derive(Serialize)]
struct PushTokenHeader {
    alg: String,
    typ: String,
    kid: String,
}
