//! Structural search: query schema graphs using panproto expressions.
//!
//! This is Cospan's differentiating search capability — users can search
//! for schema elements (vertices, edges, constraints) across all repos
//! using panproto's expression language.
//!
//! Example queries:
//!   "find all functions named 'validate'"
//!   "find all structs with a field of type 'email'"
//!   "find all schemas where breaking change count > 0"

use std::collections::HashMap;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct Params {
    /// panproto expression language query string
    pub q: String,
    /// Schema vertex kind to anchor the search (e.g., "function", "struct", "field")
    pub anchor: Option<String>,
    /// Limit results
    pub limit: Option<usize>,
}

pub async fn handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<Params>,
) -> Result<Json<serde_json::Value>, AppError> {
    let limit = params.limit.unwrap_or(25).min(100);
    let anchor = params.anchor.as_deref().unwrap_or("function");

    // Parse the query expression using panproto's expression parser
    let tokens = panproto_expr_parser::tokenize(&params.q)
        .map_err(|errs| AppError::NotFound(format!("query parse error: {errs:?}")))?;

    let predicate = panproto_expr_parser::parse(&tokens)
        .map_err(|errs| AppError::NotFound(format!("query parse error: {errs:?}")))?;

    // Build the InstanceQuery
    let query = panproto_inst::query::InstanceQuery {
        anchor: panproto_gat::Name::from(anchor),
        predicate: Some(predicate.clone()),
        group_by: None,
        project: None,
        limit: Some(limit),
        path: vec![],
    };

    let query_repr = panproto_expr_parser::pretty_print(&predicate);

    // Load schema instances from repos that have ref_updates in the database.
    // For each repo with ref updates, we build a WInstance from the ref_update
    // metadata and execute the query against it.
    //
    // Production will query a schema index service or cache schemas at startup.
    // For now, we construct instances on-demand from ref_update records.
    let ref_updates = sqlx::query_as::<_, crate::db::ref_update::RefUpdateRow>(
        "SELECT id, repo_did, repo_name, rkey, committer_did, ref_name, \
              old_target, new_target, protocol, migration_id, breaking_change_count, \
              lens_id, lens_quality, commit_count, created_at, indexed_at \
         FROM ref_updates \
         ORDER BY created_at DESC LIMIT $1",
    )
    .bind(500i64)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::Database)?;

    // Build a default schema for ref_update metadata querying.
    // This schema models the ref_update records as a queryable graph.
    let schema = build_ref_update_schema();

    let mut all_results = Vec::new();

    // Group ref_updates by (repo_did, repo_name) to build per-repo instances
    let mut repos: HashMap<(String, String), Vec<&crate::db::ref_update::RefUpdateRow>> =
        HashMap::new();
    for ru in &ref_updates {
        repos
            .entry((ru.repo_did.clone(), ru.repo_name.clone()))
            .or_default()
            .push(ru);
    }

    for ((repo_did, repo_name), updates) in &repos {
        let instance = build_instance_from_ref_updates(updates, anchor);
        let matches = panproto_inst::query::execute(&query, &instance, &schema);

        for m in matches {
            let mut fields_json = serde_json::Map::new();
            for (k, v) in &m.fields {
                fields_json.insert(k.clone(), value_to_json(v));
            }
            fields_json.insert(
                "_repo_did".into(),
                serde_json::Value::String(repo_did.clone()),
            );
            fields_json.insert(
                "_repo_name".into(),
                serde_json::Value::String(repo_name.clone()),
            );

            all_results.push(serde_json::Value::Object(fields_json));

            if all_results.len() >= limit {
                break;
            }
        }

        if all_results.len() >= limit {
            break;
        }
    }

    Ok(Json(serde_json::json!({
        "anchor": anchor,
        "expression": query_repr,
        "limit": limit,
        "results": all_results,
        "total": all_results.len(),
    })))
}

/// Convert a panproto Value to a serde_json Value.
fn value_to_json(v: &panproto_inst::value::Value) -> serde_json::Value {
    use panproto_inst::value::Value;
    match v {
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Int(i) => serde_json::json!(i),
        Value::Float(f) => serde_json::json!(f),
        Value::Str(s) => serde_json::Value::String(s.clone()),
        Value::Bytes(b) => serde_json::Value::String(base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            b,
        )),
        Value::CidLink(c) => serde_json::Value::String(c.clone()),
        Value::Null => serde_json::Value::Null,
        Value::Token(t) => serde_json::Value::String(t.clone()),
        Value::Blob { ref_, mime, size } => serde_json::json!({
            "ref": ref_,
            "mimeType": mime,
            "size": size,
        }),
        Value::Opaque { type_, fields } => {
            let mut m = serde_json::Map::new();
            m.insert("$type".into(), serde_json::Value::String(type_.clone()));
            for (k, v) in fields {
                m.insert(k.clone(), value_to_json(v));
            }
            serde_json::Value::Object(m)
        }
        Value::Unknown(map) => {
            let m: serde_json::Map<String, serde_json::Value> = map
                .iter()
                .map(|(k, v)| (k.clone(), value_to_json(v)))
                .collect();
            serde_json::Value::Object(m)
        }
    }
}

