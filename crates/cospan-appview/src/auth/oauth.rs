//! OAuth 2.0 route handlers for the ATProto BFF pattern.
//!
//! Routes:
//! - GET  /oauth/client-metadata.json: serve client metadata document
//! - GET  /.well-known/jwks.json     : serve JWKS document
//! - GET  /oauth/login?handle=...    : initiate PAR + redirect to PDS auth
//! - GET  /oauth/callback            : exchange auth code for tokens, create session
//! - POST /oauth/logout              : destroy session, clear cookie
//! - GET  /oauth/session             : return current session info for the frontend

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::dpop::{DpopKey, compute_code_challenge, generate_code_verifier};
use crate::auth::scope::{AuthIntent, build_scope_string, client_metadata_scope};
use crate::auth::{AuthFlowState, Session};
use crate::state::AppState;

/// Cookie name for the session ID.
const SESSION_COOKIE: &str = "cospan_session";

/// Build the OAuth router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/oauth/client-metadata.json", get(client_metadata))
        .route("/.well-known/jwks.json", get(jwks))
        .route("/oauth/login", get(login))
        .route("/oauth/callback", get(callback))
        .route("/oauth/logout", post(logout))
        .route("/oauth/session", get(session_info))
        .route("/oauth/bridge", post(bridge_session))
        .route("/oauth/bridge", delete(bridge_delete))
}

// -- GET /oauth/client-metadata.json --

async fn client_metadata(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let config = &state.oauth_config;
    let scope = client_metadata_scope(&state.permission_sets, &state.config.appview_did);

    let metadata = json!({
        "client_id": config.client_id,
        "client_name": config.client_name,
        "client_uri": config.public_url,
        "logo_uri": format!("{}/assets/logo.png", config.public_url),
        "tos_uri": format!("{}/tos", config.public_url),
        "policy_uri": format!("{}/privacy", config.public_url),
        "redirect_uris": [
            format!("{}/", config.public_url),
            config.redirect_uri,
        ],
        "grant_types": ["authorization_code", "refresh_token"],
        "response_types": ["code"],
        "scope": scope,
        "application_type": "web",
        "token_endpoint_auth_method": "none",
        "dpop_bound_access_tokens": true,
    });

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        Json(metadata),
    )
}

// -- GET /.well-known/jwks.json --

async fn jwks(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let jwks = state.dpop_key.jwks_document();
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        Json(jwks),
    )
}

// -- GET /oauth/login?handle=... --

#[derive(Deserialize)]
struct LoginQuery {
    handle: String,
    /// Access tier to request: `browse|contribute|maintain|own`.
    /// Defaults to `contribute` — the minimum for most interactive flows.
    intent: Option<String>,
}

