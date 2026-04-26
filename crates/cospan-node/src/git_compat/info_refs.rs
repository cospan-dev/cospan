//! `GET /:did/:repo/info/refs?service=<service>` handler.
//!
//! Reads refs from the persistent git mirror (see `git_compat::receive_pack`
//! for how it's written) and returns the git smart HTTP v1 ref advertisement.

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::state::NodeState;

/// Query parameters for the info/refs endpoint.
#[derive(Deserialize)]
pub struct InfoRefsParams {
    /// Must be `git-upload-pack` or `git-receive-pack`.
    pub service: String,
}

/// Format a single git pkt-line.
fn pkt_line(data: &str) -> String {
    let len = data.len() + 4;
    format!("{len:04x}{data}")
}

/// Build the full info/refs response body for a given service.
fn build_info_refs_body(
    service: &str,
    refs: &[(String, String)],
    head_ref: Option<&str>,
) -> Vec<u8> {
    let mut body = String::new();

    body.push_str(&pkt_line(&format!("# service={service}\n")));
    body.push_str("0000");

    if refs.is_empty() {
        let zero_id = "0".repeat(40);
        let capabilities = "report-status delete-refs ofs-delta";
        body.push_str(&pkt_line(&format!(
            "{zero_id} capabilities^{{}}\0{capabilities}\n"
        )));
    } else {
        // Advertise side-band-64k only for upload-pack (clone/fetch).
        // For receive-pack (push), our response is raw pkt-lines without
        // sideband framing.
        let capabilities = if service == "git-upload-pack" {
            "report-status delete-refs ofs-delta side-band-64k"
        } else {
            "report-status delete-refs ofs-delta"
        };
        let mut first = true;

        if let Some(head_name) = head_ref
            && let Some((_, oid)) = refs.iter().find(|(name, _)| name == head_name)
            && first
        {
            body.push_str(&pkt_line(&format!("{oid} HEAD\0{capabilities}\n")));
            first = false;
        }

        for (name, oid) in refs {
            if first {
                body.push_str(&pkt_line(&format!("{oid} {name}\0{capabilities}\n")));
                first = false;
            } else {
                body.push_str(&pkt_line(&format!("{oid} {name}\n")));
            }
        }
    }

    body.push_str("0000");
    body.into_bytes()
}

/// Handle `GET /:did/:repo/info/refs?service=git-upload-pack|git-receive-pack`.
///
/// For receive-pack (push), requires authentication. For upload-pack
/// (clone/fetch), access is public.
pub async fn git_info_refs(
    State(state): State<Arc<NodeState>>,
    headers: axum::http::HeaderMap,
    Path((did, repo)): Path<(String, String)>,
    Query(params): Query<InfoRefsParams>,
) -> impl IntoResponse {
    let service = &params.service;

    // Require auth for receive-pack (push) but not upload-pack (clone/fetch).
    if service == "git-receive-pack" {
        match crate::auth::push_auth::verify_push(&state, &headers, &did).await {
            crate::auth::push_auth::PushAuth::Authenticated(_) => {}
            crate::auth::push_auth::PushAuth::NoCredentials => {
                return (
                    StatusCode::UNAUTHORIZED,
                    [(
                        header::WWW_AUTHENTICATE,
                        "Basic realm=\"cospan-node\"".to_owned(),
                    )],
                    b"Authentication required for push".to_vec(),
                );
            }
            crate::auth::push_auth::PushAuth::Denied(reason) => {
                return (
                    StatusCode::FORBIDDEN,
                    [(header::CONTENT_TYPE, "text/plain".to_owned())],
                    format!("Push denied: {reason}").into_bytes(),
                );
            }
        }
    }

    if service != "git-upload-pack" && service != "git-receive-pack" {
        return (
            StatusCode::FORBIDDEN,
            [(header::CONTENT_TYPE, "text/plain".to_owned())],
            b"Invalid service parameter".to_vec(),
        );
    }

    let content_type = format!("application/x-{service}-advertisement");

    let store = state.store.lock().await;

    // For a non-existent repo:
    //  : upload-pack → 404 (nothing to clone)
    //  : receive-pack → 200 with empty ref list so the client can push
    //     and create the repo (init-on-first-push)
    if !store.has_git_mirror(&did, &repo) {
        drop(store);
        if service == "git-receive-pack" {
            let body = build_info_refs_body(service, &[], None);
            return (StatusCode::OK, [(header::CONTENT_TYPE, content_type)], body);
        }
        return (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "text/plain".to_owned())],
            format!("Repository {did}/{repo} not found").into_bytes(),
        );
    }

    let mirror = match store.open_or_init_git_mirror(&did, &repo) {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(%did, %repo, error = %e, "failed to open git mirror");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "text/plain".to_owned())],
                format!("Failed to open mirror: {e}").into_bytes(),
            );
        }
    };
    drop(store);

    // Walk the git mirror's references and collect (name, oid) pairs.
    let mut git_refs: Vec<(String, String)> = Vec::new();
    match mirror.references() {
        Ok(iter) => {
            for r in iter.flatten() {
                let Some(name) = r.name() else { continue };
                if !(name.starts_with("refs/heads/") || name.starts_with("refs/tags/")) {
                    continue;
                }
                if let Some(target) = r.target() {
                    git_refs.push((name.to_string(), target.to_string()));
                }
            }
        }
        Err(e) => {
            tracing::error!(%did, %repo, error = %e, "failed to iterate mirror refs");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "text/plain".to_owned())],
                format!("Failed to list refs: {e}").into_bytes(),
            );
        }
    }
    git_refs.sort_by(|a, b| a.0.cmp(&b.0));

    // HEAD: pick the first branch if there's one, else None.
    let head_ref = mirror
        .head()
        .ok()
        .and_then(|h| h.name().map(String::from))
        .or_else(|| git_refs.first().map(|(n, _)| n.clone()));

    let body = build_info_refs_body(service, &git_refs, head_ref.as_deref());

    (StatusCode::OK, [(header::CONTENT_TYPE, content_type)], body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkt_line_format() {
        let line = pkt_line("# service=git-upload-pack\n");
        assert!(line.starts_with("001e"));
        assert!(line.ends_with("# service=git-upload-pack\n"));
    }

    #[test]
    fn empty_repo_advertisement() {
        let body = build_info_refs_body("git-upload-pack", &[], None);
        let text = String::from_utf8(body).unwrap();
        assert!(text.starts_with("001e# service=git-upload-pack\n0000"));
        assert!(text.contains("capabilities^{}"));
        assert!(text.ends_with("0000"));
    }

    #[test]
    fn single_ref_advertisement() {
        let refs = vec![("refs/heads/main".to_owned(), "a".repeat(40))];
        let body = build_info_refs_body("git-upload-pack", &refs, None);
        let text = String::from_utf8(body).unwrap();
        assert!(text.contains(&"a".repeat(40)));
        assert!(text.contains("refs/heads/main"));
        assert!(text.contains("report-status"));
    }
}
