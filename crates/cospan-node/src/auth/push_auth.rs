//! Push token verification for git receive-pack.
//!
//! Push tokens are JWTs signed by the appview's ES256 key. The node
//! resolves the appview's JWKS (cached) from the `kid` in the JWT
//! header, verifies the signature, then checks the standard claims.
//!
//! git sends credentials via HTTP Basic Auth:
//!   username = the user's DID
//!   password = the push token JWT
//!
//! We verify:
//!   1. The JWT signature against the appview's JWKS.
//!   2. `sub` matches the DID in the URL path.
//!   3. `scope` permits `rpc:dev.cospan.vcs.push` (legacy `push` also accepted).
//!   4. `exp` > now (token is not expired).

use std::sync::Arc;

use base64::Engine;
use jsonwebtoken::{Algorithm, DecodingKey, TokenData, Validation};
use serde::Deserialize;

use crate::state::NodeState;

/// Result of push token verification.
pub enum PushAuth {
    /// Token is valid; the authenticated DID.
    Authenticated(String),
    /// No credentials provided.
    NoCredentials,
    /// Credentials provided but invalid.
    Denied(String),
}

#[derive(Debug, Deserialize)]
struct PushTokenClaims {
    sub: String,
    #[allow(dead_code)]
    iss: Option<String>,
    scope: Option<String>,
    exp: i64,
    #[allow(dead_code)]
    iat: Option<i64>,
}

/// Extract HTTP Basic Auth credentials from the Authorization header.
///
/// The username is a DID (`did:plc:abc123`) which contains colons.
/// The password is a JWT (starts with `eyJ`). We split at `:eyJ`
/// to find the boundary between DID and token.
fn extract_basic_auth(headers: &axum::http::HeaderMap) -> Option<(String, String)> {
    let auth = headers.get("authorization")?.to_str().ok()?;
    let encoded = auth.strip_prefix("Basic ")?;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()?;
    let text = String::from_utf8(decoded).ok()?;
    if let Some(pos) = text.find(":eyJ") {
        Some((text[..pos].to_string(), text[pos + 1..].to_string()))
    } else {
        let (user, pass) = text.split_once(':')?;
        Some((user.to_string(), pass.to_string()))
    }
}

/// Does the token's `scope` claim permit pushing?
///
/// Accepts either the legacy `"push"` literal or an ATProto granular
/// scope string containing `rpc:dev.cospan.vcs.push` with any audience.
fn scope_permits_push(scope: &str) -> bool {
    if scope == "push" {
        return true;
    }
    // Tokenize space-separated scopes. For each, test whether it is a
    // granular rpc scope naming the push method.
    scope.split_ascii_whitespace().any(|tok| {
        // Strip optional query: `rpc:dev.cospan.vcs.push?aud=...`
        let head = tok.split('?').next().unwrap_or(tok);
        head == "rpc:dev.cospan.vcs.push"
    })
}

/// Verify a push token from git credentials.
///
/// `expected_did` is the DID from the URL path; the token's `sub`
/// must match it.
///
/// In dev mode (`COSPAN_DEV_AUTH=1`), any DID in the username field
/// is accepted without a token.
pub async fn verify_push(
    state: &Arc<NodeState>,
    headers: &axum::http::HeaderMap,
    expected_did: &str,
) -> PushAuth {
    if std::env::var("COSPAN_DEV_AUTH").is_ok() {
        if let Some((username, _)) = extract_basic_auth(headers) {
            if username == expected_did || expected_did.is_empty() {
                return PushAuth::Authenticated(username);
            }
            return PushAuth::Denied(format!(
                "DID mismatch: got {username}, expected {expected_did}"
            ));
        }
        return PushAuth::Authenticated(expected_did.to_string());
    }

    let (username, token) = match extract_basic_auth(headers) {
        Some(pair) => pair,
        None => return PushAuth::NoCredentials,
    };

    if username != expected_did {
        return PushAuth::Denied(format!(
            "DID mismatch: authenticating as {username} but pushing to {expected_did}"
        ));
    }

    // Parse the header to find the `kid` and algorithm, then locate the
    // matching key in the appview JWKS.
    let header = match jsonwebtoken::decode_header(&token) {
        Ok(h) => h,
        Err(e) => return PushAuth::Denied(format!("invalid JWT header: {e}")),
    };

    let algorithm = header.alg;
    let kid = match header.kid.as_deref() {
        Some(k) => k,
        None => return PushAuth::Denied("push token missing `kid` header".into()),
    };

    let decoding_key = match try_push_token_key(state, kid, algorithm).await {
        Some(k) => k,
        None => {
            return PushAuth::Denied("no matching key in appview JWKS for push token kid".into());
        }
    };

    let mut validation = Validation::new(algorithm);
    validation.set_required_spec_claims(&["sub", "exp"]);
    validation.validate_exp = true;
    validation.leeway = 60;

    let verified: TokenData<PushTokenClaims> =
        match jsonwebtoken::decode(&token, &decoding_key, &validation) {
            Ok(d) => d,
            Err(e) => {
                return PushAuth::Denied(format!("JWT signature verification failed: {e}"));
            }
        };

    let claims = verified.claims;

    let now = chrono::Utc::now().timestamp();
    if claims.exp < now {
        return PushAuth::Denied(format!(
            "push token expired (exp={}, now={now})",
            claims.exp
        ));
    }

    match claims.scope.as_deref() {
        Some(s) if scope_permits_push(s) => {}
        Some(s) => return PushAuth::Denied(format!("token scope does not permit push: {s}")),
        None => return PushAuth::Denied("token missing scope claim".into()),
    }

    if claims.sub != expected_did {
        return PushAuth::Denied(format!(
            "token sub ({}) does not match target DID ({expected_did})",
            claims.sub
        ));
    }

    PushAuth::Authenticated(claims.sub)
}

async fn try_push_token_key(
    state: &Arc<NodeState>,
    kid: &str,
    algorithm: Algorithm,
) -> Option<DecodingKey> {
    super::did_auth::try_appview_jwks(state, kid, algorithm).await
}

#[cfg(test)]
mod tests {
    use super::scope_permits_push;

    #[test]
    fn legacy_push_scope_accepted() {
        assert!(scope_permits_push("push"));
    }

    #[test]
    fn granular_push_scope_accepted() {
        assert!(scope_permits_push("rpc:dev.cospan.vcs.push?aud=*"));
        assert!(scope_permits_push(
            "rpc:dev.cospan.vcs.push?aud=did:web:node.cospan.dev"
        ));
        assert!(scope_permits_push("atproto rpc:dev.cospan.vcs.push?aud=*"));
    }

    #[test]
    fn other_scopes_rejected() {
        assert!(!scope_permits_push("atproto"));
        assert!(!scope_permits_push("rpc:dev.cospan.repo.get?aud=*"));
        assert!(!scope_permits_push(""));
    }
}
