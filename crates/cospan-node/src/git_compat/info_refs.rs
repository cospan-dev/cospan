//! `GET /:did/:repo/info/refs?service=<service>` handler.
//!
//! Returns the ref advertisement in git smart HTTP format, enabling
//! `git ls-remote`, `git clone`, and `git fetch` to discover refs.

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
///
/// A pkt-line starts with a 4-hex-digit length (including the 4 length bytes
/// themselves), followed by the payload. A length of `0000` is a flush packet.
fn pkt_line(data: &str) -> String {
    let len = data.len() + 4; // 4 bytes for the length prefix itself
    format!("{len:04x}{data}")
}

/// Build the full info/refs response body for a given service.
///
/// Format (git smart HTTP v1):
/// ```text
/// <pkt-line: "# service=git-upload-pack\n">
/// 0000
/// <pkt-line: "<sha> <refname>\n" for each ref, first ref gets capabilities>
/// 0000
/// ```
fn build_info_refs_body(
    service: &str,
    refs: &[(String, String)],
    head_ref: Option<&str>,
) -> Vec<u8> {
    let mut body = String::new();

    // Service announcement
    body.push_str(&pkt_line(&format!("# service={service}\n")));
    body.push_str("0000");

    if refs.is_empty() {
        // Empty repo: advertise capabilities on a zero-id line.
        // git requires at least one ref line with capabilities.
        let zero_id = "0".repeat(40);
        let capabilities = "report-status delete-refs ofs-delta";
        body.push_str(&pkt_line(&format!(
            "{zero_id} capabilities^{{}}\0{capabilities}\n"
        )));
    } else {
        let capabilities = "report-status delete-refs ofs-delta";
        let mut first = true;

        // If we know which ref is HEAD, emit a synthetic HEAD line first.
        if let Some(head_name) = head_ref
            && let Some((_, oid)) = refs.iter().find(|(name, _)| name == head_name)
            && first
        {
            body.push_str(&pkt_line(&format!("{oid} HEAD\0{capabilities}\n")));
            first = false;
        }

        // Emit each real ref.
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
/// Reads real refs from the panproto-vcs store and returns them in the git
/// smart HTTP advertisement format.
pub async fn git_info_refs(
    State(state): State<Arc<NodeState>>,
    Path((did, repo)): Path<(String, String)>,
    Query(params): Query<InfoRefsParams>,
) -> impl IntoResponse {
    let service = &params.service;

    // Validate the requested service.
    if service != "git-upload-pack" && service != "git-receive-pack" {
        return (
            StatusCode::FORBIDDEN,
            [(header::CONTENT_TYPE, "text/plain".to_owned())],
            b"Invalid service parameter".to_vec(),
        );
    }

    let content_type = format!("application/x-{service}-advertisement");

    // Read refs from the store. We hold the lock briefly.
    let store = state.store.lock().await;

    // Check if the repo exists.
    if !store.exists(&did, &repo) {
        return (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "text/plain".to_owned())],
            format!("Repository {did}/{repo} not found").into_bytes(),
        );
    }

    let refs_result = store.list_refs(&did, &repo);
    let head_result = store.get_head(&did, &repo);

    // Release the lock before building the response.
    drop(store);

    let refs = match refs_result {
        Ok(refs) => refs,
        Err(e) => {
            tracing::error!(%did, %repo, error = %e, "failed to list refs");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "text/plain".to_owned())],
                format!("Failed to read refs: {e}").into_bytes(),
            );
        }
    };

    // panproto-vcs ObjectId is 32 bytes (blake3), displayed as 64 hex chars.
    // git expects 40 hex chars (SHA-1). For Phase 0, we truncate to 40 chars
    // to satisfy git clients. This is a lossy mapping but sufficient for
    // ref advertisement / ls-remote compatibility.
    let git_refs: Vec<(String, String)> = refs
        .into_iter()
        .map(|(name, oid)| {
            let hex = oid.to_string();
            // Truncate blake3 (64 hex) to SHA-1 length (40 hex) for git compat.
            let git_hex = if hex.len() > 40 {
                hex[..40].to_owned()
            } else {
                hex
            };
            (name, git_hex)
        })
        .collect();

    // Determine HEAD ref name for the synthetic HEAD line.
    let head_ref = match head_result {
        Ok(panproto_core::vcs::HeadState::Branch(name)) => Some(format!("refs/heads/{name}")),
        _ => None,
    };

    let body = build_info_refs_body(service, &git_refs, head_ref.as_deref());

    (StatusCode::OK, [(header::CONTENT_TYPE, content_type)], body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkt_line_format() {
        let line = pkt_line("# service=git-upload-pack\n");
        // "# service=git-upload-pack\n" is 26 bytes, plus 4 = 30 = 0x1e
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
        // Should contain the ref with capabilities on the first line
        assert!(text.contains(&"a".repeat(40)));
        assert!(text.contains("refs/heads/main"));
        assert!(text.contains("report-status"));
    }

    #[test]
    fn head_ref_appears_first() {
        let refs = vec![
            ("refs/heads/dev".to_owned(), "b".repeat(40)),
            ("refs/heads/main".to_owned(), "a".repeat(40)),
        ];
        let body = build_info_refs_body("git-upload-pack", &refs, Some("refs/heads/main"));
        let text = String::from_utf8(body).unwrap();
        // HEAD line should appear before refs/heads/dev
        let head_pos = text.find("HEAD").unwrap();
        let dev_pos = text.find("refs/heads/dev").unwrap();
        assert!(head_pos < dev_pos);
    }
}
