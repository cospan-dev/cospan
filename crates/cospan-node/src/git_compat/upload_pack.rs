//! `POST /:did/:repo/git-upload-pack` handler.
//!
//! Handles `git clone` and `git fetch` by reading want/have negotiation
//! lines from the client, building a packfile from the persistent git
//! mirror (see `git_compat::receive_pack`), and sending it back wrapped
//! in the sideband protocol if the client requested it.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::IntoResponse;

use crate::state::NodeState;

/// Negotiated client capabilities from upload-pack.
#[derive(Default, Debug)]
struct ClientCaps {
    side_band: bool,
    side_band_64k: bool,
}

/// Parse pkt-line formatted want/have negotiation from the upload-pack body.
fn parse_wants_haves(body: &[u8]) -> (Vec<String>, Vec<String>, ClientCaps) {
    let mut wants = Vec::new();
    let mut haves = Vec::new();
    let mut caps = ClientCaps::default();
    let mut pos = 0;
    let mut first_want_parsed = false;

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

        if pkt_len == 0 || pkt_len == 1 || pkt_len == 2 {
            pos += 4;
            if pkt_len == 0 {
                continue;
            } else {
                continue;
            }
        }

        if pos + pkt_len > body.len() {
            break;
        }

        let line = &body[pos + 4..pos + pkt_len];
        pos += pkt_len;

        if let Ok(line_str) = std::str::from_utf8(line) {
            let line_str = line_str.trim_end_matches('\n');

            // upload-pack first-want line format (git wire protocol v1):
            //     "want <oid> <cap1> <cap2> ... <capN>"
            // Capabilities are space-separated, NOT NUL-separated (that's
            // the receive-pack convention). Subsequent wants don't carry
            // capabilities.
            if let Some(rest) = line_str.strip_prefix("want ") {
                let mut parts = rest.split_whitespace();
                if let Some(oid) = parts.next() {
                    wants.push(oid.to_string());
                    if !first_want_parsed {
                        first_want_parsed = true;
                        for cap in parts {
                            match cap {
                                "side-band" => caps.side_band = true,
                                "side-band-64k" => caps.side_band_64k = true,
                                _ => {}
                            }
                        }
                    }
                }
            } else if let Some(rest) = line_str.strip_prefix("have ") {
                if let Some(oid) = rest.split_whitespace().next() {
                    haves.push(oid.to_string());
                }
            } else if line_str == "done" {
                break;
            }
        }
    }

    (wants, haves, caps)
}

/// Build a pkt-line byte vector.
fn pkt_line(data: &str) -> Vec<u8> {
    let len = data.len() + 4;
    format!("{len:04x}{data}").into_bytes()
}

/// Handle `POST /:did/:repo/git-upload-pack`.
pub async fn git_upload_pack(
    State(state): State<Arc<NodeState>>,
    Path((did, repo)): Path<(String, String)>,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    tracing::info!(%did, %repo, body_len = body.len(), "git-upload-pack requested");
    tracing::info!(body_hex = ?String::from_utf8_lossy(&body).chars().take(500).collect::<String>(), "upload-pack raw body");

    let (wants, haves, caps) = parse_wants_haves(&body);
    tracing::info!(?wants, ?haves, ?caps, "parsed wants/haves");

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

    // Open the persistent git mirror (single source of truth for git
    // smart HTTP). The panproto-vcs store is not consulted here.
    let store_guard = state.store.lock().await;
    if !store_guard.has_git_mirror(&did, &repo) {
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
    let mirror = match store_guard.open_or_init_git_mirror(&did, &repo) {
        Ok(m) => m,
        Err(e) => {
            let mut response = Vec::new();
            response.extend_from_slice(&pkt_line(&format!("ERR open mirror: {e}\n")));
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
    drop(store_guard);

    // Set up a revwalk that walks the wanted commits from the mirror
    // and excludes whatever the client already has. Then feed it into
    // the packbuilder so it grabs every reachable object.
    let mut packbuilder = match mirror.packbuilder() {
        Ok(pb) => pb,
        Err(e) => {
            let mut response = Vec::new();
            response.extend_from_slice(&pkt_line(&format!("ERR packbuilder: {e}\n")));
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

    let mut revwalk = match mirror.revwalk() {
        Ok(rw) => rw,
        Err(e) => {
            let mut response = Vec::new();
            response.extend_from_slice(&pkt_line(&format!("ERR revwalk: {e}\n")));
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
    let _ = revwalk.set_sorting(git2::Sort::TIME | git2::Sort::TOPOLOGICAL);

    let mut found_any = false;
    for want in &wants {
        if let Ok(oid) = git2::Oid::from_str(want)
            && mirror.find_commit(oid).is_ok()
        {
            let _ = revwalk.push(oid);
            found_any = true;
        }
    }
    for have in &haves {
        if let Ok(oid) = git2::Oid::from_str(have)
            && mirror.find_commit(oid).is_ok()
        {
            let _ = revwalk.hide(oid);
        }
    }

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

    if let Err(e) = packbuilder.insert_walk(&mut revwalk) {
        let mut response = Vec::new();
        response.extend_from_slice(&pkt_line(&format!("ERR insert_walk: {e}\n")));
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

    // Build the packfile into a buffer.
    let mut pack_data = Vec::new();
    if let Err(e) = packbuilder.foreach(|data| {
        pack_data.extend_from_slice(data);
        true
    }) {
        let mut response = Vec::new();
        response.extend_from_slice(&pkt_line(&format!("ERR pack build: {e}\n")));
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

    tracing::info!(
        object_count = packbuilder.object_count(),
        pack_bytes = pack_data.len(),
        side_band = caps.side_band,
        side_band_64k = caps.side_band_64k,
        "upload-pack built"
    );

    // Assemble the response. Always start with NAK. If the client
    // supports sideband, frame the pack in band 1 pkt-lines; otherwise
    // append the pack bytes directly after NAK.
    let mut response = Vec::new();
    response.extend_from_slice(&pkt_line("NAK\n"));

    if caps.side_band_64k || caps.side_band {
        let chunk_size: usize = if caps.side_band_64k { 65515 } else { 995 };
        for chunk in pack_data.chunks(chunk_size) {
            let pkt_len = chunk.len() + 5;
            let len_str = format!("{pkt_len:04x}");
            response.extend_from_slice(len_str.as_bytes());
            response.push(1); // sideband band 1 = pack data
            response.extend_from_slice(chunk);
        }
        response.extend_from_slice(b"0000");
    } else {
        response.extend_from_slice(&pack_data);
    }

    (
        StatusCode::OK,
        [(
            header::CONTENT_TYPE,
            "application/x-git-upload-pack-result".to_owned(),
        )],
        response,
    )
}
