pub mod did_resolver;
pub mod dpop;
pub mod oauth;
pub mod pds_client;
pub mod scope;
pub mod session;

use serde::{Deserialize, Serialize};

/// OAuth configuration for the ATProto BFF pattern.
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    /// The client_id URL: also where client-metadata.json is hosted.
    pub client_id: String,
    /// The redirect URI for OAuth callbacks.
    pub redirect_uri: String,
    /// The JWKS endpoint URL for client assertion verification.
    pub jwks_uri: String,
    /// Public-facing base URL (e.g. "https://cospan.dev").
    pub public_url: String,
    /// Client display name.
    pub client_name: String,
}

impl OAuthConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let public_url =
            std::env::var("PUBLIC_URL").unwrap_or_else(|_| "http://localhost:3000".into());

        let client_id = std::env::var("OAUTH_CLIENT_ID")
            .unwrap_or_else(|_| format!("{}/oauth/client-metadata.json", public_url));

        let redirect_uri = std::env::var("OAUTH_REDIRECT_URI")
            .unwrap_or_else(|_| format!("{}/oauth/callback", public_url));

        let jwks_uri = std::env::var("OAUTH_JWKS_URI")
            .unwrap_or_else(|_| format!("{}/.well-known/jwks.json", public_url));

        let client_name = std::env::var("OAUTH_CLIENT_NAME").unwrap_or_else(|_| "Cospan".into());

        Ok(Self {
            client_id,
            redirect_uri,
            jwks_uri,
            public_url,
            client_name,
        })
    }
}

/// A user session stored server-side.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// The user's DID (e.g. "did:plc:abc123").
    pub did: String,
    /// The user's handle (for display; may be stale).
    pub handle: Option<String>,
    /// OAuth access token (DPoP-bound).
    pub access_token: String,
    /// OAuth refresh token (single-use).
    pub refresh_token: String,
    /// Serialized DPoP ES256 private key (SEC1 DER bytes, base64-encoded).
    pub dpop_private_key_b64: String,
    /// The authorization server issuer URL.
    pub auth_server_issuer: String,
    /// The user's PDS URL.
    pub pds_url: String,
    /// Current DPoP nonce for the auth server.
    pub dpop_nonce: Option<String>,
    /// When the access token expires.
    pub expires_at: chrono::DateTime<chrono::Utc>,
    /// When the session was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Raw scope string granted by the auth server (space-separated).
    /// Parsed on demand via `auth::scope::parse_scope_string`.
    #[serde(default)]
    pub scope: String,
}

/// Temporary state stored during the OAuth authorization flow (between redirect and callback).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthFlowState {
    /// CSRF state parameter.
    pub state: String,
    /// PKCE code verifier (plaintext).
    pub code_verifier: String,
    /// Serialized DPoP ES256 private key (SEC1 DER bytes, base64).
    pub dpop_private_key_b64: String,
    /// DPoP public key as JWK JSON.
    pub dpop_public_jwk: serde_json::Value,
    /// Authorization server issuer URL.
    pub auth_server_issuer: String,
    /// Token endpoint URL.
    pub token_endpoint: String,
    /// The DID we expect to receive back in the token response `sub`.
    pub expected_did: String,
    /// The user's PDS URL.
    pub pds_url: String,
    /// The user's handle.
    pub handle: Option<String>,
    /// Current DPoP nonce for this auth server.
    pub dpop_nonce: Option<String>,
    /// When this flow state expires (short-lived, ~10 minutes).
    pub expires_at: chrono::DateTime<chrono::Utc>,
    /// Scope string sent with the PAR request (so we can verify the
    /// auth server doesn't narrow it beyond what we asked for).
    #[serde(default)]
    pub requested_scope: String,
}
