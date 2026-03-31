//! Emit TypeScript view types by applying panproto Protolens transforms to Lexicon schemas.
//!
//! The DB projection lens is expressed as elementary Protolens transforms:
//!   - drop_sort("{body}.{field}") for skip_fields
//!   - add_sort("{body}.repoDid") + add_op for uri_decompositions
//!   - rename_sort("{body}.{old}", "{body}.{new}") for field_renames
//!   - add_sort("{body}.{col}") for extra_columns
//!
//! These are composed via vertical_compose, applied to the source Lexicon schema
//! via target_schema(), and the target schema is walked to emit TypeScript.

use panproto_gat::Name;
use panproto_inst::value::Value;
use panproto_lens::{
    elementary, protolens_vertical as vertical_compose, Protolens, ProtolensChain,
};
use panproto_protocols::emit::children_by_edge;
use panproto_schema::{Protocol, Schema};

use crate::record_config::RecordConfig;

/// Build a ProtolensChain from RecordConfig lens operations, scoped to a specific body vertex.
fn build_lens_chain(body_id: &str, config: &RecordConfig) -> ProtolensChain {
    let mut steps: Vec<Protolens> = Vec::new();

    // 1. Drop skipped fields: remove vertex "{body}.{field}" and its edges
    for field in config.skip_fields {
        let vertex_id = format!("{body_id}.{field}");
        steps.push(elementary::drop_sort(vertex_id));
    }

    // 2. URI decompositions: drop source field, add decomposed fields (camelCase)
    for decomp in config.uri_decompositions {
        let source_vertex = format!("{body_id}.{}", decomp.source_field);
        steps.push(elementary::drop_sort(source_vertex));

        let did_camel = snake_to_camel(decomp.did_column);
        let did_vertex = format!("{body_id}.{did_camel}");
        steps.push(elementary::add_sort(
            did_vertex,
            "string",
            Value::Str(String::new()),
        ));
        let name_camel = snake_to_camel(decomp.name_column);
        let name_vertex = format!("{body_id}.{name_camel}");
        steps.push(elementary::add_sort(
            name_vertex,
            "string",
            Value::Str(String::new()),
        ));
    }

    // 3. URI storages: rename source field to camelCase column name
    for storage in config.uri_storages {
        let old_vertex = format!("{body_id}.{}", storage.source_field);
        let new_camel = snake_to_camel(storage.column_name);
        let new_vertex = format!("{body_id}.{new_camel}");
        steps.push(elementary::rename_sort(old_vertex, new_vertex));
    }

    // 4. Field renames (to camelCase)
    for rename in config.field_renames {
        let old_vertex = format!("{body_id}.{}", rename.source_field);
        let new_camel = snake_to_camel(rename.column_name);
        let new_vertex = format!("{body_id}.{new_camel}");
        steps.push(elementary::rename_sort(old_vertex, new_vertex));
    }

    // 5. Type overrides: drop vertex with wrong kind, re-add with correct kind
    for ovr in config.type_overrides {
        let vertex_id = format!("{body_id}.{}", ovr.source_field);
        let kind = match ovr.rust_type {
            t if t.contains("f32") || t.contains("f64") => "number",
            t if t.contains("i32") || t.contains("i64") => "integer",
            t if t.contains("bool") => "boolean",
            _ => "string",
        };
        let is_optional = ovr.rust_type.starts_with("Option<");
        // Drop the original vertex and re-add with correct kind
        steps.push(elementary::drop_sort(vertex_id.clone()));
        let default = if is_optional {
            Value::Null
        } else {
            match kind {
                "number" => Value::Float(0.0),
                "integer" => Value::Int(0),
                "boolean" => Value::Bool(false),
                _ => Value::Str(String::new()),
            }
        };
        steps.push(elementary::add_sort(vertex_id, kind, default));
    }

    // 6. Extra columns (state, comment_count, etc.)
    // Use camelCase for vertex IDs to match serde(rename_all = "camelCase")
    for extra in config.extra_columns {
        let camel = snake_to_camel(extra.name);
        let vertex_id = format!("{body_id}.{camel}");
        let (kind, default) = match extra.rust_type {
            "i32" | "i64" => ("integer", Value::Int(0)),
            "f32" | "f64" => ("number", Value::Float(0.0)),
            "bool" => ("boolean", Value::Bool(false)),
            _ => ("string", Value::Str(String::new())),
        };
        steps.push(elementary::add_sort(vertex_id, kind, default));
    }

    ProtolensChain::new(steps)
}

