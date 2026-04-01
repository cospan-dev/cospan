//! Emit TypeScript view types by applying panproto protolens combinators to Lexicon schemas.
//!
//! The DB projection lens is built from high-level combinators (v0.23.0):
//!   - combinators::remove_field for skip_fields
//!   - combinators::add_field for uri_decompositions and extra_columns
//!   - combinators::rename_field for uri_storages and field_renames (uses rename_edge_name dependent optic)
//!   - combinators::pipeline to compose all steps
//!
//! The pipeline is instantiated against the source Lexicon schema to produce
//! the target (view) schema, which is walked to emit TypeScript interfaces.

use panproto_gat::Name;
use panproto_inst::value::Value;
use panproto_lens::{combinators, ProtolensChain};
use panproto_protocols::emit::children_by_edge;
use panproto_schema::{Protocol, Schema};

use crate::record_config::RecordConfig;

/// Build a ProtolensChain from RecordConfig using panproto combinators.
///
/// Each RecordConfig operation maps to a combinator:
///   - skip_fields → combinators::remove_field(vertex)
///   - uri_decompositions → remove_field(source) + add_field(did) + add_field(name)
///   - uri_storages → combinators::rename_field(parent, field, old, new)
///   - field_renames → combinators::rename_field(parent, field, old, new)
///   - type_overrides → remove_field + add_field (with correct kind)
///   - extra_columns → combinators::add_field(parent, name, kind, default)
fn build_lens_chain(body_id: &str, config: &RecordConfig) -> ProtolensChain {
    let mut chains: Vec<ProtolensChain> = Vec::new();

    // 1. Remove skipped fields
    for field in config.skip_fields {
        let vertex_id = format!("{body_id}.{field}");
        chains.push(combinators::remove_field(vertex_id));
    }

    // 2. URI decompositions: remove source, add decomposed fields
    for decomp in config.uri_decompositions {
        let source_vertex = format!("{body_id}.{}", decomp.source_field);
        chains.push(combinators::remove_field(source_vertex));

        let did_camel = snake_to_camel(decomp.did_column);
        let did_vertex = format!("{body_id}.{did_camel}");
        chains.push(combinators::add_field(
            body_id,
            did_vertex,
            "string",
            Value::Str(String::new()),
        ));

        let name_camel = snake_to_camel(decomp.name_column);
        let name_vertex = format!("{body_id}.{name_camel}");
        chains.push(combinators::add_field(
            body_id,
            name_vertex,
            "string",
            Value::Str(String::new()),
        ));
    }

    // 3. URI storages: rename field via dependent optic (rename_edge_name)
    for storage in config.uri_storages {
        let field_vertex = format!("{body_id}.{}", storage.source_field);
        let new_camel = snake_to_camel(storage.column_name);
        chains.push(combinators::rename_field(
            body_id,
            field_vertex,
            storage.source_field,
            &*new_camel,
        ));
    }

    // 4. Field renames via dependent optic
    for rename in config.field_renames {
        let field_vertex = format!("{body_id}.{}", rename.source_field);
        let new_camel = snake_to_camel(rename.column_name);
        chains.push(combinators::rename_field(
            body_id,
            field_vertex,
            rename.source_field,
            &*new_camel,
        ));
    }

    // 5. Type overrides: remove + add with correct kind
    for ovr in config.type_overrides {
        let vertex_id = format!("{body_id}.{}", ovr.source_field);
        let kind = match ovr.rust_type {
            t if t.contains("f32") || t.contains("f64") => "number",
            t if t.contains("i32") || t.contains("i64") => "integer",
            t if t.contains("bool") => "boolean",
            _ => "string",
        };
        let default = if ovr.rust_type.starts_with("Option<") {
            Value::Null
        } else {
            match kind {
                "number" => Value::Float(0.0),
                "integer" => Value::Int(0),
                "boolean" => Value::Bool(false),
                _ => Value::Str(String::new()),
            }
        };
        chains.push(combinators::remove_field(vertex_id.clone()));
        chains.push(combinators::add_field(body_id, vertex_id, kind, default));
    }

    // 6. Extra columns via add_field combinator
    for extra in config.extra_columns {
        let camel = snake_to_camel(extra.name);
        let vertex_id = format!("{body_id}.{camel}");
        let (kind, default) = match extra.rust_type {
            "i32" | "i64" => ("integer", Value::Int(0)),
            "f32" | "f64" => ("number", Value::Float(0.0)),
            "bool" => ("boolean", Value::Bool(false)),
            _ => ("string", Value::Str(String::new())),
        };
        chains.push(combinators::add_field(body_id, vertex_id, kind, default));
    }

    combinators::pipeline(chains)
}

