use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use p256::SecretKey;
use p256::ecdsa::SigningKey;
use p256::elliptic_curve::sec1::ToEncodedPoint;
use rand::rngs::OsRng;
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};

/// An ES256 key pair used for DPoP proofs and client assertions.
#[derive(Clone)]
pub struct DpopKey {
    /// The ES256 signing key.
    signing_key: SigningKey,
    /// The public key as a JWK value (for embedding in DPoP JWT headers).
    public_jwk: serde_json::Value,
    /// Key ID for client assertions (JWKS).
    pub kid: String,
    /// SEC1 DER-encoded private key bytes, base64url-encoded (for serialization into sessions).
    pub private_key_b64: String,
}

impl std::fmt::Debug for DpopKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DpopKey")
            .field("kid", &self.kid)
            .finish_non_exhaustive()
    }
}

impl DpopKey {
    /// Generate a fresh ES256 key pair.
    pub fn generate() -> Self {
        let secret = SecretKey::random(&mut OsRng);
        Self::from_secret_key(secret, "cospan-dpop-1".to_string())
    }

    /// Generate a per-session DPoP key pair (with a unique kid).
    pub fn generate_session_key() -> Self {
        let secret = SecretKey::random(&mut OsRng);
        let kid = format!("session-{}", uuid::Uuid::new_v4());
        Self::from_secret_key(secret, kid)
    }

    fn from_secret_key(secret: SecretKey, kid: String) -> Self {
        let signing_key = SigningKey::from(&secret);
        let public_key = secret.public_key();

        // Build the JWK from the public key's uncompressed point
        let point = public_key.to_encoded_point(false);
        let x_bytes = point.x().expect("x coordinate");
        let y_bytes = point.y().expect("y coordinate");
        let public_jwk = json!({
            "kty": "EC",
            "crv": "P-256",
            "x": URL_SAFE_NO_PAD.encode(x_bytes),
            "y": URL_SAFE_NO_PAD.encode(y_bytes),
        });

        let private_key_b64 = URL_SAFE_NO_PAD.encode(secret.to_sec1_der().expect("sec1 encoding"));

        Self {
            signing_key,
            public_jwk,
            kid,
            private_key_b64,
        }
    }

    /// Reconstruct a DpopKey from a base64url-encoded SEC1 DER private key.
    pub fn from_private_key_b64(b64: &str, kid: String) -> anyhow::Result<Self> {
        let der_bytes = URL_SAFE_NO_PAD.decode(b64)?;
        let secret = SecretKey::from_sec1_der(&der_bytes)?;
        Ok(Self::from_secret_key(secret, kid))
    }

    /// Get the public key as a JWK JSON value.
    pub fn public_jwk(&self) -> &serde_json::Value {
        &self.public_jwk
    }

    /// Get the full JWKS document (for serving at /.well-known/jwks.json).
    pub fn jwks_document(&self) -> serde_json::Value {
        let mut key = self.public_jwk.clone();
        if let serde_json::Value::Object(ref mut map) = key {
            map.insert("use".to_string(), json!("sig"));
            map.insert("alg".to_string(), json!("ES256"));
            map.insert("kid".to_string(), json!(self.kid));
        }
        json!({ "keys": [key] })
    }

    /// Create a DPoP proof JWT for the given HTTP method and URL.
    ///
    /// The JWT is signed with this key pair and includes the public key in the header.
    /// If a nonce is provided (from a previous server response), it is included in the payload.
    pub fn create_dpop_proof(
        &self,
        method: &str,
        url: &str,
        nonce: Option<&str>,
    ) -> anyhow::Result<String> {
        let header = DpopHeader {
            alg: "ES256".to_string(),
            typ: "dpop+jwt".to_string(),
            jwk: self.public_jwk.clone(),
        };

        let claims = DpopClaims {
            jti: uuid::Uuid::new_v4().to_string(),
            htm: method.to_uppercase(),
            htu: url.to_string(),
            iat: chrono::Utc::now().timestamp(),
            nonce: nonce.map(String::from),
            ath: None,
        };

        encode_es256_jwt(&header, &claims, &self.signing_key)
    }

