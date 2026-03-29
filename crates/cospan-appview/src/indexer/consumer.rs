use std::sync::Arc;

use chrono::Utc;

use crate::db;
use crate::state::AppState;
use crate::xrpc::sse::IndexEvent;

/// Parse a record through panproto's schema-driven parser.
/// For Cospan records, validates and normalizes via the Lexicon schema.
/// For Tangled records, applies the compiled morphism first.
fn parse_record(
    state: &AppState,
    collection: &str,
    rec: &serde_json::Value,
) -> serde_json::Value {
    // For Tangled records, try morphism transform first
    if collection.starts_with("sh.tangled.") {
        if let Some(result) = state.tangled_interop.transform(collection, rec) {
            match result {
                Ok(cospan_rec) => return cospan_rec,
                Err(e) => {
                    tracing::warn!(collection, error = %e, "morphism transform failed");
                }
            }
        }
    }

    // Parse via Lexicon schema (Cospan records, or Tangled fallthrough)
    match state.schemas.parse_record(collection, rec) {
        Some(Ok(parsed)) => parsed,
        Some(Err(e)) => {
            tracing::debug!(collection, error = %e, "schema parse failed, using raw");
            rec.clone()
        }
        None => rec.clone(),
    }
}

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
                let row = db::node::NodeRow::from_json(did, rkey, &parse_record(state, collection, rec));
                db::node::upsert(&state.db, &row).await?;
            }
        }
        ("dev.cospan.node", "delete") => {
            db::node::delete(&state.db, did).await?;
        }

        // ─── Actor Profile ──────────────────────────────────────────
        ("dev.cospan.actor.profile", "create" | "update") => {
            if let Some(rec) = record {
                let row = db::actor_profile::ActorProfileRow::from_json(did, rkey, &parse_record(state, collection, rec));
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
                    let nodes = db::node::list(&state.db, 1000, None).await?;
                    nodes
                        .iter()
                        .find(|n| n.did == node_did)
                        .and_then(|n| n.public_endpoint.clone())
                        .unwrap_or_default()
                };

                let mut row = db::repo::RepoRow::from_json(did, rkey, &parse_record(state, collection, rec));
                row.node_did = node_did;
                row.node_url = node_url;
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
                let breaking_changes = rec
                    .get("breakingChanges")
                    .and_then(|v| v.as_array())
                    .map(|a| a.len() as i32)
                    .unwrap_or(0);

                let mut row = db::ref_update::RefUpdateRow::from_json(did, rkey, &parse_record(state, collection, rec));
                row.breaking_change_count = breaking_changes;
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
                let row = db::issue::IssueRow::from_json(did, rkey, &parse_record(state, collection, rec));
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

                let row = db::issue_comment::IssueCommentRow::from_json(did, rkey, &parse_record(state, collection, rec));

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

                let row = db::issue_state::IssueStateRow::from_json(did, rkey, &parse_record(state, collection, rec));
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
                let row = db::pull::PullRow::from_json(did, rkey, &parse_record(state, collection, rec));
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

                let row = db::pull_comment::PullCommentRow::from_json(did, rkey, &parse_record(state, collection, rec));

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

                let row = db::pull_state::PullStateRow::from_json(did, rkey, &parse_record(state, collection, rec));
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

                let row = db::star::StarRow::from_json(did, rkey, &parse_record(state, collection, rec));

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
                let row = db::follow::FollowRow::from_json(did, rkey, &parse_record(state, collection, rec));
                db::follow::upsert(&state.db, &row).await?;
            }
        }
        ("dev.cospan.graph.follow", "delete") => {
            db::follow::delete(&state.db, did, rkey).await?;
        }

        // ─── Reaction ───────────────────────────────────────────────
        ("dev.cospan.feed.reaction", "create" | "update") => {
            if let Some(rec) = record {
                let row = db::reaction::ReactionRow::from_json(did, rkey, &parse_record(state, collection, rec));
                db::reaction::upsert(&state.db, &row).await?;
            }
        }
        ("dev.cospan.feed.reaction", "delete") => {
            db::reaction::delete(&state.db, did, rkey).await?;
        }

        // ─── Label Definition ───────────────────────────────────────
        ("dev.cospan.label.definition", "create" | "update") => {
            if let Some(rec) = record {
                let row = db::label::LabelRow::from_json(did, rkey, &parse_record(state, collection, rec));
                db::label::upsert(&state.db, &row).await?;
            }
        }
        ("dev.cospan.label.definition", "delete") => {
            db::label::delete(&state.db, did, rkey).await?;
        }

        // ─── Org ────────────────────────────────────────────────────
        ("dev.cospan.org", "create" | "update") => {
            if let Some(rec) = record {
                let row = db::org::OrgRow::from_json(did, rkey, &parse_record(state, collection, rec));
                db::org::upsert(&state.db, &row).await?;
            }
        }
        ("dev.cospan.org", "delete") => {
            db::org::delete(&state.db, did, rkey).await?;
        }

        // ─── Org Member ─────────────────────────────────────────────
        ("dev.cospan.org.member", "create" | "update") => {
            if let Some(rec) = record {
                let row = db::org_member::OrgMemberRow::from_json(did, rkey, &parse_record(state, collection, rec));
                db::org_member::upsert(&state.db, &row).await?;
            }
        }
        ("dev.cospan.org.member", "delete") => {
            db::org_member::delete(&state.db, did, rkey).await?;
        }

        // ─── Collaborator ───────────────────────────────────────────
        ("dev.cospan.repo.collaborator", "create" | "update") => {
            if let Some(rec) = record {
                let row = db::collaborator::CollaboratorRow::from_json(did, rkey, &parse_record(state, collection, rec));
                db::collaborator::upsert(&state.db, &row).await?;
            }
        }
        ("dev.cospan.repo.collaborator", "delete") => {
            db::collaborator::delete(&state.db, did, rkey).await?;
        }

        // ─── Pipeline ───────────────────────────────────────────────
        ("dev.cospan.pipeline", "create" | "update") => {
            if let Some(rec) = record {
                let checks = rec.get("algebraicChecks");

                let mut row = db::pipeline::PipelineRow::from_json(did, rkey, &parse_record(state, collection, rec));
                row.gat_type_check = checks
                    .and_then(|c| c.get("gatTypeCheck"))
                    .and_then(|v| v.as_str())
                    .map(String::from);
                row.equation_verification = checks
                    .and_then(|c| c.get("equationVerification"))
                    .and_then(|v| v.as_str())
                    .map(String::from);
                row.lens_law_check = checks
                    .and_then(|c| c.get("lensLawCheck"))
                    .and_then(|v| v.as_str())
                    .map(String::from);
                row.breaking_change_check = checks
                    .and_then(|c| c.get("breakingChangeCheck"))
                    .and_then(|v| v.as_str())
                    .map(String::from);
                db::pipeline::upsert(&state.db, &row).await?;
            }
        }
        ("dev.cospan.pipeline", "delete") => {
            db::pipeline::delete(&state.db, did, rkey).await?;
        }

        // ─── Dependency ─────────────────────────────────────────────
        ("dev.cospan.repo.dependency", "create" | "update") => {
            if let Some(rec) = record {
                let row = db::dependency::DependencyRow::from_json(did, rkey, &parse_record(state, collection, rec));
                db::dependency::upsert(&state.db, &row).await?;
            }
        }
        ("dev.cospan.repo.dependency", "delete") => {
            db::dependency::delete(&state.db, did, rkey).await?;
        }

        // ─── Tangled interop ────────────────────────────────────────
        // Translate sh.tangled.* records into dev.cospan.* equivalents and index
        // them with source="tangled" and source_uri set to the original AT-URI.

        // ─── Tangled Star ──────────────────────────────────────────
        ("sh.tangled.feed.star", "create" | "update") => {
            if let Some(rec) = record {
                let row = db::star::StarRow::from_json(did, rkey, &parse_record(state, collection, rec));
                let existing = db::star::get(&state.db, did, rkey).await?;
                db::star::upsert(&state.db, &row).await?;

                if existing.is_none() {
                    let (repo_did, repo_name) = parse_repo_at_uri(&row.subject);
                    db::star::increment_repo_star_count(&state.db, &repo_did, &repo_name).await?;
                }

                tracing::debug!(did, rkey, "indexed tangled star");
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
                let row = db::follow::FollowRow::from_json(did, rkey, &parse_record(state, collection, rec));
                db::follow::upsert(&state.db, &row).await?;
                tracing::debug!(did, rkey, "indexed tangled follow");
            }
        }
        ("sh.tangled.graph.follow", "delete") => {
            db::follow::delete(&state.db, did, rkey).await?;
        }

        // ─── Tangled Reaction ──────────────────────────────────────
        ("sh.tangled.feed.reaction", "create" | "update") => {
            if let Some(rec) = record {
                let row = db::reaction::ReactionRow::from_json(did, rkey, &parse_record(state, collection, rec));
                db::reaction::upsert(&state.db, &row).await?;
                tracing::debug!(did, rkey, "indexed tangled reaction");
            }
        }
        ("sh.tangled.feed.reaction", "delete") => {
            db::reaction::delete(&state.db, did, rkey).await?;
        }

        // ─── Tangled Issue ─────────────────────────────────────────
        ("sh.tangled.repo.issue", "create" | "update") => {
            if let Some(rec) = record {
                let row = db::issue::IssueRow::from_json(did, rkey, &parse_record(state, collection, rec));
                db::issue::upsert(&state.db, &row).await?;
                tracing::debug!(did, rkey, "indexed tangled issue");
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

        // ─── Tangled Issue State ───────────────────────────────────
        ("sh.tangled.repo.issue.state", "create" | "update") => {
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

                let row = db::issue_state::IssueStateRow::from_json(did, rkey, &parse_record(state, collection, rec));
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
                    }
                }
                tracing::debug!(did, rkey, "indexed tangled issue state");
            }
        }
        ("sh.tangled.repo.issue.state", "delete") => {
            db::issue_state::delete(&state.db, did, rkey).await?;
        }

        // ─── Tangled Issue Comment ─────────────────────────────────
        ("sh.tangled.repo.issue.comment", "create" | "update") => {
            if let Some(rec) = record {
                let issue_uri = rec
                    .get("issue")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let row = db::issue_comment::IssueCommentRow::from_json(did, rkey, &parse_record(state, collection, rec));

                let existing = db::issue_comment::get(&state.db, did, rkey).await?;
                db::issue_comment::upsert(&state.db, &row).await?;

                if existing.is_none() {
                    let (issue_did, issue_rkey) = parse_at_uri_did_rkey(&issue_uri);
                    db::issue::increment_comment_count(&state.db, &issue_did, &issue_rkey).await?;
                }
                tracing::debug!(did, rkey, "indexed tangled issue comment");
            }
        }
        ("sh.tangled.repo.issue.comment", "delete") => {
            if let Some(comment) = db::issue_comment::get(&state.db, did, rkey).await? {
                let (issue_did, issue_rkey) = parse_at_uri_did_rkey(&comment.issue_uri);
                db::issue::decrement_comment_count(&state.db, &issue_did, &issue_rkey).await?;
            }
            db::issue_comment::delete(&state.db, did, rkey).await?;
        }

        // ─── Tangled Pull Request ──────────────────────────────────
        ("sh.tangled.repo.pull", "create" | "update") => {
            if let Some(rec) = record {
                let row = db::pull::PullRow::from_json(did, rkey, &parse_record(state, collection, rec));
                db::pull::upsert(&state.db, &row).await?;
                tracing::debug!(did, rkey, "indexed tangled pull");
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

        // ─── Tangled Pull Status → Pull State ─────────────────────
        ("sh.tangled.repo.pull.status", "create" | "update") => {
            if let Some(rec) = record {
                let pull_uri = rec
                    .get("pull")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let row = db::pull_state::PullStateRow::from_json(did, rkey, &parse_record(state, collection, rec));
                let new_state = row.state.clone();
                db::pull_state::upsert(&state.db, &row).await?;

                // Update the pull's state and repo counters
                let (pull_did, pull_rkey) = parse_at_uri_did_rkey(&pull_uri);
                if let Some(pull) = db::pull::get_by_pk(&state.db, &pull_did, &pull_rkey).await? {
                    let old_state = &pull.state;
                    if old_state != &new_state {
                        db::pull::update_state(&state.db, &pull_did, &pull_rkey, &new_state)
                            .await?;

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
                    }
                }
                tracing::debug!(did, rkey, "indexed tangled pull status as pull state");
            }
        }
        ("sh.tangled.repo.pull.status", "delete") => {
            db::pull_state::delete(&state.db, did, rkey).await?;
        }

        // ─── Tangled Pull Comment ──────────────────────────────────
        ("sh.tangled.repo.pull.comment", "create" | "update") => {
            if let Some(rec) = record {
                let pull_uri = rec
                    .get("pull")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let row = db::pull_comment::PullCommentRow::from_json(did, rkey, &parse_record(state, collection, rec));

                let existing = db::pull_comment::get(&state.db, did, rkey).await?;
                db::pull_comment::upsert(&state.db, &row).await?;

                if existing.is_none() {
                    let (pull_did, pull_rkey) = parse_at_uri_did_rkey(&pull_uri);
                    db::pull::increment_comment_count(&state.db, &pull_did, &pull_rkey).await?;
                }
                tracing::debug!(did, rkey, "indexed tangled pull comment");
            }
        }
        ("sh.tangled.repo.pull.comment", "delete") => {
            if let Some(comment) = db::pull_comment::get(&state.db, did, rkey).await? {
                let (pull_did, pull_rkey) = parse_at_uri_did_rkey(&comment.pull_uri);
                db::pull::decrement_comment_count(&state.db, &pull_did, &pull_rkey).await?;
            }
            db::pull_comment::delete(&state.db, did, rkey).await?;
        }

        // ─── Tangled Collaborator ──────────────────────────────────
        ("sh.tangled.repo.collaborator", "create" | "update") => {
            if let Some(rec) = record {
                let row = db::collaborator::CollaboratorRow::from_json(did, rkey, &parse_record(state, collection, rec));
                db::collaborator::upsert(&state.db, &row).await?;
                tracing::debug!(did, rkey, "indexed tangled collaborator");
            }
        }
        ("sh.tangled.repo.collaborator", "delete") => {
            db::collaborator::delete(&state.db, did, rkey).await?;
        }

        // ─── Tangled Knot → Node ──────────────────────────────────
        ("sh.tangled.knot", "create" | "update") => {
            if let Some(rec) = record {
                let row = db::node::NodeRow::from_json(did, rkey, &parse_record(state, collection, rec));
                db::node::upsert(&state.db, &row).await?;
                tracing::debug!(did, rkey, "indexed tangled knot as node");
            }
        }
        ("sh.tangled.knot", "delete") => {
            db::node::delete(&state.db, did).await?;
        }

        // ─── Tangled Spindle → Org ────────────────────────────────
        ("sh.tangled.spindle", "create" | "update") => {
            if let Some(rec) = record {
                // Use the DID as a fallback name; Tangled spindles don't carry a name field
                let name = rec
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(did)
                    .to_string();

                let row = db::org::OrgRow {
                    did: did.to_string(),
                    rkey: rkey.to_string(),
                    name,
                    description: rec
                        .get("description")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    avatar_cid: None,
                    created_at: parse_datetime(rec, "createdAt"),
                    indexed_at: Utc::now(),
                };
                db::org::upsert(&state.db, &row).await?;
                tracing::debug!(did, rkey, "indexed tangled spindle as org");
            }
        }
        ("sh.tangled.spindle", "delete") => {
            db::org::delete(&state.db, did, rkey).await?;
        }

        // ─── Tangled Actor Profile ─────────────────────────────────
        ("sh.tangled.actor.profile", "create" | "update") => {
            if let Some(rec) = record {
                let row = db::actor_profile::ActorProfileRow::from_json(did, rkey, &parse_record(state, collection, rec));
                db::actor_profile::upsert(&state.db, &row).await?;
                tracing::debug!(did, rkey, "indexed tangled actor profile");
            }
        }
        ("sh.tangled.actor.profile", "delete") => {
            db::actor_profile::delete(&state.db, did).await?;
        }

        // ─── Tangled Repo ──────────────────────────────────────────
        ("sh.tangled.repo", "create" | "update") => {
            if let Some(rec) = record {
                let row = db::repo::RepoRow::from_json(did, rkey, &parse_record(state, collection, rec));
                db::repo::upsert(&state.db, &row).await?;
                tracing::debug!(did, rkey, "indexed tangled repo");
            }
        }
        ("sh.tangled.repo", "delete") => {
            tracing::warn!(did, rkey, "tangled repo delete (need rkey->name lookup)");
        }

        // ─── Tangled Knot Member → Org Member ─────────────────────
        ("sh.tangled.knot.member", "create" | "update") => {
            if let Some(rec) = record {
                let row = db::org_member::OrgMemberRow::from_json(did, rkey, &parse_record(state, collection, rec));
                db::org_member::upsert(&state.db, &row).await?;
                tracing::debug!(did, rkey, "indexed tangled knot member as org member");
            }
        }
        ("sh.tangled.knot.member", "delete") => {
            db::org_member::delete(&state.db, did, rkey).await?;
        }

        // ─── Tangled Spindle Member → Org Member ──────────────────
        ("sh.tangled.spindle.member", "create" | "update") => {
            if let Some(rec) = record {
                let org_uri = format!("at://{did}/sh.tangled.spindle/self");

                let row = db::org_member::OrgMemberRow {
                    did: did.to_string(),
                    rkey: rkey.to_string(),
                    org_uri,
                    member_did: rec
                        .get("subject")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    role: "member".to_string(),
                    created_at: parse_datetime(rec, "createdAt"),
                    indexed_at: Utc::now(),
                };
                db::org_member::upsert(&state.db, &row).await?;
                tracing::debug!(did, rkey, "indexed tangled spindle member as org member");
            }
        }
        ("sh.tangled.spindle.member", "delete") => {
            db::org_member::delete(&state.db, did, rkey).await?;
        }

        // ─── Tangled Label Definition ──────────────────────────────
        ("sh.tangled.label.definition", "create" | "update") => {
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
                    // Tangled may store description in valueType
                    description: rec
                        .get("description")
                        .or_else(|| rec.get("valueType"))
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    created_at: parse_datetime(rec, "createdAt"),
                    indexed_at: Utc::now(),
                };
                db::label::upsert(&state.db, &row).await?;
                tracing::debug!(did, rkey, "indexed tangled label definition");
            }
        }
        ("sh.tangled.label.definition", "delete") => {
            db::label::delete(&state.db, did, rkey).await?;
        }

        // ─── Tangled Pipeline ──────────────────────────────────────
        ("sh.tangled.pipeline", "create" | "update") => {
            if let Some(rec) = record {
                let row = db::pipeline::PipelineRow::from_json(did, rkey, &parse_record(state, collection, rec));
                db::pipeline::upsert(&state.db, &row).await?;
                tracing::debug!(did, rkey, "indexed tangled pipeline");
            }
        }
        ("sh.tangled.pipeline", "delete") => {
            db::pipeline::delete(&state.db, did, rkey).await?;
        }

        // ─── Tangled Pipeline Status → Update pipeline ─────────────
        ("sh.tangled.pipeline.status", "create" | "update") => {
            if let Some(rec) = record {
                // The pipeline AT-URI to look up
                let pipeline_uri = rec.get("pipeline").and_then(|v| v.as_str()).unwrap_or("");
                let (pipeline_did, pipeline_rkey) = parse_at_uri_did_rkey(pipeline_uri);

                // Map Tangled status values to Cospan equivalents
                let raw_status = rec
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("pending");
                let mapped_status = match raw_status {
                    "success" => "passed",
                    "failed" => "failed",
                    "cancelled" | "canceled" => "cancelled",
                    "running" | "in_progress" => "running",
                    other => other,
                };

                if !pipeline_did.is_empty() && !pipeline_rkey.is_empty() {
                    // Update the pipeline status in-place
                    sqlx::query(
                        "UPDATE pipelines SET status = $1, indexed_at = NOW() \
                         WHERE did = $2 AND rkey = $3",
                    )
                    .bind(mapped_status)
                    .bind(&pipeline_did)
                    .bind(&pipeline_rkey)
                    .execute(&state.db)
                    .await?;
                }
                tracing::debug!(
                    did,
                    rkey,
                    status = mapped_status,
                    "indexed tangled pipeline status"
                );
            }
        }
        ("sh.tangled.pipeline.status", "delete") => {
            // Pipeline status deletes are no-ops (status is stored on the pipeline row)
            tracing::debug!(did, rkey, "tangled pipeline status delete (no-op)");
        }

        // ─── Tangled Git RefUpdate ─────────────────────────────────
        ("sh.tangled.git.refUpdate", "create" | "update") => {
            if let Some(rec) = record {
                let row = db::ref_update::RefUpdateRow::from_json(did, rkey, &parse_record(state, collection, rec));
                db::ref_update::upsert(&state.db, &row).await?;
                tracing::debug!(did, rkey, "indexed tangled git refUpdate");
            }
        }
        ("sh.tangled.git.refUpdate", "delete") => {
            db::ref_update::delete(&state.db, did, rkey).await?;
        }

        // ─── Tangled-only records (no Cospan equivalent) ──────────
        // These are display-only features in Tangled with no Cospan schema.
        // Log and skip.
        (
            "sh.tangled.publicKey"
            | "sh.tangled.string"
            | "sh.tangled.repo.artifact"
            | "sh.tangled.label.op",
            _,
        ) => {
            tracing::debug!(
                collection,
                did,
                rkey,
                "tangled-only record skipped (no cospan equivalent)"
            );
        }

        // ─── Catch-all for other tangled collections ───────────────
        (c, _) if c.starts_with("sh.tangled.") => {
            tracing::debug!(
                collection = c,
                "tangled record received for unhandled collection"
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
