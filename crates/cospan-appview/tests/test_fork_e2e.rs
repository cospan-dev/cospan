//! End-to-end fork tests.
//!
//! These tests spin up the whole fork stack: a local test PDS (real
//! HTTP server implementing com.atproto.repo.createRecord with DPoP
//! verification), two cospan-node instances (source + destination)
//! with real git smart HTTP endpoints, and the appview with a test DB.
//!
//! Scenario covered by the headline test `fork_copies_git_objects`:
//!
//!   1. Spawn a test PDS.
//!   2. Spawn source and destination cospan-node servers.
//!   3. Seed the source node with a git repo (one commit, two files)
//!      via `git push`.
//!   4. Seed the appview DB with a node + repo row pointing at the
//!      source node.
//!   5. Register a test account on the PDS and seed a session in the
//!      appview session store whose DPoP key + access token match.
//!   6. POST /xrpc/dev.cospan.repo.fork with a cookie for that session.
//!   7. Assert the PDS received an authenticated createRecord call.
//!   8. Wait for the background git copy task to mark the fork_job as
//!      completed.
//!   9. Clone from the destination node and verify the files are there.

mod support;

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use reqwest::Client;
use sqlx::PgPool;

use cospan_appview::auth::dpop::DpopKey;
use cospan_appview::auth::session::InMemorySessionStore;
use cospan_appview::auth::{OAuthConfig, Session};
use cospan_appview::config::AppConfig;
use cospan_appview::db;
use cospan_appview::state::AppState;

use crate::support::test_node::{
    CommitSpec, TestNode, clone_from_node, seed_git_repo, seed_git_repo_with_history,
};
use crate::support::test_pds::TestPds;

// Users cache: `sh.tangled.repo` source creates the db row on the source node.
// For the e2e test we seed synthetic repos directly.

async fn build_state(
    pool: PgPool,
    dest_node_did: String,
    dest_node_url: String,
) -> Arc<AppState> {
    let config = AppConfig {
        database_url: String::new(),
        jetstream_url: "wss://localhost:0/unused".to_string(),
        listen: "127.0.0.1:0".to_string(),
        lexicons_dir: "packages/lexicons".to_string(),
        default_node_did: dest_node_did,
        default_node_url: dest_node_url,
    };

    let oauth_config = OAuthConfig {
        client_id: "http://localhost/oauth/client-metadata.json".to_string(),
        redirect_uri: "http://localhost/oauth/callback".to_string(),
        jwks_uri: "http://localhost/.well-known/jwks.json".to_string(),
        public_url: "http://localhost".to_string(),
        client_name: "Cospan E2E Test".to_string(),
    };

    let session_store = Arc::new(InMemorySessionStore::new());
    let dpop_key = DpopKey::generate();

    Arc::new(
        AppState::new(config, pool, oauth_config, session_store, dpop_key)
            .await
            .unwrap(),
    )
}

async fn start_appview(state: Arc<AppState>) -> String {
    let app = cospan_appview::xrpc::router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}

/// Seed a node row and a source repo row in the appview DB.
async fn seed_source_repo(
    pool: &PgPool,
    node_did: &str,
    node_url: &str,
    owner_did: &str,
    repo_name: &str,
) {
    let now = Utc::now();

    db::node::upsert(
        pool,
        &db::node::NodeRow {
            did: node_did.to_string(),
            rkey: "self".to_string(),
            public_endpoint: Some(node_url.to_string()),
            created_at: now,
            indexed_at: now,
        },
    )
    .await
    .unwrap();

    db::repo::upsert(
        pool,
        &db::repo::RepoRow {
            did: owner_did.to_string(),
            rkey: repo_name.to_string(),
            name: repo_name.to_string(),
            description: Some("a test source repo".to_string()),
            protocol: "raw-file".to_string(),
            node_did: node_did.to_string(),
            node_url: node_url.to_string(),
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
        },
    )
    .await
    .unwrap();
}

/// Seed the destination node's entry (so the FK on repos is satisfied
/// when the fork handler inserts the fork row).
async fn seed_dest_node(pool: &PgPool, node_did: &str, node_url: &str) {
    let now = Utc::now();
    db::node::upsert(
        pool,
        &db::node::NodeRow {
            did: node_did.to_string(),
            rkey: "self".to_string(),
            public_endpoint: Some(node_url.to_string()),
            created_at: now,
            indexed_at: now,
        },
    )
    .await
    .unwrap();
}

