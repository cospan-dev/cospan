//! `GET /xrpc/dev.panproto.node.getCommitSchemaStats`
//!
//! For a range of commits, returns per-commit schema statistics by
//! reading the already-imported schemas from the panproto-vcs store.
//! Each commit's schema was parsed and stored during git push via
//! `import_git_repo_incremental`, so this is a cheap read operation
//! (no re-parsing). Breaking/non-breaking change counts come from
//! diffing adjacent schemas via `panproto_check::diff` + `classify`.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use panproto_core::vcs::{self, Object, Store};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::NodeError;
use crate::state::NodeState;

use super::list_commits::{resolve_default, resolve_ref};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Params {
    pub did: String,
    pub repo: String,
    #[serde(rename = "ref")]
    pub ref_name: Option<String>,
    pub limit: Option<usize>,
}

pub async fn get_commit_schema_stats(
    State(state): State<Arc<NodeState>>,
    Query(params): Query<Params>,
) -> Result<Json<Value>, NodeError> {
    let limit = params.limit.unwrap_or(30).min(100);

    let store_guard = state.store.lock().await;
    if !store_guard.has_git_mirror(&params.did, &params.repo) {
        return Err(NodeError::RefNotFound(format!(
            "repo {}/{} not found",
            params.did, params.repo
        )));
    }
    let mirror = store_guard
        .open_or_init_git_mirror(&params.did, &params.repo)
        .map_err(|e| NodeError::Internal(format!("open mirror: {e}")))?;

    // Open the panproto-vcs store where imported schemas live.
    let vcs_store = match store_guard.open(&params.did, &params.repo) {
        Ok(s) => s,
        Err(_) => {
            // VCS store not yet initialized (no push has happened).
            // Fall back to empty stats.
            drop(store_guard);
            return Ok(Json(json!({ "commits": [] })));
        }
    };

    // Load the import marks to map git OIDs to panproto-vcs ObjectIds.
    let marks = store_guard.load_import_marks(&params.did, &params.repo);
    drop(store_guard);

    let start_oid = match params.ref_name.as_deref() {
        Some(name) => resolve_ref(&mirror, name)?,
        None => resolve_default(&mirror)?,
    };

    let mut walk = mirror
        .revwalk()
        .map_err(|e| NodeError::Internal(format!("revwalk: {e}")))?;
    walk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)
        .map_err(|e| NodeError::Internal(format!("set sorting: {e}")))?;
    walk.push(start_oid)
        .map_err(|e| NodeError::Internal(format!("push start: {e}")))?;

    let mut commits: Vec<Value> = Vec::new();
    let mut prev_schema: Option<panproto_schema::Schema> = None;

    for oid_result in walk.take(limit) {
        let oid = match oid_result {
            Ok(o) => o,
            Err(_) => break,
        };
        let git_commit = match mirror.find_commit(oid) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let summary = git_commit.summary().unwrap_or_default().to_string();
        let timestamp = u64::try_from(git_commit.time().seconds()).unwrap_or(0);

        // Look up the panproto-vcs commit via the import marks.
        let (total_vc, total_ec, breaking, non_breaking) = if let Some(pp_id) = marks.get(&oid) {
            match vcs_store.get(pp_id) {
                Ok(Object::Commit(pp_commit)) => {
                    // Per-file content addressing (panproto issue #49):
                    // the commit's `schema_id` is a SchemaTree root;
                    // assemble to get the flat schema used below.
                    let schema = vcs::resolve_commit_schema(&vcs_store, &pp_commit).ok();

                    let vc = schema.as_ref().map_or(0, |s| s.vertices.len());
                    let ec = schema.as_ref().map_or(0, |s| s.edges.len());

                    // Diff against the previous commit's schema for
                    // breaking/non-breaking classification.
                    let (b, nb) = match (&schema, &prev_schema) {
                        (Some(curr), Some(prev)) => {
                            let raw_diff = panproto_check::diff(prev, curr);
                            let protocol = super::structural::resolve_protocol(&curr.protocol);
                            match protocol {
                                Some(p) => {
                                    let report = panproto_check::classify(&raw_diff, &p);
                                    (report.breaking.len(), report.non_breaking.len())
                                }
                                None => (0, 0),
                            }
                        }
                        _ => (0, 0),
                    };

                    if let Some(s) = schema {
                        prev_schema = Some(s);
                    }

                    (vc, ec, b, nb)
                }
                _ => (0, 0, 0, 0),
            }
        } else {
            (0, 0, 0, 0)
        };

        commits.push(json!({
            "oid": oid.to_string(),
            "timestamp": timestamp,
            "summary": summary,
            "totalVertexCount": total_vc,
            "totalEdgeCount": total_ec,
            "breakingChangeCount": breaking,
            "nonBreakingChangeCount": non_breaking,
        }));
    }

    Ok(Json(json!({ "commits": commits })))
}
