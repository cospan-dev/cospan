//! Generic dispatch table for simple record types.
//!
//! A "simple" record is one where the create/update handler is:
//!   1. Deserialize transformed JSON into the row type
//!   2. Set did, rkey, indexed_at
//!   3. Upsert
//!
//! And the delete handler is just a direct delete call.
//!
//! Records with side effects (counter updates, SSE events, state transitions,
//! custom field extraction) are handled as special cases in consumer.rs.

use std::sync::Arc;

use chrono::Utc;

use crate::db;
use crate::state::AppState;

use super::consumer::transform_record;

/// Dispatch a create/update for a simple record type.
/// Returns `Ok(true)` if handled, `Ok(false)` if not a simple record.
pub async fn dispatch_simple_upsert(
    state: &Arc<AppState>,
    collection: &str,
    did: &str,
    rkey: &str,
    record: &serde_json::Value,
) -> anyhow::Result<bool> {
    // Macro to reduce boilerplate for the common pattern:
    //   deserialize -> set did+rkey+indexed_at -> upsert
    macro_rules! simple_upsert {
        // Standard: has both did and rkey
        ($mod:ident, $Row:ident) => {{
            let mut row: db::$mod::$Row =
                serde_json::from_value(transform_record(state, collection, record))?;
            row.did = did.to_string();
            row.rkey = rkey.to_string();
            row.indexed_at = Utc::now();
            db::$mod::upsert(&state.db, &row).await?;
            return Ok(true);
        }};
        // No rkey (e.g., actor.profile with literal:self key)
        ($mod:ident, $Row:ident, no_rkey) => {{
            let mut row: db::$mod::$Row =
                serde_json::from_value(transform_record(state, collection, record))?;
            row.did = did.to_string();
            row.indexed_at = Utc::now();
            db::$mod::upsert(&state.db, &row).await?;
            return Ok(true);
        }};
        // No did (e.g., ref_update where did is committer_did)
        ($mod:ident, $Row:ident, no_did) => {{
            let mut row: db::$mod::$Row =
                serde_json::from_value(transform_record(state, collection, record))?;
            row.rkey = rkey.to_string();
            row.indexed_at = Utc::now();
            db::$mod::upsert(&state.db, &row).await?;
            return Ok(true);
        }};
        // Has repo FK: ensure parent repo exists before upserting
        ($mod:ident, $Row:ident, repo_fk) => {{
            let mut row: db::$mod::$Row =
                serde_json::from_value(transform_record(state, collection, record))?;
            row.did = did.to_string();
            row.rkey = rkey.to_string();
            row.indexed_at = Utc::now();
            let source = if collection.starts_with("sh.tangled.") { "tangled" } else { "cospan" };
            db::repo::ensure_exists(&state.db, &row.repo_did, &row.repo_name, source).await?;
            db::$mod::upsert(&state.db, &row).await?;
            return Ok(true);
        }};
    }

    match collection {
        // ─── dev.cospan simple records ──────────────────────────────
        "dev.cospan.node" => simple_upsert!(node, NodeRow),
        "dev.cospan.actor.profile" => simple_upsert!(actor_profile, ActorProfileRow, no_rkey),
        "dev.cospan.graph.follow" => simple_upsert!(follow, FollowRow),
        "dev.cospan.feed.reaction" => simple_upsert!(reaction, ReactionRow),
        "dev.cospan.label.definition" => simple_upsert!(label, LabelRow, repo_fk),
        "dev.cospan.org" => simple_upsert!(org, OrgRow),
        "dev.cospan.org.member" => simple_upsert!(org_member, OrgMemberRow),
        "dev.cospan.repo.collaborator" => simple_upsert!(collaborator, CollaboratorRow, repo_fk),
        "dev.cospan.repo.dependency" => {
            let mut row: db::dependency::DependencyRow =
                serde_json::from_value(transform_record(state, collection, record))?;
            row.did = did.to_string();
            row.rkey = rkey.to_string();
            row.indexed_at = Utc::now();
            db::repo::ensure_exists(&state.db, &row.source_repo_did, &row.source_repo_name, "cospan").await?;
            db::repo::ensure_exists(&state.db, &row.target_repo_did, &row.target_repo_name, "cospan").await?;
            db::dependency::upsert(&state.db, &row).await?;
            return Ok(true);
        }

        // ─── sh.tangled simple records (same DB tables) ─────────────
        "sh.tangled.knot" => simple_upsert!(node, NodeRow),
        "sh.tangled.actor.profile" => simple_upsert!(actor_profile, ActorProfileRow, no_rkey),
        "sh.tangled.graph.follow" => simple_upsert!(follow, FollowRow),
        "sh.tangled.feed.reaction" => simple_upsert!(reaction, ReactionRow),
        "sh.tangled.repo.collaborator" => simple_upsert!(collaborator, CollaboratorRow, repo_fk),
        "sh.tangled.knot.member" => simple_upsert!(org_member, OrgMemberRow),
        // NOTE: sh.tangled.repo, pipeline, refUpdate, issue, pull have side effects
        // (node URL lookup, breaking change count, SSE events) and are handled
        // in consumer.rs dispatch_special_upsert, not here.
        _ => Ok(false),
    }
}

