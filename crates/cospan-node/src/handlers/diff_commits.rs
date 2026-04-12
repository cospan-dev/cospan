//! `GET /xrpc/dev.panproto.node.diffCommits` and cospan alias.
//!
//! Computes the diff between two commits in the persistent git mirror.
//! Uses panproto-xrpc's typed response structs for wire-format
//! compatibility with panproto clients.
//!
//! Returns, for each changed path:
//!  - old/new OID
//!  - file status (added / modified / removed / renamed)
//!  - line-level hunks (unified diff format)
//!  - total additions / deletions
//!  - **structural diff**: for parseable files (248 tree-sitter
//!     languages + all panproto-protocols schema formats), parses
//!     both sides into panproto schemas, runs `panproto_check::diff`
//!     + `classify`, and attaches the result.
//!
//! Query parameters:
//!   - `did`: repo owner
//!   - `repo`: repo name
//!   - `from`: base commit OID
//!   - `to`: head commit OID
//!   - `contextLines`: optional unified context lines (default 3)

use std::cell::RefCell;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use panproto_xrpc::{DiffCommitsResult, FileDiff};
use serde::Deserialize;
use serde_json::json;

use crate::error::NodeError;
use crate::state::NodeState;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
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
) -> Result<Json<DiffCommitsResult>, NodeError> {
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

    // First pass: walk deltas to build file entries with hunks.
    struct FileEntry {
        path: String,
        old_path: Option<String>,
        status: &'static str,
        old_oid: String,
        new_oid: String,
        additions: u64,
        deletions: u64,
        hunks: Vec<serde_json::Value>,
        binary: bool,
    }
    struct HunkBuilder {
        old_start: u32,
        old_lines: u32,
        new_start: u32,
        new_lines: u32,
        header: String,
        lines: Vec<serde_json::Value>,
    }

    let files_cell: RefCell<Vec<FileEntry>> = RefCell::new(Vec::new());
    let hunk_cell: RefCell<Option<HunkBuilder>> = RefCell::new(None);

    diff.foreach(
        &mut |delta, _progress| {
            // Flush any pending hunk from the previous file.
            if let Some(hunk) = hunk_cell.borrow_mut().take() {
                if let Some(last) = files_cell.borrow_mut().last_mut() {
                    last.hunks.push(json!({
                        "oldStart": hunk.old_start,
                        "oldLines": hunk.old_lines,
                        "newStart": hunk.new_start,
                        "newLines": hunk.new_lines,
                        "header": hunk.header,
                        "lines": hunk.lines,
                    }));
                }
            }
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
            // Flush any previous hunk.
            if let Some(prev) = hunk_cell.borrow_mut().take() {
                if let Some(last) = files_cell.borrow_mut().last_mut() {
                    last.hunks.push(json!({
                        "oldStart": prev.old_start,
                        "oldLines": prev.old_lines,
                        "newStart": prev.new_start,
                        "newLines": prev.new_lines,
                        "header": prev.header,
                        "lines": prev.lines,
                    }));
                }
            }
            *hunk_cell.borrow_mut() = Some(HunkBuilder {
                old_start: hunk.old_start(),
                old_lines: hunk.old_lines(),
                new_start: hunk.new_start(),
                new_lines: hunk.new_lines(),
                header: String::from_utf8_lossy(hunk.header()).into_owned(),
                lines: Vec::new(),
            });
            true
        }),
        Some(&mut |_delta, _hunk, line| {
            let origin = line.origin();
            let content = String::from_utf8_lossy(line.content()).into_owned();
            let mut files_mut = files_cell.borrow_mut();
            if let Some(last_file) = files_mut.last_mut() {
                match origin {
                    '+' => last_file.additions += 1,
                    '-' => last_file.deletions += 1,
                    _ => {}
                }
            }
            if let Some(ref mut hunk) = *hunk_cell.borrow_mut() {
                hunk.lines.push(json!({
                    "origin": origin.to_string(),
                    "content": content,
                    "oldLineno": line.old_lineno(),
                    "newLineno": line.new_lineno(),
                }));
            }
            true
        }),
    )
    .map_err(|e| NodeError::Internal(format!("diff.foreach: {e}")))?;

    // Flush final hunk.
    if let Some(hunk) = hunk_cell.borrow_mut().take() {
        if let Some(last) = files_cell.borrow_mut().last_mut() {
            last.hunks.push(json!({
                "oldStart": hunk.old_start,
                "oldLines": hunk.old_lines,
                "newStart": hunk.new_start,
                "newLines": hunk.new_lines,
                "header": hunk.header,
                "lines": hunk.lines,
            }));
        }
    }

    let entries = files_cell.into_inner();

    // Totals
    let total_additions: u64 = entries.iter().map(|f| f.additions).sum();
    let total_deletions: u64 = entries.iter().map(|f| f.deletions).sum();

    // Pass 2: panproto structural diff per file.
    let registry = panproto_parse::ParserRegistry::new();
    let files: Vec<FileDiff> = entries
        .iter()
        .map(|f| {
            let structural_diff = if f.binary {
                None
            } else {
                let old_bytes = if f.status != "added" {
                    load_blob(&mirror, &f.old_oid)
                } else {
                    None
                };
                let new_bytes = if f.status != "removed" {
                    load_blob(&mirror, &f.new_oid)
                } else {
                    None
                };
                super::structural::try_structural_diff(
                    &registry,
                    &f.path,
                    old_bytes.as_deref(),
                    new_bytes.as_deref(),
                )
                .map(|s| super::structural::structural_diff_to_json(&s))
            };

            FileDiff {
                path: f.path.clone(),
                old_path: f.old_path.clone(),
                status: f.status.to_string(),
                old_oid: Some(f.old_oid.clone()),
                new_oid: Some(f.new_oid.clone()),
                additions: f.additions,
                deletions: f.deletions,
                binary: f.binary,
                hunks: f.hunks.clone(),
                structural_diff,
            }
        })
        .collect();

    Ok(Json(DiffCommitsResult {
        from: params.from,
        to: params.to,
        file_count: files.len() as u64,
        files,
        total_additions,
        total_deletions,
    }))
}

/// Load a blob's raw contents from the git mirror, if it exists.
fn load_blob(mirror: &git2::Repository, oid_str: &str) -> Option<Vec<u8>> {
    let zero = "0".repeat(40);
    if oid_str == zero || oid_str.is_empty() {
        return None;
    }
    let oid = git2::Oid::from_str(oid_str).ok()?;
    mirror
        .find_blob(oid)
        .ok()
        .map(|b| b.content().to_vec())
}
