use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use jsonwebtoken::{Algorithm, DecodingKey, TokenData, Validation};
use serde::Deserialize;

use crate::error::NodeError;
use crate::state::NodeState;

/// JWT claims expected in the Bearer token.
#[derive(Debug, Deserialize)]
struct JwtClaims {
    /// The DID of the authenticated user (subject claim).
    sub: String,
    /// Token expiration time (Unix timestamp).
    #[serde(default)]
    exp: Option<u64>,
}

/// Axum extractor that verifies DID authentication from the Authorization header.
///
/// Parses the Bearer token as a JWT, extracts the `sub` claim as the DID,
/// verifies expiration, and validates the signature against the public key
/// from the DID document (fetched from plc.directory or .well-known/did.json).
///
/// For development, raw DID strings (e.g., `Bearer did:plc:xyz`) are accepted
/// as a fallback when the `COSPAN_DEV_AUTH` env var is set.
pub struct DidAuth {
    pub did: String,
}

impl FromRequestParts<std::sync::Arc<NodeState>> for DidAuth {
    type Rejection = NodeError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &std::sync::Arc<NodeState>,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| NodeError::Unauthorized("missing Authorization header".into()))?;

        let did = if let Some(token) = auth_header.strip_prefix("Bearer ") {
            let token = token.trim();
            extract_did_from_bearer(token, state).await?
        } else if let Some(token) = auth_header.strip_prefix("DPoP ") {
            let token = token.trim();
            extract_did_from_bearer(token, state).await?
        } else {
            return Err(NodeError::Unauthorized(
                "invalid Authorization scheme".into(),
            ));
        };

        if !did.starts_with("did:") {
            return Err(NodeError::Unauthorized("invalid DID format".into()));
        }

        Ok(DidAuth { did })
    }
}

/// DID document structure from plc.directory or .well-known/did.json.
#[derive(Debug, Deserialize)]
struct DidDocument {
    #[serde(default)]
    _id: String,
    #[serde(default, rename = "verificationMethod")]
    verification_method: Vec<VerificationMethod>,
}

#[derive(Debug, Deserialize)]
struct VerificationMethod {
    #[serde(default)]
    _id: String,
    #[serde(default, rename = "type")]
    _key_type: String,
    #[serde(default, rename = "publicKeyMultibase")]
    public_key_multibase: Option<String>,
    #[serde(default, rename = "publicKeyJwk")]
    public_key_jwk: Option<serde_json::Value>,
}

/// Extract the DID from a Bearer token with full verification.
async fn extract_did_from_bearer(
    token: &str,
    state: &std::sync::Arc<NodeState>,
) -> Result<String, NodeError> {
    // Dev mode: accept raw DID strings when COSPAN_DEV_AUTH is set.
    let dev_auth = std::env::var("COSPAN_DEV_AUTH").is_ok();
    if dev_auth && token.starts_with("did:") && !token.contains('.') {
        tracing::debug!(did = token, "dev mode: accepting raw DID as auth");
        return Ok(token.to_string());
    }

    // Decode JWT header to get the algorithm, then decode claims without
    // verifying signature first to extract the DID (sub claim).
    let header = jsonwebtoken::decode_header(token)
        .map_err(|e| NodeError::Unauthorized(format!("invalid JWT header: {e}")))?;

    let algorithm = header.alg;

    // Decode without verification to extract the DID for key lookup.
    let mut unverified = Validation::new(algorithm);
    unverified.insecure_disable_signature_validation();
    unverified.set_required_spec_claims(&["sub"]);
    unverified.validate_exp = false;
    let dummy_key = DecodingKey::from_secret(b"unused");
    let unverified_data: TokenData<JwtClaims> =
        jsonwebtoken::decode(token, &dummy_key, &unverified)
            .map_err(|e| NodeError::Unauthorized(format!("invalid JWT: {e}")))?;

    let did = &unverified_data.claims.sub;
    if !did.starts_with("did:") {
        return Err(NodeError::Unauthorized(
            "JWT sub claim is not a valid DID".into(),
        ));
    }

    // Push tokens from the appview have a `kid` in the header and are
    // signed by the appview's DPoP key. Verify against the appview's
    // JWKS endpoint instead of the user's DID document.
    let decoding_key = if let Some(ref kid) = header.kid {
        if let Some(key) = try_appview_jwks(state, kid, algorithm).await {
            tracing::debug!(kid, "verified push token via appview JWKS");
            key
        } else {
            // Fall back to DID document resolution.
            resolve_key_from_did(did, algorithm).await?
        }
    } else {
        resolve_key_from_did(did, algorithm).await?
    };

    // Verify the signature with the real key.
    let mut verified_validation = Validation::new(algorithm);
    verified_validation.set_required_spec_claims(&["sub"]);
    verified_validation.validate_exp = true;
    verified_validation.leeway = 60;

    let verified: TokenData<JwtClaims> =
        jsonwebtoken::decode(token, &decoding_key, &verified_validation).map_err(|e| {
            NodeError::Unauthorized(format!("JWT signature verification failed: {e}"))
        })?;

    // Check expiration.
    if let Some(exp) = verified.claims.exp {
        let now = chrono::Utc::now().timestamp() as u64;
        if now > exp + 60 {
            return Err(NodeError::Unauthorized("token expired".into()));
        }
    }

    tracing::debug!(did = %verified.claims.sub, "authenticated via verified JWT");
    Ok(verified.claims.sub)
}

