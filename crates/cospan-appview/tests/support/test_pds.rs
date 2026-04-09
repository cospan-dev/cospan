//! Minimal in-process ATProto PDS for integration tests.
//!
//! Implements the endpoints needed to exercise authenticated record writes:
//!
//! - `POST /xrpc/com.atproto.repo.createRecord`
//! - `GET  /xrpc/com.atproto.repo.getRecord`
//! - `GET  /xrpc/com.atproto.repo.listRecords`
//! - `POST /xrpc/com.atproto.repo.deleteRecord`
//! - `GET  /xrpc/com.atproto.repo.describeRepo`
//!
//! Authentication: verifies that every mutating request carries
//! `Authorization: DPoP <token>` and a `DPoP: <jwt>` proof header. The proof
//! is decoded as a JWT and its claims are checked for shape (htm, htu, jti,
//! iat, ath). Signature verification is skipped because the test PDS trusts
//! the local session store's keypair — what we want to verify is that our
//! fork handler emits well-formed DPoP proofs, not that libsodium works.
//!
//! Records are stored in memory. Each `TestPds` gets an isolated store so
//! tests don't interfere.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::extract::{Json, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;

/// A running test PDS.
pub struct TestPds {
    pub url: String,
    pub store: Arc<PdsStore>,
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
}

impl TestPds {
    /// Start a new test PDS on an ephemeral port. Returns once the server
    /// is listening and ready to accept requests.
    pub async fn spawn() -> Self {
        let store = Arc::new(PdsStore::default());
        let app = router(store.clone());

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr: SocketAddr = listener.local_addr().unwrap();

        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = rx.await;
                })
                .await
                .unwrap();
        });

        Self {
            url: format!("http://{addr}"),
            store,
            shutdown: Some(tx),
        }
    }

    /// Register a test account. Returns an access token that can be used
    /// with `auth::pds_client::create_record` (via a seeded session).
    pub async fn register_account(&self, did: &str) -> String {
        let token = format!("test-token-{did}");
        self.store.accounts.write().await.insert(
            did.to_string(),
            Account {
                access_token: token.clone(),
            },
        );
        token
    }

    /// Return all records stored for a DID in a given collection.
    pub async fn list_records(&self, did: &str, collection: &str) -> Vec<StoredRecord> {
        self.store
            .records
            .read()
            .await
            .iter()
            .filter(|r| r.did == did && r.collection == collection)
            .cloned()
            .collect()
    }

    /// Count how many createRecord calls have been made (useful for
    /// asserting that fork actually hit the PDS).
    pub async fn create_record_count(&self) -> usize {
        *self.store.create_record_count.read().await
    }

    /// Return the most recent DPoP proof header value (for inspection).
    pub async fn last_dpop_proof(&self) -> Option<String> {
        self.store.last_dpop_proof.read().await.clone()
    }
}

impl Drop for TestPds {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
    }
}

#[derive(Clone, Debug)]
pub struct Account {
    pub access_token: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct StoredRecord {
    pub did: String,
    pub collection: String,
    pub rkey: String,
    pub value: Value,
    pub cid: String,
}

#[derive(Default)]
pub struct PdsStore {
    pub accounts: RwLock<HashMap<String, Account>>,
    pub records: RwLock<Vec<StoredRecord>>,
    pub create_record_count: RwLock<usize>,
    pub last_dpop_proof: RwLock<Option<String>>,
}

fn router(store: Arc<PdsStore>) -> Router {
    Router::new()
        .route(
            "/xrpc/com.atproto.repo.createRecord",
            post(create_record_handler),
        )
        .route(
            "/xrpc/com.atproto.repo.getRecord",
            get(get_record_handler),
        )
        .route(
            "/xrpc/com.atproto.repo.listRecords",
            get(list_records_handler),
        )
        .route(
            "/xrpc/com.atproto.repo.deleteRecord",
            post(delete_record_handler),
        )
        .route(
            "/xrpc/com.atproto.repo.describeRepo",
            get(describe_repo_handler),
        )
        .with_state(store)
}

/// Extract the bearer token from an `Authorization: DPoP <token>` header.
fn extract_dpop_token(headers: &HeaderMap) -> Result<String, PdsError> {
    let auth = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| PdsError::new(401, "AuthRequired", "missing Authorization header"))?;