/// Apply the lens chain to a source schema, producing the target (view) schema.
fn apply_lens(source: &Schema, nsid: &str, config: &RecordConfig) -> Schema {
    let body_id = find_record_body(source, nsid);
    let chain = build_lens_chain(&body_id, config);
    let protocol = Protocol::default();

    match chain.instantiate(source, &protocol) {
        Ok(lens) => {
            // The lens contains the target schema
            lens.tgt_schema
        }
        Err(e) => {
            eprintln!("  warn: lens instantiation for {nsid}: {e:?}, using source");
            source.clone()
        }
    }
}

/// Emit TypeScript interface from the target schema.
fn emit_view_from_target(target: &Schema, nsid: &str, config: &RecordConfig) -> String {
    let view_name = format!(
        "{}View",
        config
            .row_struct_name
            .strip_suffix("Row")
            .unwrap_or(config.row_struct_name)
    );
    let body_id = find_record_body(target, nsid);

    let mut out = String::new();
    out.push_str(&format!("// {nsid} (via Protolens)\n"));
    out.push_str(&format!("export interface {view_name} {{\n"));

    // Standard ATProto columns
    if config.include_did {
        out.push_str("  did: string;\n");
    }
    if config.include_rkey {
        out.push_str("  rkey: string;\n");
    }

    // Walk target schema's prop edges from the body vertex
    let props = children_by_edge(target, &body_id, "prop");
    for (edge, prop_vertex) in &props {
        let field_name = edge
            .name
            .as_ref()
            .map(|n| n.as_str())
            .unwrap_or("unknown");
        let ts_type = vertex_kind_to_ts(&prop_vertex.kind);
        let is_required = is_field_required(target, &body_id, field_name);
        if is_required {
            out.push_str(&format!("  {field_name}: {ts_type};\n"));
        } else {
            out.push_str(&format!("  {field_name}: {ts_type} | null;\n"));
        }
    }

    // Extra columns added by the lens (they appear as new vertices, not as prop edges)
    // Walk vertices that start with body_id but aren't in the prop edges
    let prop_targets: std::collections::HashSet<String> = props
        .iter()
        .map(|(_, v)| v.id.to_string())
        .collect();
    let body_prefix = format!("{body_id}.");
    let mut extra_vertices: Vec<_> = target
        .vertices
        .iter()
        .filter(|(id, _)| {
            let id_str = id.to_string();
            id_str.starts_with(&body_prefix)
                && !id_str[body_prefix.len()..].contains('.')
                && !id_str[body_prefix.len()..].contains(':')
                && !prop_targets.contains(&id_str)
        })
        .collect();
    extra_vertices.sort_by_key(|(id, _)| id.to_string());

    for (id, v) in &extra_vertices {
        let field_name = id.as_str().strip_prefix(&body_prefix).unwrap_or(id.as_str());
        let ts_type = vertex_kind_to_ts(&v.kind);
        // Extra columns from add_sort are always present (have defaults)
        out.push_str(&format!("  {field_name}: {ts_type};\n"));
    }

    // indexedAt (always added by appview)
    out.push_str("  indexedAt: string;\n");
    out.push_str("}\n\n");

    // Normalization function
    out.push_str(&format!(
        "export function normalize{view_name}(raw: Partial<{view_name}>): {view_name} {{\n"
    ));
    out.push_str("  return {\n");
    if config.include_did {
        out.push_str("    did: raw.did ?? '',\n");
    }
    if config.include_rkey {
        out.push_str("    rkey: raw.rkey ?? '',\n");
    }
    for (edge, prop_vertex) in &props {
        let field_name = edge.name.as_ref().map(|n| n.as_str()).unwrap_or("unknown");
        let is_required = is_field_required(target, &body_id, field_name);
        let default = if !is_required {
            "null"
        } else {
            default_for_kind(&prop_vertex.kind, field_name, config)
        };
        out.push_str(&format!("    {field_name}: raw.{field_name} ?? {default},\n"));
    }
    for (id, v) in &extra_vertices {
        let field_name = id.as_str().strip_prefix(&body_prefix).unwrap_or(id.as_str());
        let default = default_for_kind(&v.kind, field_name, config);
        out.push_str(&format!("    {field_name}: raw.{field_name} ?? {default},\n"));
    }
    out.push_str("    indexedAt: raw.indexedAt ?? '',\n");
    out.push_str("  };\n");
    out.push_str("}\n\n");

    out
}