/// Dispatch a delete for a simple record type.
/// Returns `Ok(true)` if handled, `Ok(false)` if not a simple record.
pub async fn dispatch_simple_delete(
    state: &Arc<AppState>,
    collection: &str,
    did: &str,
    rkey: &str,
) -> anyhow::Result<bool> {
    // Macro for delete: most use (did, rkey), some use (did) only
    macro_rules! simple_delete {
        ($mod:ident, did_rkey) => {{
            db::$mod::delete(&state.db, did, rkey).await?;
            return Ok(true);
        }};
        ($mod:ident, did_only) => {{
            db::$mod::delete(&state.db, did).await?;
            return Ok(true);
        }};
    }

    match collection {
        // ─── dev.cospan simple deletes ──────────────────────────────
        "dev.cospan.node" => simple_delete!(node, did_only),
        "dev.cospan.actor.profile" => simple_delete!(actor_profile, did_only),
        "dev.cospan.graph.follow" => simple_delete!(follow, did_rkey),
        "dev.cospan.feed.reaction" => simple_delete!(reaction, did_rkey),
        "dev.cospan.label.definition" => simple_delete!(label, did_rkey),
        "dev.cospan.org" => simple_delete!(org, did_rkey),
        "dev.cospan.org.member" => simple_delete!(org_member, did_rkey),
        "dev.cospan.repo.collaborator" => simple_delete!(collaborator, did_rkey),
        "dev.cospan.repo.dependency" => simple_delete!(dependency, did_rkey),
        "dev.cospan.repo.issue.state" => simple_delete!(issue_state, did_rkey),
        "dev.cospan.repo.pull.state" => simple_delete!(pull_state, did_rkey),
        "dev.cospan.vcs.refUpdate" => simple_delete!(ref_update, did_rkey),

        // ─── sh.tangled simple deletes ──────────────────────────────
        "sh.tangled.knot" => simple_delete!(node, did_only),
        "sh.tangled.actor.profile" => simple_delete!(actor_profile, did_only),
        "sh.tangled.graph.follow" => simple_delete!(follow, did_rkey),
        "sh.tangled.feed.reaction" => simple_delete!(reaction, did_rkey),
        "sh.tangled.repo.collaborator" => simple_delete!(collaborator, did_rkey),
        "sh.tangled.knot.member" => simple_delete!(org_member, did_rkey),
        "sh.tangled.spindle" => simple_delete!(org, did_rkey),
        "sh.tangled.spindle.member" => simple_delete!(org_member, did_rkey),
        "sh.tangled.label.definition" => simple_delete!(label, did_rkey),
        "sh.tangled.pipeline" => simple_delete!(pipeline, did_rkey),
        "sh.tangled.git.refUpdate" => simple_delete!(ref_update, did_rkey),
        "sh.tangled.repo.issue.state" => simple_delete!(issue_state, did_rkey),
        "sh.tangled.repo.pull.status" => simple_delete!(pull_state, did_rkey),

        _ => Ok(false),
    }
}
