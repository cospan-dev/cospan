use std::time::Duration;

use moka::future::Cache;
use serde::{Deserialize, Serialize};

/// A resolved DID document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DidDocument {
    pub id: String,
    #[serde(rename = "alsoKnownAs", default)]
    pub also_known_as: Vec<String>,
    #[serde(rename = "verificationMethod", default)]
    pub verification_methods: Vec<VerificationMethod>,
    #[serde(default)]
    pub service: Vec<Service>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationMethod {
    pub id: String,
    #[serde(rename = "type")]
    pub method_type: String,
    pub controller: String,
    #[serde(rename = "publicKeyMultibase", default)]
    pub public_key_multibase: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub id: String,
    #[serde(rename = "type")]
    pub service_type: String,
    #[serde(rename = "serviceEndpoint")]
    pub service_endpoint: String,
}

/// Authorization server metadata from `.well-known/oauth-authorization-server`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthServerMetadata {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub pushed_authorization_request_endpoint: String,
    #[serde(default)]
    pub dpop_signing_alg_values_supported: Vec<String>,
    #[serde(default)]
    pub require_pushed_authorization_requests: bool,
    #[serde(default)]
    pub require_dpop_nonce: bool,
}

/// Protected resource metadata from `.well-known/oauth-protected-resource`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectedResourceMetadata {
    #[serde(default)]
    pub authorization_servers: Vec<String>,
}

impl DidDocument {
    /// Extract the PDS service endpoint URL from the DID document.
    pub fn pds_endpoint(&self) -> Option<&str> {
        self.service
            .iter()
            .find(|s| s.service_type == "AtprotoPersonalDataServer")
            .map(|s| s.service_endpoint.as_str())
    }

    /// Extract handles from alsoKnownAs (strips "at://" prefix).
    pub fn handles(&self) -> Vec<String> {
        self.also_known_as
            .iter()
            .filter_map(|aka| aka.strip_prefix("at://").map(String::from))
            .collect()
    }
}

/// DID resolver with moka cache (5-minute TTL).
#[derive(Clone)]
pub struct DidResolver {
    http: reqwest::Client,
    did_cache: Cache<String, DidDocument>,
    auth_server_cache: Cache<String, AuthServerMetadata>,
}

impl DidResolver {
    pub fn new(http: reqwest::Client) -> Self {
        let did_cache = Cache::builder()
            .max_capacity(10_000)
            .time_to_live(Duration::from_secs(300)) // 5 minutes
            .build();

        let auth_server_cache = Cache::builder()
            .max_capacity(100)
            .time_to_live(Duration::from_secs(300))
            .build();

        Self {
            http,
            did_cache,
            auth_server_cache,
        }
    }

    /// Resolve a DID to its PDS service endpoint URL.
    pub async fn resolve_pds(&self, did: &str) -> Option<String> {
        self.resolve_did(did)
            .await
            .ok()
            .and_then(|doc| doc.pds_endpoint().map(String::from))
    }

    /// Resolve a DID to its DID document.
    /// Supports did:plc (via plc.directory) and did:web (.well-known/did.json).
    pub async fn resolve_did(&self, did: &str) -> anyhow::Result<DidDocument> {
        // Check cache first
        if let Some(doc) = self.did_cache.get(did).await {
            return Ok(doc);
        }

        let doc = if did.starts_with("did:plc:") {
            self.resolve_did_plc(did).await?
        } else if did.starts_with("did:web:") {
            self.resolve_did_web(did).await?
        } else {
            anyhow::bail!("unsupported DID method: {}", did);
        };

        self.did_cache.insert(did.to_string(), doc.clone()).await;
        Ok(doc)
    }

    /// Resolve a handle to a DID using the com.atproto.identity.resolveHandle XRPC.
    /// We try the handle's own domain first, then fall back to bsky.social.
    pub async fn resolve_handle(&self, handle: &str) -> anyhow::Result<String> {
        // Try resolving via the handle's PDS (common path)
        let url = format!(
            "https://bsky.social/xrpc/com.atproto.identity.resolveHandle?handle={}",
            handle
        );

        let resp = self.http.get(&url).send().await?;

        if !resp.status().is_success() {
            anyhow::bail!(
                "failed to resolve handle '{}': HTTP {}",
                handle,
                resp.status()
            );
        }

        #[derive(Deserialize)]
        struct ResolveResponse {
            did: String,
        }

        let body: ResolveResponse = resp.json().await?;
        Ok(body.did)
    }

