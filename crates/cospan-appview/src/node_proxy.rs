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

/// Proxy an arbitrary node XRPC GET request and return the raw JSON.
///
/// Used for endpoints like `listCommits` / `diffCommits` that aren't
/// modeled in panproto-xrpc's typed NodeClient yet. These belong in
/// panproto upstream — tracked at <https://github.com/panproto/panproto/issues/25>.
/// Once the typed client lands, migrate the call sites back to it.
///
/// The `did`/`repo` query parameters are injected automatically so the
/// frontend doesn't have to know them.
pub async fn proxy_get_json(
    state: &Arc<AppState>,
    repo_did: &str,
    repo_name: &str,
    endpoint: &str,
    extra_params: &[(&str, String)],
) -> Result<serde_json::Value, String> {
    let row: Option<(String,)> = sqlx::query_as::<_, (String,)>(
        "SELECT node_url FROM repos WHERE did = $1 AND name = $2",
    )
    .bind(repo_did)
    .bind(repo_name)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| format!("database error: {e}"))?;

    let (node_url,) =
        row.ok_or_else(|| format!("repo {repo_did}/{repo_name} not found"))?;
    if node_url.is_empty() {
        return Err("repo has no node URL configured".into());
    }

    let base = node_url.trim_end_matches('/');
    let url = format!("{base}/xrpc/{endpoint}");

    let mut req = state.http_client.get(&url);
    req = req.query(&[("did", repo_did), ("repo", repo_name)]);
    if !extra_params.is_empty() {
        req = req.query(extra_params);
    }

    let resp = req
        .send()
        .await
        .map_err(|e| format!("proxy request failed: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("node returned {status}: {body}"));
    }

    resp.json::<serde_json::Value>()
        .await
        .map_err(|e| format!("proxy response parse error: {e}"))
}
