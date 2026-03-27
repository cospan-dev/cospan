//! Schema morphisms: ATProto → {Rust, SQL, TypeScript}
//!
//! Transforms ATProto Lexicon schemas into target protocol schemas
//! by mapping vertex kinds, edge kinds, and constraints.

use anyhow::Result;
use panproto_gat::Name;
use panproto_protocols::emit::children_by_edge;
use panproto_schema::SchemaBuilder;

// ---------------------------------------------------------------------------
// ATProto → Rust
// ---------------------------------------------------------------------------

pub fn atproto_to_rust(
    schema: &panproto_schema::Schema,
    nsid: &str,
) -> Result<panproto_schema::Schema> {
    let protocol = panproto_schema::Protocol::default();
    let mut builder = SchemaBuilder::new(&protocol);

    let struct_name = nsid_to_pascal(nsid);
    let body_id = find_record_body(schema, nsid);

    builder = builder.vertex(&struct_name, "struct", None)?;

    // Pre-create all needed primitive type vertices (deduped)
    let props = children_by_edge(schema, &body_id, "prop");
    let mut seen_types = std::collections::HashSet::new();
    for (_, prop_vertex) in &props {
        let (type_name, type_kind) = atproto_kind_to_rust(&prop_vertex.kind);
        if seen_types.insert(type_name.clone()) {
            builder = builder.vertex(&type_name, &type_kind, None)?;
        }
    }

    for (edge, prop_vertex) in &props {
        let field_name = edge.name.as_ref().map(|n| n.as_str()).unwrap_or("unknown");
        let snake_name = camel_to_snake(field_name);
        let field_id = format!("{struct_name}.{snake_name}");

        builder = builder.vertex(&field_id, "field", None)?;
        builder = builder.edge(&struct_name, &field_id, "field-of", Some(&snake_name))?;

        let (type_name, _) = atproto_kind_to_rust(&prop_vertex.kind);
        builder = builder.edge(&field_id, &type_name, "type-of", None)?;

        // Optional if not in required set
        if !is_field_required(schema, &body_id, field_name) {
            builder = builder.constraint(&field_id, "optional", "true");
        }

        // Rename for serde (snake_case field, camelCase on wire)
        if snake_name != field_name {
            builder = builder.constraint(&field_id, "rename", field_name);
        }
    }

    builder
        .build()
        .map_err(|e| anyhow::anyhow!("build rust schema for {nsid}: {e}"))
}

// ---------------------------------------------------------------------------
// ATProto → SQL
// ---------------------------------------------------------------------------

pub fn atproto_to_sql(
    schema: &panproto_schema::Schema,
    nsid: &str,
) -> Result<panproto_schema::Schema> {
    let protocol = panproto_schema::Protocol::default();
    let mut builder = SchemaBuilder::new(&protocol);

    let table_name = nsid_to_table_name(nsid);
    let body_id = find_record_body(schema, nsid);

    builder = builder.vertex(&table_name, "table", None)?;

    // Standard ATProto index columns
    for (col, kind, not_null) in [
        ("did", "string", true),
        ("rkey", "string", true),
        ("indexed_at", "timestamp", true),
    ] {
        let col_id = format!("{table_name}.{col}");
        builder = builder.vertex(&col_id, kind, None)?;
        builder = builder.edge(&table_name, &col_id, "prop", Some(col))?;
        if not_null {
            builder = builder.constraint(&col_id, "NOT NULL", "true");
        }
    }
    builder = builder.constraint(&format!("{table_name}.did"), "PRIMARY KEY", "true");

    // Lexicon properties → columns
    let props = children_by_edge(schema, &body_id, "prop");
    for (edge, prop_vertex) in &props {
        let field_name = edge.name.as_ref().map(|n| n.as_str()).unwrap_or("unknown");
        let col_name = camel_to_snake(field_name);
        let col_id = format!("{table_name}.{col_name}");

        // Skip columns that collide with standard columns (did, rkey, indexed_at)
        if col_name == "did" || col_name == "rkey" || col_name == "indexed_at" {
            continue;
        }

        let sql_kind = atproto_kind_to_sql(&prop_vertex.kind);
        builder = builder.vertex(&col_id, &sql_kind, None)?;
        builder = builder.edge(&table_name, &col_id, "prop", Some(&col_name))?;

        if is_field_required(schema, &body_id, field_name) {
            builder = builder.constraint(&col_id, "NOT NULL", "true");
        }
    }

    builder
        .build()
        .map_err(|e| anyhow::anyhow!("build sql schema for {nsid}: {e}"))
}

// ---------------------------------------------------------------------------
// ATProto → TypeScript
// ---------------------------------------------------------------------------