/// Apply the lens chain to a source schema, producing the target (view) schema.
fn apply_lens(source: &Schema, nsid: &str, config: &RecordConfig) -> Schema {
    let body_id = find_record_body(source, nsid);
    let chain = build_lens_chain(&body_id, config);
    let protocol = Protocol::default();

    match chain.instantiate(source, &protocol) {
        Ok(lens) => lens.tgt_schema,
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
    out.push_str(&format!("// {nsid} (via panproto combinators)\n"));
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

    // Fields added by add_field combinator (they have prop edges now!)
    // Walk vertices that are children of body but not in the original prop list
    let body_prefix = format!("{body_id}.");
    let prop_targets: std::collections::HashSet<String> =
        props.iter().map(|(_, v)| v.id.to_string()).collect();
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

/// Emit TypeScript from target schema with explicit config (lens-file path).
fn emit_view_from_target_with(
    target: &Schema,
    nsid: &str,
    view_name: &str,
    include_did: bool,
    include_rkey: bool,
    column_defaults: &std::collections::HashMap<String, String>,
) -> String {
    let body_id = find_record_body(target, nsid);
    let mut out = String::new();
    out.push_str(&format!("// {nsid} (via lens file)\n"));
    out.push_str(&format!("export interface {view_name} {{\n"));

    if include_did { out.push_str("  did: string;\n"); }
    if include_rkey { out.push_str("  rkey: string;\n"); }

    let props = children_by_edge(target, &body_id, "prop");
    for (edge, prop_vertex) in &props {
        let field_name = edge.name.as_ref().map(|n| n.as_str()).unwrap_or("unknown");
        let ts_type = vertex_kind_to_ts(&prop_vertex.kind);
        let is_required = is_field_required(target, &body_id, field_name);
        if is_required {
            out.push_str(&format!("  {field_name}: {ts_type};\n"));
        } else {
            out.push_str(&format!("  {field_name}: {ts_type} | null;\n"));
        }
    }

    let body_prefix = format!("{body_id}.");
    let prop_targets: std::collections::HashSet<String> =
        props.iter().map(|(_, v)| v.id.to_string()).collect();
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
        out.push_str(&format!("  {field_name}: {ts_type};\n"));
    }

    out.push_str("  indexedAt: string;\n");
    out.push_str("}\n\n");

    // Normalization
    out.push_str(&format!("export function normalize{view_name}(raw: Partial<{view_name}>): {view_name} {{\n"));
    out.push_str("  return {\n");
    if include_did { out.push_str("    did: raw.did ?? '',\n"); }
    if include_rkey { out.push_str("    rkey: raw.rkey ?? '',\n"); }
    for (edge, prop_vertex) in &props {
        let field_name = edge.name.as_ref().map(|n| n.as_str()).unwrap_or("unknown");
        let is_required = is_field_required(target, &body_id, field_name);
        let default = if !is_required { "null" } else {
            default_for_kind_with_map(&prop_vertex.kind, field_name, column_defaults)
        };
        out.push_str(&format!("    {field_name}: raw.{field_name} ?? {default},\n"));
    }
    for (id, v) in &extra_vertices {
        let field_name = id.as_str().strip_prefix(&body_prefix).unwrap_or(id.as_str());
        let default = default_for_kind_with_map(&v.kind, field_name, column_defaults);
        out.push_str(&format!("    {field_name}: raw.{field_name} ?? {default},\n"));
    }
    out.push_str("    indexedAt: raw.indexedAt ?? '',\n");
    out.push_str("  };\n");
    out.push_str("}\n\n");

    out
}

fn default_for_kind_with_map(kind: &Name, field_name: &str, defaults: &std::collections::HashMap<String, String>) -> &'static str {
    if let Some(expr) = defaults.get(field_name) {
        return match expr.as_str() {
            "'open'" => "'open'",
            "'pending'" => "'pending'",
            "'public'" => "'public'",
            "'main'" => "'main'",
            "0" => "0",
            _ => "''",
        };
    }
    match kind.as_str() {
        "string" | "cid-link" | "ref" | "token" | "bytes" => "''",
        "integer" | "number" | "float" => "0",
        "boolean" => "false",
        _ => "''",
    }
}

