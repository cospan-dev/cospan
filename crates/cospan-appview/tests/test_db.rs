//! Database module tests using sqlx::test for automatic test database lifecycle.
//!
//! Each test receives a fresh PgPool with migrations applied.
//! Requires DATABASE_URL in the environment.

use chrono::Utc;
use sqlx::PgPool;

use cospan_appview::db;

// ─── Nodes ───────────────────────────────────────────────────────

#[sqlx::test(migrator = "cospan_appview::MIGRATOR")]
async fn node_upsert_and_list(pool: PgPool) {
    let now = Utc::now();

    let node = db::node::NodeRow {
        did: "did:plc:node1".to_string(),
        rkey: "self".to_string(),
        public_endpoint: Some("https://node1.example.com".to_string()),
        created_at: now,
        indexed_at: now,
    };

    db::node::upsert(&pool, &node).await.unwrap();

    let node2 = db::node::NodeRow {
        did: "did:plc:node2".to_string(),
        rkey: "self".to_string(),
        public_endpoint: Some("https://node2.example.com".to_string()),
        created_at: now,
        indexed_at: now,
    };

    db::node::upsert(&pool, &node2).await.unwrap();

    let nodes = db::node::list(&pool, 10).await.unwrap();
    assert_eq!(nodes.len(), 2);

    // Upsert same DID updates the row (no new row)
    let updated = db::node::NodeRow {
        did: "did:plc:node1".to_string(),
        rkey: "self".to_string(),
        public_endpoint: Some("https://new-node1.example.com".to_string()),
        created_at: now,
        indexed_at: now,
    };
    db::node::upsert(&pool, &updated).await.unwrap();

    let nodes = db::node::list(&pool, 10).await.unwrap();
    assert_eq!(nodes.len(), 2);
}

// ─── Actor Profiles ──────────────────────────────────────────────

#[sqlx::test(migrator = "cospan_appview::MIGRATOR")]
async fn actor_profile_upsert_and_get(pool: PgPool) {
    let now = Utc::now();

    let profile = db::actor_profile::ActorProfileRow {
        did: "did:plc:alice".to_string(),
        bluesky: "alice.bsky.social".to_string(),
        display_name: Some("Alice".to_string()),
        description: Some("A developer".to_string()),
        avatar_cid: None,
        indexed_at: now,
    };

    db::actor_profile::upsert(&pool, &profile).await.unwrap();

    let fetched = db::actor_profile::get(&pool, "did:plc:alice")
        .await
        .unwrap()
        .expect("profile should exist");
    assert_eq!(fetched.display_name, Some("Alice".to_string()));
    assert_eq!(fetched.bluesky, "alice.bsky.social");

    // Get nonexistent
    let missing = db::actor_profile::get(&pool, "did:plc:nobody")
        .await
        .unwrap();
    assert!(missing.is_none());
}

// ─── Repos ───────────────────────────────────────────────────────

/// Insert a prerequisite node so repos can satisfy the node_did FK.
async fn insert_test_node(pool: &PgPool) {
    let now = Utc::now();
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
}

/// Insert a prerequisite repo so child tables (ref_updates, issues, stars) can satisfy FKs.
async fn insert_test_repo(pool: &PgPool, did: &str, name: &str) {
    let now = Utc::now();
    insert_test_node(pool).await;
    db::repo::upsert(
        pool,
        &db::repo::RepoRow {
            did: did.to_string(),
            rkey: "rkey1".to_string(),
            name: name.to_string(),
            description: None,
            protocol: "typescript".to_string(),
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
        },
    )
    .await
    .unwrap();
}