async fn login(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LoginQuery>,
) -> Result<Response, OAuthError> {
    let handle = query.handle.trim().to_lowercase();
    let intent = query
        .intent
        .as_deref()
        .and_then(AuthIntent::parse)
        .unwrap_or(AuthIntent::Contribute);
    let requested_scope =
        build_scope_string(intent, &state.permission_sets, &state.config.appview_did);
    tracing::info!(handle = %handle, ?intent, scope = %requested_scope, "OAuth login initiated");

    // Step 1: Resolve handle -> DID -> DID document
    let (did, doc) = state
        .did_resolver
        .resolve_identity(&handle)
        .await
        .map_err(|e| OAuthError::IdentityResolution(e.to_string()))?;

    let pds_url = doc
        .pds_endpoint()
        .ok_or_else(|| OAuthError::IdentityResolution("no PDS endpoint in DID document".into()))?
        .to_string();

    tracing::info!(did = %did, pds = %pds_url, "resolved identity");

    // Step 2: Discover authorization server
    let auth_server = state
        .did_resolver
        .discover_auth_server(&pds_url)
        .await
        .map_err(|e| OAuthError::AuthServerDiscovery(e.to_string()))?;

    tracing::info!(issuer = %auth_server.issuer, "discovered auth server");

    // Step 3: Generate cryptographic material
    let code_verifier = generate_code_verifier();
    let code_challenge = compute_code_challenge(&code_verifier);
    let session_dpop_key = DpopKey::generate_session_key();
    let oauth_state = uuid::Uuid::new_v4().to_string();

    // Step 4: Pushed Authorization Request (PAR)
    let par_url = &auth_server.pushed_authorization_request_endpoint;
    let client_id = &state.oauth_config.client_id;
    let redirect_uri = &state.oauth_config.redirect_uri;

    let par_params = [
        ("client_id", client_id.as_str()),
        ("response_type", "code"),
        ("redirect_uri", redirect_uri.as_str()),
        ("scope", requested_scope.as_str()),
        ("state", &oauth_state),
        ("code_challenge", &code_challenge),
        ("code_challenge_method", "S256"),
        ("login_hint", &handle),
        (
            "client_assertion_type",
            "urn:ietf:params:oauth:client-assertion-type:jwt-bearer",
        ),
    ];

    // First PAR attempt (likely to get 400 with dpop-nonce)
    let client_assertion = state
        .dpop_key
        .create_client_assertion(client_id, &auth_server.issuer)
        .map_err(|e| OAuthError::Internal(format!("client assertion: {}", e)))?;

    let dpop_proof = session_dpop_key
        .create_dpop_proof("POST", par_url, None)
        .map_err(|e| OAuthError::Internal(format!("dpop proof: {}", e)))?;

    let mut form_params: Vec<(&str, &str)> = par_params.to_vec();
    form_params.push(("client_assertion", &client_assertion));

    let resp = state
        .http_client
        .post(par_url)
        .header("DPoP", &dpop_proof)
        .form(&form_params)
        .send()
        .await
        .map_err(|e| OAuthError::Internal(format!("PAR request: {}", e)))?;

    // Handle DPoP nonce requirement (expected on first attempt)
    let (par_response, dpop_nonce) = if resp.status() == StatusCode::BAD_REQUEST {
        let nonce = resp
            .headers()
            .get("dpop-nonce")
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        if let Some(ref nonce_str) = nonce {
            tracing::debug!("got dpop-nonce from PAR, retrying");

            // Retry with nonce
            let client_assertion_retry = state
                .dpop_key
                .create_client_assertion(client_id, &auth_server.issuer)
                .map_err(|e| OAuthError::Internal(format!("client assertion retry: {}", e)))?;

            let dpop_proof_retry = session_dpop_key
                .create_dpop_proof("POST", par_url, Some(nonce_str))
                .map_err(|e| OAuthError::Internal(format!("dpop proof retry: {}", e)))?;

            let mut form_params_retry: Vec<(&str, &str)> = par_params.to_vec();
            form_params_retry.push(("client_assertion", &client_assertion_retry));

            let resp_retry = state
                .http_client
                .post(par_url)
                .header("DPoP", &dpop_proof_retry)
                .form(&form_params_retry)
                .send()
                .await
                .map_err(|e| OAuthError::Internal(format!("PAR retry: {}", e)))?;

            // Update nonce if the server provides a new one
            let updated_nonce = resp_retry
                .headers()
                .get("dpop-nonce")
                .and_then(|v| v.to_str().ok())
                .map(String::from)
                .or(nonce);

            if !resp_retry.status().is_success() {
                let status = resp_retry.status();
                let body = resp_retry.text().await.unwrap_or_default();
                return Err(OAuthError::ParFailed(format!(
                    "PAR retry failed: HTTP {} - {}",
                    status, body
                )));
            }

            let par: ParResponse = resp_retry
                .json()
                .await
                .map_err(|e| OAuthError::Internal(format!("parse PAR response: {}", e)))?;

            (par, updated_nonce)
        } else {
            let body = resp.text().await.unwrap_or_default();
            return Err(OAuthError::ParFailed(format!(
                "PAR returned 400 without dpop-nonce: {}",
                body
            )));
        }
    } else if resp.status().is_success() {
        let nonce = resp
            .headers()
            .get("dpop-nonce")
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        let par: ParResponse = resp
            .json()
            .await
            .map_err(|e| OAuthError::Internal(format!("parse PAR response: {}", e)))?;

        (par, nonce)
    } else {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(OAuthError::ParFailed(format!(
            "PAR failed: HTTP {} - {}",
            status, body
        )));
    };

    tracing::info!(request_uri = %par_response.request_uri, "PAR successful");

    // Step 5: Store auth flow state
    let flow_state = AuthFlowState {
        state: oauth_state.clone(),
        code_verifier,
        dpop_private_key_b64: session_dpop_key.private_key_b64.clone(),
        dpop_public_jwk: session_dpop_key.public_jwk().clone(),
        auth_server_issuer: auth_server.issuer.clone(),
        token_endpoint: auth_server.token_endpoint.clone(),
        expected_did: did,
        pds_url,
        handle: Some(handle),
        dpop_nonce,
        expires_at: chrono::Utc::now() + chrono::Duration::minutes(10),
        requested_scope: requested_scope.clone(),
    };

    state
        .session_store
        .put_auth_flow(&oauth_state, flow_state)
        .await
        .map_err(|e| OAuthError::Internal(format!("store auth flow: {}", e)))?;

    // Step 6: Redirect user to authorization endpoint
    let auth_url = format!(
        "{}?client_id={}&request_uri={}",
        auth_server.authorization_endpoint,
        urlencoding::encode(client_id),
        urlencoding::encode(&par_response.request_uri),
    );

    Ok(Redirect::temporary(&auth_url).into_response())
}