    if let Some(token) = auth.strip_prefix("DPoP ") {
        Ok(token.to_string())
    } else if let Some(token) = auth.strip_prefix("Bearer ") {
        Ok(token.to_string())
    } else {
        Err(PdsError::new(
            401,
            "InvalidToken",
            "Authorization must start with 'DPoP ' or 'Bearer '",
        ))
    }
}

/// Validate the DPoP proof header (shape only, not signature).
fn validate_dpop_proof(
    headers: &HeaderMap,
    method: &str,
    access_token: &str,
) -> Result<DpopClaims, PdsError> {
    let proof = headers
        .get("dpop")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| PdsError::new(401, "DpopRequired", "missing DPoP header"))?;

    let parts: Vec<&str> = proof.split('.').collect();
    if parts.len() != 3 {
        return Err(PdsError::new(
            401,
            "InvalidDpop",
            "DPoP proof must be a JWT with 3 parts",
        ));
    }

    // Decode header — must have alg + typ=dpop+jwt + jwk.
    let header_bytes = URL_SAFE_NO_PAD
        .decode(parts[0])
        .map_err(|_| PdsError::new(401, "InvalidDpop", "invalid DPoP header encoding"))?;
    let header: Value = serde_json::from_slice(&header_bytes)
        .map_err(|_| PdsError::new(401, "InvalidDpop", "invalid DPoP header JSON"))?;

    if header.get("typ").and_then(|v| v.as_str()) != Some("dpop+jwt") {
        return Err(PdsError::new(
            401,
            "InvalidDpop",
            "DPoP header typ must be 'dpop+jwt'",
        ));
    }
    if header.get("alg").and_then(|v| v.as_str()) != Some("ES256") {
        return Err(PdsError::new(
            401,
            "InvalidDpop",
            "DPoP header alg must be 'ES256'",
        ));
    }
    if header.get("jwk").is_none() {
        return Err(PdsError::new(
            401,
            "InvalidDpop",
            "DPoP header must contain a jwk",
        ));
    }

    // Decode claims.
    let claims_bytes = URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|_| PdsError::new(401, "InvalidDpop", "invalid DPoP claims encoding"))?;
    let claims: DpopClaims = serde_json::from_slice(&claims_bytes)
        .map_err(|_| PdsError::new(401, "InvalidDpop", "invalid DPoP claims JSON"))?;

    if claims.htm.to_uppercase() != method.to_uppercase() {
        return Err(PdsError::new(
            401,
            "InvalidDpop",
            &format!("htm mismatch: {} vs {}", claims.htm, method),
        ));
    }

    // Verify the ath claim matches the access token (SHA-256, base64url).
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(access_token.as_bytes());
    let expected_ath = URL_SAFE_NO_PAD.encode(hasher.finalize());
    if claims.ath.as_deref() != Some(expected_ath.as_str()) {
        return Err(PdsError::new(
            401,
            "InvalidDpop",
            "ath claim does not match access token hash",
        ));
    }

    Ok(claims)
}

#[derive(Deserialize)]
struct DpopClaims {
    jti: String,
    htm: String,
    htu: String,
    #[allow(dead_code)]
    iat: i64,
    ath: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    nonce: Option<String>,
}

// ─── Handlers ──────────────────────────────────────────────────────

#[derive(Deserialize)]
struct CreateRecordInput {
    repo: String,
    collection: String,
    #[serde(default)]
    rkey: Option<String>,
    record: Value,
}

async fn create_record_handler(
    State(store): State<Arc<PdsStore>>,
    headers: HeaderMap,
    Json(input): Json<CreateRecordInput>,
) -> Result<Json<Value>, PdsError> {
    *store.create_record_count.write().await += 1;
    if let Some(p) = headers.get("dpop").and_then(|v| v.to_str().ok()) {
        *store.last_dpop_proof.write().await = Some(p.to_string());
    }

    let token = extract_dpop_token(&headers)?;
    validate_dpop_proof(&headers, "POST", &token)?;

    // Verify the token matches a registered account for the repo.
    let accounts = store.accounts.read().await;
    let account = accounts.get(&input.repo).ok_or_else(|| {
        PdsError::new(403, "AccountNotFound", "no test account for this DID")
    })?;
    if account.access_token != token {
        return Err(PdsError::new(
            403,
            "TokenMismatch",
            "access token does not belong to this account",
        ));
    }
    drop(accounts);

    let rkey = input
        .rkey
        .unwrap_or_else(|| format!("3test{}", random_tid()));
    let uri = format!("at://{}/{}/{}", input.repo, input.collection, rkey);
    let cid = format!("bafytest{rkey}");

    store.records.write().await.push(StoredRecord {
        did: input.repo.clone(),
        collection: input.collection.clone(),
        rkey,
        value: input.record,
        cid: cid.clone(),
    });

    Ok(Json(serde_json::json!({ "uri": uri, "cid": cid })))
}

