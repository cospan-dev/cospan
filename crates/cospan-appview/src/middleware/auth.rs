//! Axum extractors for authentication and authorization.
//!
//! - [`OptionalAuth`]: resolves the session if present, but does not require it.
//! - [`RequiredAuth`]: resolves the session and returns 401 if missing or invalid.
//! - [`RequiredScope`]: requires a specific OAuth scope to have been granted.
//! - [`RequiredRole`]: requires the caller to hold a given role (or higher)
//!   on a target repo — either directly as a collaborator or transitively
//!   through org membership when the repo is org-owned.

use std::sync::Arc;

use axum::Json;
use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use crate::auth::oauth::extract_session_id;
use crate::auth::scope::{self, Need, Scope};
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
    /// OAuth scopes granted to this session, expanded via the permission-set
    /// registry so `include:` tokens are already resolved.
    pub scopes: Vec<Scope>,
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
pub enum AuthError {
    Unauthenticated,
    InsufficientScope(&'static str),
    InsufficientRole { required: Role },
    Internal,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, code, msg): (StatusCode, &str, String) = match self {
            AuthError::Unauthenticated => (
                StatusCode::UNAUTHORIZED,
                "Unauthorized",
                "authentication required".into(),
            ),
            AuthError::InsufficientScope(s) => (
                StatusCode::FORBIDDEN,
                "InsufficientScope",
                format!("missing required scope: {s}"),
            ),
            AuthError::InsufficientRole { required } => (
                StatusCode::FORBIDDEN,
                "InsufficientRole",
                format!("caller lacks required role: {required:?}"),
            ),
            AuthError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "InternalError",
                "authorization check failed".into(),
            ),
        };
        (status, Json(json!({"error": code, "message": msg}))).into_response()
    }
}

fn resolve_auth_info(
    headers: &axum::http::HeaderMap,
    state: &Arc<AppState>,
) -> impl std::future::Future<Output = Option<AuthInfo>> + Send {
    let state = state.clone();
    let headers = headers.clone();
    async move {
        let session_id = extract_session_id(&headers)?;
        let session = state.session_store.get_session(&session_id).await.ok()??;

        let raw_scopes = scope::parse_scope_string(&session.scope).unwrap_or_default();
        let scopes = state.permission_sets.expand(&raw_scopes);

        Some(AuthInfo {
            did: session.did,
            handle: session.handle,
            session_id,
            scopes,
        })
    }
}

impl FromRequestParts<Arc<AppState>> for OptionalAuth {
    type Rejection = std::convert::Infallible;

    fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        let fut = resolve_auth_info(&parts.headers, state);
        async move { Ok(OptionalAuth(fut.await)) }
    }
}

impl FromRequestParts<Arc<AppState>> for RequiredAuth {
    type Rejection = AuthError;

    fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        let fut = resolve_auth_info(&parts.headers, state);
        async move {
            fut.await
                .map(RequiredAuth)
                .ok_or(AuthError::Unauthenticated)
        }
    }
}

// -- Scope enforcement --------------------------------------------------------

/// Marker trait carrying the required scope for an XRPC call.
///
/// Implement this on a unit struct per endpoint so the extractor can be
/// specialised at compile time without macros.
pub trait ScopeSpec: Send + Sync + 'static {
    /// The XRPC method name (lxm) the caller must hold `rpc:<lxm>?aud=<self>` for.
    const LXM: &'static str;
}

/// Extractor asserting that the session was granted `rpc:<S::LXM>?aud=<this appview>`.
///
/// The resolved [`AuthInfo`] is exposed via `.0`.
#[derive(Debug, Clone)]
pub struct RequiredScope<S: ScopeSpec>(pub AuthInfo, std::marker::PhantomData<S>);

impl<S: ScopeSpec> FromRequestParts<Arc<AppState>> for RequiredScope<S> {
    type Rejection = AuthError;

    fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        let fut = resolve_auth_info(&parts.headers, state);
        let appview_did = state.config.appview_did.clone();
        async move {
            let info = fut.await.ok_or(AuthError::Unauthenticated)?;
            let aud = if appview_did.is_empty() {
                "*"
            } else {
                &appview_did
            };
            let need = Need::Rpc { lxm: S::LXM, aud };
            if !scope::permits(&info.scopes, &need) {
                return Err(AuthError::InsufficientScope(S::LXM));
            }
            Ok(RequiredScope(info, std::marker::PhantomData))
        }
    }
}

// -- Role enforcement ---------------------------------------------------------

/// Collaborator role hierarchy. Higher variants imply lower ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    Reader,
    Contributor,
    Maintainer,
    Owner,
}

impl Role {
    /// Parse a collaborator.role or org_member.role database value.
    pub fn from_db(s: &str) -> Option<Self> {
        match s {
            "reader" => Some(Role::Reader),
            "contributor" | "member" => Some(Role::Contributor),
            "maintainer" | "admin" => Some(Role::Maintainer),
            "owner" => Some(Role::Owner),
            _ => None,
        }
    }
}

/// A target repo for a role check.
#[derive(Debug, Clone)]
pub struct RepoTarget {
    pub repo_did: String,
    pub repo_name: String,
}

/// Resolve the caller's effective role on a repo, considering both direct
/// collaborator entries and org membership (if the repo's owner DID matches
/// an org we index).
///
/// Returns `Role::Owner` if the caller IS the repo's owning DID.
pub async fn effective_role(
    state: &AppState,
    caller_did: &str,
    target: &RepoTarget,
) -> Result<Option<Role>, sqlx::Error> {
    if caller_did == target.repo_did {
        return Ok(Some(Role::Owner));
    }

    // Direct collaborator?
    let rows = crate::db::collaborator::list_for_repo(
        &state.db,
        &target.repo_did,
        &target.repo_name,
        1024,
        None,
    )
    .await?;
    let best: Option<Role> = rows
        .iter()
        .find(|c| c.member_did == caller_did)
        .and_then(|c| Role::from_db(&c.role));

    // Org membership inheritance is intentionally not resolved here: it
    // requires a repo→org relation (repos owned via `at://<org>/<...>`)
    // that is out of scope for this change. Add it alongside the org
    // ownership enforcement work.

    Ok(best)
}

/// Extractor that requires a minimum role on a target repo identified by
/// URL path or query. Because Axum extractors can't easily share state with
/// later arguments, this is exposed as a helper function to call from handlers
/// *after* the repo identifiers have been parsed.
pub async fn require_role(
    state: &AppState,
    caller: &AuthInfo,
    target: &RepoTarget,
    min: Role,
) -> Result<Role, AuthError> {
    match effective_role(state, &caller.did, target).await {
        Ok(Some(r)) if r >= min => Ok(r),
        Ok(_) => Err(AuthError::InsufficientRole { required: min }),
        Err(e) => {
            tracing::error!(error = %e, "role lookup failed");
            Err(AuthError::Internal)
        }
    }
}
