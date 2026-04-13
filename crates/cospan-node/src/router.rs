use axum::Router;
use axum::routing::{get, post};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::git_compat;
use crate::handlers;
use crate::state::NodeState;

pub fn build(state: Arc<NodeState>) -> Router {
    let xrpc = Router::new()
        .route("/xrpc/dev.cospan.node.getObject", get(handlers::get_object))
        .route(
            "/xrpc/dev.cospan.node.putObject",
            post(handlers::put_object),
        )
        .route("/xrpc/dev.cospan.node.getRef", get(handlers::get_ref))
        .route("/xrpc/dev.cospan.node.setRef", post(handlers::set_ref))
        .route("/xrpc/dev.cospan.node.listRefs", get(handlers::list_refs))
        .route(
            "/xrpc/dev.cospan.node.listCommits",
            get(handlers::list_commits),
        )
        .route(
            "/xrpc/dev.cospan.node.diffCommits",
            get(handlers::diff_commits),
        )
        .route("/xrpc/dev.cospan.node.getHead", get(handlers::get_head))
        .route("/xrpc/dev.cospan.node.negotiate", post(handlers::negotiate))
        .route(
            "/xrpc/dev.cospan.node.getRepoInfo",
            get(handlers::get_repo_info),
        )
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
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
