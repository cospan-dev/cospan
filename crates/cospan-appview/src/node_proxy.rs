//! Node proxy: fetches objects, schemas, and refs from cospan nodes
//! using panproto-xrpc's NodeClient.

use std::sync::Arc;

use panproto_core::vcs::{HeadState, Object, ObjectId};
use panproto_xrpc::NodeClient;

use crate::state::AppState;

/// Create a NodeClient for a repo by looking up its node URL from the database.
pub async fn client_for_repo(
    state: &Arc<AppState>,
    repo_did: &str,
    repo_name: &str,
) -> Result<NodeClient, String> {
    let row: Option<(String, String)> = sqlx::query_as::<_, (String, String)>(
        "SELECT node_url, node_did FROM repos WHERE did = $1 AND name = $2",
    )
    .bind(repo_did)
    .bind(repo_name)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| format!("database error: {e}"))?;

    let (node_url, _node_did) =
        row.ok_or_else(|| format!("repo {repo_did}/{repo_name} not found"))?;

    if node_url.is_empty() {
        return Err("repo has no node URL configured".into());
    }

    Ok(NodeClient::new(&node_url, repo_did, repo_name))
}

/// Fetch an object from a node.
pub async fn fetch_object(
    state: &Arc<AppState>,
    repo_did: &str,
    repo_name: &str,
    object_id: &str,
) -> Result<Object, String> {
    let client = client_for_repo(state, repo_did, repo_name).await?;
    let id: ObjectId = object_id
        .parse()
        .map_err(|_| format!("invalid object ID: {object_id}"))?;
    client
        .get_object(&id)
        .await
        .map_err(|e| format!("node fetch error: {e}"))
}

/// List refs from a node.
pub async fn list_refs(
    state: &Arc<AppState>,
    repo_did: &str,
    repo_name: &str,
) -> Result<Vec<(String, ObjectId)>, String> {
    let client = client_for_repo(state, repo_did, repo_name).await?;
    client
        .list_refs()
        .await
        .map_err(|e| format!("node list_refs error: {e}"))
}

/// Get HEAD state from a node.
pub async fn get_head(
    state: &Arc<AppState>,
    repo_did: &str,
    repo_name: &str,
) -> Result<HeadState, String> {
    let client = client_for_repo(state, repo_did, repo_name).await?;
    client
        .get_head()
        .await
        .map_err(|e| format!("node get_head error: {e}"))
}