pub fn atproto_to_typescript(
    schema: &panproto_schema::Schema,
    nsid: &str,
) -> Result<panproto_schema::Schema> {
    let protocol = panproto_schema::Protocol::default();
    let mut builder = SchemaBuilder::new(&protocol);

    let iface_name = nsid_to_pascal(nsid);
    let body_id = find_record_body(schema, nsid);

    builder = builder.vertex(&iface_name, "interface", None)?;

    let props = children_by_edge(schema, &body_id, "prop");
    let mut seen_types = std::collections::HashSet::new();
    for (_, prop_vertex) in &props {
        let (type_name, type_kind) = atproto_kind_to_ts(&prop_vertex.kind);
        if seen_types.insert(type_name.clone()) {
            builder = builder.vertex(&type_name, &type_kind, None)?;
        }
    }

    for (edge, prop_vertex) in &props {
        let field_name = edge.name.as_ref().map(|n| n.as_str()).unwrap_or("unknown");
        let field_id = format!("{iface_name}.{field_name}");

        builder = builder.vertex(&field_id, "field", None)?;
        builder = builder.edge(&iface_name, &field_id, "field-of", Some(field_name))?;

        let (type_name, _) = atproto_kind_to_ts(&prop_vertex.kind);
        builder = builder.edge(&field_id, &type_name, "type-of", None)?;

        if !is_field_required(schema, &body_id, field_name) {
            builder = builder.constraint(&field_id, "optional", "true");
        }
    }

    builder
        .build()
        .map_err(|e| anyhow::anyhow!("build ts schema for {nsid}: {e}"))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn find_record_body(schema: &panproto_schema::Schema, nsid: &str) -> String {
    let children = children_by_edge(schema, nsid, "record-schema");
    if let Some((_, body)) = children.first() {
        return body.id.to_string();
    }
    nsid.to_string()
}

fn is_field_required(schema: &panproto_schema::Schema, body_id: &str, field_name: &str) -> bool {
    schema
        .required
        .get(&Name::from(body_id))
        .map(|reqs| {
            reqs.iter()
                .any(|e| e.name.as_ref().map(|n| n.as_str()) == Some(field_name))
        })
        .unwrap_or(false)
}

/// Returns (rust_type_name, rust_serde_vertex_kind)
fn atproto_kind_to_rust(kind: &Name) -> (String, String) {
    match kind.as_str() {
        "string" => ("String".into(), "string".into()),
        "integer" => ("i64".into(), "i64".into()),
        "number" => ("f64".into(), "f64".into()),
        "boolean" => ("bool".into(), "bool".into()),
        "bytes" => ("Vec<u8>".into(), "vec".into()),
        "cid-link" => ("String".into(), "string".into()),
        "ref" => ("String".into(), "string".into()),
        "blob" => ("serde_json::Value".into(), "string".into()),
        "array" => ("Vec<serde_json::Value>".into(), "vec".into()),
        "object" => ("serde_json::Value".into(), "string".into()),
        "union" => ("serde_json::Value".into(), "string".into()),
        "unknown" => ("serde_json::Value".into(), "string".into()),
        "token" => ("String".into(), "string".into()),
        _ => ("serde_json::Value".into(), "string".into()),
    }
}

fn atproto_kind_to_sql(kind: &Name) -> String {
    match kind.as_str() {
        "string" | "cid-link" | "ref" | "token" => "string",
        "integer" => "integer",
        "number" => "number",
        "boolean" => "boolean",
        "bytes" => "bytes",
        "blob" | "array" | "object" | "union" | "unknown" => "json",
        _ => "string",
    }
    .to_string()
}

/// Returns (ts_type_name, ts_protocol_vertex_kind)
fn atproto_kind_to_ts(kind: &Name) -> (String, String) {
    match kind.as_str() {
        "string" | "cid-link" | "ref" | "token" => ("string".into(), "string".into()),
        "integer" | "number" => ("number".into(), "number".into()),
        "boolean" => ("boolean".into(), "boolean".into()),
        "bytes" => ("Uint8Array".into(), "type-alias".into()),
        "blob" => ("BlobRef".into(), "type-alias".into()),
        "array" => ("unknown[]".into(), "type-alias".into()),
        "object" => ("Record<string, unknown>".into(), "type-alias".into()),
        "union" => ("unknown".into(), "type-alias".into()),
        "unknown" => ("unknown".into(), "type-alias".into()),
        _ => ("unknown".into(), "type-alias".into()),
    }
}

fn nsid_to_pascal(nsid: &str) -> String {
    nsid.split('.')
        .skip(2)
        .map(|p| {
            let mut c = p.chars();
            match c.next() {
                None => String::new(),
                Some(ch) => ch.to_uppercase().to_string() + c.as_str(),
            }
        })
        .collect()
}

fn nsid_to_table_name(nsid: &str) -> String {
    let parts: Vec<&str> = nsid.split('.').skip(2).collect();
    let joined: Vec<String> = parts.iter().map(|p| camel_to_snake(p)).collect();
    let mut name = joined.join("_");
    if !name.ends_with('s') {
        name.push('s');
    }
    name
}

fn camel_to_snake(s: &str) -> String {
    let mut r = String::with_capacity(s.len() + 4);
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            r.push('_');
        }
        r.push(c.to_lowercase().next().unwrap_or(c));
    }
    r
}