// -- GET /oauth/callback?code=...&state=...&iss=... --

#[derive(Deserialize)]
struct CallbackQuery {
    code: String,
    state: String,
    iss: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

async fn callback(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CallbackQuery>,
) -> Result<Response, OAuthError> {
    // Check for error from the auth server
    if let Some(error) = &query.error {
        let desc = query
            .error_description
            .as_deref()
            .unwrap_or("no description");
        tracing::warn!(error = %error, description = %desc, "OAuth callback error");
        return Err(OAuthError::AuthorizationDenied(format!(
            "{}: {}",
            error, desc
        )));
    }

    // Step 1: Retrieve and consume the auth flow state
    let flow = state
        .session_store
        .take_auth_flow(&query.state)
        .await
        .map_err(|e| OAuthError::Internal(format!("retrieve auth flow: {}", e)))?
        .ok_or(OAuthError::InvalidState)?;

    // Step 2: Verify issuer matches
    if let Some(ref iss) = query.iss
        && *iss != flow.auth_server_issuer
    {
        tracing::error!(
            expected = %flow.auth_server_issuer,
            got = %iss,
            "issuer mismatch in callback"
        );
        return Err(OAuthError::IssuerMismatch);
    }

    // Step 3: Reconstruct the session DPoP key
    let session_dpop_key =
        DpopKey::from_private_key_b64(&flow.dpop_private_key_b64, "session-dpop".to_string())
            .map_err(|e| OAuthError::Internal(format!("reconstruct dpop key: {}", e)))?;

    // Step 4: Token exchange
    let token_endpoint = &flow.token_endpoint;
    let client_id = &state.oauth_config.client_id;
    let redirect_uri = &state.oauth_config.redirect_uri;

    let client_assertion = state
        .dpop_key
        .create_client_assertion(client_id, &flow.auth_server_issuer)
        .map_err(|e| OAuthError::Internal(format!("client assertion: {}", e)))?;

    let dpop_proof = session_dpop_key
        .create_dpop_proof("POST", token_endpoint, flow.dpop_nonce.as_deref())
        .map_err(|e| OAuthError::Internal(format!("dpop proof: {}", e)))?;

    let token_params = [
        ("grant_type", "authorization_code"),
        ("client_id", client_id.as_str()),
        ("code", &query.code),
        ("redirect_uri", redirect_uri.as_str()),
        ("code_verifier", &flow.code_verifier),
        (
            "client_assertion_type",
            "urn:ietf:params:oauth:client-assertion-type:jwt-bearer",
        ),
        ("client_assertion", &client_assertion),
    ];

    let resp = state
        .http_client
        .post(token_endpoint)
        .header("DPoP", &dpop_proof)
        .form(&token_params)
        .send()
        .await
        .map_err(|e| OAuthError::Internal(format!("token exchange: {}", e)))?;

    // Handle DPoP nonce retry on token exchange too
    let (token_response, final_nonce) = if resp.status() == StatusCode::BAD_REQUEST {
        let nonce = resp
            .headers()
            .get("dpop-nonce")
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        if let Some(ref nonce_str) = nonce {
            tracing::debug!("got dpop-nonce from token endpoint, retrying");

            let client_assertion_retry = state
                .dpop_key
                .create_client_assertion(client_id, &flow.auth_server_issuer)
                .map_err(|e| OAuthError::Internal(format!("client assertion retry: {}", e)))?;

            let dpop_proof_retry = session_dpop_key
                .create_dpop_proof("POST", token_endpoint, Some(nonce_str))
                .map_err(|e| OAuthError::Internal(format!("dpop proof retry: {}", e)))?;

            let resp_retry = state
                .http_client
                .post(token_endpoint)
                .header("DPoP", &dpop_proof_retry)
                .form(&[
                    ("grant_type", "authorization_code"),
                    ("client_id", client_id.as_str()),
                    ("code", &query.code),
                    ("redirect_uri", redirect_uri.as_str()),
                    ("code_verifier", &flow.code_verifier),
                    (
                        "client_assertion_type",
                        "urn:ietf:params:oauth:client-assertion-type:jwt-bearer",
                    ),
                    ("client_assertion", &client_assertion_retry),
                ])
                .send()
                .await
                .map_err(|e| OAuthError::Internal(format!("token exchange retry: {}", e)))?;

            let updated_nonce = resp_retry
                .headers()
                .get("dpop-nonce")
                .and_then(|v| v.to_str().ok())
                .map(String::from)
                .or(nonce);

            if !resp_retry.status().is_success() {
                let status = resp_retry.status();
                let body = resp_retry.text().await.unwrap_or_default();
                return Err(OAuthError::TokenExchangeFailed(format!(
                    "HTTP {} - {}",
                    status, body
                )));
            }

            let token: TokenResponse = resp_retry
                .json()
                .await
                .map_err(|e| OAuthError::Internal(format!("parse token response: {}", e)))?;

            (token, updated_nonce)
        } else {
            let body = resp.text().await.unwrap_or_default();
            return Err(OAuthError::TokenExchangeFailed(format!(
                "400 without dpop-nonce: {}",
                body
            )));
        }
    } else if resp.status().is_success() {
        let nonce = resp
            .headers()
            .get("dpop-nonce")
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        let token: TokenResponse = resp
            .json()
            .await
            .map_err(|e| OAuthError::Internal(format!("parse token response: {}", e)))?;

        (token, nonce.or(flow.dpop_nonce))
    } else {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(OAuthError::TokenExchangeFailed(format!(
            "HTTP {} - {}",
            status, body
        )));
    };

    // Step 5: Verify token response
    if token_response.token_type.to_lowercase() != "dpop" {
        return Err(OAuthError::TokenVerification(format!(
            "expected token_type DPoP, got {}",
            token_response.token_type
        )));
    }

    if token_response.sub != flow.expected_did {
        return Err(OAuthError::TokenVerification(format!(
            "sub mismatch: expected {}, got {}",
            flow.expected_did, token_response.sub
        )));
    }

    // Step 6: Create session
    let session_id = uuid::Uuid::new_v4().to_string();
    let expires_at = chrono::Utc::now()
        + chrono::Duration::seconds(token_response.expires_in.unwrap_or(7200) as i64);

    let granted_scope = token_response
        .scope
        .clone()
        .unwrap_or_else(|| flow.requested_scope.clone());
    let session = Session {
        did: token_response.sub.clone(),
        handle: flow.handle.clone(),
        access_token: token_response.access_token,
        refresh_token: token_response.refresh_token,
        dpop_private_key_b64: session_dpop_key.private_key_b64,
        auth_server_issuer: flow.auth_server_issuer,
        pds_url: flow.pds_url,
        dpop_nonce: final_nonce,
        expires_at,
        created_at: chrono::Utc::now(),
        scope: granted_scope,
    };

    state
        .session_store
        .put_session(&session_id, session)
        .await
        .map_err(|e| OAuthError::Internal(format!("store session: {}", e)))?;

    tracing::info!(
        did = %token_response.sub,
        "OAuth login successful, session created"
    );

    // Step 7: Set httpOnly cookie and redirect to frontend
    let cookie_value = format!(
        "{}={}; HttpOnly; SameSite=Lax; Path=/; Max-Age=604800{}",
        SESSION_COOKIE,
        session_id,
        if state.oauth_config.public_url.starts_with("https://") {
            "; Secure"
        } else {
            ""
        }
    );

    let redirect_url = format!("{}/", state.oauth_config.public_url);

    Ok((
        StatusCode::FOUND,
        [
            (header::SET_COOKIE, cookie_value),
            (header::LOCATION, redirect_url),
        ],
    )
        .into_response())
}

// -- POST /oauth/logout --

async fn logout(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<impl IntoResponse, OAuthError> {
    if let Some(session_id) = extract_session_id(&headers) {
        state
            .session_store
            .delete_session(&session_id)
            .await
            .map_err(|e| OAuthError::Internal(format!("delete session: {}", e)))?;

        tracing::info!("session destroyed");
    }

    // Clear the cookie
    let clear_cookie = format!(
        "{}=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0{}",
        SESSION_COOKIE,
        if state.oauth_config.public_url.starts_with("https://") {
            "; Secure"
        } else {
            ""
        }
    );

    Ok((
        StatusCode::OK,
        [(header::SET_COOKIE, clear_cookie)],
        Json(json!({ "ok": true })),
    ))
}

// -- GET /oauth/session --

#[derive(Serialize)]
struct SessionResponse {
    authenticated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    did: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    handle: Option<String>,
    /// Raw granted scope string from the PDS (space-separated tokens).
    /// Frontend parses this to gate intent upgrades.
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<String>,
}

async fn session_info(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<SessionResponse>, OAuthError> {
    let session_id = match extract_session_id(&headers) {
        Some(id) => id,
        None => {
            return Ok(Json(SessionResponse {
                authenticated: false,
                did: None,
                handle: None,
                scope: None,
            }));
        }
    };

    let session = state
        .session_store
        .get_session(&session_id)
        .await
        .map_err(|e| OAuthError::Internal(format!("get session: {}", e)))?;

    match session {
        Some(s) => Ok(Json(SessionResponse {
            authenticated: true,
            did: Some(s.did),
            handle: s.handle,
            scope: if s.scope.is_empty() {
                None
            } else {
                Some(s.scope)
            },
        })),
        None => Ok(Json(SessionResponse {
            authenticated: false,
            did: None,
            handle: None,
            scope: None,
        })),
    }
}

// -- Helpers --

/// Extract the session ID from the Cookie header.
pub fn extract_session_id(headers: &axum::http::HeaderMap) -> Option<String> {
    let cookie_header = headers.get(header::COOKIE)?.to_str().ok()?;
    for cookie in cookie_header.split(';') {
        let cookie = cookie.trim();
        if let Some(value) = cookie.strip_prefix(&format!("{}=", SESSION_COOKIE)) {
            let value = value.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

// -- Response types --

#[derive(Deserialize)]
struct ParResponse {
    request_uri: String,
    #[allow(dead_code)]
    expires_in: Option<u64>,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    #[allow(dead_code)]
    expires_in: Option<u64>,
    refresh_token: String,
    scope: Option<String>,
    sub: String,
}

// -- Error type --

#[derive(Debug, thiserror::Error)]
pub enum OAuthError {
    #[error("identity resolution failed: {0}")]
    IdentityResolution(String),

    #[error("auth server discovery failed: {0}")]
    AuthServerDiscovery(String),

    #[error("PAR request failed: {0}")]
    ParFailed(String),

    #[error("invalid or expired state parameter")]
    InvalidState,

    #[error("issuer mismatch in callback")]
    IssuerMismatch,

    #[error("token exchange failed: {0}")]
    TokenExchangeFailed(String),

    #[error("token verification failed: {0}")]
    TokenVerification(String),

    #[error("authorization denied: {0}")]
    AuthorizationDenied(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl IntoResponse for OAuthError {
    fn into_response(self) -> Response {
        let (status, error_name) = match &self {
            OAuthError::IdentityResolution(_) => {
                (StatusCode::BAD_REQUEST, "IdentityResolutionFailed")
            }
            OAuthError::AuthServerDiscovery(_) => {
                (StatusCode::BAD_GATEWAY, "AuthServerDiscoveryFailed")
            }
            OAuthError::ParFailed(_) => (StatusCode::BAD_GATEWAY, "PARFailed"),
            OAuthError::InvalidState => (StatusCode::BAD_REQUEST, "InvalidState"),
            OAuthError::IssuerMismatch => (StatusCode::BAD_REQUEST, "IssuerMismatch"),
            OAuthError::TokenExchangeFailed(_) => (StatusCode::BAD_GATEWAY, "TokenExchangeFailed"),
            OAuthError::TokenVerification(_) => {
                (StatusCode::BAD_GATEWAY, "TokenVerificationFailed")
            }
            OAuthError::AuthorizationDenied(_) => (StatusCode::FORBIDDEN, "AuthorizationDenied"),
            OAuthError::Internal(msg) => {
                tracing::error!(error = %msg, "internal OAuth error");
                (StatusCode::INTERNAL_SERVER_ERROR, "InternalError")
            }
        };

        (
            status,
            Json(json!({
                "error": error_name,
                "message": self.to_string(),
            })),
        )
            .into_response()
    }
}

// -- Bridge: connect browser OAuth to server-side session --
//
// The frontend uses @atproto/oauth-client-browser which stores tokens
// in IndexedDB. Server-side features (form actions, SSR) need a
// session cookie. This endpoint bridges the two: the frontend POSTs
// the authenticated DID after browser OAuth completes, and we create
// a lightweight server-side session + cookie.

#[derive(Deserialize)]
struct BridgeInput {
    did: String,
    handle: Option<String>,
}

async fn bridge_session(
    State(state): State<Arc<AppState>>,
    Json(input): Json<BridgeInput>,
) -> impl IntoResponse {
    let session_id = uuid::Uuid::new_v4().to_string();
    let session = Session {
        did: input.did.clone(),
        handle: input.handle.clone(),
        // Bridge sessions don't have PDS tokens (the browser has them).
        // They only exist so the server can identify the user for form
        // actions like createPushToken.
        access_token: String::new(),
        refresh_token: String::new(),
        dpop_private_key_b64: String::new(),
        auth_server_issuer: String::new(),
        pds_url: String::new(),
        dpop_nonce: None,
        expires_at: chrono::Utc::now() + chrono::Duration::days(7),
        created_at: chrono::Utc::now(),
        scope: String::new(),
    };

    if let Err(e) = state.session_store.put_session(&session_id, session).await {
        tracing::error!(error = %e, "bridge: failed to store session");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "session store failed"})),
        )
            .into_response();
    }

    tracing::info!(did = %input.did, "bridge: created server session");

    let cookie_value = format!(
        "{}={}; HttpOnly; SameSite=Lax; Path=/; Max-Age=604800{}",
        SESSION_COOKIE,
        session_id,
        if state.oauth_config.public_url.starts_with("https://") {
            "; Secure"
        } else {
            ""
        }
    );

    (
        StatusCode::OK,
        [(header::SET_COOKIE, cookie_value)],
        Json(json!({"ok": true, "did": input.did})),
    )
        .into_response()
}

async fn bridge_delete(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    if let Some(session_id) = extract_session_id(&headers) {
        let _ = state.session_store.delete_session(&session_id).await;
    }
    let clear_cookie = format!(
        "{}=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0{}",
        SESSION_COOKIE,
        if state.oauth_config.public_url.starts_with("https://") {
            "; Secure"
        } else {
            ""
        }
    );
    (
        StatusCode::OK,
        [(header::SET_COOKIE, clear_cookie)],
        Json(json!({"ok": true})),
    )
}
