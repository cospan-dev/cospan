//! XRPC endpoint integration tests.
//!
//! Starts the full appview server, seeds test data into the DB, and
//! tests XRPC endpoints via reqwest.
//!
//! Requires DATABASE_URL in the environment.

use std::sync::Arc;

use chrono::Utc;
use reqwest::Client;
use sqlx::PgPool;

use cospan_appview::auth::dpop::DpopKey;
use cospan_appview::auth::session::InMemorySessionStore;
use cospan_appview::auth::{OAuthConfig, Session};
use cospan_appview::config::AppConfig;
use cospan_appview::db;
use cospan_appview::state::AppState;

async fn build_state(pool: PgPool) -> Arc<AppState> {
    build_state_with_config(
        pool,
        "did:plc:cospan-node".to_string(),
        "http://localhost:0".to_string(),
    )
    .await
}

async fn build_state_with_config(
    pool: PgPool,
    default_node_did: String,
    default_node_url: String,
) -> Arc<AppState> {
    let config = AppConfig {
        database_url: String::new(), // not used, pool already connected
        jetstream_url: "wss://localhost:0/unused".to_string(),
        listen: "127.0.0.1:0".to_string(),
        lexicons_dir: "packages/lexicons".to_string(),
        default_node_did,
        default_node_url,
        appview_did: String::new(),
    };

    let oauth_config = OAuthConfig {
        client_id: "http://localhost/oauth/client-metadata.json".to_string(),
        redirect_uri: "http://localhost/oauth/callback".to_string(),
        jwks_uri: "http://localhost/.well-known/jwks.json".to_string(),
        public_url: "http://localhost".to_string(),
        client_name: "Cospan Test".to_string(),
    };

    let session_store = Arc::new(InMemorySessionStore::new());
    let dpop_key = DpopKey::generate();

    Arc::new(
        AppState::new(config, pool, oauth_config, session_store, dpop_key)
            .await
            .unwrap(),
    )
}

async fn start_server(pool: PgPool) -> String {
    let state = build_state(pool).await;
    start_server_with_state(state).await.0
}