    /// Create a DPoP proof JWT for a resource-server (PDS) request.
    /// Includes the `ath` claim (SHA-256 hash of the access token, base64url).
    pub fn create_resource_proof(
        &self,
        method: &str,
        url: &str,
        access_token: &str,
        nonce: Option<&str>,
    ) -> anyhow::Result<String> {
        let mut hasher = Sha256::new();
        hasher.update(access_token.as_bytes());
        let ath = URL_SAFE_NO_PAD.encode(hasher.finalize());

        let header = DpopHeader {
            alg: "ES256".to_string(),
            typ: "dpop+jwt".to_string(),
            jwk: self.public_jwk.clone(),
        };

        let claims = DpopClaims {
            jti: uuid::Uuid::new_v4().to_string(),
            htm: method.to_uppercase(),
            htu: url.to_string(),
            iat: chrono::Utc::now().timestamp(),
            nonce: nonce.map(String::from),
            ath: Some(ath),
        };

        encode_es256_jwt(&header, &claims, &self.signing_key)
    }

    /// Create a client assertion JWT for token endpoint authentication.
    ///
    /// This is a JWT with iss=sub=client_id, aud=auth_server_issuer, signed with our key.
    pub fn create_client_assertion(
        &self,
        client_id: &str,
        auth_server_issuer: &str,
    ) -> anyhow::Result<String> {
        let header = ClientAssertionHeader {
            alg: "ES256".to_string(),
            typ: "JWT".to_string(),
            kid: self.kid.clone(),
        };

        let now = chrono::Utc::now().timestamp();
        let claims = ClientAssertionClaims {
            iss: client_id.to_string(),
            sub: client_id.to_string(),
            aud: auth_server_issuer.to_string(),
            iat: now,
            exp: now + 60,
            jti: uuid::Uuid::new_v4().to_string(),
        };

        encode_es256_jwt(&header, &claims, &self.signing_key)
    }
}

// -- JWT types --

#[derive(Serialize)]
struct DpopHeader {
    alg: String,
    typ: String,
    jwk: serde_json::Value,
}

#[derive(Serialize)]
struct DpopClaims {
    jti: String,
    htm: String,
    htu: String,
    iat: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    nonce: Option<String>,
    /// Access token hash (SHA-256, base64url): required for resource server requests.
    #[serde(skip_serializing_if = "Option::is_none")]
    ath: Option<String>,
}

#[derive(Serialize)]
struct ClientAssertionHeader {
    alg: String,
    typ: String,
    kid: String,
}

#[derive(Serialize)]
struct ClientAssertionClaims {
    iss: String,
    sub: String,
    aud: String,
    iat: i64,
    exp: i64,
    jti: String,
}

// -- PKCE utilities --

/// Generate a PKCE code verifier (43-128 characters of unreserved URI characters).
pub fn generate_code_verifier() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Compute the S256 code challenge from a code verifier.
pub fn compute_code_challenge(verifier: &str) -> String {
    let hash = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hash)
}

// -- Internal JWT encoding --
//
// We manually encode JWTs because the `jsonwebtoken` crate does not support
// custom header fields like `jwk` (for DPoP) cleanly. ES256 signing with p256
// is straightforward.

/// Sign an arbitrary JWT with a DpopKey. Used by push token issuance.
pub fn encode_es256_jwt_public<H: Serialize, C: Serialize>(
    header: &H,
    claims: &C,
    key: &DpopKey,
) -> anyhow::Result<String> {
    encode_es256_jwt(header, claims, &key.signing_key)
}

fn encode_es256_jwt<H: Serialize, C: Serialize>(
    header: &H,
    claims: &C,
    signing_key: &SigningKey,
) -> anyhow::Result<String> {
    use p256::ecdsa::Signature;
    use p256::ecdsa::signature::Signer;

    let header_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_vec(header)?);
    let claims_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_vec(claims)?);
    let message = format!("{}.{}", header_b64, claims_b64);

    let signature: Signature = signing_key.sign(message.as_bytes());
    let sig_bytes = signature.to_bytes();
    let sig_b64 = URL_SAFE_NO_PAD.encode(sig_bytes);

    Ok(format!("{}.{}", message, sig_b64))
}
