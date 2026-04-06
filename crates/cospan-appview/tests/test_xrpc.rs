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

use cospan_appview::auth::OAuthConfig;
use cospan_appview::auth::dpop::DpopKey;
use cospan_appview::auth::session::InMemorySessionStore;
use cospan_appview::config::AppConfig;
use cospan_appview::db;
use cospan_appview::state::AppState;

async fn build_state(pool: PgPool) -> Arc<AppState> {
    let config = AppConfig {
        database_url: String::new(), // not used, pool already connected
        jetstream_url: "wss://localhost:0/unused".to_string(),
        listen: "127.0.0.1:0".to_string(),
        lexicons_dir: "packages/lexicons".to_string(),
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
    let app = cospan_appview::xrpc::router(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    format!("http://{addr}")
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

#[sqlx::test(migrator = "cospan_appview::MIGRATOR")]
async fn fork_creates_new_repo(pool: PgPool) {
    seed_test_data(&pool).await;
    let base = start_server(pool).await;
    let client = Client::new();

    let resp = client
        .post(format!("{base}/xrpc/dev.cospan.repo.fork"))
        .json(&serde_json::json!({
            "sourceRepo": "at://did:plc:alice/dev.cospan.repo/test-project",
            "did": "did:plc:carol",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["did"].as_str().unwrap(), "did:plc:carol");
    assert_eq!(json["name"].as_str().unwrap(), "test-project");
    assert!(json["uri"].as_str().unwrap().starts_with("at://did:plc:carol/dev.cospan.repo/"));
    assert_eq!(json["sourceRepo"].as_str().unwrap(), "at://did:plc:alice/dev.cospan.repo/test-project");
}

#[sqlx::test(migrator = "cospan_appview::MIGRATOR")]
async fn fork_with_custom_name(pool: PgPool) {
    seed_test_data(&pool).await;
    let base = start_server(pool).await;
    let client = Client::new();

    let resp = client
        .post(format!("{base}/xrpc/dev.cospan.repo.fork"))
        .json(&serde_json::json!({
            "sourceRepo": "at://did:plc:alice/dev.cospan.repo/test-project",
            "did": "did:plc:carol",
            "name": "my-fork",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["name"].as_str().unwrap(), "my-fork");
}

#[sqlx::test(migrator = "cospan_appview::MIGRATOR")]
async fn fork_increments_source_fork_count(pool: PgPool) {
    seed_test_data(&pool).await;
    let base = start_server(pool.clone()).await;
    let client = Client::new();

    // Fork the repo
    let resp = client
        .post(format!("{base}/xrpc/dev.cospan.repo.fork"))
        .json(&serde_json::json!({
            "sourceRepo": "at://did:plc:alice/dev.cospan.repo/test-project",
            "did": "did:plc:carol",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

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
    let base = start_server(pool).await;
    let client = Client::new();

    let resp = client
        .post(format!("{base}/xrpc/dev.cospan.repo.fork"))
        .json(&serde_json::json!({
            "sourceRepo": "at://did:plc:nobody/dev.cospan.repo/nonexistent",
            "did": "did:plc:carol",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

#[sqlx::test(migrator = "cospan_appview::MIGRATOR")]
async fn fork_invalid_uri_returns_400(pool: PgPool) {
    let base = start_server(pool).await;
    let client = Client::new();

    let resp = client
        .post(format!("{base}/xrpc/dev.cospan.repo.fork"))
        .json(&serde_json::json!({
            "sourceRepo": "not-a-valid-uri",
            "did": "did:plc:carol",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
}

#[sqlx::test(migrator = "cospan_appview::MIGRATOR")]
async fn health_endpoint(pool: PgPool) {
    let base = start_server(pool).await;
    let client = Client::new();

    let resp = client.get(format!("{base}/health")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.text().await.unwrap(), "ok");
}
