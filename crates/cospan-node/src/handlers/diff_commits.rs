//! `GET /xrpc/dev.panproto.node.diffCommits` and cospan alias.
//!
//! Computes the diff between two commits in the persistent git mirror.
//!
//! Returns, for each changed path:
//!   - old/new OID
//!   - file status (added / modified / removed / renamed)
//!   - line-level hunks (unified diff format)
//!   - total additions / deletions
//!
//! Query parameters:
//!   - `did` — repo owner
//!   - `repo` — repo name
//!   - `from` — base commit OID
//!   - `to` — head commit OID
//!   - `contextLines` — optional unified context lines (default 3)

use std::cell::RefCell;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;
use serde_json::json;

use crate::error::NodeError;
use crate::state::NodeState;

#[derive(Deserialize)]
pub struct DiffCommitsParams {
    pub did: String,
    pub repo: String,
    pub from: String,
    pub to: String,
    pub context_lines: Option<u32>,
}

pub async fn diff_commits(
    State(state): State<Arc<NodeState>>,
    Query(params): Query<DiffCommitsParams>,
) -> Result<Json<serde_json::Value>, NodeError> {
    let store = state.store.lock().await;
    if !store.has_git_mirror(&params.did, &params.repo) {
        return Err(NodeError::RefNotFound(format!(
            "repo {}/{} not found",
            params.did, params.repo
        )));
    }
    let mirror = store
        .open_or_init_git_mirror(&params.did, &params.repo)
        .map_err(|e| NodeError::Internal(format!("open mirror: {e}")))?;
    drop(store);

    let from_oid = git2::Oid::from_str(&params.from)
        .map_err(|e| NodeError::InvalidRequest(format!("bad 'from' oid: {e}")))?;
    let to_oid = git2::Oid::from_str(&params.to)
        .map_err(|e| NodeError::InvalidRequest(format!("bad 'to' oid: {e}")))?;

    let from_commit = mirror
        .find_commit(from_oid)
        .map_err(|_| NodeError::ObjectNotFound(params.from.clone()))?;
    let to_commit = mirror
        .find_commit(to_oid)
        .map_err(|_| NodeError::ObjectNotFound(params.to.clone()))?;

    let from_tree = from_commit
        .tree()
        .map_err(|e| NodeError::Internal(format!("from tree: {e}")))?;
    let to_tree = to_commit
        .tree()
        .map_err(|e| NodeError::Internal(format!("to tree: {e}")))?;

    let mut diff_opts = git2::DiffOptions::new();
    diff_opts.context_lines(params.context_lines.unwrap_or(3));
    diff_opts.id_abbrev(40);
    let diff = mirror
        .diff_tree_to_tree(Some(&from_tree), Some(&to_tree), Some(&mut diff_opts))
        .map_err(|e| NodeError::Internal(format!("diff trees: {e}")))?;

    // First pass: walk deltas to build a path → file entry.
    struct FileEntry {
        path: String,
        old_path: Option<String>,
        status: &'static str,
        old_oid: String,
        new_oid: String,
        additions: u32,
        deletions: u32,
        hunks: Vec<HunkEntry>,
        binary: bool,
    }
    struct HunkEntry {
        old_start: u32,
        old_lines: u32,
        new_start: u32,
        new_lines: u32,
        header: String,
        lines: Vec<LineEntry>,
    }
    struct LineEntry {
        origin: char,
        content: String,
        old_lineno: Option<u32>,
        new_lineno: Option<u32>,
    }

    let files_cell: RefCell<Vec<FileEntry>> = RefCell::new(Vec::new());

    diff.foreach(
        &mut |delta, _progress| {
            let status = match delta.status() {
                git2::Delta::Added => "added",
                git2::Delta::Deleted => "removed",
                git2::Delta::Modified => "modified",
                git2::Delta::Renamed => "renamed",
                git2::Delta::Copied => "copied",
                git2::Delta::Typechange => "typechange",
                _ => "modified",
            };
            let new_file = delta.new_file();
            let old_file = delta.old_file();
            let new_path = new_file
                .path()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();
            let old_path_str = old_file
                .path()
                .map(|p| p.to_string_lossy().into_owned());
            let old_path = match (status, old_path_str.as_deref(), new_path.as_str()) {
                ("renamed", Some(op), np) if op != np => old_path_str.clone(),
                _ => None,
            };
            files_cell.borrow_mut().push(FileEntry {
                path: new_path,
                old_path,
                status,
                old_oid: old_file.id().to_string(),
                new_oid: new_file.id().to_string(),
                additions: 0,
                deletions: 0,
                hunks: Vec::new(),
                binary: delta.flags().contains(git2::DiffFlags::BINARY),
            });
            true
        },
        None,
        Some(&mut |_delta, hunk| {
            let mut files_mut = files_cell.borrow_mut();
            if let Some(last) = files_mut.last_mut() {
                let header = String::from_utf8_lossy(hunk.header()).into_owned();
                last.hunks.push(HunkEntry {
                    old_start: hunk.old_start(),
                    old_lines: hunk.old_lines(),
                    new_start: hunk.new_start(),
                    new_lines: hunk.new_lines(),
                    header,
                    lines: Vec::new(),
                });
            }
            true
        }),
        Some(&mut |_delta, _hunk, line| {
            let mut files_mut = files_cell.borrow_mut();
            if let Some(last_file) = files_mut.last_mut() {
                let origin = line.origin();
                let content = String::from_utf8_lossy(line.content()).into_owned();
                match origin {
                    '+' => last_file.additions += 1,
                    '-' => last_file.deletions += 1,
                    _ => {}
                }
                if let Some(last_hunk) = last_file.hunks.last_mut() {
                    last_hunk.lines.push(LineEntry {
                        origin,
                        content,
                        old_lineno: line.old_lineno(),
                        new_lineno: line.new_lineno(),
                    });
                }
            }
            true
        }),
    )
    .map_err(|e| NodeError::Internal(format!("diff.foreach: {e}")))?;

    let files = files_cell.into_inner();

    // Totals
    let total_additions: u32 = files.iter().map(|f| f.additions).sum();
    let total_deletions: u32 = files.iter().map(|f| f.deletions).sum();

    let files_json: Vec<serde_json::Value> = files
        .iter()
        .map(|f| {
            let hunks_json: Vec<serde_json::Value> = f
                .hunks
                .iter()
                .map(|h| {
                    let lines_json: Vec<serde_json::Value> = h
                        .lines
                        .iter()
                        .map(|l| {
                            json!({
                                "origin": l.origin.to_string(),
                                "content": l.content,
                                "oldLineno": l.old_lineno,
                                "newLineno": l.new_lineno,
                            })
                        })
                        .collect();
                    json!({
                        "oldStart": h.old_start,
                        "oldLines": h.old_lines,
                        "newStart": h.new_start,
                        "newLines": h.new_lines,
                        "header": h.header,
                        "lines": lines_json,
                    })
                })
                .collect();
            json!({
                "path": f.path,
                "oldPath": f.old_path,
                "status": f.status,
                "oldOid": f.old_oid,
                "newOid": f.new_oid,
                "additions": f.additions,
                "deletions": f.deletions,
                "binary": f.binary,
                "hunks": hunks_json,
            })
        })
        .collect();

    Ok(Json(json!({
        "from": params.from,
        "to": params.to,
        "files": files_json,
        "totalAdditions": total_additions,
        "totalDeletions": total_deletions,
        "fileCount": files.len(),
    })))
}