/// Build a minimal panproto schema describing ref_update metadata as a queryable graph.
///
/// Vertices: "refUpdate" (the anchor for queries), "repo" (parent grouping).
/// Each ref_update has extra_fields: ref_name, committer_did, protocol,
/// breaking_change_count, commit_count, migration_id, lens_id, lens_quality.
fn build_ref_update_schema() -> panproto_schema::Schema {
    use panproto_schema::{Protocol, SchemaBuilder};

    let protocol = Protocol::default();
    SchemaBuilder::new(&protocol)
        .vertex("repo", "record", None)
        .unwrap_or_else(|_| panic!("schema builder: repo vertex"))
        .vertex("refUpdate", "record", None)
        .unwrap_or_else(|_| panic!("schema builder: refUpdate vertex"))
        .vertex("function", "record", None)
        .unwrap_or_else(|_| panic!("schema builder: function vertex"))
        .vertex("struct", "record", None)
        .unwrap_or_else(|_| panic!("schema builder: struct vertex"))
        .vertex("field", "record", None)
        .unwrap_or_else(|_| panic!("schema builder: field vertex"))
        .edge("repo", "refUpdate", "refUpdates", None)
        .unwrap_or_else(|_| panic!("schema builder: refUpdates edge"))
        .build()
        .unwrap_or_else(|_| panic!("schema builder: build"))
}

/// Build a WInstance from a set of ref_update rows for a single repo.
///
/// Each ref_update becomes a node with anchor matching the requested anchor kind.
/// The node's extra_fields are populated from the ref_update's metadata columns,
/// making them available for predicate evaluation.
fn build_instance_from_ref_updates(
    updates: &[&crate::db::ref_update::RefUpdateRow],
    anchor: &str,
) -> panproto_inst::wtype::WInstance {
    use panproto_gat::Name;
    use panproto_inst::metadata::Node;
    use panproto_inst::value::Value;

    let mut nodes = HashMap::new();
    let mut arcs = Vec::new();

    // Node 0: repo root
    let repo_node = Node::new(0, "repo");
    nodes.insert(0u32, repo_node);

    for (i, ru) in updates.iter().enumerate() {
        let node_id = (i as u32) + 1;
        let mut node = Node::new(node_id, anchor);
        node.extra_fields
            .insert("ref_name".into(), Value::Str(ru.ref_name.clone()));
        node.extra_fields
            .insert("name".into(), Value::Str(ru.ref_name.clone()));
        node.extra_fields
            .insert("committer_did".into(), Value::Str(ru.committer_did.clone()));
        node.extra_fields
            .insert("protocol".into(), Value::Str(ru.protocol.clone()));
        node.extra_fields.insert(
            "breaking_change_count".into(),
            Value::Int(i64::from(ru.breaking_change_count)),
        );
        node.extra_fields.insert(
            "commit_count".into(),
            Value::Int(i64::from(ru.commit_count)),
        );
        node.extra_fields
            .insert("new_target".into(), Value::Str(ru.new_target.clone()));
        if let Some(ref old) = ru.old_target {
            node.extra_fields
                .insert("old_target".into(), Value::Str(old.clone()));
        }
        if let Some(ref mid) = ru.migration_id {
            node.extra_fields
                .insert("migration_id".into(), Value::Str(mid.clone()));
        }
        if let Some(ref lid) = ru.lens_id {
            node.extra_fields
                .insert("lens_id".into(), Value::Str(lid.clone()));
        }
        if let Some(lq) = ru.lens_quality {
            node.extra_fields
                .insert("lens_quality".into(), Value::Float(f64::from(lq)));
        }
        node.extra_fields
            .insert("repo_did".into(), Value::Str(ru.repo_did.clone()));
        node.extra_fields
            .insert("repo_name".into(), Value::Str(ru.repo_name.clone()));

        nodes.insert(node_id, node);

        // Arc from repo to this ref_update node
        let edge = panproto_schema::Edge {
            src: Name::from("repo"),
            tgt: Name::from(anchor),
            kind: Name::from("refUpdates"),
            name: None,
        };
        arcs.push((0u32, node_id, edge));
    }

    panproto_inst::wtype::WInstance::new(nodes, arcs, vec![], 0, Name::from("repo"))
}
