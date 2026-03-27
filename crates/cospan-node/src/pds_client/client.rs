use async_trait::async_trait;

use crate::error::NodeError;

/// Client for creating records in a user's PDS.
#[async_trait]
pub trait PdsClient: Send + Sync {
    async fn create_record(
        &self,
        did: &str,
        collection: &str,
        record: &serde_json::Value,
    ) -> Result<String, NodeError>;
}

/// HTTP-based PDS client.
///
/// Resolves the user's PDS URL from their DID (via plc.directory for did:plc),
/// then POSTs to `{pds_url}/xrpc/com.atproto.repo.createRecord`.
///
/// Authentication uses a Bearer token sourced from the `PDS_ACCESS_TOKEN`
/// environment variable for Phase 0. Phase 1 will implement the full OAuth
/// DPoP flow with token management.
pub struct HttpPdsClient {
    http: reqwest::Client,
    /// Cached access token from the PDS_ACCESS_TOKEN env var.
    access_token: Option<String>,
}

impl Default for HttpPdsClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpPdsClient {
    pub fn new() -> Self {
        let access_token = std::env::var("PDS_ACCESS_TOKEN").ok();
        if access_token.is_none() {
            tracing::warn!(
                "PDS_ACCESS_TOKEN not set; PDS record creation will fail unless token is provided"
            );
        }
        Self {
            http: reqwest::Client::new(),
            access_token,
        }
    }
}

/// Response from plc.directory DID resolution.
#[derive(serde::Deserialize)]
struct PlcDocument {
    #[serde(default)]
    service: Vec<PlcService>,
}

#[derive(serde::Deserialize)]
struct PlcService {
    #[serde(rename = "type")]
    service_type: String,
    #[serde(rename = "serviceEndpoint")]
    service_endpoint: String,
}

/// Response from com.atproto.repo.createRecord.
#[derive(serde::Deserialize)]
struct CreateRecordResponse {
    uri: String,
}

/// Resolve a DID to its PDS service URL.
///
/// For `did:plc:*`, queries `https://plc.directory/{did}` and extracts the
/// `AtprotoPersonalDataServer` service endpoint.
///
/// For `did:web:*`, constructs the URL from the domain and fetches
/// `/.well-known/did.json`.
async fn resolve_pds_url(http: &reqwest::Client, did: &str) -> Result<String, NodeError> {
    if did.starts_with("did:plc:") {
        let url = format!("https://plc.directory/{did}");
        let resp = http.get(&url).send().await.map_err(|e| {
            NodeError::Internal(format!("failed to resolve DID via plc.directory: {e}"))
        })?;

        if !resp.status().is_success() {
            return Err(NodeError::Internal(format!(
                "plc.directory returned {} for {did}",
                resp.status()
            )));
        }

        let doc: PlcDocument = resp.json().await.map_err(|e| {
            NodeError::Internal(format!("failed to parse plc.directory response: {e}"))
        })?;

        let pds_service = doc
            .service
            .iter()
            .find(|s| s.service_type == "AtprotoPersonalDataServer")
            .ok_or_else(|| {
                NodeError::Internal(format!(
                    "no AtprotoPersonalDataServer service found in DID document for {did}"
                ))
            })?;

        Ok(pds_service.service_endpoint.clone())
    } else if did.starts_with("did:web:") {
        // did:web:example.com → https://example.com/.well-known/did.json
        let domain = did.strip_prefix("did:web:").unwrap_or(did); // strip "did:web:"
        let domain = domain.replace(':', "/"); // colons become path separators
        let url = format!("https://{domain}/.well-known/did.json");

        let resp = http.get(&url).send().await.map_err(|e| {
            NodeError::Internal(format!("failed to resolve did:web DID document: {e}"))
        })?;

        if !resp.status().is_success() {
            return Err(NodeError::Internal(format!(
                "did:web resolution returned {} for {did}",
                resp.status()
            )));
        }

        let doc: PlcDocument = resp
            .json()
            .await
            .map_err(|e| NodeError::Internal(format!("failed to parse did:web document: {e}")))?;

        let pds_service = doc
            .service
            .iter()
            .find(|s| s.service_type == "AtprotoPersonalDataServer")
            .ok_or_else(|| {
                NodeError::Internal(format!(
                    "no AtprotoPersonalDataServer service found in DID document for {did}"
                ))
            })?;

        Ok(pds_service.service_endpoint.clone())
    } else {
        Err(NodeError::Internal(format!(
            "unsupported DID method for PDS resolution: {did}"
        )))
    }
}

#[async_trait]
impl PdsClient for HttpPdsClient {
    async fn create_record(
        &self,
        did: &str,
        collection: &str,
        record: &serde_json::Value,
    ) -> Result<String, NodeError> {
        let token = self.access_token.as_deref().ok_or_else(|| {
            NodeError::Internal(
                "PDS_ACCESS_TOKEN not set; cannot create records on user's PDS".into(),
            )
        })?;

        // Step 1: Resolve the user's PDS URL from their DID.
        let pds_url = resolve_pds_url(&self.http, did).await?;
        tracing::debug!(did = did, pds_url = %pds_url, "resolved PDS URL");

        // Step 2: POST to com.atproto.repo.createRecord.
        let endpoint = format!(
            "{}/xrpc/com.atproto.repo.createRecord",
            pds_url.trim_end_matches('/')
        );

        let body = serde_json::json!({
            "repo": did,
            "collection": collection,
            "record": record,
        });

        let resp = self
            .http
            .post(&endpoint)
            .header("Authorization", format!("Bearer {token}"))
            .json(&body)
            .send()
            .await
            .map_err(|e| NodeError::Internal(format!("failed to POST to PDS createRecord: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let error_body = resp.text().await.unwrap_or_default();
            return Err(NodeError::Internal(format!(
                "PDS createRecord returned {status}: {error_body}"
            )));
        }

        // Step 3: Parse the response to extract the AT-URI.
        let create_resp: CreateRecordResponse = resp.json().await.map_err(|e| {
            NodeError::Internal(format!("failed to parse PDS createRecord response: {e}"))
        })?;

        tracing::info!(
            did = did,
            collection = collection,
            uri = %create_resp.uri,
            "PDS record created"
        );

        Ok(create_resp.uri)
    }
}
