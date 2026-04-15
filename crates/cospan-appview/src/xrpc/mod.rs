mod actor_get_profile;
mod collaborator_add;
mod collaborator_list;
mod dependency_list;
mod follow_list;
mod follow_toggle;
mod issue_comment_create;
mod issue_comment_list;
mod issue_create;
mod issue_get;
mod issue_list;
mod issue_state_change;
mod issue_timeline;
mod label_create;
mod node_list;
pub mod node_proxy;
mod org_get;
mod org_list;
mod org_member_list;
mod pipeline_get;
mod pipeline_list;
mod profile_put;
mod pull_comment_list;
mod pull_get;
mod pull_list;
mod reaction_list;
mod ref_update_list;
mod push_token;
mod repo_create;
mod repo_delete;
mod repo_fork;
mod repo_get;
mod repo_import;
mod repo_list;
mod search_repos;
mod search_structural;
mod star_list;
mod star_toggle;

mod feed_breaking;
mod feed_timeline;
pub mod sse;

use std::sync::Arc;

use axum::Router;
use axum::routing::{get, post};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::auth::oauth;
use crate::state::AppState;

pub fn router(state: Arc<AppState>) -> Router {
    let xrpc = Router::new()
        // Repo
        .route("/xrpc/dev.cospan.repo.get", get(repo_get::handler))
        .route("/xrpc/dev.cospan.repo.list", get(repo_list::handler))
        // Actor
        .route(
            "/xrpc/dev.cospan.actor.getProfile",
            get(actor_get_profile::handler),
        )
        // VCS
        .route(
            "/xrpc/dev.cospan.vcs.refUpdate.list",
            get(ref_update_list::handler),
        )
        // Node
        .route("/xrpc/dev.cospan.node.list", get(node_list::handler))
        // Issues
        .route("/xrpc/dev.cospan.repo.issue.get", get(issue_get::handler))
        .route("/xrpc/dev.cospan.repo.issue.list", get(issue_list::handler))
        .route(
            "/xrpc/dev.cospan.repo.issue.comment.list",
            get(issue_comment_list::handler),
        )
        .route(
            "/xrpc/dev.cospan.repo.issue.getTimeline",
            get(issue_timeline::handler),
        )
        // Pulls
        .route("/xrpc/dev.cospan.repo.pull.get", get(pull_get::handler))
        .route("/xrpc/dev.cospan.repo.pull.list", get(pull_list::handler))
        .route(
            "/xrpc/dev.cospan.repo.pull.comment.list",
            get(pull_comment_list::handler),
        )
        // Social
        .route("/xrpc/dev.cospan.feed.star.list", get(star_list::handler))
        .route(
            "/xrpc/dev.cospan.graph.follow.list",
            get(follow_list::handler),
        )
        .route(
            "/xrpc/dev.cospan.feed.reaction.list",
            get(reaction_list::handler),
        )
        // Orgs
        .route("/xrpc/dev.cospan.org.get", get(org_get::handler))
        .route("/xrpc/dev.cospan.org.list", get(org_list::handler))
        .route(
            "/xrpc/dev.cospan.org.member.list",
            get(org_member_list::handler),
        )
        // Pipelines
        .route("/xrpc/dev.cospan.pipeline.get", get(pipeline_get::handler))
        .route(
            "/xrpc/dev.cospan.pipeline.list",
            get(pipeline_list::handler),
        )
        // Dependencies
        .route(
            "/xrpc/dev.cospan.repo.dependency.list",
            get(dependency_list::handler),
        )
        // Collaborators
        .route(
            "/xrpc/dev.cospan.repo.collaborator.list",
            get(collaborator_list::handler),
        )
        // Search
        .route("/xrpc/dev.cospan.search.repos", get(search_repos::handler))
        .route(
            "/xrpc/dev.cospan.search.structural",
            get(search_structural::handler),
        )
        // SSE
        .route(
            "/xrpc/dev.cospan.sync.subscribeEvents",
            get(sse::subscribe_events),
        )
        // Feed generators
        .route(
            "/xrpc/dev.cospan.feed.getBreakingChanges",
            get(feed_breaking::handler),
        )
        .route(
            "/xrpc/dev.cospan.feed.getTimeline",
            get(feed_timeline::handler),
        )
        // Node proxy (fetches from cospan nodes via panproto-xrpc)
        .route(
            "/xrpc/dev.panproto.node.proxy.listRefs",
            get(node_proxy::proxy_list_refs),
        )
        .route(
            "/xrpc/dev.panproto.node.proxy.getHead",
            get(node_proxy::proxy_get_head),
        )
        .route(
            "/xrpc/dev.panproto.node.proxy.getObject",
            get(node_proxy::proxy_get_object),
        )
        .route(
            "/xrpc/dev.panproto.node.proxy.listCommits",
            get(node_proxy::proxy_list_commits),
        )
        .route(
            "/xrpc/dev.panproto.node.proxy.diffCommits",
            get(node_proxy::proxy_diff_commits),
        )
        .route(
            "/xrpc/dev.panproto.node.proxy.getProjectSchema",
            get(node_proxy::proxy_get_project_schema),
        )
        .route(
            "/xrpc/dev.panproto.node.proxy.getCommitSchemaStats",
            get(node_proxy::proxy_get_commit_schema_stats),
        )
        .route(
            "/xrpc/dev.panproto.node.proxy.getFileSchema",
            get(node_proxy::proxy_get_file_schema),
        )
        .route(
            "/xrpc/dev.panproto.node.proxy.compareBranchSchemas",
            get(node_proxy::proxy_compare_branch_schemas),
        )
        .route(
            "/xrpc/dev.panproto.node.proxy.getDependencyGraph",
            get(node_proxy::proxy_get_dependency_graph),
        )
        .route(
            "/xrpc/dev.panproto.node.proxy.listTree",
            get(node_proxy::proxy_list_tree),
        )
        .route(
            "/xrpc/dev.panproto.node.proxy.getBlob",
            get(node_proxy::proxy_get_blob),
        )
        .route(
            "/xrpc/dev.panproto.node.proxy.getImportStatus",
            get(node_proxy::proxy_get_import_status),
        )
        // --- Procedures (POST) ---
        .route(
            "/xrpc/dev.cospan.feed.star.toggle",
            post(star_toggle::handler),
        )
        .route(
            "/xrpc/dev.cospan.graph.follow.toggle",
            post(follow_toggle::handler),
        )
        .route(
            "/xrpc/dev.cospan.repo.issue.create",
            post(issue_create::handler),
        )
        .route(
            "/xrpc/dev.cospan.repo.issue.comment.create",
            post(issue_comment_create::handler),
        )
        .route(
            "/xrpc/dev.cospan.repo.issue.state.change",
            post(issue_state_change::handler),
        )
        .route("/xrpc/dev.cospan.repo.create", post(repo_create::handler))
        .route("/xrpc/dev.cospan.repo.fork", post(repo_fork::handler))
        .route("/xrpc/dev.cospan.repo.delete", post(repo_delete::handler))
        .route("/xrpc/dev.cospan.repo.import", post(repo_import::handler))
        .route(
            "/xrpc/dev.cospan.repo.createPushToken",
            post(push_token::handler),
        )
        .route(
            "/xrpc/dev.cospan.actor.profile.put",
            post(profile_put::handler),
        )
        .route(
            "/xrpc/dev.cospan.label.definition.create",
            post(label_create::handler),
        )
        .route(
            "/xrpc/dev.cospan.repo.collaborator.add",
            post(collaborator_add::handler),
        );

    let health = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/ready", get(|| async { "ok" }));

    Router::new()
        .merge(xrpc)
        .merge(health)
        .merge(oauth::router())
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