/// Build a Session whose `access_token` and `pds_url` match the test PDS.
async fn seed_session(
    state: &Arc<AppState>,
    did: &str,
    access_token: &str,
    pds_url: &str,
) -> String {
    let dpop_key = DpopKey::generate_session_key();
    let session = Session {
        did: did.to_string(),
        handle: Some("test.example".to_string()),
        access_token: access_token.to_string(),
        refresh_token: "test-refresh".to_string(),
        dpop_private_key_b64: dpop_key.private_key_b64.clone(),
        auth_server_issuer: pds_url.to_string(),
        pds_url: pds_url.to_string(),
        dpop_nonce: None,
        expires_at: Utc::now() + chrono::Duration::hours(1),
        created_at: Utc::now(),
    };
    let session_id = uuid::Uuid::new_v4().to_string();
    state
        .session_store
        .put_session(&session_id, session)
        .await
        .unwrap();
    format!("cospan_session={session_id}")
}

async fn wait_for_fork_job(
    pool: &PgPool,
    job_id: uuid::Uuid,
    timeout: Duration,
) -> db::fork_job::ForkJob {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        let job = db::fork_job::get(pool, job_id)
            .await
            .unwrap()
            .expect("fork job should exist");
        if job.state == "completed" || job.state == "failed" {
            return job;
        }
        if std::time::Instant::now() > deadline {
            panic!(
                "timed out waiting for fork job {job_id}; last state = {}",
                job.state
            );
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

// ═══════════════════════════════════════════════════════════════════
// Headline test: full fork round trip
// ═══════════════════════════════════════════════════════════════════

#[sqlx::test(migrator = "cospan_appview::MIGRATOR")]
async fn fork_copies_git_objects(pool: PgPool) {
    // 1. Spawn test PDS and register an account.
    let pds = TestPds::spawn().await;
    let user_did = "did:plc:carol";
    let access_token = pds.register_account(user_did).await;

    // 2. Spawn source and destination cospan-nodes.
    let source_node = TestNode::spawn().await;
    let dest_node = TestNode::spawn().await;

    // 3. Seed the source node with a git repo.
    let source_owner = "did:plc:alice";
    let source_repo_name = "hello-world";
    let source_git_url = source_node.git_url(source_owner, source_repo_name);
    let (commit_oid, _src_tmp) = seed_git_repo(
        &source_git_url,
        &[
            ("README.md", "# hello\n\nthis is the source repo\n"),
            ("src/main.txt", "hello from main\n"),
        ],
    )
    .await;

    // 4. Seed appview DB with source node + source repo row, plus the
    //    destination node (FK required by repos.node_did).
    seed_source_repo(
        &pool,
        "did:plc:sourcenode",
        &source_node.url,
        source_owner,
        source_repo_name,
    )
    .await;
    seed_dest_node(&pool, "did:plc:destnode", &dest_node.url).await;

    // 5. Build the appview state with the destination node as default.
    let state = build_state(
        pool.clone(),
        "did:plc:destnode".to_string(),
        dest_node.url.clone(),
    )
    .await;
    let appview_url = start_appview(state.clone()).await;

    // 6. Seed a session. Its pds_url points at our test PDS.
    let cookie = seed_session(&state, user_did, &access_token, &pds.url).await;

    // 7. POST /xrpc/dev.cospan.repo.fork.
    let client = Client::new();
    let source_at_uri =
        format!("at://{source_owner}/dev.cospan.repo/{source_repo_name}");
    let resp = client
        .post(format!("{appview_url}/xrpc/dev.cospan.repo.fork"))
        .header("Cookie", cookie)
        .json(&serde_json::json!({
            "sourceRepo": source_at_uri,
        }))
        .send()
        .await
        .unwrap();

    let status = resp.status();
    let body = resp.text().await.unwrap();
    assert_eq!(status, 200, "fork failed: {body}");

    let resp_json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(resp_json["did"].as_str().unwrap(), user_did);
    assert_eq!(resp_json["name"].as_str().unwrap(), source_repo_name);
    let fork_rkey = resp_json["rkey"].as_str().unwrap().to_string();
    let fork_job_id: uuid::Uuid = resp_json["forkJobId"]
        .as_str()
        .unwrap()
        .parse()
        .expect("valid uuid");

    // 8. Assert the PDS received exactly one createRecord call and a
    //    well-formed DPoP proof.
    assert_eq!(pds.create_record_count().await, 1);
    let pds_records = pds.list_records(user_did, "dev.cospan.repo").await;
    assert_eq!(pds_records.len(), 1);
    assert_eq!(pds_records[0].value["name"], source_repo_name);
    assert_eq!(pds_records[0].value["sourceRepo"], source_at_uri);
    assert_eq!(pds_records[0].value["protocol"], "raw-file");
    let proof = pds.last_dpop_proof().await.expect("dpop proof present");
    assert!(proof.split('.').count() == 3, "proof must be a JWT");

    // 9. Wait for the background git copy to complete.
    let job = wait_for_fork_job(&pool, fork_job_id, Duration::from_secs(30)).await;
    assert_eq!(
        job.state, "completed",
        "fork job failed: {:?}",
        job.last_error
    );
    assert!(
        job.refs_copied >= 1,
        "expected at least one ref copied, got {}",
        job.refs_copied
    );

    // 10. Clone from the destination node and verify file contents.
    let dest_git_url = dest_node.git_url(user_did, &fork_rkey);
    let (cloned, _cloned_tmp) = clone_from_node(&dest_git_url).await;

    // The destination must have the main branch pointing at the same
    // commit as the source.
    let head = cloned.head().unwrap();
    let cloned_oid = head
        .target()
        .map(|o| o.to_string())
        .expect("head has target");
    assert_eq!(cloned_oid, commit_oid, "fork should point at source commit");

    // Walk the commit's tree and verify both files exist with the
    // expected contents.
    let commit = cloned.find_commit(head.target().unwrap()).unwrap();
    let tree = commit.tree().unwrap();

    let readme = tree.get_name("README.md").expect("README.md in tree");
    let readme_blob = readme.to_object(&cloned).unwrap();
    let readme_content = String::from_utf8_lossy(
        readme_blob.as_blob().unwrap().content(),
    )
    .to_string();
    assert!(readme_content.contains("hello"));

    let src_dir = tree.get_name("src").expect("src/ in tree");
    let src_tree = src_dir.to_object(&cloned).unwrap();
    let src_tree = src_tree.as_tree().unwrap();
    let main_txt = src_tree.get_name("main.txt").expect("src/main.txt");
    let main_blob = main_txt.to_object(&cloned).unwrap();
    let main_content = String::from_utf8_lossy(
        main_blob.as_blob().unwrap().content(),
    )
    .to_string();
    assert_eq!(main_content, "hello from main\n");
}

// ═══════════════════════════════════════════════════════════════════
// Unit tests for the git_copy module using real nodes
// ═══════════════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread")]
async fn git_copy_transfers_all_files() {
    let source = TestNode::spawn().await;
    let dest = TestNode::spawn().await;

    let src_url = source.git_url("did:plc:owner", "repo");
    let dst_url = dest.git_url("did:plc:owner", "repo-fork");

    let (_commit, _tmp) = seed_git_repo(
        &src_url,
        &[
            ("a.txt", "file a\n"),
            ("b.txt", "file b\n"),
            ("dir/c.txt", "file c in subdir\n"),
        ],
    )
    .await;

    let report = tokio::task::spawn_blocking(move || {
        cospan_appview::git_copy::copy_repo(
            &src_url,
            &dst_url,
            cospan_appview::git_copy::CopyOptions {
                dest_creds: Some(cospan_appview::git_copy::basic_auth_creds(
                    "cospan-appview".to_string(),
                    String::new(),
                )),
                ..cospan_appview::git_copy::CopyOptions::default()
            },
        )
    })
    .await
    .unwrap()
    .expect("copy should succeed");

    assert!(report.refs_copied >= 1);
    assert!(
        report.refs.iter().any(|(name, _)| name == "refs/heads/main"),
        "expected main branch in copied refs"
    );

    // Verify by cloning from the destination.
    let (cloned, _tmp2) = clone_from_node(&dest.git_url("did:plc:owner", "repo-fork")).await;
    let head = cloned.head().unwrap();
    let commit = cloned.find_commit(head.target().unwrap()).unwrap();
    let tree = commit.tree().unwrap();

    assert!(tree.get_name("a.txt").is_some());
    assert!(tree.get_name("b.txt").is_some());
    assert!(tree.get_name("dir").is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn git_copy_reports_empty_source() {
    let source = TestNode::spawn().await;
    let dest = TestNode::spawn().await;
    let src_url = source.git_url("did:plc:owner", "nothing");
    let dst_url = dest.git_url("did:plc:owner", "nothing-fork");

    let err = tokio::task::spawn_blocking(move || {
        cospan_appview::git_copy::copy_repo(
            &src_url,
            &dst_url,
            cospan_appview::git_copy::CopyOptions::default(),
        )
    })
    .await
    .unwrap()
    .unwrap_err();

    // Empty source should fail — either no refs to fetch or fetch errors.
    let msg = err.to_string();
    assert!(
        msg.contains("refs")
            || msg.contains("empty")
            || msg.contains("404")
            || msg.contains("not found"),
        "unexpected error: {msg}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn probe_info_refs_receive_pack() {
    // Debug: verify info/refs returns a valid response for a new repo
    // when requested with service=git-receive-pack.
    let node = TestNode::spawn().await;
    let client = reqwest::Client::new();
    let url = format!(
        "{}/did:plc:probe/new-repo/info/refs?service=git-receive-pack",
        node.url
    );
    eprintln!("probe: GET {url}");
    let resp = client.get(&url).send().await.unwrap();
    let status = resp.status();
    let body = resp.text().await.unwrap();
    eprintln!("probe: status={status} body_len={}", body.len());
    eprintln!("probe: body = {body:?}");
    assert_eq!(status, 200, "info/refs should return 200 for receive-pack on empty repo");
    assert!(body.contains("git-receive-pack"));
}

// ═══════════════════════════════════════════════════════════════════
// listCommits / diffCommits tests (commit graph + structural diff)
// ═══════════════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread")]
async fn list_commits_returns_history_with_parents() {
    let node = TestNode::spawn().await;
    let git_url = node.git_url("did:plc:owner", "graph-test");

    let (oids, _tmp) = seed_git_repo_with_history(
        &git_url,
        &[
            CommitSpec {
                message: "initial commit",
                files: &[("README.md", "# hello\n")],
            },
            CommitSpec {
                message: "add main.txt",
                files: &[("README.md", "# hello\n"), ("main.txt", "first\n")],
            },
            CommitSpec {
                message: "update main.txt",
                files: &[("README.md", "# hello\n"), ("main.txt", "second\n")],
            },
        ],
    )
    .await;

    let url = format!(
        "{}/xrpc/dev.cospan.node.listCommits?did=did:plc:owner&repo=graph-test&limit=50",
        node.url
    );
    let client = Client::new();
    let resp = client.get(&url).send().await.unwrap();
    assert_eq!(resp.status(), 200, "listCommits failed");
    let json: serde_json::Value = resp.json().await.unwrap();

    let commits = json["commits"].as_array().expect("commits array");
    assert_eq!(commits.len(), 3, "should have 3 commits");

    // Newest first (topological + time sort)
    assert_eq!(commits[0]["oid"], oids[2]);
    assert_eq!(commits[1]["oid"], oids[1]);
    assert_eq!(commits[2]["oid"], oids[0]);

    // Parent linkage
    let parents_0 = commits[0]["parents"].as_array().unwrap();
    assert_eq!(parents_0.len(), 1);
    assert_eq!(parents_0[0], oids[1]);
    let parents_2 = commits[2]["parents"].as_array().unwrap();
    assert_eq!(parents_2.len(), 0, "root commit has no parents");

    assert_eq!(commits[0]["summary"], "update main.txt");
    assert_eq!(commits[2]["summary"], "initial commit");
}

#[tokio::test(flavor = "multi_thread")]
async fn list_commits_returns_404_for_unknown_repo() {
    let node = TestNode::spawn().await;
    let url = format!(
        "{}/xrpc/dev.cospan.node.listCommits?did=did:plc:nobody&repo=nothing",
        node.url
    );
    let client = Client::new();
    let resp = client.get(&url).send().await.unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test(flavor = "multi_thread")]
async fn diff_commits_shows_added_modified_removed_files() {
    let node = TestNode::spawn().await;
    let git_url = node.git_url("did:plc:owner", "diff-test");

    let (oids, _tmp) = seed_git_repo_with_history(
        &git_url,
        &[
            CommitSpec {
                message: "initial",
                files: &[("a.txt", "a contents\n"), ("b.txt", "b original\n")],
            },
            CommitSpec {
                message: "changes",
                // a.txt removed (not in this commit's files), b.txt modified, c.txt added
                files: &[("b.txt", "b modified\n"), ("c.txt", "c new\n")],
            },
        ],
    )
    .await;

    let url = format!(
        "{}/xrpc/dev.cospan.node.diffCommits?did=did:plc:owner&repo=diff-test&from={}&to={}",
        node.url, oids[0], oids[1]
    );
    let client = Client::new();
    let resp = client.get(&url).send().await.unwrap();
    assert_eq!(resp.status(), 200, "diffCommits failed");
    let json: serde_json::Value = resp.json().await.unwrap();

    let files = json["files"].as_array().expect("files array");
    assert_eq!(files.len(), 3, "should have 3 changed files");

    // libgit2 reports removed files with the old path in `old_file.path()`
    // and no new path — we copy the `new_file.path()` into our `path`,
    // which for a delete is the empty string. So look up by iterating.
    let find = |status: &str| -> &serde_json::Value {
        files
            .iter()
            .find(|f| f["status"].as_str() == Some(status))
            .unwrap_or_else(|| panic!("no file with status {status}"))
    };

    let removed = find("removed");
    assert!(removed["deletions"].as_i64().unwrap() >= 1);

    let modified = find("modified");
    assert_eq!(modified["path"], "b.txt");
    assert!(modified["additions"].as_i64().unwrap() >= 1);
    assert!(modified["deletions"].as_i64().unwrap() >= 1);

    let added = find("added");
    assert_eq!(added["path"], "c.txt");
    assert!(added["additions"].as_i64().unwrap() >= 1);

    let total_add = json["totalAdditions"].as_i64().unwrap();
    let total_del = json["totalDeletions"].as_i64().unwrap();
    assert!(total_add >= 2);
    assert!(total_del >= 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn diff_commits_returns_hunks_with_line_content() {
    let node = TestNode::spawn().await;
    let git_url = node.git_url("did:plc:owner", "hunk-test");

    let (oids, _tmp) = seed_git_repo_with_history(
        &git_url,
        &[
            CommitSpec {
                message: "initial",
                files: &[("code.txt", "line1\nline2\nline3\n")],
            },
            CommitSpec {
                message: "edit",
                files: &[("code.txt", "line1\nline2 MODIFIED\nline3\nline4\n")],
            },
        ],
    )
    .await;

    let url = format!(
        "{}/xrpc/dev.cospan.node.diffCommits?did=did:plc:owner&repo=hunk-test&from={}&to={}",
        node.url, oids[0], oids[1]
    );
    let client = Client::new();
    let resp = client.get(&url).send().await.unwrap();
    let json: serde_json::Value = resp.json().await.unwrap();

    let files = json["files"].as_array().unwrap();
    assert_eq!(files.len(), 1);
    let hunks = files[0]["hunks"].as_array().expect("hunks array");
    assert!(!hunks.is_empty(), "should have at least one hunk");

    let lines = hunks[0]["lines"].as_array().expect("lines array");
    let origins: Vec<&str> = lines
        .iter()
        .map(|l| l["origin"].as_str().unwrap_or_default())
        .collect();
    assert!(origins.contains(&"-"), "should have a removed line");
    assert!(origins.contains(&"+"), "should have an added line");

    let content_added: Vec<String> = lines
        .iter()
        .filter(|l| l["origin"].as_str() == Some("+"))
        .map(|l| l["content"].as_str().unwrap_or_default().to_string())
        .collect();
    assert!(content_added.iter().any(|c| c.contains("MODIFIED")));
}

#[tokio::test(flavor = "multi_thread")]
async fn diff_commits_reports_no_changes_for_same_commit() {
    let node = TestNode::spawn().await;
    let git_url = node.git_url("did:plc:owner", "same-test");
    let (oids, _tmp) = seed_git_repo_with_history(
        &git_url,
        &[CommitSpec {
            message: "only",
            files: &[("f.txt", "hello\n")],
        }],
    )
    .await;

    let url = format!(
        "{}/xrpc/dev.cospan.node.diffCommits?did=did:plc:owner&repo=same-test&from={}&to={}",
        node.url, oids[0], oids[0]
    );
    let client = Client::new();
    let resp = client.get(&url).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["fileCount"].as_i64().unwrap(), 0);
    assert_eq!(json["totalAdditions"].as_i64().unwrap(), 0);
    assert_eq!(json["totalDeletions"].as_i64().unwrap(), 0);
}

#[tokio::test]
async fn git_copy_errors_on_empty_source_url() {
    let err = tokio::task::spawn_blocking(|| {
        cospan_appview::git_copy::copy_repo(
            "",
            "http://dest/",
            cospan_appview::git_copy::CopyOptions::default(),
        )
    })
    .await
    .unwrap()
    .unwrap_err();
    matches!(err, cospan_appview::git_copy::GitCopyError::MissingSource);
}

#[tokio::test]
async fn git_copy_errors_on_empty_dest_url() {
    let err = tokio::task::spawn_blocking(|| {
        cospan_appview::git_copy::copy_repo(
            "http://src/",
            "",
            cospan_appview::git_copy::CopyOptions::default(),
        )
    })
    .await
    .unwrap()
    .unwrap_err();
    matches!(err, cospan_appview::git_copy::GitCopyError::MissingDest);
}