/// Resolve the signing key from the user's DID document.
async fn resolve_key_from_did(did: &str, algorithm: Algorithm) -> Result<DecodingKey, NodeError> {
    let did_doc = resolve_did_document(did)
        .await
        .map_err(|e| NodeError::Unauthorized(format!("failed to resolve DID {did}: {e}")))?;

    find_decoding_key(&did_doc, algorithm).ok_or_else(|| {
        NodeError::Unauthorized(format!(
            "no suitable verification method found in DID document for {did} (algorithm: {algorithm:?})"
        ))
    })
}

/// Try to verify against the appview's JWKS endpoint.
///
/// Returns the matching `DecodingKey` if the JWKS URL is configured
/// and a key with the given `kid` and algorithm is found.
async fn try_appview_jwks(
    state: &NodeState,
    kid: &str,
    algorithm: Algorithm,
) -> Option<DecodingKey> {
    let jwks_url = state
        .config
        .auth
        .appview_jwks_url
        .as_deref()
        .or_else(|| {
            // Default: derive from APPVIEW_URL env var.
            None
        })?;

    let http = reqwest::Client::new();
    let resp = http.get(jwks_url).send().await.ok()?;
    if !resp.status().is_success() {
        tracing::warn!(url = jwks_url, status = %resp.status(), "JWKS fetch failed");
        return None;
    }

    let jwks: JwksDocument = resp.json().await.ok()?;
    for key in &jwks.keys {
        if key.kid.as_deref() == Some(kid) {
            if let Some(decoding_key) = try_decoding_key_from_jwk(&key.raw, algorithm) {
                return Some(decoding_key);
            }
        }
    }

    tracing::warn!(kid, "no matching key in appview JWKS");
    None
}

/// JWKS document structure.
#[derive(Debug, Deserialize)]
struct JwksDocument {
    keys: Vec<JwksKey>,
}

#[derive(Debug, Deserialize)]
struct JwksKey {
    kid: Option<String>,
    #[serde(flatten)]
    raw: serde_json::Value,
}

/// Resolve a DID to its DID document.
async fn resolve_did_document(did: &str) -> Result<DidDocument, String> {
    let http = reqwest::Client::new();

    let url = if did.starts_with("did:plc:") {
        format!("https://plc.directory/{did}")
    } else if did.starts_with("did:web:") {
        let domain = did.strip_prefix("did:web:").unwrap_or(did);
        let domain = domain.replace(':', "/");
        format!("https://{domain}/.well-known/did.json")
    } else {
        return Err(format!("unsupported DID method: {did}"));
    };

    let resp = http
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("DID resolution returned {}", resp.status()));
    }

    resp.json::<DidDocument>()
        .await
        .map_err(|e| format!("failed to parse DID document: {e}"))
}

/// Extract a DecodingKey from the DID document's verification methods.
fn find_decoding_key(doc: &DidDocument, algorithm: Algorithm) -> Option<DecodingKey> {
    for vm in &doc.verification_method {
        // Try JWK-based keys first.
        if let Some(ref jwk) = vm.public_key_jwk
            && let Some(key) = try_decoding_key_from_jwk(jwk, algorithm)
        {
            return Some(key);
        }

        // Try multibase-encoded keys (P-256 compressed points for ES256).
        if let Some(ref multibase) = vm.public_key_multibase
            && let Some(key) = try_decoding_key_from_multibase(multibase, algorithm)
        {
            return Some(key);
        }
    }
    None
}

/// Try to construct a DecodingKey from a JWK value.
fn try_decoding_key_from_jwk(jwk: &serde_json::Value, algorithm: Algorithm) -> Option<DecodingKey> {
    let kty = jwk.get("kty")?.as_str()?;
    let crv = jwk.get("crv").and_then(|v| v.as_str());

    match (algorithm, kty, crv) {
        (Algorithm::ES256, "EC", Some("P-256"))
        | (Algorithm::ES256, "EC", Some("secp256r1"))
        | (Algorithm::ES384, "EC", Some("P-384")) => {
            let x = jwk.get("x")?.as_str()?;
            let y = jwk.get("y")?.as_str()?;
            DecodingKey::from_ec_components(x, y).ok()
        }
        _ => None,
    }
}

/// Try to construct a DecodingKey from a multibase-encoded public key.
///
/// ATProto typically uses `z`-prefixed base58btc multibase encoding with
/// a multicodec prefix for the key type.
fn try_decoding_key_from_multibase(multibase: &str, algorithm: Algorithm) -> Option<DecodingKey> {
    // The `z` prefix indicates base58btc encoding.
    if !multibase.starts_with('z') {
        return None;
    }

    // For now, we handle the most common case: compressed P-256 points.
    // The multicodec prefix for P-256 compressed is 0x1200.
    // Full multibase decoding would require a multibase library.
    // This is a known limitation: JWK keys are preferred.
    let _ = (multibase, algorithm);
    None
}
