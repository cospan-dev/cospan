//! `POST /:did/:repo/git-receive-pack` handler.
//!
//! Handles `git push` by receiving packfile data from the git client,
//! materializing it into a temporary git repository, then using
//! panproto-git's `import_git_repo` to convert the git objects into
//! panproto-vcs objects stored in the FsStore.
//!
//! The git smart HTTP receive-pack protocol sends:
//! 1. Reference update commands (old-oid new-oid refname)
//! 2. A packfile containing the objects
//!
//! We parse the update commands, unpack the packfile via libgit2, import
//! the objects through panproto-git, and update refs accordingly.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::IntoResponse;

use crate::state::NodeState;

/// Parse pkt-line formatted reference update commands from the receive-pack body.
///
/// Returns a list of (old_oid, new_oid, refname) tuples and the remaining
/// bytes (the packfile data after the flush packet).
fn parse_ref_updates(body: &[u8]) -> (Vec<(String, String, String)>, &[u8]) {
    let mut updates = Vec::new();
    let mut pos = 0;

    loop {
        if pos + 4 > body.len() {
            break;
        }

        // Read 4-char hex length prefix.
        let len_str = match std::str::from_utf8(&body[pos..pos + 4]) {
            Ok(s) => s,
            Err(_) => break,
        };

        let pkt_len = match usize::from_str_radix(len_str, 16) {
            Ok(l) => l,
            Err(_) => break,
        };

        // Flush packet (0000) marks end of commands.
        if pkt_len == 0 {
            pos += 4;
            break;
        }

        if pos + pkt_len > body.len() {
            break;
        }

        let line = &body[pos + 4..pos + pkt_len];
        pos += pkt_len;

        // Parse "old-oid new-oid refname\n" (strip capabilities after NUL if present).
        if let Ok(line_str) = std::str::from_utf8(line) {
            let line_str = line_str.trim();
            // Capabilities are separated by NUL on the first line.
            let command_part = line_str.split('\0').next().unwrap_or(line_str);
            let parts: Vec<&str> = command_part.splitn(3, ' ').collect();
            if parts.len() == 3 {
                updates.push((
                    parts[0].to_string(),
                    parts[1].to_string(),
                    parts[2].to_string(),
                ));
            }
        }
    }

    (updates, &body[pos..])
}

/// Build a pkt-line formatted response.
fn pkt_line(data: &str) -> String {
    let len = data.len() + 4;
    format!("{len:04x}{data}")
}

/// Handle `POST /:did/:repo/git-receive-pack`.
///
/// Receives a packfile from the git client, imports the objects through
/// panproto-git into the panproto-vcs store, and updates refs.
pub async fn git_receive_pack(
    State(state): State<Arc<NodeState>>,
    Path((did, repo)): Path<(String, String)>,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    tracing::info!(%did, %repo, body_len = body.len(), "git-receive-pack requested");

    // Parse reference update commands and extract packfile data.
    let (ref_updates, packfile_data) = parse_ref_updates(&body);

    if ref_updates.is_empty() {
        let msg = pkt_line("unpack ok\n");
        return (
            StatusCode::OK,
            [(
                header::CONTENT_TYPE,
                "application/x-git-receive-pack-result",
            )],
            format!("{msg}0000"),
        );
    }

    // Create a temporary directory to materialize the packfile as a git repo.
    let temp_dir = match tempfile::tempdir() {
        Ok(d) => d,
        Err(e) => {
            let err_msg = format!("ERR failed to create temp dir: {e}\n");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(
                    header::CONTENT_TYPE,
                    "application/x-git-receive-pack-result",
                )],
                pkt_line(&err_msg) + "0000",
            );
        }
    };

    // Initialize a bare git repo in the temp directory.
    let git_repo = match git2::Repository::init_bare(temp_dir.path()) {
        Ok(r) => r,
        Err(e) => {
            let err_msg = format!("ERR failed to init temp git repo: {e}\n");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(
                    header::CONTENT_TYPE,
                    "application/x-git-receive-pack-result",
                )],
                pkt_line(&err_msg) + "0000",
            );
        }
    };

    // Write the packfile data and index it via libgit2's ODB.
    if !packfile_data.is_empty() {
        let odb = match git_repo.odb() {
            Ok(o) => o,
            Err(e) => {
                let err_msg = format!("ERR failed to open ODB: {e}\n");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [(
                        header::CONTENT_TYPE,
                        "application/x-git-receive-pack-result",
                    )],
                    pkt_line(&err_msg) + "0000",
                );
            }
        };

        // Write the packfile to the ODB so libgit2 can index it.
        let pack_path = temp_dir.path().join("objects").join("pack");
        let _ = std::fs::create_dir_all(&pack_path);
        let pack_file = pack_path.join("incoming.pack");
        if let Err(e) = std::fs::write(&pack_file, packfile_data) {
            let err_msg = format!("ERR failed to write packfile: {e}\n");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(
                    header::CONTENT_TYPE,
                    "application/x-git-receive-pack-result",
                )],
                pkt_line(&err_msg) + "0000",
            );
        }

        // Use git index-pack to index the packfile.
        // libgit2's indexer API handles this.
        match odb.add_disk_alternate(pack_path.to_str().unwrap_or(".")) {
            Ok(_) => {}
            Err(e) => {
                tracing::warn!("failed to add pack alternate: {e}");
            }
        }
    }

    // For each ref update, set the ref in the temp repo so import can walk it.
    for (_, new_oid, refname) in &ref_updates {
        let zero_oid = "0".repeat(40);
        if new_oid != &zero_oid
            && let Ok(oid) = git2::Oid::from_str(new_oid)
        {
            let _ = git_repo.reference(refname, oid, true, "receive-pack");
        }
    }

    // Import the git objects into panproto-vcs via panproto-git.
    let store = state.store.lock().await;
    let mut vcs_store = match store.open_or_init(&did, &repo) {
        Ok(s) => s,
        Err(e) => {
            let err_msg = format!("ERR failed to open panproto store: {e}\n");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(
                    header::CONTENT_TYPE,
                    "application/x-git-receive-pack-result",
                )],
                pkt_line(&err_msg) + "0000",
            );
        }
    };

    // Import from the first ref update's new target.
    let mut response = String::new();

    for (_old_oid, new_oid, refname) in &ref_updates {
        let zero_oid = "0".repeat(40);

        if new_oid == &zero_oid {
            // Ref deletion — not supported through git bridge.
            response.push_str(&pkt_line(&format!("ng {refname} deletion not supported\n")));
            continue;
        }

        match panproto_git::import_git_repo(&git_repo, &mut vcs_store, new_oid) {
            Ok(import_result) => {
                // Update the panproto ref to point to the imported commit.
                if let Err(e) =
                    panproto_vcs::Store::set_ref(&mut vcs_store, refname, import_result.head_id)
                {
                    response.push_str(&pkt_line(&format!("ng {refname} store error: {e}\n")));
                } else {
                    tracing::info!(
                        %did, %repo, %refname,
                        commits = import_result.commit_count,
                        "imported git commits into panproto-vcs"
                    );
                    response.push_str(&pkt_line(&format!("ok {refname}\n")));
                }
            }
            Err(e) => {
                tracing::error!(%did, %repo, %refname, error = %e, "git import failed");
                response.push_str(&pkt_line(&format!("ng {refname} import failed: {e}\n")));
            }
        }
    }

    // Prepend unpack status.
    let full_response = format!("{}{}0000", pkt_line("unpack ok\n"), response);

    (
        StatusCode::OK,
        [(
            header::CONTENT_TYPE,
            "application/x-git-receive-pack-result",
        )],
        full_response,
    )
}
