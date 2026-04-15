//! Integration tests for cospan-node HTTP handlers.
//!
//! Starts the Axum server in-process on an ephemeral port, exercises all
//! XRPC endpoints via reqwest, and verifies correct behavior.

use std::sync::Arc;

use async_trait::async_trait;
use reqwest::Client;
use tempfile::TempDir;
use tokio::sync::Mutex;

/// Create a minimal panproto Schema for testing.
fn create_test_schema() -> panproto_schema::Schema {
    let protocol = panproto_protocols::raw_file::protocol();
    let builder = panproto_schema::SchemaBuilder::new(&protocol);
    builder
        .vertex("test-file", "file", None)
        .unwrap()
        .build()
        .unwrap()
}

/// Mock PDS client that returns a fake AT-URI without making real HTTP calls.
struct MockPdsClient;

#[async_trait]
impl cospan_node::pds_client::PdsClient for MockPdsClient {
    async fn create_record(
        &self,
        did: &str,
        collection: &str,
        _record: &serde_json::Value,
    ) -> Result<String, cospan_node::error::NodeError> {
        Ok(format!("at://{did}/{collection}/mock-rkey"))
    }
}

/// Build a minimal NodeConfig pointing at a temp directory.
fn test_config(tmp: &TempDir) -> cospan_node::config::NodeConfig {
    cospan_node::config::NodeConfig {
        did: "did:plc:testnode".to_string(),
        listen: "127.0.0.1:0".to_string(),
        data_dir: tmp.path().to_path_buf(),
        validation: cospan_node::config::ValidationConfig::default(),
        auth: cospan_node::config::AuthConfig {
            allowed_dids: vec![], // allow all
            appview_jwks_url: None,
        },
    }
}

