//! Git smart HTTP protocol compatibility layer.
//!
//! Implements the three endpoints required for git's HTTP smart protocol:
//!
//! - `GET  /:did/:repo/info/refs`        — ref advertisement (`git ls-remote`)
//! - `POST /:did/:repo/git-upload-pack`  — packfile serving (`git clone`/`git fetch`)
//! - `POST /:did/:repo/git-receive-pack` — packfile receiving (`git push`)
//!
//! `info/refs` reads real refs from the panproto-vcs store.
//! `upload-pack` and `receive-pack` bridge via panproto-git and git2.

mod info_refs;
mod receive_pack;
mod upload_pack;

use std::sync::Arc;

use axum::Router;
use axum::routing::{get, post};

use crate::state::NodeState;

pub use info_refs::git_info_refs;
pub use receive_pack::git_receive_pack;
pub use upload_pack::git_upload_pack;

/// Build the git smart HTTP protocol routes.
///
/// These are mounted alongside the XRPC routes in the main router.
/// Path parameters `:did` and `:repo` are extracted by Axum.
pub fn git_routes() -> Router<Arc<NodeState>> {
    Router::new()
        .route("/{did}/{repo}/info/refs", get(git_info_refs))
        .route("/{did}/{repo}/git-upload-pack", post(git_upload_pack))
        .route("/{did}/{repo}/git-receive-pack", post(git_receive_pack))
}
