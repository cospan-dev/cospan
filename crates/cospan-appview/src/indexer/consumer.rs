use std::sync::Arc;

use chrono::Utc;

use crate::at_uri;
use crate::db;
use crate::state::AppState;
use crate::xrpc::sse::IndexEvent;

use super::dispatch;

/// Transform a record through the pre-compiled panproto morphism.
/// Handles both Cospan (DB projection) and Tangled (interop + DB projection).
pub(super) fn transform_record(
    state: &AppState,
    collection: &str,
    rec: &serde_json::Value,
) -> serde_json::Value {
    match state.transformer.transform(collection, rec) {
        Some(Ok(transformed)) => transformed,
        Some(Err(e)) => {
            tracing::warn!(collection, error = %e, "panproto transform failed, using raw");
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

    match operation {
        "create" | "update" => {
            if let Some(rec) = record {
                // Try generic dispatch first for simple records
                if dispatch::dispatch_simple_upsert(state, collection, did, rkey, rec).await? {
                    return Ok(());
                }
                // Fall through to special-case handling
                dispatch_special_upsert(state, collection, did, rkey, rec).await?;
            }
        }
        "delete" => {
            // Try generic dispatch first for simple deletes
            if dispatch::dispatch_simple_delete(state, collection, did, rkey).await? {
                return Ok(());
            }
            // Fall through to special-case handling
            dispatch_special_delete(state, collection, did, rkey).await?;
        }
        _ => {}
    }

    Ok(())
}

// ─── Special-case upserts (records with side effects) ─────────────────────

async fn dispatch_special_upsert(
    state: &Arc<AppState>,
    collection: &str,
    did: &str,
    rkey: &str,
    rec: &serde_json::Value,
) -> anyhow::Result<()> {
    match collection {
        // ─── Repo (node URL lookup) ─────────────────────────────────
        "dev.cospan.repo" | "sh.tangled.repo" => {
            // DB projection extracts nodeDid from the AT-URI via panproto expression
            let mut row: db::repo::RepoRow =
                serde_json::from_value(transform_record(state, collection, rec))?;
            row.did = did.to_string();
            row.rkey = rkey.to_string();
            row.indexed_at = Utc::now();

            // Set source based on collection origin
            if collection.starts_with("sh.tangled.") {
                row.source = "tangled".to_string();
                row.source_uri = Some(format!("at://{did}/{collection}/{rkey}"));
            }

            // Look up node URL from nodes table (business logic, not schema-derivable)
            if !row.node_did.is_empty() {
                let nodes = db::node::list(&state.db, 1000, None).await?;
                if let Some(url) = nodes
                    .iter()
                    .find(|n| n.did == row.node_did)
                    .and_then(|n| n.public_endpoint.clone())
                {
                    row.node_url = url;
                }
            }
            db::repo::upsert(&state.db, &row).await?;
        }

        // ─── Ref Update (breaking change count computed by DB projection + SSE) ──
        "dev.cospan.vcs.refUpdate" | "sh.tangled.git.refUpdate" => {
            // DB projection compute_array_len handles breakingChanges → breakingChangeCount
            let mut row: db::ref_update::RefUpdateRow =
                serde_json::from_value(transform_record(state, collection, rec))?;
            row.rkey = rkey.to_string();
            row.indexed_at = Utc::now();
            db::ref_update::upsert(&state.db, &row).await?;

            let _ = state.event_tx.send(IndexEvent::RefUpdate {
                repo_did: row.repo_did.clone(),
                repo_name: row.repo_name.clone(),
                ref_name: row.ref_name.clone(),
                new_target: row.new_target.clone(),
                committer_did: row.committer_did.clone(),
                breaking_change_count: row.breaking_change_count,
            });
        }

        // ─── Issue (SSE event on create) ────────────────────────────
        "dev.cospan.repo.issue" | "sh.tangled.repo.issue" => {
            let mut row: db::issue::IssueRow =
                serde_json::from_value(transform_record(state, collection, rec))?;
            row.did = did.to_string();
            row.rkey = rkey.to_string();
            row.indexed_at = Utc::now();
            db::issue::upsert(&state.db, &row).await?;

            let _ = state.event_tx.send(IndexEvent::IssueCreated {
                repo_did: row.repo_did.clone(),
                repo_name: row.repo_name.clone(),
                issue_rkey: row.rkey.clone(),
                title: row.title.clone(),
                author_did: row.did.clone(),
            });
        }

        // ─── Issue Comment (comment count increment) ────────────────
        "dev.cospan.repo.issue.comment" | "sh.tangled.repo.issue.comment" => {
            let issue_uri = rec
                .get("issue")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let mut row: db::issue_comment::IssueCommentRow =
                serde_json::from_value(transform_record(state, collection, rec))?;
            row.did = did.to_string();
            row.rkey = rkey.to_string();
            row.indexed_at = Utc::now();

            let existing = db::issue_comment::get(&state.db, did, rkey).await?;
            db::issue_comment::upsert(&state.db, &row).await?;

            if existing.is_none() {
                let (issue_did, issue_rkey) = at_uri::parse_did_rkey(&issue_uri);
                db::issue::increment_comment_count(&state.db, &issue_did, &issue_rkey).await?;
            }
        }

        // ─── Issue State (state transitions + counter adjustments + SSE) ─
        "dev.cospan.repo.issue.state" | "sh.tangled.repo.issue.state" => {
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

            let mut row: db::issue_state::IssueStateRow =
                serde_json::from_value(transform_record(state, collection, rec))?;
            row.did = did.to_string();
            row.rkey = rkey.to_string();
            row.indexed_at = Utc::now();
            db::issue_state::upsert(&state.db, &row).await?;

            let (issue_did, issue_rkey) = at_uri::parse_did_rkey(&issue_uri);
            if let Some(issue) = db::issue::get_by_pk(&state.db, &issue_did, &issue_rkey).await? {
                let old_state = &issue.state;
                if old_state != &new_state {
                    db::issue::update_state(&state.db, &issue_did, &issue_rkey, &new_state).await?;

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

                    // SSE only for cospan-native events
                    if collection.starts_with("dev.cospan.") {
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

        // ─── Pull Request (SSE event on create) ─────────────────────
        "dev.cospan.repo.pull" | "sh.tangled.repo.pull" => {
            let mut row: db::pull::PullRow =
                serde_json::from_value(transform_record(state, collection, rec))?;
            row.did = did.to_string();
            row.rkey = rkey.to_string();
            row.indexed_at = Utc::now();
            db::pull::upsert(&state.db, &row).await?;

            let _ = state.event_tx.send(IndexEvent::PullCreated {
                repo_did: row.repo_did.clone(),
                repo_name: row.repo_name.clone(),
                pull_rkey: row.rkey.clone(),
                title: row.title.clone(),
                author_did: row.did.clone(),
            });
        }

        // ─── Pull Comment (comment count increment) ─────────────────
        "dev.cospan.repo.pull.comment" | "sh.tangled.repo.pull.comment" => {
            let pull_uri = rec
                .get("pull")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let mut row: db::pull_comment::PullCommentRow =
                serde_json::from_value(transform_record(state, collection, rec))?;
            row.did = did.to_string();
            row.rkey = rkey.to_string();
            row.indexed_at = Utc::now();

            let existing = db::pull_comment::get(&state.db, did, rkey).await?;
            db::pull_comment::upsert(&state.db, &row).await?;

            if existing.is_none() {
                let (pull_did, pull_rkey) = at_uri::parse_did_rkey(&pull_uri);
                db::pull::increment_comment_count(&state.db, &pull_did, &pull_rkey).await?;
            }
        }

        // ─── Pull State (state transitions + counter adjustments + SSE) ─
        "dev.cospan.repo.pull.state" | "sh.tangled.repo.pull.status" => {
            let pull_uri = rec
                .get("pull")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let mut row: db::pull_state::PullStateRow =
                serde_json::from_value(transform_record(state, collection, rec))?;
            row.did = did.to_string();
            row.rkey = rkey.to_string();
            row.indexed_at = Utc::now();
            let new_state = row.state.clone();
            db::pull_state::upsert(&state.db, &row).await?;

            let (pull_did, pull_rkey) = at_uri::parse_did_rkey(&pull_uri);
            if let Some(pull) = db::pull::get_by_pk(&state.db, &pull_did, &pull_rkey).await? {
                let old_state = &pull.state;
                if old_state != &new_state {
                    db::pull::update_state(&state.db, &pull_did, &pull_rkey, &new_state).await?;

                    if old_state == "open" && new_state != "open" {
                        decrement_repo_open_mr_count(&state.db, &pull.repo_did, &pull.repo_name)
                            .await?;
                    } else if old_state != "open" && new_state == "open" {
                        increment_repo_open_mr_count(&state.db, &pull.repo_did, &pull.repo_name)
                            .await?;
                    }

                    // SSE only for cospan-native events
                    if collection.starts_with("dev.cospan.") {
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

        // ─── Star (counter increment + SSE) ─────────────────────────
        "dev.cospan.feed.star" | "sh.tangled.feed.star" => {
            let mut row: db::star::StarRow =
                serde_json::from_value(transform_record(state, collection, rec))?;
            row.did = did.to_string();
            row.rkey = rkey.to_string();
            row.indexed_at = Utc::now();

            let existing = db::star::get(&state.db, did, rkey).await?;
            db::star::upsert(&state.db, &row).await?;

            if existing.is_none() {
                let (repo_did, repo_name) = at_uri::parse_did_rkey(&row.subject);
                db::star::increment_repo_star_count(&state.db, &repo_did, &repo_name).await?;

                // SSE only for cospan-native events
                if collection.starts_with("dev.cospan.") {
                    let _ = state.event_tx.send(IndexEvent::StarCreated {
                        did: did.to_string(),
                        subject: row.subject.clone(),
                    });
                }
            }
        }

        // ─── Pipeline (algebraicChecks extraction) ──────────────────
        "dev.cospan.pipeline" | "sh.tangled.pipeline" => {
            // DB projection path_extract transforms handle algebraicChecks flattening
            let mut row: db::pipeline::PipelineRow =
                serde_json::from_value(transform_record(state, collection, rec))?;
            row.did = did.to_string();
            row.rkey = rkey.to_string();
            row.indexed_at = Utc::now();
            db::pipeline::upsert(&state.db, &row).await?;
        }

        // ─── Tangled Spindle → Org (via panproto morphism) ───────────
        "sh.tangled.spindle" => {
            let mut row: db::org::OrgRow =
                serde_json::from_value(transform_record(state, collection, rec))?;
            row.did = did.to_string();
            row.rkey = rkey.to_string();
            // Tangled spindles don't carry a name field; use DID as fallback
            if row.name.is_empty() {
                row.name = did.to_string();
            }
            row.indexed_at = Utc::now();
            db::org::upsert(&state.db, &row).await?;
        }

        // ─── Tangled Spindle Member → Org Member (via panproto morphism)
        "sh.tangled.spindle.member" => {
            let mut row: db::org_member::OrgMemberRow =
                serde_json::from_value(transform_record(state, collection, rec))?;
            row.did = did.to_string();
            row.rkey = rkey.to_string();
            // Synthesize the org AT-URI from the DID
            if row.org_uri.is_empty() {
                row.org_uri = format!("at://{did}/sh.tangled.spindle/self");
            }
            // Tangled has no role field; default to "member"
            if row.role.is_empty() {
                row.role = "member".to_string();
            }
            row.indexed_at = Utc::now();
            db::org_member::upsert(&state.db, &row).await?;
        }

        // ─── Tangled Label Definition (via panproto morphism) ───────
        "sh.tangled.label.definition" => {
            let mut row: db::label::LabelRow =
                serde_json::from_value(transform_record(state, collection, rec))?;
            row.did = did.to_string();
            row.rkey = rkey.to_string();
            row.indexed_at = Utc::now();
            db::label::upsert(&state.db, &row).await?;
        }

        // ─── Tangled Pipeline Status (SQL update) ───────────────────
        "sh.tangled.pipeline.status" => {
            let pipeline_uri = rec.get("pipeline").and_then(|v| v.as_str()).unwrap_or("");
            let (pipeline_did, pipeline_rkey) = at_uri::parse_did_rkey(pipeline_uri);

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
        }

        // ─── Tangled-only records (no Cospan equivalent) ────────────
        "sh.tangled.publicKey"
        | "sh.tangled.string"
        | "sh.tangled.repo.artifact"
        | "sh.tangled.label.op" => {
            tracing::debug!(
                collection,
                did,
                rkey,
                "tangled-only record skipped (no cospan equivalent)"
            );
        }

        // ─── Catch-alls ─────────────────────────────────────────────
        c if c.starts_with("sh.tangled.") => {
            tracing::debug!(
                collection = c,
                "tangled record received for unhandled collection"
            );
        }
        c if c.starts_with("dev.cospan.") => {
            tracing::debug!(collection = c, "unhandled dev.cospan collection");
        }

        _ => {}
    }

    Ok(())
}

// ─── Special-case deletes (records with side effects) ──────────────────────

async fn dispatch_special_delete(
    state: &Arc<AppState>,
    collection: &str,
    did: &str,
    rkey: &str,
) -> anyhow::Result<()> {
    match collection {
        // ─── Repo (incomplete implementation) ───────────────────────
        "dev.cospan.repo" => {
            tracing::warn!(
                did,
                rkey,
                "repo delete not fully implemented (need rkey->name lookup)"
            );
        }
        "sh.tangled.repo" => {
            tracing::warn!(did, rkey, "tangled repo delete (need rkey->name lookup)");
        }

        // ─── Issue (counter decrement on delete) ────────────────────
        "dev.cospan.repo.issue" | "sh.tangled.repo.issue" => {
            if let Some(issue) = db::issue::get_by_pk(&state.db, did, rkey).await?
                && issue.state == "open"
            {
                decrement_repo_open_issue_count(&state.db, &issue.repo_did, &issue.repo_name)
                    .await?;
            }
            db::issue::delete(&state.db, did, rkey).await?;
        }

        // ─── Issue Comment (counter decrement) ──────────────────────
        "dev.cospan.repo.issue.comment" | "sh.tangled.repo.issue.comment" => {
            if let Some(comment) = db::issue_comment::get(&state.db, did, rkey).await? {
                let (issue_did, issue_rkey) = at_uri::parse_did_rkey(&comment.issue_uri);
                db::issue::decrement_comment_count(&state.db, &issue_did, &issue_rkey).await?;
            }
            db::issue_comment::delete(&state.db, did, rkey).await?;
        }

        // ─── Pull (counter decrement on delete) ─────────────────────
        "dev.cospan.repo.pull" | "sh.tangled.repo.pull" => {
            if let Some(pull) = db::pull::get_by_pk(&state.db, did, rkey).await?
                && pull.state == "open"
            {
                decrement_repo_open_mr_count(&state.db, &pull.repo_did, &pull.repo_name).await?;
            }
            db::pull::delete(&state.db, did, rkey).await?;
        }

        // ─── Pull Comment (counter decrement) ───────────────────────
        "dev.cospan.repo.pull.comment" | "sh.tangled.repo.pull.comment" => {
            if let Some(comment) = db::pull_comment::get(&state.db, did, rkey).await? {
                let (pull_did, pull_rkey) = at_uri::parse_did_rkey(&comment.pull_uri);
                db::pull::decrement_comment_count(&state.db, &pull_did, &pull_rkey).await?;
            }
            db::pull_comment::delete(&state.db, did, rkey).await?;
        }

        // ─── Star (counter decrement + SSE) ─────────────────────────
        "dev.cospan.feed.star" | "sh.tangled.feed.star" => {
            if let Some(star) = db::star::get(&state.db, did, rkey).await? {
                let (repo_did, repo_name) = at_uri::parse_did_rkey(&star.subject);
                db::star::decrement_repo_star_count(&state.db, &repo_did, &repo_name).await?;

                if collection.starts_with("dev.cospan.") {
                    let _ = state.event_tx.send(IndexEvent::StarDeleted {
                        did: did.to_string(),
                        subject: star.subject.clone(),
                    });
                }
            }
            db::star::delete(&state.db, did, rkey).await?;
        }

        // ─── Pipeline Status (no-op) ────────────────────────────────
        "sh.tangled.pipeline.status" => {
            tracing::debug!(did, rkey, "tangled pipeline status delete (no-op)");
        }

        // ─── Tangled-only records ───────────────────────────────────
        "sh.tangled.publicKey"
        | "sh.tangled.string"
        | "sh.tangled.repo.artifact"
        | "sh.tangled.label.op" => {
            tracing::debug!(
                collection,
                did,
                rkey,
                "tangled-only record skipped (no cospan equivalent)"
            );
        }

        // ─── Catch-alls ─────────────────────────────────────────────
        c if c.starts_with("sh.tangled.") => {
            tracing::debug!(
                collection = c,
                "tangled record received for unhandled collection"
            );
        }
        c if c.starts_with("dev.cospan.") => {
            tracing::debug!(collection = c, "unhandled dev.cospan collection");
        }

        _ => {}
    }

    Ok(())
}

// ─── Helper functions ───────────────────────────────────────────────────────

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