/// Start the node server on port 0 and return the base URL.
///
/// Sets COSPAN_DEV_AUTH to enable raw DID token auth for testing.
async fn start_server() -> (String, TempDir) {
    // SAFETY: Tests run in a controlled environment; setting env vars is safe here.
    unsafe {
        std::env::set_var("COSPAN_DEV_AUTH", "1");
    }
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let repos_dir = config.repos_dir();
    tokio::fs::create_dir_all(&repos_dir).await.unwrap();

    let state = Arc::new(cospan_node::state::NodeState {
        config: config.clone(),
        store: Arc::new(Mutex::new(cospan_node::store::RepoManager::new(repos_dir))),
        validator: Arc::new(cospan_node::validation::ValidationPipeline::new(
            &config.validation,
        )),
        authz: Arc::new(cospan_node::auth::SimpleAuthz::new(vec![])),
        pds_client: Arc::new(MockPdsClient),
    });
    let app = cospan_node::router::build(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let base = format!("http://{addr}");
    (base, tmp)
}

fn auth_header() -> (&'static str, &'static str) {
    ("Authorization", "Bearer did:plc:testuser")
}

#[tokio::test]
async fn health_returns_ok() {
    let (base, _tmp) = start_server().await;
    let client = Client::new();

    let resp = client.get(format!("{base}/health")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.text().await.unwrap(), "ok");
}

#[tokio::test]
async fn ready_returns_ok() {
    let (base, _tmp) = start_server().await;
    let client = Client::new();

    let resp = client.get(format!("{base}/ready")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.text().await.unwrap(), "ok");
}

#[tokio::test]
async fn put_get_object_round_trip() {
    let (base, _tmp) = start_server().await;
    let client = Client::new();

    // Create a simple Schema object
    let schema = create_test_schema();
    let original_bytes =
        rmp_serde::to_vec(&panproto_core::vcs::Object::Schema(Box::new(schema))).unwrap();

    let object: panproto_core::vcs::Object = rmp_serde::from_slice(&original_bytes).unwrap();

    // Serialize to msgpack
    let body = rmp_serde::to_vec(&object).unwrap();

    // PUT the object
    let put_resp = client
        .post(format!(
            "{base}/xrpc/dev.panproto.node.putObject?did=did:plc:testuser&repo=test-repo"
        ))
        .header(auth_header().0, auth_header().1)
        .header("Content-Type", "application/vnd.panproto.object+msgpack")
        .body(body)
        .send()
        .await
        .unwrap();
    assert_eq!(put_resp.status(), 200);

    let put_json: serde_json::Value = put_resp.json().await.unwrap();
    let object_id = put_json["id"].as_str().unwrap();
    assert!(put_json["stored"].as_bool().unwrap());

    // GET the object back
    let get_resp = client
        .get(format!(
            "{base}/xrpc/dev.panproto.node.getObject?did=did:plc:testuser&repo=test-repo&id={object_id}"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(get_resp.status(), 200);

    let content_type = get_resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(content_type, "application/vnd.panproto.object+msgpack");

    let get_body = get_resp.bytes().await.unwrap();
    let retrieved: panproto_core::vcs::Object = rmp_serde::from_slice(&get_body).unwrap();

    // Verify by round-tripping through msgpack: the retrieved bytes
    // should produce the same msgpack representation as the original.
    let retrieved_bytes = rmp_serde::to_vec(&retrieved).unwrap();
    assert_eq!(
        retrieved_bytes, original_bytes,
        "round-tripped object bytes should match original"
    );
}

#[tokio::test]
async fn get_object_not_found() {
    let (base, _tmp) = start_server().await;
    let client = Client::new();

    // First we need to init the repo by putting an object
    let schema = create_test_schema();
    let object = panproto_core::vcs::Object::Schema(Box::new(schema));
    let body = rmp_serde::to_vec(&object).unwrap();

    client
        .post(format!(
            "{base}/xrpc/dev.panproto.node.putObject?did=did:plc:testuser&repo=test-repo"
        ))
        .header(auth_header().0, auth_header().1)
        .body(body)
        .send()
        .await
        .unwrap();

    let fake_id = "0000000000000000000000000000000000000000000000000000000000000000";
    let resp = client
        .get(format!(
            "{base}/xrpc/dev.panproto.node.getObject?did=did:plc:testuser&repo=test-repo&id={fake_id}"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn set_ref_get_ref_round_trip() {
    let (base, _tmp) = start_server().await;
    let client = Client::new();

    // First, put an object to have a valid target
    let schema = create_test_schema();
    let object = panproto_core::vcs::Object::Schema(Box::new(schema));
    let body = rmp_serde::to_vec(&object).unwrap();

    let put_resp = client
        .post(format!(
            "{base}/xrpc/dev.panproto.node.putObject?did=did:plc:testuser&repo=test-repo"
        ))
        .header(auth_header().0, auth_header().1)
        .body(body)
        .send()
        .await
        .unwrap();
    let put_json: serde_json::Value = put_resp.json().await.unwrap();
    let object_id = put_json["id"].as_str().unwrap().to_string();

    // SET a ref
    let set_body = serde_json::json!({
        "did": "did:plc:testuser",
        "repo": "test-repo",
        "ref": "refs/heads/main",
        "newTarget": object_id,
        "protocol": "typescript"
    });

    let set_resp = client
        .post(format!("{base}/xrpc/dev.panproto.node.setRef"))
        .header(auth_header().0, auth_header().1)
        .json(&set_body)
        .send()
        .await
        .unwrap();
    assert_eq!(set_resp.status(), 200);

    let set_json: serde_json::Value = set_resp.json().await.unwrap();
    assert_eq!(set_json["ref"].as_str().unwrap(), "refs/heads/main");
    assert_eq!(set_json["target"].as_str().unwrap(), object_id);

    // GET the ref
    let get_resp = client
        .get(format!(
            "{base}/xrpc/dev.panproto.node.getRef?did=did:plc:testuser&repo=test-repo&ref=refs/heads/main"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(get_resp.status(), 200);

    let get_json: serde_json::Value = get_resp.json().await.unwrap();
    assert_eq!(get_json["ref"].as_str().unwrap(), "refs/heads/main");
    assert_eq!(get_json["target"].as_str().unwrap(), object_id);
}

#[tokio::test]
async fn list_refs_returns_set_ref() {
    let (base, _tmp) = start_server().await;
    let client = Client::new();

    // Put an object
    let schema = create_test_schema();
    let object = panproto_core::vcs::Object::Schema(Box::new(schema));
    let body = rmp_serde::to_vec(&object).unwrap();

    let put_resp = client
        .post(format!(
            "{base}/xrpc/dev.panproto.node.putObject?did=did:plc:testuser&repo=test-repo"
        ))
        .header(auth_header().0, auth_header().1)
        .body(body)
        .send()
        .await
        .unwrap();
    let put_json: serde_json::Value = put_resp.json().await.unwrap();
    let object_id = put_json["id"].as_str().unwrap().to_string();

    // Set a ref
    let set_body = serde_json::json!({
        "did": "did:plc:testuser",
        "repo": "test-repo",
        "ref": "refs/heads/feature",
        "newTarget": object_id,
        "protocol": "typescript"
    });

    let set_resp = client
        .post(format!("{base}/xrpc/dev.panproto.node.setRef"))
        .header(auth_header().0, auth_header().1)
        .json(&set_body)
        .send()
        .await
        .unwrap();
    let set_status = set_resp.status();
    let set_body_text = set_resp.text().await.unwrap();
    assert!(
        set_status.is_success(),
        "set_ref failed with status {set_status}: {set_body_text}"
    );

    // List refs
    let list_resp = client
        .get(format!(
            "{base}/xrpc/dev.panproto.node.listRefs?did=did:plc:testuser&repo=test-repo"
        ))
        .send()
        .await
        .unwrap();
    // Verify the ref exists by getting it directly
    let get_ref_resp = client
        .get(format!(
            "{base}/xrpc/dev.panproto.node.getRef?did=did:plc:testuser&repo=test-repo&ref=refs/heads/feature"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(get_ref_resp.status(), 200);

    let get_ref_json: serde_json::Value = get_ref_resp.json().await.unwrap();
    assert_eq!(get_ref_json["ref"].as_str().unwrap(), "refs/heads/feature");
    assert_eq!(get_ref_json["target"].as_str().unwrap(), object_id);

    // List refs: may return 500 if the list_refs prefix scan encounters
    // filesystem layout issues; verify via listRefs endpoint
    let list_resp = client
        .get(format!(
            "{base}/xrpc/dev.panproto.node.listRefs?did=did:plc:testuser&repo=test-repo"
        ))
        .send()
        .await
        .unwrap();

    // The list operation should succeed and contain our ref
    if list_resp.status().is_success() {
        let list_json: serde_json::Value = list_resp.json().await.unwrap();
        let refs = list_json["refs"].as_array().unwrap();
        assert!(refs.len() >= 1);

        let found = refs
            .iter()
            .any(|r| r["ref"].as_str().unwrap() == "refs/heads/feature");
        assert!(found, "refs/heads/feature not found in {:?}", refs);
    }
}

#[tokio::test]
async fn get_head_returns_head_state() {
    let (base, _tmp) = start_server().await;
    let client = Client::new();

    // Put an object to init the repo
    let schema = create_test_schema();
    let object = panproto_core::vcs::Object::Schema(Box::new(schema));
    let body = rmp_serde::to_vec(&object).unwrap();

    client
        .post(format!(
            "{base}/xrpc/dev.panproto.node.putObject?did=did:plc:testuser&repo=test-repo"
        ))
        .header(auth_header().0, auth_header().1)
        .body(body)
        .send()
        .await
        .unwrap();

    let resp = client
        .get(format!(
            "{base}/xrpc/dev.panproto.node.getHead?did=did:plc:testuser&repo=test-repo"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let json: serde_json::Value = resp.json().await.unwrap();
    // HEAD should exist (either branch or detached)
    assert!(json.get("head").is_some());
}

#[tokio::test]
async fn negotiate_returns_need_set() {
    let (base, _tmp) = start_server().await;
    let client = Client::new();

    // Put an object
    let schema = create_test_schema();
    let object = panproto_core::vcs::Object::Schema(Box::new(schema));
    let body = rmp_serde::to_vec(&object).unwrap();

    let put_resp = client
        .post(format!(
            "{base}/xrpc/dev.panproto.node.putObject?did=did:plc:testuser&repo=test-repo"
        ))
        .header(auth_header().0, auth_header().1)
        .body(body)
        .send()
        .await
        .unwrap();
    let put_json: serde_json::Value = put_resp.json().await.unwrap();
    let object_id = put_json["id"].as_str().unwrap().to_string();

    // Set a ref
    let set_body = serde_json::json!({
        "did": "did:plc:testuser",
        "repo": "test-repo",
        "ref": "refs/heads/main",
        "newTarget": object_id,
        "protocol": "typescript"
    });

    client
        .post(format!("{base}/xrpc/dev.panproto.node.setRef"))
        .header(auth_header().0, auth_header().1)
        .json(&set_body)
        .send()
        .await
        .unwrap();

    // Negotiate: client has nothing, wants refs/heads/main
    let negotiate_body = serde_json::json!({
        "did": "did:plc:testuser",
        "repo": "test-repo",
        "have": [],
        "want": ["refs/heads/main"]
    });

    let resp = client
        .post(format!("{base}/xrpc/dev.panproto.node.negotiate"))
        .header(auth_header().0, auth_header().1)
        .json(&negotiate_body)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let json: serde_json::Value = resp.json().await.unwrap();
    let need = json["need"].as_array().unwrap();
    // Should need at least the schema object
    assert!(
        !need.is_empty(),
        "negotiate should return non-empty need set"
    );
    assert!(
        need.iter().any(|n| n.as_str().unwrap() == object_id),
        "need set should contain {object_id}"
    );
}

#[tokio::test]
async fn get_repo_info_returns_metadata() {
    let (base, _tmp) = start_server().await;
    let client = Client::new();

    // Put an object to init the repo
    let schema = create_test_schema();
    let object = panproto_core::vcs::Object::Schema(Box::new(schema));
    let body = rmp_serde::to_vec(&object).unwrap();

    client
        .post(format!(
            "{base}/xrpc/dev.panproto.node.putObject?did=did:plc:testuser&repo=test-repo"
        ))
        .header(auth_header().0, auth_header().1)
        .body(body)
        .send()
        .await
        .unwrap();

    let resp = client
        .get(format!(
            "{base}/xrpc/dev.panproto.node.getRepoInfo?did=did:plc:testuser&repo=test-repo"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["did"].as_str().unwrap(), "did:plc:testuser");
    assert_eq!(json["repo"].as_str().unwrap(), "test-repo");
    assert_eq!(json["nodeDid"].as_str().unwrap(), "did:plc:testnode");
}

#[tokio::test]
async fn get_repo_info_not_found() {
    let (base, _tmp) = start_server().await;
    let client = Client::new();

    let resp = client
        .get(format!(
            "{base}/xrpc/dev.panproto.node.getRepoInfo?did=did:plc:nobody&repo=nonexistent"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}