#[sqlx::test(migrator = "cospan_appview::MIGRATOR")]
async fn repo_upsert_get_and_list_recent(pool: PgPool) {
    let now = Utc::now();
    insert_test_node(&pool).await;

    let repo = db::repo::RepoRow {
        did: "did:plc:alice".to_string(),
        rkey: "repo1".to_string(),
        name: "my-project".to_string(),
        description: Some("A test project".to_string()),
        protocol: "typescript".to_string(),
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

    db::repo::upsert(&pool, &repo).await.unwrap();

    // Get by did + name
    let fetched = db::repo::get(&pool, "did:plc:alice", "my-project")
        .await
        .unwrap()
        .expect("repo should exist");
    assert_eq!(fetched.protocol, "typescript");
    assert_eq!(fetched.description, Some("A test project".to_string()));

    // Upsert another repo
    let repo2 = db::repo::RepoRow {
        did: "did:plc:bob".to_string(),
        rkey: "repo2".to_string(),
        name: "another-project".to_string(),
        description: Some("Bob's project".to_string()),
        protocol: "python".to_string(),
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
    db::repo::upsert(&pool, &repo2).await.unwrap();

    // List recent
    let recent = db::repo::list_recent(&pool, 10, None).await.unwrap();
    assert_eq!(recent.len(), 2);

    // Get nonexistent
    let missing = db::repo::get(&pool, "did:plc:alice", "nonexistent")
        .await
        .unwrap();
    assert!(missing.is_none());
}

// ─── Ref Updates ─────────────────────────────────────────────────

#[sqlx::test(migrator = "cospan_appview::MIGRATOR")]
async fn ref_update_upsert_and_list_for_repo(pool: PgPool) {
    let now = Utc::now();
    insert_test_repo(&pool, "did:plc:alice", "my-project").await;

    let update = db::ref_update::RefUpdateRow {
        id: 0,
        repo_did: "did:plc:alice".to_string(),
        repo_name: "my-project".to_string(),
        rkey: "update1".to_string(),
        committer_did: "did:plc:alice".to_string(),
        ref_name: "refs/heads/main".to_string(),
        old_target: None,
        new_target: "abc123".to_string(),
        protocol: "typescript".to_string(),
        migration_id: None,
        breaking_change_count: 0,
        lens_id: None,
        lens_quality: None,
        commit_count: 1,
        created_at: now,
        indexed_at: now,
    };

    db::ref_update::upsert(&pool, &update).await.unwrap();

    let update2 = db::ref_update::RefUpdateRow {
        id: 0,
        repo_did: "did:plc:alice".to_string(),
        repo_name: "my-project".to_string(),
        rkey: "update2".to_string(),
        committer_did: "did:plc:alice".to_string(),
        ref_name: "refs/heads/main".to_string(),
        old_target: Some("abc123".to_string()),
        new_target: "def456".to_string(),
        protocol: "typescript".to_string(),
        migration_id: None,
        breaking_change_count: 2,
        lens_id: Some("auto:abc->def".to_string()),
        lens_quality: Some(0.95),
        commit_count: 3,
        created_at: now,
        indexed_at: now,
    };

    db::ref_update::upsert(&pool, &update2).await.unwrap();

    let updates = db::ref_update::list_for_repo(&pool, "did:plc:alice", "my-project", 10, None)
        .await
        .unwrap();
    assert_eq!(updates.len(), 2);
}

// ─── Issues ──────────────────────────────────────────────────────

#[sqlx::test(migrator = "cospan_appview::MIGRATOR")]
async fn issue_upsert_get_and_list_for_repo(pool: PgPool) {
    let now = Utc::now();
    insert_test_repo(&pool, "did:plc:bob", "my-project").await;

    let issue = db::issue::IssueRow {
        did: "did:plc:alice".to_string(),
        rkey: "issue1".to_string(),
        repo_did: "did:plc:bob".to_string(),
        repo_name: "my-project".to_string(),
        title: "Bug: something is broken".to_string(),
        body: Some("Details about the bug".to_string()),
        state: "open".to_string(),
        comment_count: 0,
        created_at: now,
        indexed_at: now,
    };

    db::issue::upsert(&pool, &issue).await.unwrap();

    // Get by repo + rkey
    let fetched = db::issue::get(&pool, "did:plc:bob", "my-project", "issue1")
        .await
        .unwrap()
        .expect("issue should exist");
    assert_eq!(fetched.title, "Bug: something is broken");
    assert_eq!(fetched.state, "open");

    // Get by PK
    let fetched_pk = db::issue::get_by_pk(&pool, "did:plc:alice", "issue1")
        .await
        .unwrap()
        .expect("issue should exist");
    assert_eq!(fetched_pk.title, "Bug: something is broken");

    // List for repo
    let issues = db::issue::list_for_repo(&pool, "did:plc:bob", "my-project", None, 10, None)
        .await
        .unwrap();
    assert_eq!(issues.len(), 1);

    // List with state filter
    let open = db::issue::list_for_repo(&pool, "did:plc:bob", "my-project", Some("open"), 10, None)
        .await
        .unwrap();
    assert_eq!(open.len(), 1);

    let closed =
        db::issue::list_for_repo(&pool, "did:plc:bob", "my-project", Some("closed"), 10, None)
            .await
            .unwrap();
    assert_eq!(closed.len(), 0);
}

// ─── Stars ───────────────────────────────────────────────────────

#[sqlx::test(migrator = "cospan_appview::MIGRATOR")]
async fn star_upsert_and_list_by_user_with_count(pool: PgPool) {
    let now = Utc::now();
    insert_test_node(&pool).await;

    // First, create the repo to star
    let repo = db::repo::RepoRow {
        did: "did:plc:bob".to_string(),
        rkey: "repo1".to_string(),
        name: "cool-project".to_string(),
        description: Some("A cool project".to_string()),
        protocol: "typescript".to_string(),
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
    db::repo::upsert(&pool, &repo).await.unwrap();

    // Star the repo
    let star = db::star::StarRow {
        did: "did:plc:alice".to_string(),
        rkey: "star1".to_string(),
        subject: "at://did:plc:bob/dev.cospan.repo/cool-project".to_string(),
        created_at: now,
        indexed_at: now,
    };
    db::star::upsert(&pool, &star).await.unwrap();
    db::star::increment_repo_star_count(&pool, "did:plc:bob", "cool-project")
        .await
        .unwrap();

    // Verify star_count incremented
    let repo = db::repo::get(&pool, "did:plc:bob", "cool-project")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(repo.star_count, 1);

    // List by user
    let stars = db::star::list_by_user(&pool, "did:plc:alice", 10, None)
        .await
        .unwrap();
    assert_eq!(stars.len(), 1);
    assert_eq!(
        stars[0].subject,
        "at://did:plc:bob/dev.cospan.repo/cool-project"
    );

    // Star again from a different user
    let star2 = db::star::StarRow {
        did: "did:plc:charlie".to_string(),
        rkey: "star2".to_string(),
        subject: "at://did:plc:bob/dev.cospan.repo/cool-project".to_string(),
        created_at: now,
        indexed_at: now,
    };
    db::star::upsert(&pool, &star2).await.unwrap();
    db::star::increment_repo_star_count(&pool, "did:plc:bob", "cool-project")
        .await
        .unwrap();

    // Verify star_count is now 2
    let repo = db::repo::get(&pool, "did:plc:bob", "cool-project")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(repo.star_count, 2);

    // Decrement on unstar
    db::star::decrement_repo_star_count(&pool, "did:plc:bob", "cool-project")
        .await
        .unwrap();
    let repo = db::repo::get(&pool, "did:plc:bob", "cool-project")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(repo.star_count, 1);
}