async fn start_server_with_state(state: Arc<AppState>) -> (String, Arc<AppState>) {
    let app = cospan_appview::xrpc::router(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (format!("http://{addr}"), state)
}

/// Seed a user session into the session store and return the session cookie value.
/// `pds_url` should be the mock PDS base URL from wiremock.
async fn seed_session(state: &Arc<AppState>, did: &str, pds_url: &str) -> String {
    let dpop_key = DpopKey::generate_session_key();
    let session = Session {
        did: did.to_string(),
        handle: Some(format!("{}.test", did.replace(':', "-"))),
        access_token: "test-access-token".to_string(),
        refresh_token: "test-refresh-token".to_string(),
        dpop_private_key_b64: dpop_key.private_key_b64.clone(),
        auth_server_issuer: pds_url.to_string(),
        pds_url: pds_url.to_string(),
        dpop_nonce: None,
        expires_at: Utc::now() + chrono::Duration::hours(1),
        created_at: Utc::now(),
        scope: String::new(),
    };
    let session_id = uuid::Uuid::new_v4().to_string();
    state
        .session_store
        .put_session(&session_id, session)
        .await
        .unwrap();
    format!("cospan_session={session_id}")
}

async fn seed_test_data(pool: &PgPool) {
    let now = Utc::now();

    // Insert prerequisite node
    db::node::upsert(
        pool,
        &db::node::NodeRow {
            did: "did:plc:node1".to_string(),
            rkey: "self".to_string(),
            public_endpoint: Some("https://node1.example.com".to_string()),
            created_at: now,
            indexed_at: now,
        },
    )
    .await
    .unwrap();

    // Insert the default cospan node used by fork/create handlers.
    db::node::upsert(
        pool,
        &db::node::NodeRow {
            did: "did:plc:cospan-node".to_string(),
            rkey: "self".to_string(),
            public_endpoint: Some("https://node.test".to_string()),
            created_at: now,
            indexed_at: now,
        },
    )
    .await
    .unwrap();

    // Seed repos
    for (did, name, desc, protocol) in [
        (
            "did:plc:alice",
            "test-project",
            "A test project for search",
            "typescript",
        ),
        (
            "did:plc:alice",
            "another-repo",
            "Another repository",
            "python",
        ),
        ("did:plc:bob", "bob-repo", "Bob's test repo", "rust"),
    ] {
        let repo = db::repo::RepoRow {
            did: did.to_string(),
            rkey: name.to_string(),
            name: name.to_string(),
            description: Some(desc.to_string()),
            protocol: protocol.to_string(),
            node_did: "did:plc:node1".to_string(),
            node_url: "https://node1.example.com".to_string(),
            default_branch: "main".to_string(),
            visibility: "public".to_string(),
            source_repo: None,
            star_count: 0,
            fork_count: 0,
            open_issue_count: 0,
            open_mr_count: 0,
            source: "cospan".to_string(),
            source_uri: None,
            created_at: now,
            indexed_at: now,
        };
        db::repo::upsert(pool, &repo).await.unwrap();
    }
}

#[sqlx::test(migrator = "cospan_appview::MIGRATOR")]
async fn repo_list_returns_seeded_repos(pool: PgPool) {
    seed_test_data(&pool).await;
    let base = start_server(pool).await;
    let client = Client::new();

    let resp = client
        .get(format!("{base}/xrpc/dev.cospan.repo.list"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let json: serde_json::Value = resp.json().await.unwrap();
    let repos = json["repos"].as_array().unwrap();
    assert_eq!(repos.len(), 3);
}

#[sqlx::test(migrator = "cospan_appview::MIGRATOR")]
async fn repo_list_filtered_by_did(pool: PgPool) {
    seed_test_data(&pool).await;
    let base = start_server(pool).await;
    let client = Client::new();

    let resp = client
        .get(format!(
            "{base}/xrpc/dev.cospan.repo.list?did=did:plc:alice"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let json: serde_json::Value = resp.json().await.unwrap();
    let repos = json["repos"].as_array().unwrap();
    assert_eq!(repos.len(), 2);
}

#[sqlx::test(migrator = "cospan_appview::MIGRATOR")]
async fn repo_get_returns_specific_repo(pool: PgPool) {
    seed_test_data(&pool).await;
    let base = start_server(pool).await;
    let client = Client::new();

    let resp = client
        .get(format!(
            "{base}/xrpc/dev.cospan.repo.get?did=did:plc:alice&name=test-project"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["name"].as_str().unwrap(), "test-project");
    assert_eq!(json["protocol"].as_str().unwrap(), "typescript");
}

#[sqlx::test(migrator = "cospan_appview::MIGRATOR")]
async fn repo_get_not_found(pool: PgPool) {
    let base = start_server(pool).await;
    let client = Client::new();

    let resp = client
        .get(format!(
            "{base}/xrpc/dev.cospan.repo.get?did=did:plc:nobody&name=nonexistent"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

#[sqlx::test(migrator = "cospan_appview::MIGRATOR")]
async fn search_repos_finds_matching(pool: PgPool) {
    seed_test_data(&pool).await;
    let base = start_server(pool).await;
    let client = Client::new();

    let resp = client
        .get(format!("{base}/xrpc/dev.cospan.search.repos?q=test"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let json: serde_json::Value = resp.json().await.unwrap();
    let repos = json["repos"].as_array().unwrap();
    // "test-project" and "bob-repo" (contains "test" in description) should match
    assert!(!repos.is_empty(), "search for 'test' should return results");
}

// ─── Fork tests ──────────────────────────────────────────────────────
//
// The fork handler requires an authenticated session and writes a
// dev.cospan.repo record to the user's PDS via OAuth/DPoP. These tests
// use wiremock to simulate the PDS and InMemorySessionStore to seed a
// test session.

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn start_mock_pds() -> MockServer {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/xrpc/com.atproto.repo.createRecord"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "uri": "at://did:plc:carol/dev.cospan.repo/3mockrkey123",
            "cid": "bafymockedcidvalue",
        })))
        .mount(&mock)
        .await;
    mock
}

#[sqlx::test(migrator = "cospan_appview::MIGRATOR")]
async fn fork_creates_new_repo(pool: PgPool) {
    seed_test_data(&pool).await;
    let mock_pds = start_mock_pds().await;
    let state = build_state_with_config(
        pool,
        "did:plc:cospan-node".to_string(),
        "https://node.test".to_string(),
    )
    .await;
    let (base, state) = start_server_with_state(state).await;
    let cookie = seed_session(&state, "did:plc:carol", &mock_pds.uri()).await;
    let client = Client::new();

    let resp = client
        .post(format!("{base}/xrpc/dev.cospan.repo.fork"))
        .header("Cookie", cookie)
        .json(&serde_json::json!({
            "sourceRepo": "at://did:plc:alice/dev.cospan.repo/test-project",
        }))
        .send()
        .await
        .unwrap();
    let status = resp.status();
    let body = resp.text().await.unwrap();
    assert_eq!(status, 200, "fork failed: {body}");

    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["did"].as_str().unwrap(), "did:plc:carol");
    assert_eq!(json["name"].as_str().unwrap(), "test-project");
    assert_eq!(
        json["uri"].as_str().unwrap(),
        "at://did:plc:carol/dev.cospan.repo/3mockrkey123"
    );
    assert_eq!(json["cid"].as_str().unwrap(), "bafymockedcidvalue");
    assert_eq!(
        json["sourceRepo"].as_str().unwrap(),
        "at://did:plc:alice/dev.cospan.repo/test-project"
    );
    assert_eq!(json["nodeDid"].as_str().unwrap(), "did:plc:cospan-node");
}

#[sqlx::test(migrator = "cospan_appview::MIGRATOR")]
async fn fork_requires_authentication(pool: PgPool) {
    seed_test_data(&pool).await;
    let base = start_server(pool).await;
    let client = Client::new();

    let resp = client
        .post(format!("{base}/xrpc/dev.cospan.repo.fork"))
        .json(&serde_json::json!({
            "sourceRepo": "at://did:plc:alice/dev.cospan.repo/test-project",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}

#[sqlx::test(migrator = "cospan_appview::MIGRATOR")]
async fn fork_with_custom_name(pool: PgPool) {
    seed_test_data(&pool).await;
    let mock_pds = start_mock_pds().await;
    let state = build_state_with_config(
        pool,
        "did:plc:cospan-node".to_string(),
        "https://node.test".to_string(),
    )
    .await;
    let (base, state) = start_server_with_state(state).await;
    let cookie = seed_session(&state, "did:plc:carol", &mock_pds.uri()).await;
    let client = Client::new();

    let resp = client
        .post(format!("{base}/xrpc/dev.cospan.repo.fork"))
        .header("Cookie", cookie)
        .json(&serde_json::json!({
            "sourceRepo": "at://did:plc:alice/dev.cospan.repo/test-project",
            "name": "my-fork",
        }))
        .send()
        .await
        .unwrap();
    let status = resp.status();
    let body = resp.text().await.unwrap();
    assert_eq!(status, 200, "fork failed: {body}");

    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["name"].as_str().unwrap(), "my-fork");
}

#[sqlx::test(migrator = "cospan_appview::MIGRATOR")]
async fn fork_increments_source_fork_count(pool: PgPool) {
    seed_test_data(&pool).await;
    let mock_pds = start_mock_pds().await;
    let state = build_state_with_config(
        pool.clone(),
        "did:plc:cospan-node".to_string(),
        "https://node.test".to_string(),
    )
    .await;
    let (base, state) = start_server_with_state(state).await;
    let cookie = seed_session(&state, "did:plc:carol", &mock_pds.uri()).await;
    let client = Client::new();

    let resp = client
        .post(format!("{base}/xrpc/dev.cospan.repo.fork"))
        .header("Cookie", cookie)
        .json(&serde_json::json!({
            "sourceRepo": "at://did:plc:alice/dev.cospan.repo/test-project",
        }))
        .send()
        .await
        .unwrap();
    let status = resp.status();
    let body = resp.text().await.unwrap();
    assert_eq!(status, 200, "fork failed: {body}");

    // Check source repo's fork_count
    let resp = client
        .get(format!(
            "{base}/xrpc/dev.cospan.repo.get?did=did:plc:alice&name=test-project"
        ))
        .send()
        .await
        .unwrap();
    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["forkCount"].as_i64().unwrap(), 1);
}

#[sqlx::test(migrator = "cospan_appview::MIGRATOR")]
async fn fork_nonexistent_source_returns_404(pool: PgPool) {
    seed_test_data(&pool).await;
    let mock_pds = start_mock_pds().await;
    let state = build_state_with_config(
        pool,
        "did:plc:cospan-node".to_string(),
        "https://node.test".to_string(),
    )
    .await;
    let (base, state) = start_server_with_state(state).await;
    let cookie = seed_session(&state, "did:plc:carol", &mock_pds.uri()).await;
    let client = Client::new();

    let resp = client
        .post(format!("{base}/xrpc/dev.cospan.repo.fork"))
        .header("Cookie", cookie)
        .json(&serde_json::json!({
            "sourceRepo": "at://did:plc:nobody/dev.cospan.repo/nonexistent",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

#[sqlx::test(migrator = "cospan_appview::MIGRATOR")]
async fn fork_invalid_uri_returns_400(pool: PgPool) {
    seed_test_data(&pool).await;
    let mock_pds = start_mock_pds().await;
    let state = build_state_with_config(
        pool,
        "did:plc:cospan-node".to_string(),
        "https://node.test".to_string(),
    )
    .await;
    let (base, state) = start_server_with_state(state).await;
    let cookie = seed_session(&state, "did:plc:carol", &mock_pds.uri()).await;
    let client = Client::new();

    let resp = client
        .post(format!("{base}/xrpc/dev.cospan.repo.fork"))
        .header("Cookie", cookie)
        .json(&serde_json::json!({
            "sourceRepo": "not-a-valid-uri",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
}

#[sqlx::test(migrator = "cospan_appview::MIGRATOR")]
async fn fork_pds_error_returns_upstream_error(pool: PgPool) {
    seed_test_data(&pool).await;
    // Mock PDS that returns a 500 error
    let mock_pds = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/xrpc/com.atproto.repo.createRecord"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "error": "InternalError",
            "message": "PDS is on fire",
        })))
        .mount(&mock_pds)
        .await;

    let state = build_state_with_config(
        pool,
        "did:plc:cospan-node".to_string(),
        "https://node.test".to_string(),
    )
    .await;
    let (base, state) = start_server_with_state(state).await;
    let cookie = seed_session(&state, "did:plc:carol", &mock_pds.uri()).await;
    let client = Client::new();

    let resp = client
        .post(format!("{base}/xrpc/dev.cospan.repo.fork"))
        .header("Cookie", cookie)
        .json(&serde_json::json!({
            "sourceRepo": "at://did:plc:alice/dev.cospan.repo/test-project",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 502); // BAD_GATEWAY for upstream errors
}

#[sqlx::test(migrator = "cospan_appview::MIGRATOR")]
async fn health_endpoint(pool: PgPool) {
    let base = start_server(pool).await;
    let client = Client::new();

    let resp = client.get(format!("{base}/health")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.text().await.unwrap(), "ok");
}
