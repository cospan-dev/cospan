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
        .route("/xrpc/dev.cospan.node.getHead", get(handlers::get_head))
        .route("/xrpc/dev.cospan.node.negotiate", post(handlers::negotiate))
        .route(
            "/xrpc/dev.cospan.node.getRepoInfo",
            get(handlers::get_repo_info),
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