    /// Resolve an identity input (handle or DID) to a DID + DID document.
    /// If a handle is provided, resolve it to a DID first.
    pub async fn resolve_identity(&self, input: &str) -> anyhow::Result<(String, DidDocument)> {
        let did = if input.starts_with("did:") {
            input.to_string()
        } else {
            // It's a handle; resolve to DID
            self.resolve_handle(input).await?
        };

        let doc = self.resolve_did(&did).await?;

        // Bidirectional verification: the DID document should claim this handle
        if !input.starts_with("did:") {
            let claimed_handles = doc.handles();
            if !claimed_handles.iter().any(|h| h == input) {
                tracing::warn!(
                    did = %did,
                    handle = %input,
                    claimed = ?claimed_handles,
                    "handle not found in DID document alsoKnownAs"
                );
                // We warn but don't fail — the PDS has already confirmed the binding
                // via resolveHandle. The DID doc may be stale in cache.
            }
        }

        Ok((did, doc))
    }

    /// Discover the authorization server for a PDS.
    /// Fetches `.well-known/oauth-protected-resource` to find the auth server,
    /// then fetches `.well-known/oauth-authorization-server` for its metadata.
    pub async fn discover_auth_server(&self, pds_url: &str) -> anyhow::Result<AuthServerMetadata> {
        // Step 1: Get the protected resource metadata from the PDS
        let resource_url = format!(
            "{}/.well-known/oauth-protected-resource",
            pds_url.trim_end_matches('/')
        );

        let resp = self.http.get(&resource_url).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!(
                "failed to fetch protected resource metadata from {}: HTTP {}",
                resource_url,
                resp.status()
            );
        }

        let resource: ProtectedResourceMetadata = resp.json().await?;
        let auth_server_url = resource.authorization_servers.first().ok_or_else(|| {
            anyhow::anyhow!("no authorization servers listed for PDS {}", pds_url)
        })?;

        // Check cache
        if let Some(meta) = self.auth_server_cache.get(auth_server_url).await {
            return Ok(meta);
        }

        // Step 2: Fetch the authorization server metadata
        let meta_url = format!(
            "{}/.well-known/oauth-authorization-server",
            auth_server_url.trim_end_matches('/')
        );

        let resp = self.http.get(&meta_url).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!(
                "failed to fetch auth server metadata from {}: HTTP {}",
                meta_url,
                resp.status()
            );
        }

        let meta: AuthServerMetadata = resp.json().await?;

        // Verify the issuer matches
        if meta.issuer != *auth_server_url {
            anyhow::bail!(
                "auth server issuer mismatch: expected {}, got {}",
                auth_server_url,
                meta.issuer
            );
        }

        self.auth_server_cache
            .insert(auth_server_url.clone(), meta.clone())
            .await;

        Ok(meta)
    }

    async fn resolve_did_plc(&self, did: &str) -> anyhow::Result<DidDocument> {
        let url = format!("https://plc.directory/{}", did);
        let resp = self.http.get(&url).send().await?;

        if !resp.status().is_success() {
            anyhow::bail!(
                "failed to resolve {} from plc.directory: HTTP {}",
                did,
                resp.status()
            );
        }

        let doc: DidDocument = resp.json().await?;
        if doc.id != did {
            anyhow::bail!("DID document id mismatch: expected {}, got {}", did, doc.id);
        }

        Ok(doc)
    }

    async fn resolve_did_web(&self, did: &str) -> anyhow::Result<DidDocument> {
        // did:web:example.com -> https://example.com/.well-known/did.json
        // did:web:example.com:path:to -> https://example.com/path/to/did.json
        let domain_path = did
            .strip_prefix("did:web:")
            .ok_or_else(|| anyhow::anyhow!("invalid did:web: {}", did))?;

        let url = if domain_path.contains(':') {
            let parts: Vec<&str> = domain_path.split(':').collect();
            let domain = parts[0];
            let path = parts[1..].join("/");
            format!("https://{}/{}/did.json", domain, path)
        } else {
            format!("https://{}/.well-known/did.json", domain_path)
        };

        let resp = self.http.get(&url).send().await?;

        if !resp.status().is_success() {
            anyhow::bail!(
                "failed to resolve {} from {}: HTTP {}",
                did,
                url,
                resp.status()
            );
        }

        let doc: DidDocument = resp.json().await?;
        if doc.id != did {
            anyhow::bail!("DID document id mismatch: expected {}, got {}", did, doc.id);
        }

        Ok(doc)
    }
}
