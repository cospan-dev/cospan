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

use crate::error::AppError;
use crate::middleware::auth::RequiredAuth;
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
    RequiredAuth(caller): RequiredAuth,
    _headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    // Build the push token JWT. The `scope` claim uses the granular
    // `rpc:<lxm>?aud=<node>` form so the node can enforce it with the
    // same parser as the appview.
    let now = chrono::Utc::now().timestamp();
    let ttl = 3600; // 1 hour
    let claims = PushTokenClaims {
        sub: caller.did.clone(),
        iss: state.config.public_url().to_string(),
        scope: "rpc:dev.cospan.vcs.push?aud=*".to_string(),
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
        "did": caller.did,
        "expiresIn": ttl,
        "usage": "Use as git password when pushing. Your DID is the username.",
        "example": format!("git push https://node.cospan.dev/{}/REPO main", caller.did),
    })))
}

#[derive(Serialize)]
struct PushTokenHeader {
    alg: String,
    typ: String,
    kid: String,
}