fn emit_list_response(type_name: &str, wrapper_key: &str) -> String {
    format!(
        "export interface {type_name}ListResponse {{\n  {wrapper_key}: {type_name}View[];\n  cursor: string | null;\n}}\n\n"
    )
}

/// Emit the full generated TypeScript views file.
pub fn emit_all_views(schemas: &[(Schema, String)], configs: &[RecordConfig]) -> String {
    let mut out = String::new();
    out.push_str("// Auto-generated by cospan-codegen via panproto Protolens.\n");
    out.push_str("// Source: Lexicon schemas transformed through DB projection lens.\n");
    out.push_str("// Do not edit manually.\n\n");

    for config in configs {
        if let Some((schema, _)) = schemas.iter().find(|(_, nsid)| nsid == config.nsid) {
            let target = apply_lens(schema, config.nsid, config);
            out.push_str(&emit_view_from_target(&target, config.nsid, config));
        }
    }

    let list_endpoints = [
        ("Repo", "repos"),
        ("Issue", "issues"),
        ("IssueComment", "comments"),
        ("Pull", "pulls"),
        ("PullComment", "comments"),
        ("Star", "stars"),
        ("Follow", "follows"),
        ("Node", "nodes"),
        ("Org", "orgs"),
        ("OrgMember", "members"),
        ("Collaborator", "collaborators"),
        ("RefUpdate", "refUpdates"),
        ("Label", "labels"),
        ("Pipeline", "pipelines"),
        ("Reaction", "reactions"),
    ];
    for (type_name, wrapper_key) in &list_endpoints {
        out.push_str(&emit_list_response(type_name, wrapper_key));
    }

    out
}

// ---------------------------------------------------------------------------
// Schema helpers — these use panproto's schema structure, not string munging
// ---------------------------------------------------------------------------

fn find_record_body(schema: &Schema, nsid: &str) -> String {
    let children = children_by_edge(schema, nsid, "record-schema");
    if let Some((_, body)) = children.first() {
        return body.id.to_string();
    }
    let body_id = format!("{nsid}:body");
    if schema.has_vertex(&body_id) {
        return body_id;
    }
    nsid.to_string()
}

fn is_field_required(schema: &Schema, body_id: &str, field_name: &str) -> bool {
    schema
        .required
        .get(&Name::from(body_id))
        .map(|reqs| {
            reqs.iter()
                .any(|e| e.name.as_ref().map(|n| n.as_str()) == Some(field_name))
        })
        .unwrap_or(false)
}

fn snake_to_camel(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;
    for (i, c) in s.chars().enumerate() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else if i == 0 {
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

/// Map panproto vertex kind → TypeScript type.
/// The vertex kind IS the panproto type — no Rust type string mapping.
fn vertex_kind_to_ts(kind: &Name) -> &'static str {
    match kind.as_str() {
        "string" | "cid-link" | "ref" | "token" | "bytes" => "string",
        "integer" | "number" | "float" => "number",
        "boolean" => "boolean",
        "array" => "unknown[]",
        "object" | "union" => "Record<string, unknown>",
        _ => {
            // Log unhandled kinds during codegen for debugging
            eprintln!("    warn: unhandled vertex kind '{}', mapping to unknown", kind);
            "unknown"
        }
    }
}

/// Get default value from lens column_defaults or vertex kind.
fn default_for_kind(kind: &Name, field_name: &str, config: &RecordConfig) -> &'static str {
    for cd in config.column_defaults {
        if cd.column == field_name {
            return match cd.expression {
                "'open'" => "'open'",
                "'pending'" => "'pending'",
                "'public'" => "'public'",
                "'main'" => "'main'",
                "0" => "0",
                _ => "''",
            };
        }
    }
    match kind.as_str() {
        "string" | "cid-link" | "ref" | "token" => "''",
        "integer" | "number" => "0",
        "boolean" => "false",
        _ => "''",
    }
}