// ---------------------------------------------------------------------------
// Emit functions (local, since panproto removed sql/typescript/rust_serde emit)
// ---------------------------------------------------------------------------

use panproto_protocols::emit::{IndentWriter, constraint_value};

/// Emit SQL DDL from a SQL-protocol schema.
pub fn emit_ddl(schema: &panproto_schema::Schema) -> Result<String> {
    let mut w = IndentWriter::new("  ");
    let structural = &["prop"];
    let roots = panproto_protocols::emit::find_roots(schema, structural);
    for root in &roots {
        if root.kind.as_str() != "table" {
            continue;
        }
        w.line(&format!("CREATE TABLE {} (", root.id));
        w.indent();
        let cols = children_by_edge(schema, &root.id, "prop");
        let col_count = cols.len();
        for (i, (edge, col)) in cols.iter().enumerate() {
            let col_name = edge.name.as_ref().map(|n| n.as_str()).unwrap_or(&col.id);
            let sql_type = match col.kind.as_str() {
                "integer" => "INTEGER",
                "boolean" => "BOOLEAN",
                "number" => "FLOAT",
                "bytes" => "BYTEA",
                "timestamp" => "TIMESTAMP",
                "json" => "JSONB",
                _ => "TEXT",
            };
            let mut parts = vec![format!("{col_name} {sql_type}")];
            if constraint_value(schema, &col.id, "NOT NULL").is_some() {
                parts.push("NOT NULL".into());
            }
            if constraint_value(schema, &col.id, "PRIMARY KEY").is_some() {
                parts.push("PRIMARY KEY".into());
            }
            let comma = if i < col_count - 1 { "," } else { "" };
            w.line(&format!("{}{comma}", parts.join(" ")));
        }
        w.dedent();
        w.line(");");
        w.blank();
    }
    Ok(w.finish())
}

/// Emit Rust serde structs from a Rust-protocol schema.
pub fn emit_rust_types(schema: &panproto_schema::Schema) -> Result<String> {
    let structural = &["field-of", "variant-of", "type-of"];
    let roots = panproto_protocols::emit::find_roots(schema, structural);
    let mut w = IndentWriter::new("    ");
    for root in &roots {
        if root.kind.as_str() != "struct" {
            continue;
        }
        w.line("#[derive(Serialize, Deserialize)]");
        w.line(&format!("pub struct {} {{", root.id));
        w.indent();
        let fields = children_by_edge(schema, &root.id, "field-of");
        for (edge, field) in &fields {
            let field_name = edge.name.as_ref().map(|n| n.as_str()).unwrap_or(&field.id);
            let type_name = schema
                .outgoing_edges(&field.id)
                .iter()
                .find(|e| e.kind == "type-of")
                .and_then(|e| schema.vertices.get(&e.tgt))
                .map(|v| v.id.to_string())
                .unwrap_or_else(|| "String".into());
            let is_optional =
                constraint_value(schema, &field.id, "optional").is_some_and(|v| v == "true");
            let rename = constraint_value(schema, &field.id, "rename");
            if let Some(r) = rename {
                w.line(&format!("#[serde(rename = \"{r}\")]"));
            }
            let ty = if is_optional {
                format!("Option<{type_name}>")
            } else {
                type_name
            };
            w.line(&format!("pub {field_name}: {ty},"));
        }
        w.dedent();
        w.line("}");
        w.blank();
    }
    Ok(w.finish())
}

/// Emit TypeScript interfaces from a TS-protocol schema.
pub fn emit_ts_types(schema: &panproto_schema::Schema) -> Result<String> {
    let structural = &["field-of", "type-of"];
    let roots = panproto_protocols::emit::find_roots(schema, structural);
    let mut w = IndentWriter::new("  ");
    for root in &roots {
        if root.kind.as_str() != "interface" {
            continue;
        }
        w.line(&format!("interface {} {{", root.id));
        w.indent();
        let fields = children_by_edge(schema, &root.id, "field-of");
        for (edge, field) in &fields {
            let field_name = edge.name.as_ref().map(|n| n.as_str()).unwrap_or(&field.id);
            let type_name = schema
                .outgoing_edges(&field.id)
                .iter()
                .find(|e| e.kind == "type-of")
                .and_then(|e| schema.vertices.get(&e.tgt))
                .map(|v| v.id.to_string())
                .unwrap_or_else(|| "unknown".into());
            let optional =
                constraint_value(schema, &field.id, "optional").is_some_and(|v| v == "true");
            let opt = if optional { "?" } else { "" };
            w.line(&format!("{field_name}{opt}: {type_name};"));
        }
        w.dedent();
        w.line("}");
        w.blank();
    }
    Ok(w.finish())
}
