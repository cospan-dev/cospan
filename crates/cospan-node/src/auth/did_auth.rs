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
    _state: &std::sync::Arc<NodeState>,
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

    // Resolve the DID document to get the signing public key.
    let did_doc = resolve_did_document(did)
        .await
        .map_err(|e| NodeError::Unauthorized(format!("failed to resolve DID {did}: {e}")))?;

    // Find a verification method with a JWK public key.
    let decoding_key = find_decoding_key(&did_doc, algorithm)
        .ok_or_else(|| NodeError::Unauthorized(format!(
            "no suitable verification method found in DID document for {did} (algorithm: {algorithm:?})"
        )))?;

    // Now verify the signature with the real key.
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
        (Algorithm::ES256, "EC", Some("P-256")) | (Algorithm::ES256, "EC", Some("secp256r1")) => {
            let x = jwk.get("x")?.as_str()?;
            let y = jwk.get("y")?.as_str()?;

            // Reconstruct the uncompressed EC point (04 || x || y).
            use base64::Engine;
            let x_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(x)
                .ok()?;
            let y_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(y)
                .ok()?;

            let mut ec_point = Vec::with_capacity(1 + x_bytes.len() + y_bytes.len());
            ec_point.push(0x04); // uncompressed point prefix
            ec_point.extend_from_slice(&x_bytes);
            ec_point.extend_from_slice(&y_bytes);

            Some(DecodingKey::from_ec_der(&ec_point))
        }
        // ES384 for P-384 keys (less common in ATProto)
        (Algorithm::ES384, "EC", Some("P-384")) => {
            let x = jwk.get("x")?.as_str()?;
            let y = jwk.get("y")?.as_str()?;

            use base64::Engine;
            let x_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(x)
                .ok()?;
            let y_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(y)
                .ok()?;

            let mut ec_point = Vec::with_capacity(1 + x_bytes.len() + y_bytes.len());
            ec_point.push(0x04);
            ec_point.extend_from_slice(&x_bytes);
            ec_point.extend_from_slice(&y_bytes);

            Some(DecodingKey::from_ec_der(&ec_point))
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
