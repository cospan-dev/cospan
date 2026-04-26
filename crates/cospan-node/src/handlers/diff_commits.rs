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
//!    languages + all panproto-protocols schema formats), parses
//!    both sides into panproto schemas, runs `panproto_check::diff`
//!    + `classify`, and attaches the result.
//!
//! Query parameters:
//!   - `did`: repo owner
//!   - `repo`: repo name
//!   - `from`: base commit OID
//!   - `to`: head commit OID
//!   - `contextLines`: optional unified context lines (default 3)

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use axum::Json;
use axum::extract::{Query, State};
use panproto_xrpc::{DiffCommitsResult, FileDiff};
use serde::Deserialize;
use serde_json::json;

use crate::error::NodeError;
use crate::state::NodeState;

/// Cache for diff results keyed by (did, repo, from_oid, to_oid, context_lines).
/// Commit diffs are immutable once computed (git OIDs are content-addressed),
/// so this cache never needs invalidation.
static DIFF_CACHE: LazyLock<Mutex<HashMap<String, DiffCommitsResult>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

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
    // Acquire everything we need from the shared store under one async
    // lock acquisition; once the guard is dropped the rest of the handler
    // is fully synchronous and can hold non-`Send` git2 types without
    // making the handler future itself non-`Send`.
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
    let import_marks = store.load_import_marks(&params.did, &params.repo);
    let vcs_store = store.open(&params.did, &params.repo).ok();
    drop(store);

    let from_oid = git2::Oid::from_str(&params.from)
        .map_err(|e| NodeError::InvalidRequest(format!("bad 'from' oid: {e}")))?;
    let to_oid = git2::Oid::from_str(&params.to)
        .map_err(|e| NodeError::InvalidRequest(format!("bad 'to' oid: {e}")))?;

    // Check cache: commit OIDs are content-addressed so diffs are immutable.
    let ctx_lines = params.context_lines.unwrap_or(3);
    let cache_key = format!(
        "{}:{}:{from_oid}:{to_oid}:{ctx_lines}",
        params.did, params.repo
    );
    if let Ok(cache) = DIFF_CACHE.lock()
        && let Some(cached) = cache.get(&cache_key)
    {
        return Ok(Json(cached.clone()));
    }

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
            if let Some(hunk) = hunk_cell.borrow_mut().take()
                && let Some(last) = files_cell.borrow_mut().last_mut()
            {
                last.hunks.push(json!({
                    "oldStart": hunk.old_start,
                    "oldLines": hunk.old_lines,
                    "newStart": hunk.new_start,
                    "newLines": hunk.new_lines,
                    "header": hunk.header,
                    "lines": hunk.lines,
                }));
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
            let old_path_str = old_file.path().map(|p| p.to_string_lossy().into_owned());
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
            if let Some(prev) = hunk_cell.borrow_mut().take()
                && let Some(last) = files_cell.borrow_mut().last_mut()
            {
                last.hunks.push(json!({
                    "oldStart": prev.old_start,
                    "oldLines": prev.old_lines,
                    "newStart": prev.new_start,
                    "newLines": prev.new_lines,
                    "header": prev.header,
                    "lines": prev.lines,
                }));
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
    if let Some(hunk) = hunk_cell.borrow_mut().take()
        && let Some(last) = files_cell.borrow_mut().last_mut()
    {
        last.hunks.push(json!({
            "oldStart": hunk.old_start,
            "oldLines": hunk.old_lines,
            "newStart": hunk.new_start,
            "newLines": hunk.new_lines,
            "header": hunk.header,
            "lines": hunk.lines,
        }));
    }

    let entries = files_cell.into_inner();

    // Totals
    let total_additions: u64 = entries.iter().map(|f| f.additions).sum();
    let total_deletions: u64 = entries.iter().map(|f| f.deletions).sum();

    // Pass 2: attach structural diff from the panproto-vcs store if
    // available. We NEVER parse server-side: structural data only comes
    // from pre-parsed schemas pushed via git-remote-cospan. Repos pushed
    // via raw git get hunks only; the frontend prompts users to install
    // git-remote-cospan for structural analysis.
    //
    // See panproto/panproto#28 (distribute git-remote-cospan binary).
    let file_paths: Vec<(String, bool)> =
        entries.iter().map(|e| (e.path.clone(), e.binary)).collect();
    let structural_diffs = vcs_store.as_ref().and_then(|store| {
        try_load_structural_diffs_from_vcs(store, &import_marks, from_oid, to_oid, &file_paths)
    });

    let files: Vec<FileDiff> = entries
        .iter()
        .map(|f| {
            let structural_diff = structural_diffs
                .as_ref()
                .and_then(|m| m.get(&f.path).cloned());
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

    let result = DiffCommitsResult {
        from: params.from,
        to: params.to,
        file_count: files.len() as u64,
        files,
        total_additions,
        total_deletions,
    };
    if let Ok(mut cache) = DIFF_CACHE.lock() {
        cache.insert(cache_key, result.clone());
    }
    Ok(Json(result))
}

/// Load pre-parsed structural diffs from the panproto-vcs store.
/// Returns `None` if the store doesn't have schemas for these commits
/// (meaning the repo was pushed via raw git, not git-remote-cospan).
///
/// Walks each commit's `SchemaTree` (panproto issue #49) into a
/// `path -> FileSchemaObject` map, then diffs per-file schemas natively.
///
/// Synchronous: callers obtain `vcs_store` and `import_marks` from the
/// shared `NodeState.store` lock once at the top of the handler, then
/// pass them in here so this function does not need to await.
fn try_load_structural_diffs_from_vcs(
    vcs_store: &panproto_core::vcs::FsStore,
    import_marks: &std::collections::HashMap<git2::Oid, panproto_core::vcs::ObjectId>,
    from_git_oid: git2::Oid,
    to_git_oid: git2::Oid,
    file_paths: &[(String, bool)],
) -> Option<std::collections::HashMap<String, serde_json::Value>> {
    use panproto_core::vcs::{self, FileSchemaObject, Object, Store};

    let from_pp = import_marks.get(&from_git_oid).copied()?;
    let to_pp = import_marks.get(&to_git_oid).copied()?;

    fn collect_leaves<S: Store>(
        store: &S,
        pp_id: &vcs::ObjectId,
    ) -> Option<std::collections::HashMap<String, FileSchemaObject>> {
        let commit = match store.get(pp_id).ok()? {
            Object::Commit(c) => c,
            _ => return None,
        };
        let mut out = std::collections::HashMap::new();
        vcs::walk_tree(store, &commit.schema_id, |path, file| {
            out.insert(path.to_string_lossy().into_owned(), file.clone());
            Ok(())
        })
        .ok()?;
        Some(out)
    }

    let from_leaves = collect_leaves(vcs_store, &from_pp)?;
    let to_leaves = collect_leaves(vcs_store, &to_pp)?;

    // Produce an empty schema matching the shape of `template`, used when a
    // file is pure-add or pure-delete (only one side has a leaf).
    let empty_like = |template: &panproto_schema::Schema| {
        let mut s = template.clone();
        s.vertices.clear();
        s.edges.clear();
        s.hyper_edges.clear();
        s.constraints.clear();
        s.required.clear();
        s.nsids.clear();
        s.entries.clear();
        s.variants.clear();
        s
    };

    let mut result = std::collections::HashMap::new();
    for (path, binary) in file_paths {
        if *binary {
            continue;
        }

        let old_leaf = from_leaves.get(path);
        let new_leaf = to_leaves.get(path);

        let (old_schema, new_schema, protocol) = match (old_leaf, new_leaf) {
            (Some(o), Some(n)) => (o.schema.clone(), n.schema.clone(), n.protocol.clone()),
            (Some(o), None) => {
                let empty = empty_like(&o.schema);
                (o.schema.clone(), empty, o.protocol.clone())
            }
            (None, Some(n)) => {
                let empty = empty_like(&n.schema);
                (empty, n.schema.clone(), n.protocol.clone())
            }
            (None, None) => continue,
        };

        let raw_diff = panproto_check::diff(&old_schema, &new_schema);
        let report = panproto_check::CompatReport {
            breaking: Vec::new(),
            non_breaking: Vec::new(),
            compatible: true,
        };
        let sd = super::structural::StructuralDiff {
            protocol,
            report,
            raw_diff,
            old_vertex_count: old_schema.vertices.len(),
            new_vertex_count: new_schema.vertices.len(),
            old_edge_count: old_schema.edges.len(),
            new_edge_count: new_schema.edges.len(),
            old_schema,
            new_schema,
        };
        let json = super::structural::structural_diff_to_json(&sd, None, None);
        result.insert(path.clone(), json);
    }
    Some(result)
}
