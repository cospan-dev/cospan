//! `POST /:did/:repo/git-upload-pack` handler.
//!
//! Handles `git clone` and `git fetch` by reading want/have negotiation
//! lines from the client, exporting panproto-vcs objects to a temporary
//! git repository via panproto-git's `export_to_git`, and sending the
//! resulting packfile back.
//!
//! The git smart HTTP upload-pack protocol:
//! 1. Client sends want/have lines (which objects it wants/has)
//! 2. Server responds with NAK/ACK and a packfile

use std::collections::HashMap;

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::IntoResponse;

use crate::state::NodeState;

/// Parse pkt-line formatted want/have negotiation from the upload-pack body.
///
/// Returns (want_oids, have_oids).
fn parse_wants_haves(body: &[u8]) -> (Vec<String>, Vec<String>) {
    let mut wants = Vec::new();
    let mut haves = Vec::new();
    let mut pos = 0;

    loop {
        if pos + 4 > body.len() {
            break;
        }

        let len_str = match std::str::from_utf8(&body[pos..pos + 4]) {
            Ok(s) => s,
            Err(_) => break,
        };

        let pkt_len = match usize::from_str_radix(len_str, 16) {
            Ok(l) => l,
            Err(_) => break,
        };

        // Flush packet or special packets.
        if pkt_len == 0 {
            pos += 4;
            continue;
        }
        if pkt_len == 1 || pkt_len == 2 {
            pos += 4;
            continue;
        }

        if pos + pkt_len > body.len() {
            break;
        }

        let line = &body[pos + 4..pos + pkt_len];
        pos += pkt_len;

        if let Ok(line_str) = std::str::from_utf8(line) {
            let line_str = line_str.trim();
            // Strip capabilities after NUL.
            let command_part = line_str.split('\0').next().unwrap_or(line_str);

            if let Some(oid) = command_part.strip_prefix("want ") {
                wants.push(oid.split_whitespace().next().unwrap_or(oid).to_string());
            } else if let Some(oid) = command_part.strip_prefix("have ") {
                haves.push(oid.split_whitespace().next().unwrap_or(oid).to_string());
            } else if command_part == "done" {
                break;
            }
        }
    }

    (wants, haves)
}

/// Build a pkt-line.
fn pkt_line(data: &str) -> Vec<u8> {
    let len = data.len() + 4;
    format!("{len:04x}{data}").into_bytes()
}