#[derive(Deserialize)]
struct GetRecordQuery {
    repo: String,
    collection: String,
    rkey: String,
}

async fn get_record_handler(
    State(store): State<Arc<PdsStore>>,
    Query(q): Query<GetRecordQuery>,
) -> Result<Json<Value>, PdsError> {
    let records = store.records.read().await;
    let rec = records
        .iter()
        .find(|r| r.did == q.repo && r.collection == q.collection && r.rkey == q.rkey)
        .ok_or_else(|| PdsError::new(404, "RecordNotFound", "record not found"))?;

    Ok(Json(serde_json::json!({
        "uri": format!("at://{}/{}/{}", rec.did, rec.collection, rec.rkey),
        "cid": rec.cid,
        "value": rec.value,
    })))
}

#[derive(Deserialize)]
struct ListRecordsQuery {
    repo: String,
    collection: String,
    #[serde(default)]
    #[allow(dead_code)]
    limit: Option<i64>,
    #[serde(default)]
    #[allow(dead_code)]
    cursor: Option<String>,
}

async fn list_records_handler(
    State(store): State<Arc<PdsStore>>,
    Query(q): Query<ListRecordsQuery>,
) -> Json<Value> {
    let records = store.records.read().await;
    let matching: Vec<Value> = records
        .iter()
        .filter(|r| r.did == q.repo && r.collection == q.collection)
        .map(|r| {
            serde_json::json!({
                "uri": format!("at://{}/{}/{}", r.did, r.collection, r.rkey),
                "cid": r.cid,
                "value": r.value,
            })
        })
        .collect();

    Json(serde_json::json!({
        "records": matching,
        "cursor": null,
    }))
}

#[derive(Deserialize)]
struct DeleteRecordInput {
    repo: String,
    collection: String,
    rkey: String,
}

async fn delete_record_handler(
    State(store): State<Arc<PdsStore>>,
    headers: HeaderMap,
    Json(input): Json<DeleteRecordInput>,
) -> Result<Json<Value>, PdsError> {
    let token = extract_dpop_token(&headers)?;
    validate_dpop_proof(&headers, "POST", &token)?;

    let mut records = store.records.write().await;
    records.retain(|r| {
        !(r.did == input.repo && r.collection == input.collection && r.rkey == input.rkey)
    });

    Ok(Json(serde_json::json!({})))
}

#[derive(Deserialize)]
struct DescribeRepoQuery {
    repo: String,
}

async fn describe_repo_handler(
    State(store): State<Arc<PdsStore>>,
    Query(q): Query<DescribeRepoQuery>,
) -> Result<Json<Value>, PdsError> {
    let accounts = store.accounts.read().await;
    if !accounts.contains_key(&q.repo) {
        return Err(PdsError::new(404, "AccountNotFound", "no account"));
    }
    Ok(Json(serde_json::json!({
        "did": q.repo,
        "handle": format!("{}.test", q.repo.replace(':', "-")),
        "didDoc": {},
        "collections": [],
        "handleIsCorrect": true,
    })))
}

// ─── Error type ────────────────────────────────────────────────────

#[derive(Debug)]
struct PdsError {
    status: u16,
    error: String,
    message: String,
}

impl PdsError {
    fn new(status: u16, error: &str, message: &str) -> Self {
        Self {
            status,
            error: error.to_string(),
            message: message.to_string(),
        }
    }
}

impl IntoResponse for PdsError {
    fn into_response(self) -> axum::response::Response {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (
            status,
            Json(serde_json::json!({
                "error": self.error,
                "message": self.message,
            })),
        )
            .into_response()
    }
}

fn random_tid() -> String {
    // Not a real TID, just something unique per call.
    uuid::Uuid::new_v4()
        .to_string()
        .replace('-', "")
        .chars()
        .take(10)
        .collect()
}
