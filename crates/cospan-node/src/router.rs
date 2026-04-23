use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

/// Maximum body size for putObject. Project schemas can be tens of MB
/// for repos with many files (one schema vertex per AST node), and the
/// default 2 MiB axum limit caused 413s on first push. 256 MiB matches
/// what the node can comfortably hold under its 1 GiB memory cap.
const PUT_OBJECT_BODY_LIMIT: usize = 256 * 1024 * 1024;

use crate::git_compat;
use crate::handlers;
use crate::state::NodeState;

pub fn build(state: Arc<NodeState>) -> Router {
    // All VCS endpoints live under dev.panproto.node.* (owned by panproto).
    // The cospan namespace is reserved for cospan-specific features (social:
    // stars, follows, issues, MRs) which are served by the appview, not the node.
    let xrpc = Router::new()
        // Core VCS operations (used by git-remote-cospan and panproto-xrpc)
        .route("/xrpc/dev.panproto.node.getObject", get(handlers::get_object))
        .route("/xrpc/dev.panproto.node.putObject", post(handlers::put_object))
        .route("/xrpc/dev.panproto.node.getRef", get(handlers::get_ref))
        .route("/xrpc/dev.panproto.node.setRef", post(handlers::set_ref))
        .route("/xrpc/dev.panproto.node.listRefs", get(handlers::list_refs))
        .route("/xrpc/dev.panproto.node.listCommits", get(handlers::list_commits))
        .route("/xrpc/dev.panproto.node.diffCommits", get(handlers::diff_commits))
        .route("/xrpc/dev.panproto.node.getHead", get(handlers::get_head))
        .route("/xrpc/dev.panproto.node.negotiate", post(handlers::negotiate))
        .route(
            "/xrpc/dev.panproto.node.getRepoInfo",
            get(handlers::get_repo_info),
        )
        // Schema-intelligence endpoints
        .route(
            "/xrpc/dev.panproto.node.getProjectSchema",
            get(handlers::get_project_schema),
        )
        .route(
            "/xrpc/dev.panproto.node.getFileSchema",
            get(handlers::get_file_schema),
        )
        .route(
            "/xrpc/dev.panproto.node.getCommitSchemaStats",
            get(handlers::get_commit_schema_stats),
        )
        .route(
            "/xrpc/dev.panproto.node.compareBranchSchemas",
            get(handlers::compare_branch_schemas),
        )
        .route(
            "/xrpc/dev.panproto.node.getDependencyGraph",
            get(handlers::get_dependency_graph),
        )
        .route(
            "/xrpc/dev.panproto.node.getImportStatus",
            get(handlers::get_import_status),
        )
        .route(
            "/xrpc/dev.panproto.node.listTree",
            get(handlers::list_tree),
        )
        .route(
            "/xrpc/dev.panproto.node.getBlob",
            get(handlers::get_blob),
        );

    let git = git_compat::git_routes();

    let health = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/ready", get(|| async { "ok" }));

    Router::new()
        .merge(xrpc)
        .merge(git)
        .merge(health)
        .layer(DefaultBodyLimit::max(PUT_OBJECT_BODY_LIMIT))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