/// Handle `POST /:did/:repo/git-upload-pack`.
///
/// Receives want/have negotiation from the git client, exports panproto-vcs
/// objects to git format via panproto-git, and sends the packfile back.
pub async fn git_upload_pack(
    State(state): State<Arc<NodeState>>,
    Path((did, repo)): Path<(String, String)>,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    tracing::info!(%did, %repo, body_len = body.len(), "git-upload-pack requested");

    let (wants, _haves) = parse_wants_haves(&body);

    if wants.is_empty() {
        let mut response = Vec::new();
        response.extend_from_slice(&pkt_line("NAK\n"));
        response.extend_from_slice(b"0000");
        return (
            StatusCode::OK,
            [(
                header::CONTENT_TYPE,
                "application/x-git-upload-pack-result".to_owned(),
            )],
            response,
        );
    }

    // Open the panproto-vcs store for this repo.
    let store_guard = state.store.lock().await;

    if !store_guard.exists(&did, &repo) {
        let mut response = Vec::new();
        response.extend_from_slice(&pkt_line("ERR repository not found\n"));
        response.extend_from_slice(b"0000");
        return (
            StatusCode::NOT_FOUND,
            [(
                header::CONTENT_TYPE,
                "application/x-git-upload-pack-result".to_owned(),
            )],
            response,
        );
    }

    let vcs_store = match store_guard.open(&did, &repo) {
        Ok(s) => s,
        Err(e) => {
            let mut response = Vec::new();
            response.extend_from_slice(&pkt_line(&format!("ERR store error: {e}\n")));
            response.extend_from_slice(b"0000");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(
                    header::CONTENT_TYPE,
                    "application/x-git-upload-pack-result".to_owned(),
                )],
                response,
            );
        }
    };

    // Create a temporary git repository for the export.
    let temp_dir = match tempfile::tempdir() {
        Ok(d) => d,
        Err(e) => {
            let mut response = Vec::new();
            response.extend_from_slice(&pkt_line(&format!("ERR temp dir failed: {e}\n")));
            response.extend_from_slice(b"0000");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(
                    header::CONTENT_TYPE,
                    "application/x-git-upload-pack-result".to_owned(),
                )],
                response,
            );
        }
    };

    let git_repo = match git2::Repository::init(temp_dir.path()) {
        Ok(r) => r,
        Err(e) => {
            let mut response = Vec::new();
            response.extend_from_slice(&pkt_line(&format!("ERR git init failed: {e}\n")));
            response.extend_from_slice(b"0000");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(
                    header::CONTENT_TYPE,
                    "application/x-git-upload-pack-result".to_owned(),
                )],
                response,
            );
        }
    };

    // Get all refs from the panproto store.
    let refs = match store_guard.list_refs(&did, &repo) {
        Ok(r) => r,
        Err(e) => {
            let mut response = Vec::new();
            response.extend_from_slice(&pkt_line(&format!("ERR list refs failed: {e}\n")));
            response.extend_from_slice(b"0000");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(
                    header::CONTENT_TYPE,
                    "application/x-git-upload-pack-result".to_owned(),
                )],
                response,
            );
        }
    };

    // Export each ref's commit chain to the temporary git repo.
    // Build a mapping from panproto commit IDs to git OIDs.
    let mut panproto_to_git: HashMap<panproto_vcs::ObjectId, git2::Oid> = HashMap::new();

    for (ref_name, panproto_id) in &refs {
        match panproto_git::export_to_git(&vcs_store, &git_repo, *panproto_id, &panproto_to_git) {
            Ok(export_result) => {
                panproto_to_git.insert(*panproto_id, export_result.git_oid);
                // Set the ref in the git repo.
                let _ = git_repo.reference(ref_name, export_result.git_oid, true, "export");
                tracing::debug!(
                    %ref_name, files = export_result.file_count,
                    "exported panproto commit to git"
                );
            }
            Err(e) => {
                tracing::warn!(
                    %ref_name, error = %e,
                    "failed to export panproto commit"
                );
            }
        }
    }

    // Drop the store lock before building the packfile.
    drop(store_guard);

    // Build a packfile from the temporary git repo containing the wanted objects.
    // Use `git pack-objects` through libgit2's packbuilder.
    let mut packbuilder = match git_repo.packbuilder() {
        Ok(pb) => pb,
        Err(e) => {
            let mut response = Vec::new();
            response.extend_from_slice(&pkt_line(&format!("ERR packbuilder failed: {e}\n")));
            response.extend_from_slice(b"0000");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(
                    header::CONTENT_TYPE,
                    "application/x-git-upload-pack-result".to_owned(),
                )],
                response,
            );
        }
    };

    // Add wanted objects. The want OIDs are truncated blake3 hashes (40 chars).
    // Find the matching git OIDs from our exported refs.
    let mut found_any = false;
    for want_oid_str in &wants {
        // Try to find a matching git OID in the repo by iterating refs.
        if let Ok(oid) = git2::Oid::from_str(want_oid_str)
            && let Ok(commit) = git_repo.find_commit(oid)
        {
            let _ = packbuilder.insert_commit(commit.id());
            found_any = true;
            continue;
        }

        // The want OIDs might be truncated blake3 that we advertised via info/refs.
        // Search through our refs for matching truncated hashes.
        for (_, panproto_id) in &refs {
            let full_hex = panproto_id.to_string();
            let truncated = if full_hex.len() > 40 {
                &full_hex[..40]
            } else {
                &full_hex
            };
            if truncated == want_oid_str
                && let Some(git_oid) = panproto_to_git.get(panproto_id)
            {
                let _ = packbuilder.insert_commit(*git_oid);
                found_any = true;
            }
        }
    }

    // Remove objects the client already has.
    // For a full implementation we'd use the have list to minimize the pack,
    // but libgit2's packbuilder doesn't directly support this — it's
    // handled by the revwalk. For now, we send all wanted objects.

    if !found_any {
        let mut response = Vec::new();
        response.extend_from_slice(&pkt_line("NAK\n"));
        response.extend_from_slice(b"0000");
        return (
            StatusCode::OK,
            [(
                header::CONTENT_TYPE,
                "application/x-git-upload-pack-result".to_owned(),
            )],
            response,
        );
    }

    // Build the packfile.
    let mut pack_data = Vec::new();
    if let Err(e) = packbuilder.foreach(|data| {
        pack_data.extend_from_slice(data);
        true
    }) {
        let mut response = Vec::new();
        response.extend_from_slice(&pkt_line(&format!("ERR pack build failed: {e}\n")));
        response.extend_from_slice(b"0000");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(
                header::CONTENT_TYPE,
                "application/x-git-upload-pack-result".to_owned(),
            )],
            response,
        );
    }

    // Build the response: NAK + sideband-encoded packfile.
    // Git smart HTTP protocol sends the packfile in sideband format:
    // - Band 1: packfile data
    // - Band 2: progress messages
    // - Band 3: error messages
    let mut response = Vec::new();
    response.extend_from_slice(&pkt_line("NAK\n"));

    // Send packfile data in sideband band 1.
    // Each sideband packet: pkt-line with first byte = band number.
    const SIDEBAND_CHUNK_SIZE: usize = 65515; // Max pkt-line payload minus band byte
    for chunk in pack_data.chunks(SIDEBAND_CHUNK_SIZE) {
        let pkt_len = chunk.len() + 5; // 4 for length prefix + 1 for band byte
        let len_str = format!("{pkt_len:04x}");
        response.extend_from_slice(len_str.as_bytes());
        response.push(1); // sideband band 1 = packfile data
        response.extend_from_slice(chunk);
    }

    // Flush packet to end the response.
    response.extend_from_slice(b"0000");

    (
        StatusCode::OK,
        [(
            header::CONTENT_TYPE,
            "application/x-git-upload-pack-result".to_owned(),
        )],
        response,
    )
}
