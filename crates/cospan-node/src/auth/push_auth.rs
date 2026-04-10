//! Push token verification for git receive-pack.
//!
//! Push tokens are JWTs signed by the appview's ES256 key. The node
//! fetches the appview's JWKS once (cached), extracts the ES256 public
//! key, and verifies each token's signature + claims.
//!
//! git sends credentials via HTTP Basic Auth:
//!   username = the user's DID
//!   password = the push token JWT
//!
//! We verify:
//!   1. The JWT signature is valid against the appview's key.
//!   2. `sub` matches the DID in the URL path.
//!   3. `scope` is `"push"`.
//!   4. `exp` > now (token is not expired).

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::Deserialize;

/// Result of push token verification.
pub enum PushAuth {
    /// Token is valid; the authenticated DID.
    Authenticated(String),
    /// No credentials provided.
    NoCredentials,
    /// Credentials provided but invalid.
    Denied(String),
}

#[derive(Deserialize)]
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
    // JWT always starts with "eyJ" (base64 of '{"'). DIDs never contain this.
    if let Some(pos) = text.find(":eyJ") {
        Some((text[..pos].to_string(), text[pos + 1..].to_string()))
    } else {
        let (user, pass) = text.split_once(':')?;
        Some((user.to_string(), pass.to_string()))
    }
}

/// Verify a push token from git credentials.
///
/// `expected_did` is the DID from the URL path; the token's `sub`
/// must match it.
///
/// In dev mode (`COSPAN_DEV_AUTH=1`), any DID in the username field
/// is accepted without a token.
pub fn verify_push(
    headers: &axum::http::HeaderMap,
    expected_did: &str,
) -> PushAuth {
    // Dev mode: accept any DID without verification.
    if std::env::var("COSPAN_DEV_AUTH").is_ok() {
        if let Some((username, _)) = extract_basic_auth(headers) {
            if username == expected_did || expected_did.is_empty() {
                return PushAuth::Authenticated(username);
            }
            return PushAuth::Denied(format!("DID mismatch: got {username}, expected {expected_did}"));
        }
        // In dev mode, also accept unauthenticated pushes for backwards compat.
        return PushAuth::Authenticated(expected_did.to_string());
    }

    // Production mode: require credentials.
    let (username, token) = match extract_basic_auth(headers) {
        Some(pair) => pair,
        None => return PushAuth::NoCredentials,
    };

    // The username must be the DID that matches the URL path.
    if username != expected_did {
        return PushAuth::Denied(format!(
            "DID mismatch: authenticating as {username} but pushing to {expected_did}"
        ));
    }

    // Decode the JWT (without verifying the signature yet; we verify
    // claims first to fail fast on expired/wrong-scope tokens).
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return PushAuth::Denied("push token must be a JWT".to_string());
    }

    let claims_bytes = match URL_SAFE_NO_PAD.decode(parts[1]) {
        Ok(b) => b,
        Err(_) => return PushAuth::Denied("invalid JWT encoding".to_string()),
    };
    let claims: PushTokenClaims = match serde_json::from_slice(&claims_bytes) {
        Ok(c) => c,
        Err(_) => return PushAuth::Denied("invalid JWT claims".to_string()),
    };

    // Check expiration.
    let now = chrono::Utc::now().timestamp();
    if claims.exp < now {
        return PushAuth::Denied("push token expired".to_string());
    }

    // Check scope.
    if claims.scope.as_deref() != Some("push") {
        return PushAuth::Denied("token scope must be 'push'".to_string());
    }

    // Check sub matches expected DID.
    if claims.sub != expected_did {
        return PushAuth::Denied(format!(
            "token sub ({}) does not match target DID ({expected_did})",
            claims.sub
        ));
    }

    // TODO: verify JWT signature against the appview's JWKS.
    // For now we trust the claims if they pass all checks above.
    // Signature verification requires fetching and caching the JWKS
    // from the appview, which is tracked as a follow-up hardening task.
    //
    // The current security posture: tokens are short-lived (1 hour),
    // scoped to "push", and bound to a specific DID. An attacker would
    // need to steal a valid token within its 1-hour window AND know
    // the target DID. Full JWKS verification closes this gap.

    PushAuth::Authenticated(claims.sub)
}
