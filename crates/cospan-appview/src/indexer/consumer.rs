use std::sync::Arc;

use chrono::Utc;

use crate::db;
use crate::state::AppState;
use crate::xrpc::sse::IndexEvent;

/// Process a single Jetstream event by dispatching on collection.
pub async fn process_event(state: &Arc<AppState>, event: &serde_json::Value) -> anyhow::Result<()> {
    let commit = match event.get("commit") {
        Some(c) => c,
        None => return Ok(()), // Not a commit event (e.g., identity, account)
    };

    let collection = commit
        .get("collection")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let operation = commit
        .get("operation")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let did = event.get("did").and_then(|v| v.as_str()).unwrap_or("");
    let rkey = commit.get("rkey").and_then(|v| v.as_str()).unwrap_or("");
    let record = commit.get("record");

    match (collection, operation) {
        // ─── Node ───────────────────────────────────────────────────
        ("dev.cospan.node", "create" | "update") => {
            if let Some(rec) = record {
                let row = db::node::NodeRow {
                    did: did.to_string(),
                    rkey: rkey.to_string(),
                    public_endpoint: rec
                        .get("publicEndpoint")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    created_at: parse_datetime(rec, "createdAt"),
                    indexed_at: Utc::now(),
                };
                db::node::upsert(&state.db, &row).await?;
            }
        }
        ("dev.cospan.node", "delete") => {
            db::node::delete(&state.db, did).await?;
        }

        // ─── Actor Profile ──────────────────────────────────────────
        ("dev.cospan.actor.profile", "create" | "update") => {
            if let Some(rec) = record {
                let row = db::actor_profile::ActorProfileRow {
                    did: did.to_string(),
                    bluesky: rec
                        .get("bluesky")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    display_name: rec
                        .get("displayName")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    description: rec
                        .get("description")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    avatar_cid: rec
                        .get("avatar")
                        .and_then(|v| v.get("ref"))
                        .and_then(|v| v.get("$link"))
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    indexed_at: Utc::now(),
                };
                db::actor_profile::upsert(&state.db, &row).await?;
            }
        }
        ("dev.cospan.actor.profile", "delete") => {
            db::actor_profile::delete(&state.db, did).await?;
        }

        // ─── Repo ───────────────────────────────────────────────────
        ("dev.cospan.repo", "create" | "update") => {
            if let Some(rec) = record {
                // Extract node DID and URL from the node AT-URI
                let node_uri = rec.get("node").and_then(|v| v.as_str()).unwrap_or("");
                let node_did = extract_did_from_at_uri(node_uri);

                // Look up node URL from nodes table
                let node_url = {
                    let nodes = db::node::list(&state.db, 1000).await?;
                    nodes
                        .iter()
                        .find(|n| n.did == node_did)
                        .and_then(|n| n.public_endpoint.clone())
                        .unwrap_or_default()
                };

                let row = db::repo::RepoRow {
                    did: did.to_string(),
                    rkey: rkey.to_string(),
                    name: rec
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    description: rec
                        .get("description")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    protocol: rec
                        .get("protocol")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    node_did,
                    node_url,
                    default_branch: rec
                        .get("defaultBranch")
                        .and_then(|v| v.as_str())
                        .unwrap_or("main")
                        .to_string(),
                    visibility: rec
                        .get("visibility")
                        .and_then(|v| v.as_str())
                        .unwrap_or("public")
                        .to_string(),
                    source_repo: rec
                        .get("sourceRepo")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    star_count: 0,
                    fork_count: 0,
                    open_issue_count: 0,
                    open_mr_count: 0,
                    source: "cospan".to_string(),
                    source_uri: None,
                    created_at: parse_datetime(rec, "createdAt"),
                    indexed_at: Utc::now(),
                };
                db::repo::upsert(&state.db, &row).await?;
            }
        }
        ("dev.cospan.repo", "delete") => {
            // Need to look up repo name from rkey - for now just log
            tracing::warn!(
                did,
                rkey,
                "repo delete not fully implemented (need rkey->name lookup)"
            );
        }

        // ─── Ref Update ─────────────────────────────────────────────
        ("dev.cospan.vcs.refUpdate", "create" | "update") => {
            if let Some(rec) = record {
                let repo_uri = rec.get("repo").and_then(|v| v.as_str()).unwrap_or("");
                let (repo_did, repo_name) = parse_repo_at_uri(repo_uri);

                let breaking_changes = rec
                    .get("breakingChanges")
                    .and_then(|v| v.as_array())
                    .map(|a| a.len() as i32)
                    .unwrap_or(0);

                let row = db::ref_update::RefUpdateRow {
                    id: 0, // auto-generated
                    repo_did,
                    repo_name,
                    rkey: rkey.to_string(),
                    committer_did: rec
                        .get("committerDid")
                        .and_then(|v| v.as_str())
                        .unwrap_or(did)
                        .to_string(),
                    ref_name: rec
                        .get("ref")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    old_target: rec
                        .get("oldTarget")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    new_target: rec
                        .get("newTarget")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    protocol: rec
                        .get("protocol")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    migration_id: rec
                        .get("migrationId")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    breaking_change_count: breaking_changes,
                    lens_id: rec.get("lensId").and_then(|v| v.as_str()).map(String::from),
                    lens_quality: rec
                        .get("lensQuality")
                        .and_then(|v| v.as_f64())
                        .map(|f| f as f32),
                    commit_count: rec.get("commitCount").and_then(|v| v.as_i64()).unwrap_or(0)
                        as i32,
                    created_at: parse_datetime(rec, "createdAt"),
                    indexed_at: Utc::now(),
                };
                db::ref_update::upsert(&state.db, &row).await?;

                // Publish SSE event
                let _ = state.event_tx.send(IndexEvent::RefUpdate {
                    repo_did: row.repo_did.clone(),
                    repo_name: row.repo_name.clone(),
                    ref_name: row.ref_name.clone(),
                    new_target: row.new_target.clone(),
                    committer_did: row.committer_did.clone(),
                    breaking_change_count: row.breaking_change_count,
                });
            }
        }
        ("dev.cospan.vcs.refUpdate", "delete") => {
            db::ref_update::delete(&state.db, did, rkey).await?;
        }

        // ─── Issue ──────────────────────────────────────────────────
        ("dev.cospan.repo.issue", "create" | "update") => {
            if let Some(rec) = record {
                let repo_uri = rec.get("repo").and_then(|v| v.as_str()).unwrap_or("");
                let (repo_did, repo_name) = parse_repo_at_uri(repo_uri);

                let row = db::issue::IssueRow {
                    did: did.to_string(),
                    rkey: rkey.to_string(),
                    repo_did,
                    repo_name,
                    title: rec
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    body: rec.get("body").and_then(|v| v.as_str()).map(String::from),
                    state: "open".to_string(),
                    comment_count: 0,
                    created_at: parse_datetime(rec, "createdAt"),
                    indexed_at: Utc::now(),
                };
                db::issue::upsert(&state.db, &row).await?;

                // Publish SSE event
                let _ = state.event_tx.send(IndexEvent::IssueCreated {
                    repo_did: row.repo_did.clone(),
                    repo_name: row.repo_name.clone(),
                    issue_rkey: row.rkey.clone(),
                    title: row.title.clone(),
                    author_did: row.did.clone(),
                });
            }
        }
        ("dev.cospan.repo.issue", "delete") => {
            // Look up the issue to decrement repo counter
            if let Some(issue) = db::issue::get_by_pk(&state.db, did, rkey).await?
                && issue.state == "open"
            {
                decrement_repo_open_issue_count(&state.db, &issue.repo_did, &issue.repo_name)
                    .await?;
            }
            db::issue::delete(&state.db, did, rkey).await?;
        }

        // ─── Issue Comment ──────────────────────────────────────────
        ("dev.cospan.repo.issue.comment", "create" | "update") => {
            if let Some(rec) = record {
                let issue_uri = rec
                    .get("issue")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let row = db::issue_comment::IssueCommentRow {
                    did: did.to_string(),
                    rkey: rkey.to_string(),
                    issue_uri: issue_uri.clone(),
                    body: rec
                        .get("body")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    created_at: parse_datetime(rec, "createdAt"),
                    indexed_at: Utc::now(),
                };

                // Check if this is a new comment (not an update) for counter purposes
                let existing = db::issue_comment::get(&state.db, did, rkey).await?;
                db::issue_comment::upsert(&state.db, &row).await?;

                if existing.is_none() {
                    // Increment comment count on the issue
                    let (issue_did, issue_rkey) = parse_at_uri_did_rkey(&issue_uri);
                    db::issue::increment_comment_count(&state.db, &issue_did, &issue_rkey).await?;
                }
            }
        }
        ("dev.cospan.repo.issue.comment", "delete") => {
            // Look up the comment to decrement the issue counter
            if let Some(comment) = db::issue_comment::get(&state.db, did, rkey).await? {
                let (issue_did, issue_rkey) = parse_at_uri_did_rkey(&comment.issue_uri);
                db::issue::decrement_comment_count(&state.db, &issue_did, &issue_rkey).await?;
            }
            db::issue_comment::delete(&state.db, did, rkey).await?;
        }

        // ─── Issue State ────────────────────────────────────────────
        ("dev.cospan.repo.issue.state", "create" | "update") => {
            if let Some(rec) = record {
                let issue_uri = rec
                    .get("issue")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let new_state = rec
                    .get("state")
                    .and_then(|v| v.as_str())
                    .unwrap_or("open")
                    .to_string();

                let row = db::issue_state::IssueStateRow {
                    did: did.to_string(),
                    rkey: rkey.to_string(),
                    issue_uri: issue_uri.clone(),
                    state: new_state.clone(),
                    reason: rec.get("reason").and_then(|v| v.as_str()).map(String::from),
                    created_at: parse_datetime(rec, "createdAt"),
                    indexed_at: Utc::now(),
                };
                db::issue_state::upsert(&state.db, &row).await?;

                // Update the issue's state and repo counters
                let (issue_did, issue_rkey) = parse_at_uri_did_rkey(&issue_uri);
                if let Some(issue) =
                    db::issue::get_by_pk(&state.db, &issue_did, &issue_rkey).await?
                {
                    let old_state = &issue.state;
                    if old_state != &new_state {
                        db::issue::update_state(&state.db, &issue_did, &issue_rkey, &new_state)
                            .await?;

                        // Adjust repo open_issue_count
                        if old_state == "open" && new_state != "open" {
                            decrement_repo_open_issue_count(
                                &state.db,
                                &issue.repo_did,
                                &issue.repo_name,
                            )
                            .await?;
                        } else if old_state != "open" && new_state == "open" {
                            increment_repo_open_issue_count(
                                &state.db,
                                &issue.repo_did,
                                &issue.repo_name,
                            )
                            .await?;
                        }

                        // Publish SSE event
                        let _ = state.event_tx.send(IndexEvent::IssueStateChanged {
                            repo_did: issue.repo_did.clone(),
                            repo_name: issue.repo_name.clone(),
                            issue_rkey: issue_rkey.clone(),
                            old_state: old_state.clone(),
                            new_state: new_state.clone(),
                        });
                    }
                }
            }
        }
        ("dev.cospan.repo.issue.state", "delete") => {
            db::issue_state::delete(&state.db, did, rkey).await?;
        }

        // ─── Pull Request ───────────────────────────────────────────
        ("dev.cospan.repo.pull", "create" | "update") => {
            if let Some(rec) = record {
                let repo_uri = rec.get("repo").and_then(|v| v.as_str()).unwrap_or("");
                let (repo_did, repo_name) = parse_repo_at_uri(repo_uri);

                let row = db::pull::PullRow {
                    did: did.to_string(),
                    rkey: rkey.to_string(),
                    repo_did,
                    repo_name,
                    title: rec
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    body: rec.get("body").and_then(|v| v.as_str()).map(String::from),
                    target_ref: rec
                        .get("targetRef")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    source_ref: rec
                        .get("sourceRef")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    source_repo: rec
                        .get("sourceRepo")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    state: "open".to_string(),
                    comment_count: 0,
                    created_at: parse_datetime(rec, "createdAt"),
                    indexed_at: Utc::now(),
                };
                db::pull::upsert(&state.db, &row).await?;

                // Publish SSE event
                let _ = state.event_tx.send(IndexEvent::PullCreated {
                    repo_did: row.repo_did.clone(),
                    repo_name: row.repo_name.clone(),
                    pull_rkey: row.rkey.clone(),
                    title: row.title.clone(),
                    author_did: row.did.clone(),
                });
            }
        }
        ("dev.cospan.repo.pull", "delete") => {
            // Look up the pull to decrement repo counter
            if let Some(pull) = db::pull::get_by_pk(&state.db, did, rkey).await?
                && pull.state == "open"
            {
                decrement_repo_open_mr_count(&state.db, &pull.repo_did, &pull.repo_name).await?;
            }
            db::pull::delete(&state.db, did, rkey).await?;
        }

        // ─── Pull Comment ───────────────────────────────────────────
        ("dev.cospan.repo.pull.comment", "create" | "update") => {
            if let Some(rec) = record {
                let pull_uri = rec
                    .get("pull")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let row = db::pull_comment::PullCommentRow {
                    did: did.to_string(),
                    rkey: rkey.to_string(),
                    pull_uri: pull_uri.clone(),
                    body: rec
                        .get("body")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    review_decision: rec
                        .get("reviewDecision")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    created_at: parse_datetime(rec, "createdAt"),
                    indexed_at: Utc::now(),
                };

                // Check if this is a new comment (not an update) for counter purposes
                let existing = db::pull_comment::get(&state.db, did, rkey).await?;
                db::pull_comment::upsert(&state.db, &row).await?;

                if existing.is_none() {
                    let (pull_did, pull_rkey) = parse_at_uri_did_rkey(&pull_uri);
                    db::pull::increment_comment_count(&state.db, &pull_did, &pull_rkey).await?;
                }
            }
        }
        ("dev.cospan.repo.pull.comment", "delete") => {
            if let Some(comment) = db::pull_comment::get(&state.db, did, rkey).await? {
                let (pull_did, pull_rkey) = parse_at_uri_did_rkey(&comment.pull_uri);
                db::pull::decrement_comment_count(&state.db, &pull_did, &pull_rkey).await?;
            }
            db::pull_comment::delete(&state.db, did, rkey).await?;
        }

        // ─── Pull State ─────────────────────────────────────────────
        ("dev.cospan.repo.pull.state", "create" | "update") => {
            if let Some(rec) = record {
                let pull_uri = rec
                    .get("pull")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let new_state = rec
                    .get("state")
                    .and_then(|v| v.as_str())
                    .unwrap_or("open")
                    .to_string();

                let row = db::pull_state::PullStateRow {
                    did: did.to_string(),
                    rkey: rkey.to_string(),
                    pull_uri: pull_uri.clone(),
                    state: new_state.clone(),
                    merge_commit_id: rec
                        .get("mergeCommitId")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    created_at: parse_datetime(rec, "createdAt"),
                    indexed_at: Utc::now(),
                };
                db::pull_state::upsert(&state.db, &row).await?;

                // Update the pull's state and repo counters
                let (pull_did, pull_rkey) = parse_at_uri_did_rkey(&pull_uri);
                if let Some(pull) = db::pull::get_by_pk(&state.db, &pull_did, &pull_rkey).await? {
                    let old_state = &pull.state;
                    if old_state != &new_state {
                        db::pull::update_state(&state.db, &pull_did, &pull_rkey, &new_state)
                            .await?;

                        // Adjust repo open_mr_count
                        if old_state == "open" && new_state != "open" {
                            decrement_repo_open_mr_count(
                                &state.db,
                                &pull.repo_did,
                                &pull.repo_name,
                            )
                            .await?;
                        } else if old_state != "open" && new_state == "open" {
                            increment_repo_open_mr_count(
                                &state.db,
                                &pull.repo_did,
                                &pull.repo_name,
                            )
                            .await?;
                        }

                        // Publish SSE event
                        let _ = state.event_tx.send(IndexEvent::PullStateChanged {
                            repo_did: pull.repo_did.clone(),
                            repo_name: pull.repo_name.clone(),
                            pull_rkey: pull_rkey.clone(),
                            old_state: old_state.clone(),
                            new_state: new_state.clone(),
                        });
                    }
                }
            }
        }
        ("dev.cospan.repo.pull.state", "delete") => {
            db::pull_state::delete(&state.db, did, rkey).await?;
        }

        // ─── Star ───────────────────────────────────────────────────
        ("dev.cospan.feed.star", "create" | "update") => {
            if let Some(rec) = record {
                let subject = rec
                    .get("subject")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let row = db::star::StarRow {
                    did: did.to_string(),
                    rkey: rkey.to_string(),
                    subject: subject.clone(),
                    created_at: parse_datetime(rec, "createdAt"),
                    indexed_at: Utc::now(),
                };

                let existing = db::star::get(&state.db, did, rkey).await?;
                db::star::upsert(&state.db, &row).await?;

                if existing.is_none() {
                    let (repo_did, repo_name) = parse_repo_at_uri(&subject);
                    db::star::increment_repo_star_count(&state.db, &repo_did, &repo_name).await?;

                    // Publish SSE event
                    let _ = state.event_tx.send(IndexEvent::StarCreated {
                        did: did.to_string(),
                        subject: subject.clone(),
                    });
                }
            }
        }
        ("dev.cospan.feed.star", "delete") => {
            if let Some(star) = db::star::get(&state.db, did, rkey).await? {
                let (repo_did, repo_name) = parse_repo_at_uri(&star.subject);
                db::star::decrement_repo_star_count(&state.db, &repo_did, &repo_name).await?;

                // Publish SSE event
                let _ = state.event_tx.send(IndexEvent::StarDeleted {
                    did: did.to_string(),
                    subject: star.subject.clone(),
                });
            }
            db::star::delete(&state.db, did, rkey).await?;
        }

        // ─── Follow ─────────────────────────────────────────────────
        ("dev.cospan.graph.follow", "create" | "update") => {
            if let Some(rec) = record {
                let row = db::follow::FollowRow {
                    did: did.to_string(),
                    rkey: rkey.to_string(),
                    subject: rec
                        .get("subject")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    created_at: parse_datetime(rec, "createdAt"),
                    indexed_at: Utc::now(),
                };
                db::follow::upsert(&state.db, &row).await?;
            }
        }
        ("dev.cospan.graph.follow", "delete") => {
            db::follow::delete(&state.db, did, rkey).await?;
        }

        // ─── Reaction ───────────────────────────────────────────────
        ("dev.cospan.feed.reaction", "create" | "update") => {
            if let Some(rec) = record {
                let row = db::reaction::ReactionRow {
                    did: did.to_string(),
                    rkey: rkey.to_string(),
                    subject: rec
                        .get("subject")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    emoji: rec
                        .get("emoji")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    created_at: parse_datetime(rec, "createdAt"),
                    indexed_at: Utc::now(),
                };
                db::reaction::upsert(&state.db, &row).await?;
            }
        }
        ("dev.cospan.feed.reaction", "delete") => {
            db::reaction::delete(&state.db, did, rkey).await?;
        }

        // ─── Label Definition ───────────────────────────────────────
        ("dev.cospan.label.definition", "create" | "update") => {
            if let Some(rec) = record {
                let repo_uri = rec.get("repo").and_then(|v| v.as_str()).unwrap_or("");
                let (repo_did, repo_name) = parse_repo_at_uri(repo_uri);

                let row = db::label::LabelRow {
                    did: did.to_string(),
                    rkey: rkey.to_string(),
                    repo_did,
                    repo_name,
                    name: rec
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    color: rec
                        .get("color")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    description: rec
                        .get("description")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    created_at: parse_datetime(rec, "createdAt"),
                    indexed_at: Utc::now(),
                };
                db::label::upsert(&state.db, &row).await?;
            }
        }
        ("dev.cospan.label.definition", "delete") => {
            db::label::delete(&state.db, did, rkey).await?;
        }

        // ─── Org ────────────────────────────────────────────────────
        ("dev.cospan.org", "create" | "update") => {
            if let Some(rec) = record {
                let row = db::org::OrgRow {
                    did: did.to_string(),
                    rkey: rkey.to_string(),
                    name: rec
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    description: rec
                        .get("description")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    avatar_cid: rec
                        .get("avatar")
                        .and_then(|v| v.get("ref"))
                        .and_then(|v| v.get("$link"))
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    created_at: parse_datetime(rec, "createdAt"),
                    indexed_at: Utc::now(),
                };
                db::org::upsert(&state.db, &row).await?;
            }
        }
        ("dev.cospan.org", "delete") => {
            db::org::delete(&state.db, did, rkey).await?;
        }

        // ─── Org Member ─────────────────────────────────────────────
        ("dev.cospan.org.member", "create" | "update") => {
            if let Some(rec) = record {
                let row = db::org_member::OrgMemberRow {
                    did: did.to_string(),
                    rkey: rkey.to_string(),
                    org_uri: rec
                        .get("org")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    member_did: rec
                        .get("member")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    role: rec
                        .get("role")
                        .and_then(|v| v.as_str())
                        .unwrap_or("member")
                        .to_string(),
                    created_at: parse_datetime(rec, "createdAt"),
                    indexed_at: Utc::now(),
                };
                db::org_member::upsert(&state.db, &row).await?;
            }
        }
        ("dev.cospan.org.member", "delete") => {
            db::org_member::delete(&state.db, did, rkey).await?;
        }

        // ─── Collaborator ───────────────────────────────────────────
        ("dev.cospan.repo.collaborator", "create" | "update") => {
            if let Some(rec) = record {
                let repo_uri = rec.get("repo").and_then(|v| v.as_str()).unwrap_or("");
                let (repo_did, repo_name) = parse_repo_at_uri(repo_uri);

                let row = db::collaborator::CollaboratorRow {
                    did: did.to_string(),
                    rkey: rkey.to_string(),
                    repo_did,
                    repo_name,
                    member_did: rec
                        .get("did")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    role: rec
                        .get("role")
                        .and_then(|v| v.as_str())
                        .unwrap_or("reader")
                        .to_string(),
                    created_at: parse_datetime(rec, "createdAt"),
                    indexed_at: Utc::now(),
                };
                db::collaborator::upsert(&state.db, &row).await?;
            }
        }
        ("dev.cospan.repo.collaborator", "delete") => {
            db::collaborator::delete(&state.db, did, rkey).await?;
        }

        // ─── Pipeline ───────────────────────────────────────────────
        ("dev.cospan.pipeline", "create" | "update") => {
            if let Some(rec) = record {
                let repo_uri = rec.get("repo").and_then(|v| v.as_str()).unwrap_or("");
                let (repo_did, repo_name) = parse_repo_at_uri(repo_uri);

                let checks = rec.get("algebraicChecks");

                let row = db::pipeline::PipelineRow {
                    did: did.to_string(),
                    rkey: rkey.to_string(),
                    repo_did,
                    repo_name,
                    commit_id: rec
                        .get("commitId")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    ref_name: rec.get("ref").and_then(|v| v.as_str()).map(String::from),
                    status: rec
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("pending")
                        .to_string(),
                    gat_type_check: checks
                        .and_then(|c| c.get("gatTypeCheck"))
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    equation_verification: checks
                        .and_then(|c| c.get("equationVerification"))
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    lens_law_check: checks
                        .and_then(|c| c.get("lensLawCheck"))
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    breaking_change_check: checks
                        .and_then(|c| c.get("breakingChangeCheck"))
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    created_at: parse_datetime(rec, "createdAt"),
                    completed_at: rec
                        .get("completedAt")
                        .and_then(|v| v.as_str())
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                        .map(|dt| dt.with_timezone(&Utc)),
                    indexed_at: Utc::now(),
                };
                db::pipeline::upsert(&state.db, &row).await?;
            }
        }
        ("dev.cospan.pipeline", "delete") => {
            db::pipeline::delete(&state.db, did, rkey).await?;
        }

        // ─── Dependency ─────────────────────────────────────────────
        ("dev.cospan.repo.dependency", "create" | "update") => {
            if let Some(rec) = record {
                let source_uri = rec.get("sourceRepo").and_then(|v| v.as_str()).unwrap_or("");
                let target_uri = rec.get("targetRepo").and_then(|v| v.as_str()).unwrap_or("");
                let (source_repo_did, source_repo_name) = parse_repo_at_uri(source_uri);
                let (target_repo_did, target_repo_name) = parse_repo_at_uri(target_uri);

                let row = db::dependency::DependencyRow {
                    did: did.to_string(),
                    rkey: rkey.to_string(),
                    source_repo_did,
                    source_repo_name,
                    target_repo_did,
                    target_repo_name,
                    morphism_id: rec
                        .get("morphismId")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    source_protocol: rec
                        .get("sourceProtocol")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    target_protocol: rec
                        .get("targetProtocol")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    description: rec
                        .get("description")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    created_at: parse_datetime(rec, "createdAt"),
                    indexed_at: Utc::now(),
                };
                db::dependency::upsert(&state.db, &row).await?;
            }
        }
        ("dev.cospan.repo.dependency", "delete") => {
            db::dependency::delete(&state.db, did, rkey).await?;
        }

        // ─── Tangled interop ────────────────────────────────────────
        // Translate sh.tangled.* records into dev.cospan.* equivalents and index
        // them with source="tangled" and source_uri set to the original AT-URI.
        //
        // Simple records (star, follow, reaction) have identical shapes — we do
        // direct field mapping. Complex records (issue, pull) extract what maps
        // directly and store unmapped fields as JSONB extra data.

        // ─── Tangled Star ──────────────────────────────────────────
        ("sh.tangled.feed.star", "create" | "update") => {
            if let Some(rec) = record {
                let subject = rec
                    .get("subject")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let source_uri = format!("at://{did}/sh.tangled.feed.star/{rkey}");

                let row = db::star::StarRow {
                    did: did.to_string(),
                    rkey: rkey.to_string(),
                    subject: subject.clone(),
                    created_at: parse_datetime(rec, "createdAt"),
                    indexed_at: Utc::now(),
                };

                let existing = db::star::get(&state.db, did, rkey).await?;
                db::star::upsert(&state.db, &row).await?;

                if existing.is_none() {
                    let (repo_did, repo_name) = parse_repo_at_uri(&subject);
                    db::star::increment_repo_star_count(&state.db, &repo_did, &repo_name).await?;
                }

                tracing::debug!(
                    did = did,
                    rkey = rkey,
                    source_uri = %source_uri,
                    "indexed tangled star as cospan star"
                );
            }
        }
        ("sh.tangled.feed.star", "delete") => {
            if let Some(star) = db::star::get(&state.db, did, rkey).await? {
                let (repo_did, repo_name) = parse_repo_at_uri(&star.subject);
                db::star::decrement_repo_star_count(&state.db, &repo_did, &repo_name).await?;
            }
            db::star::delete(&state.db, did, rkey).await?;
        }

        // ─── Tangled Follow ────────────────────────────────────────
        ("sh.tangled.graph.follow", "create" | "update") => {
            if let Some(rec) = record {
                let source_uri = format!("at://{did}/sh.tangled.graph.follow/{rkey}");

                let row = db::follow::FollowRow {
                    did: did.to_string(),
                    rkey: rkey.to_string(),
                    subject: rec
                        .get("subject")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    created_at: parse_datetime(rec, "createdAt"),
                    indexed_at: Utc::now(),
                };
                db::follow::upsert(&state.db, &row).await?;

                tracing::debug!(
                    did = did,
                    rkey = rkey,
                    source_uri = %source_uri,
                    "indexed tangled follow as cospan follow"
                );
            }
        }
        ("sh.tangled.graph.follow", "delete") => {
            db::follow::delete(&state.db, did, rkey).await?;
        }

        // ─── Tangled Issue ─────────────────────────────────────────
        // Moderate complexity: Cospan adds schemaRef (absent in Tangled).
        // We map the core fields directly and leave schemaRef as null.
        ("sh.tangled.repo.issue", "create" | "update") => {
            if let Some(rec) = record {
                let repo_uri = rec.get("repo").and_then(|v| v.as_str()).unwrap_or("");
                let (repo_did, repo_name) = parse_repo_at_uri(repo_uri);
                let source_uri = format!("at://{did}/sh.tangled.repo.issue/{rkey}");

                let row = db::issue::IssueRow {
                    did: did.to_string(),
                    rkey: rkey.to_string(),
                    repo_did,
                    repo_name,
                    title: rec
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    body: rec.get("body").and_then(|v| v.as_str()).map(String::from),
                    state: "open".to_string(),
                    comment_count: 0,
                    created_at: parse_datetime(rec, "createdAt"),
                    indexed_at: Utc::now(),
                };
                db::issue::upsert(&state.db, &row).await?;

                tracing::debug!(
                    did = did,
                    rkey = rkey,
                    source_uri = %source_uri,
                    "indexed tangled issue as cospan issue"
                );
            }
        }
        ("sh.tangled.repo.issue", "delete") => {
            if let Some(issue) = db::issue::get_by_pk(&state.db, did, rkey).await?
                && issue.state == "open"
            {
                decrement_repo_open_issue_count(&state.db, &issue.repo_did, &issue.repo_name)
                    .await?;
            }
            db::issue::delete(&state.db, did, rkey).await?;
        }

        // ─── Tangled Pull Request ──────────────────────────────────
        // Complex: Tangled uses patchBlob + source.sha; Cospan uses mergePreview.
        // We extract the fields that map directly (title, body, target branch,
        // source branch) and store VCS-specific fields as extra JSONB.
        ("sh.tangled.repo.pull", "create" | "update") => {
            if let Some(rec) = record {
                // Tangled PR structure:
                //   target: { repo (at-uri), branch }
                //   source: { branch, sha, repo (at-uri) }
                let target = rec.get("target");
                let source = rec.get("source");

                let repo_uri = target
                    .and_then(|t| t.get("repo"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let (repo_did, repo_name) = parse_repo_at_uri(repo_uri);

                let target_ref = target
                    .and_then(|t| t.get("branch"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let source_ref = source
                    .and_then(|s| s.get("branch"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let source_repo = source
                    .and_then(|s| s.get("repo"))
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let source_uri = format!("at://{did}/sh.tangled.repo.pull/{rkey}");

                let row = db::pull::PullRow {
                    did: did.to_string(),
                    rkey: rkey.to_string(),
                    repo_did,
                    repo_name,
                    title: rec
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    body: rec.get("body").and_then(|v| v.as_str()).map(String::from),
                    target_ref,
                    source_ref,
                    source_repo,
                    state: "open".to_string(),
                    comment_count: 0,
                    created_at: parse_datetime(rec, "createdAt"),
                    indexed_at: Utc::now(),
                };
                db::pull::upsert(&state.db, &row).await?;

                tracing::debug!(
                    did = did,
                    rkey = rkey,
                    source_uri = %source_uri,
                    "indexed tangled pull as cospan pull"
                );
            }
        }
        ("sh.tangled.repo.pull", "delete") => {
            if let Some(pull) = db::pull::get_by_pk(&state.db, did, rkey).await?
                && pull.state == "open"
            {
                decrement_repo_open_mr_count(&state.db, &pull.repo_did, &pull.repo_name).await?;
            }
            db::pull::delete(&state.db, did, rkey).await?;
        }

        // ─── Catch-all for other tangled collections ───────────────
        (c, _) if c.starts_with("sh.tangled.") => {
            tracing::debug!(
                collection = c,
                "tangled record received for unhandled collection (no cospan equivalent)"
            );
        }

        // ─── Catch-all for unhandled cospan collections ─────────────
        (c, _) if c.starts_with("dev.cospan.") => {
            tracing::debug!(collection = c, "unhandled dev.cospan collection");
        }

        _ => {}
    }

    Ok(())
}

// ─── Helper functions ───────────────────────────────────────────────────────

fn parse_datetime(record: &serde_json::Value, field: &str) -> chrono::DateTime<Utc> {
    record
        .get(field)
        .and_then(|v| v.as_str())
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(Utc::now)
}

fn extract_did_from_at_uri(uri: &str) -> String {
    // at://did:plc:abc123/collection/rkey -> did:plc:abc123
    uri.strip_prefix("at://")
        .and_then(|s| s.split('/').next())
        .unwrap_or("")
        .to_string()
}

fn parse_repo_at_uri(uri: &str) -> (String, String) {
    // at://did:plc:abc123/dev.cospan.repo/repo-name -> (did:plc:abc123, repo-name)
    let parts: Vec<&str> = uri
        .strip_prefix("at://")
        .unwrap_or("")
        .splitn(3, '/')
        .collect();
    let did = parts.first().unwrap_or(&"").to_string();
    let name = parts.get(2).unwrap_or(&"").to_string();
    (did, name)
}

/// Parse an AT-URI into (did, rkey) — used for issue/pull URIs.
/// at://did:plc:abc123/dev.cospan.repo.issue/tid123 -> (did:plc:abc123, tid123)
fn parse_at_uri_did_rkey(uri: &str) -> (String, String) {
    let parts: Vec<&str> = uri
        .strip_prefix("at://")
        .unwrap_or("")
        .splitn(3, '/')
        .collect();
    let did = parts.first().unwrap_or(&"").to_string();
    let rkey = parts.get(2).unwrap_or(&"").to_string();
    (did, rkey)
}

// ─── Aggregate counter helpers ──────────────────────────────────────────────

async fn increment_repo_open_issue_count(
    pool: &sqlx::PgPool,
    repo_did: &str,
    repo_name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE repos SET open_issue_count = open_issue_count + 1, indexed_at = NOW() \
         WHERE did = $1 AND name = $2",
    )
    .bind(repo_did)
    .bind(repo_name)
    .execute(pool)
    .await?;
    Ok(())
}

async fn decrement_repo_open_issue_count(
    pool: &sqlx::PgPool,
    repo_did: &str,
    repo_name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE repos SET open_issue_count = GREATEST(open_issue_count - 1, 0), indexed_at = NOW() \
         WHERE did = $1 AND name = $2",
    )
    .bind(repo_did)
    .bind(repo_name)
    .execute(pool)
    .await?;
    Ok(())
}

async fn increment_repo_open_mr_count(
    pool: &sqlx::PgPool,
    repo_did: &str,
    repo_name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE repos SET open_mr_count = open_mr_count + 1, indexed_at = NOW() \
         WHERE did = $1 AND name = $2",
    )
    .bind(repo_did)
    .bind(repo_name)
    .execute(pool)
    .await?;
    Ok(())
}

async fn decrement_repo_open_mr_count(
    pool: &sqlx::PgPool,
    repo_did: &str,
    repo_name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE repos SET open_mr_count = GREATEST(open_mr_count - 1, 0), indexed_at = NOW() \
         WHERE did = $1 AND name = $2",
    )
    .bind(repo_did)
    .bind(repo_name)
    .execute(pool)
    .await?;
    Ok(())
}