fn nsid_to_pascal(nsid: &str) -> String {
    nsid.split('.').map(|s| {
        let mut c = s.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        }
    }).collect()
}

fn emit_list_response(type_name: &str, wrapper_key: &str) -> String {
    format!(
        "export interface {type_name}ListResponse {{\n  {wrapper_key}: {type_name}View[];\n  cursor: string | null;\n}}\n\n"
    )
}

/// Emit views from JSON lens files (primary path).
pub fn emit_all_views_from_lenses(
    schemas: &[(Schema, String)],
    lenses: &[crate::lens_config::LensFile],
) -> String {
    let mut out = String::new();
    out.push_str("// Auto-generated by cospan-codegen via panproto protolens (from JSON lens files).\n");
    out.push_str("// Source: packages/lenses/*.lens.json\n");
    out.push_str("// Do not edit manually.\n\n");

    for lens in crate::lens_config::db_projection_lenses(lenses) {
        if let Some((schema, _)) = schemas.iter().find(|(_, nsid)| nsid == &lens.source) {
            let body_id = find_record_body(schema, &lens.source);
            let chain = crate::lens_config::steps_to_protolens_chain(&lens.steps, &body_id);
            let protocol = Protocol::default();
            let target = match chain.instantiate(schema, &protocol) {
                Ok(l) => l.tgt_schema,
                Err(e) => {
                    eprintln!("  warn: lens instantiation for {}: {e:?}", lens.source);
                    schema.clone()
                }
            };

            let table = lens.table.as_ref();
            let view_name = table
                .map(|t| {
                    t.row_struct
                        .strip_suffix("Row")
                        .unwrap_or(&t.row_struct)
                        .to_string()
                        + "View"
                })
                .unwrap_or_else(|| format!("{}View", nsid_to_pascal(&lens.source)));
            let include_did = table.map(|t| t.include_did).unwrap_or(true);
            let include_rkey = table.map(|t| t.include_rkey).unwrap_or(true);

            out.push_str(&emit_view_from_target_with(
                &target,
                &lens.source,
                &view_name,
                include_did,
                include_rkey,
                table.map(|t| &t.column_defaults).unwrap_or(&std::collections::HashMap::new()),
            ));
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

/// Emit views from RecordConfig (legacy path, kept for backward compat).
pub fn emit_all_views(schemas: &[(Schema, String)], configs: &[RecordConfig]) -> String {
    let mut out = String::new();
    out.push_str("// Auto-generated by cospan-codegen via panproto protolens combinators.\n");
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
// Schema helpers
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

/// Map panproto vertex kind → TypeScript type.
fn vertex_kind_to_ts(kind: &Name) -> &'static str {
    match kind.as_str() {
        "string" | "cid-link" | "ref" | "token" | "bytes" => "string",
        "integer" | "number" | "float" => "number",
        "boolean" => "boolean",
        "array" => "unknown[]",
        "object" | "union" => "Record<string, unknown>",
        _ => {
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
        "string" | "cid-link" | "ref" | "token" | "bytes" => "''",
        "integer" | "number" | "float" => "0",
        "boolean" => "false",
        _ => "''",
    }
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
