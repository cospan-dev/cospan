//! `POST /:did/:repo/git-receive-pack` handler.
//!
//! Handles `git push` by receiving packfile data from the git client,
//! writing it into a persistent bare git mirror via libgit2's packwriter,
//! updating refs in the mirror, and THEN importing the same objects into
//! the panproto-vcs store so schema-aware operations continue to work.
//!
//! Keeping the git mirror is essential: subsequent upload-pack / info-refs
//! calls can serve the original git objects directly without having to
//! re-export panproto-vcs objects (that export is not deterministic, so
//! two calls would advertise different OIDs and break `git clone`).
//!
//! Protocol sketch:
//!
//! 1. Client sends reference update commands (old-oid new-oid refname).
//! 2. Client sends a packfile containing the objects.
//! 3. We index the pack into the persistent mirror, update mirror refs,
//!    and import the corresponding commits into panproto-vcs.

use std::io::Write;
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

/// Build a pkt-line formatted response chunk.
fn pkt_line(data: &str) -> String {
    let len = data.len() + 4;
    format!("{len:04x}{data}")
}

fn err_response(msg: String) -> impl IntoResponse {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        [(
            header::CONTENT_TYPE,
            "application/x-git-receive-pack-result",
        )],
        pkt_line(&format!("ERR {msg}\n")) + "0000",
    )
}

/// Handle `POST /:did/:repo/git-receive-pack`.
///
/// Requires authentication: the pusher must provide HTTP Basic Auth
/// with username=DID, password=push-token (a JWT from the appview's
/// `dev.cospan.repo.createPushToken` endpoint).
pub async fn git_receive_pack(
    State(state): State<Arc<NodeState>>,
    headers: axum::http::HeaderMap,
    Path((did, repo)): Path<(String, String)>,
    body: axum::body::Bytes,
) -> axum::response::Response {
    tracing::info!(%did, %repo, body_len = body.len(), "git-receive-pack requested");

    // 0. Authenticate the pusher.
    match crate::auth::push_auth::verify_push(&headers, &did) {
        crate::auth::push_auth::PushAuth::Authenticated(authed_did) => {
            tracing::info!(%authed_did, "push authenticated");
        }
        crate::auth::push_auth::PushAuth::NoCredentials => {
            return (
                StatusCode::UNAUTHORIZED,
                [(header::WWW_AUTHENTICATE, "Basic realm=\"cospan-node\"")],
                [(header::CONTENT_TYPE, "application/x-git-receive-pack-result")],
                pkt_line("ERR authentication required\n") + "0000",
            )
                .into_response();
        }
        crate::auth::push_auth::PushAuth::Denied(reason) => {
            tracing::warn!(%did, %reason, "push denied");
            return (
                StatusCode::FORBIDDEN,
                [(header::CONTENT_TYPE, "application/x-git-receive-pack-result")],
                pkt_line(&format!("ERR {reason}\n")) + "0000",
            )
                .into_response();
        }
    }

    // 1. Parse reference update commands and extract packfile data.
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
        )
            .into_response();
    }

    // 2. Open (or initialise) the persistent bare git mirror for this repo.
    let store_guard = state.store.lock().await;
    let git_mirror = match store_guard.open_or_init_git_mirror(&did, &repo) {
        Ok(r) => r,
        Err(e) => {
            return err_response(format!("open git mirror: {e}")).into_response();
        }
    };

    // 3. Index the packfile into the mirror via libgit2's packwriter.
    if !packfile_data.is_empty() {
        let odb = match git_mirror.odb() {
            Ok(o) => o,
            Err(e) => return err_response(format!("open ODB: {e}")).into_response(),
        };
        let mut writer = match odb.packwriter() {
            Ok(w) => w,
            Err(e) => return err_response(format!("open packwriter: {e}")).into_response(),
        };
        if let Err(e) = writer.write_all(packfile_data) {
            return err_response(format!("write pack: {e}")).into_response();
        }
        if let Err(e) = writer.commit() {
            return err_response(format!("commit pack: {e}")).into_response();
        }
    }

    // 4. Apply ref updates on the git mirror (fast: just updating ref pointers).
    let mut response = String::new();
    let mut import_tasks: Vec<(String, String)> = Vec::new(); // (new_oid, refname) for async import

    for (_old_oid, new_oid, refname) in &ref_updates {
        let zero_oid = "0".repeat(40);

        if new_oid == &zero_oid {
            match git_mirror.find_reference(refname) {
                Ok(mut r) => { let _ = r.delete(); }
                Err(_) => {}
            }
            response.push_str(&pkt_line(&format!("ok {refname}\n")));
            continue;
        }

        let git_oid = match git2::Oid::from_str(new_oid) {
            Ok(o) => o,
            Err(e) => {
                response.push_str(&pkt_line(&format!("ng {refname} bad oid: {e}\n")));
                continue;
            }
        };

        if let Err(e) = git_mirror.reference(refname, git_oid, true, "receive-pack") {
            response.push_str(&pkt_line(&format!("ng {refname} mirror ref: {e}\n")));
            continue;
        }

        import_tasks.push((new_oid.clone(), refname.clone()));
        response.push_str(&pkt_line(&format!("ok {refname}\n")));
    }

    drop(store_guard);

    // 5. Import into panproto-vcs asynchronously. The push response is
    //    sent immediately; the import runs in the background. This is
    //    necessary because import_git_repo walks the entire commit
    //    history (tracked upstream at panproto/panproto#26) and can
    //    take minutes for repos with 100+ commits.
    if !import_tasks.is_empty() {
        let store_clone = state.store.clone();
        let did_clone = did.clone();
        let repo_clone = repo.clone();
        tokio::task::spawn_blocking(move || {
            // Open the stores under the lock, then DROP the lock before
            // the expensive import. FsStore is file-backed so concurrent
            // reads (listCommits, diffCommits) work fine while we write.
            let (mirror, mut vcs_store) = {
                let store_guard = store_clone.blocking_lock();
                let mirror = match store_guard.open_or_init_git_mirror(&did_clone, &repo_clone) {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::error!(error = %e, "background import: open mirror failed");
                        return;
                    }
                };
                let vcs_store = match store_guard.open_or_init(&did_clone, &repo_clone) {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::error!(error = %e, "background import: open vcs store failed");
                        return;
                    }
                };
                (mirror, vcs_store)
                // store_guard dropped here: lock released
            };
            for (new_oid, refname) in &import_tasks {
                match panproto_git::import_git_repo(&mirror, &mut vcs_store, new_oid) {
                    Ok(result) => {
                        let _ = panproto_vcs::Store::set_ref(&mut vcs_store, refname, result.head_id);
                        tracing::info!(
                            did = %did_clone, repo = %repo_clone, %refname,
                            commits = result.commit_count,
                            "background: imported git commits into panproto-vcs"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            did = %did_clone, repo = %repo_clone, %refname, error = %e,
                            "background: panproto-vcs import failed"
                        );
                    }
                }
            }
        });
    }

    let full_response = format!("{}{}0000", pkt_line("unpack ok\n"), response);

    (
        StatusCode::OK,
        [(
            header::CONTENT_TYPE,
            "application/x-git-receive-pack-result",
        )],
        full_response,
    )
        .into_response()
}
