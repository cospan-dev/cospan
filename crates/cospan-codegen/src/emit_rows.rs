//! Emit database Row types, CRUD functions, and from_json deserializers.
//!
//! These are the "appview projection" of the Lexicon schemas — they include
//! standard ATProto columns (did, rkey, indexed_at) plus denormalized fields.

use anyhow::Result;
use panproto_protocols::emit::{IndentWriter, children_by_edge};

use crate::record_config::RecordConfig;

/// Column info extracted from the panproto schema + record config.
#[allow(dead_code)]
struct Column {
    name: String,
    camel_name: String,
    rust_type: String,
    sql_type: String,
    optional: bool,
    /// Whether this is a denormalized field (not from upsert input).
    is_counter: bool,
}

fn columns_for_record(
    schema: &panproto_schema::Schema,
    nsid: &str,
    config: &RecordConfig,
) -> Vec<Column> {
    let mut cols = Vec::new();

    // 1. Standard ATProto columns
    if config.has_serial_id {
        cols.push(Column {
            name: "id".into(),
            camel_name: "id".into(),
            rust_type: "i64".into(),
            sql_type: "BIGSERIAL".into(),
            optional: false,
            is_counter: true, // auto-generated, not from input
        });
    }
    if config.include_did {
        cols.push(Column {
            name: "did".into(),
            camel_name: "did".into(),
            rust_type: "String".into(),
            sql_type: "TEXT".into(),
            optional: false,
            is_counter: false,
        });
    }
    if config.include_rkey {
        cols.push(Column {
            name: "rkey".into(),
            camel_name: "rkey".into(),
            rust_type: "String".into(),
            sql_type: "TEXT".into(),
            optional: false,
            is_counter: false,
        });
    }

    // 2. URI decomposition columns (replace the AT-URI field with did+name)
    for decomp in config.uri_decompositions {
        cols.push(Column {
            name: decomp.did_column.into(),
            camel_name: snake_to_camel(decomp.did_column),
            rust_type: "String".into(),
            sql_type: "TEXT".into(),
            optional: false,
            is_counter: false,
        });
        cols.push(Column {
            name: decomp.name_column.into(),
            camel_name: snake_to_camel(decomp.name_column),
            rust_type: "String".into(),
            sql_type: "TEXT".into(),
            optional: false,
            is_counter: false,
        });
    }

    // 3. URI storage columns (store full AT-URI as a renamed string)
    for storage in config.uri_storages {
        cols.push(Column {
            name: storage.column_name.into(),
            camel_name: snake_to_camel(storage.column_name),
            rust_type: "String".into(),
            sql_type: "TEXT".into(),
            optional: false,
            is_counter: false,
        });
    }

    // 4. Field renames (Lexicon field stored under a different column name)
    for rename in config.field_renames {
        let rust_type = rename
            .rust_type
            .map(|t| t.to_string())
            .unwrap_or_else(|| "String".into());
        cols.push(Column {
            name: rename.column_name.into(),
            camel_name: snake_to_camel(rename.column_name),
            rust_type,
            sql_type: "TEXT".into(),
            optional: false,
            is_counter: false,
        });
    }

    // 5. Lexicon fields (excluding skipped and already-handled)
    let body_id = find_record_body(schema, nsid);
    let props = children_by_edge(schema, &body_id, "prop");
    for (edge, prop_vertex) in &props {
        let field_name = edge.name.as_ref().map(|n| n.as_str()).unwrap_or("unknown");

        // Skip fields handled by URI decomposition/storage, renames, or explicitly skipped
        if config.skip_fields.contains(&field_name) {
            continue;
        }
        // Skip standard ATProto columns already added
        if field_name == "did" || field_name == "rkey" {
            continue;
        }

        // Check for type overrides
        let type_override = config
            .type_overrides
            .iter()
            .find(|o| o.source_field == field_name);

        let mut snake = camel_to_snake(field_name);
        // Escape Rust keywords
        if snake == "ref" {
            snake = "ref_name".to_string();
        } else if snake == "type" {
            snake = "type_name".to_string();
        }

        if let Some(ovr) = type_override {
            cols.push(Column {
                name: snake,
                camel_name: field_name.into(),
                rust_type: ovr.rust_type.into(),
                sql_type: ovr.sql_type.into(),
                optional: false,
                is_counter: false,
            });
        } else {
            let is_required = is_field_required(schema, &body_id, field_name);
            let (rust_type, sql_type) = lexicon_kind_to_db_types(&prop_vertex.kind, field_name);

            cols.push(Column {
                name: snake,
                camel_name: field_name.into(),
                rust_type: if is_required {
                    rust_type
                } else {
                    format!("Option<{rust_type}>")
                },
                sql_type: sql_type.clone(),
                optional: !is_required,
                is_counter: false,
            });
        }
    }

    // 6. Extra denormalized columns
    for extra in config.extra_columns {
        cols.push(Column {
            name: extra.name.into(),
            camel_name: snake_to_camel(extra.name),
            rust_type: extra.rust_type.into(),
            sql_type: extra.sql_type.into(),
            optional: extra.optional,
            is_counter: extra.exclude_from_insert,
        });
    }

    // 7. indexed_at (always last)
    cols.push(Column {
        name: "indexed_at".into(),
        camel_name: "indexedAt".into(),
        rust_type: "DateTime<Utc>".into(),
        sql_type: "TIMESTAMPTZ".into(),
        optional: false,
        is_counter: true, // auto-set to NOW()
    });

    cols
}

// ---------------------------------------------------------------------------
// Emit Row struct
// ---------------------------------------------------------------------------

pub fn emit_row_types(
    schema: &panproto_schema::Schema,
    nsid: &str,
    config: &RecordConfig,
) -> Result<String> {
    let cols = columns_for_record(schema, nsid, config);
    let mut w = IndentWriter::new("    ");

    w.line("#[derive(Debug, sqlx::FromRow, serde::Serialize, serde::Deserialize)]");
    w.line("#[serde(rename_all = \"camelCase\")]");
    w.line(&format!("pub struct {} {{", config.row_struct_name));
    w.indent();
    for col in &cols {
        // Add #[serde(default)] for fields not in the Lexicon record
        // (did, rkey, indexed_at, counters) so serde_json::from_value works
        if col.name == "indexed_at" {
            w.line("#[serde(default = \"default_now\")]");
        } else if col.rust_type.starts_with("DateTime") {
            // DateTime fields from Lexicon are already in the transformed JSON
        } else if col.is_counter || col.name == "did" || col.name == "rkey" || col.name == "id" {
            w.line("#[serde(default)]");
        }
        w.line(&format!("pub {}: {},", col.name, col.rust_type));
    }
    w.dedent();
    w.line("}");
    w.blank();

    Ok(w.finish())
}

// ---------------------------------------------------------------------------
// Emit CRUD functions
// ---------------------------------------------------------------------------

pub fn emit_crud(
    schema: &panproto_schema::Schema,
    nsid: &str,
    config: &RecordConfig,
) -> Result<String> {
    let cols = columns_for_record(schema, nsid, config);
    let mut w = IndentWriter::new("    ");
    let row_name = config.row_struct_name;

    // --- upsert ---
    let insert_cols: Vec<&str> = cols
        .iter()
        .filter(|c| !c.is_counter)
        .map(|c| c.name.as_str())
        .collect();
    let placeholders: Vec<String> = (1..=insert_cols.len()).map(|i| format!("${i}")).collect();
    let update_sets: Vec<String> = insert_cols
        .iter()
        .filter(|c| !config.conflict_keys.contains(c))
        .map(|c| format!("{c} = EXCLUDED.{c}"))
        .collect();

    w.line(&format!(
        "pub async fn upsert(pool: &PgPool, row: &{row_name}) -> Result<(), sqlx::Error> {{"
    ));
    w.indent();
    w.line("sqlx::query(");
    w.indent();
    w.line(&format!(
        "\"INSERT INTO {} ({}) \\",
        config.table_name,
        insert_cols.join(", ")
    ));
    w.line(&format!(" VALUES ({}) \\", placeholders.join(", ")));
    w.line(&format!(
        " ON CONFLICT ({}) DO UPDATE SET \\",
        config.conflict_keys.join(", ")
    ));
    w.line(&format!(
        " {}, indexed_at = NOW()\"",
        update_sets.join(", \\\n           ")
    ));
    w.dedent();
    w.line(")");
    for col_name in &insert_cols {
        let col = cols.iter().find(|c| c.name == *col_name).unwrap();
        if col.rust_type.starts_with("Option<") || col.rust_type == "String" {
            w.line(&format!(".bind(&row.{col_name})"));
        } else {
            w.line(&format!(".bind(row.{col_name})"));
        }
    }
    w.line(".execute(pool)");
    w.line(".await?;");
    w.line("Ok(())");
    w.dedent();
    w.line("}");
    w.blank();

    // --- delete ---
    let delete_where: Vec<String> = config
        .conflict_keys
        .iter()
        .enumerate()
        .map(|(i, k)| format!("{k} = ${}", i + 1))
        .collect();
    let delete_params: Vec<String> = config
        .conflict_keys
        .iter()
        .map(|k| format!("{k}: &str"))
        .collect();

    w.line(&format!(
        "pub async fn delete(pool: &PgPool, {}) -> Result<(), sqlx::Error> {{",
        delete_params.join(", ")
    ));
    w.indent();
    w.line(&format!(
        "sqlx::query(\"DELETE FROM {} WHERE {}\")",
        config.table_name,
        delete_where.join(" AND ")
    ));
    for key in config.conflict_keys {
        w.line(&format!(".bind({key})"));
    }
    w.line(".execute(pool)");
    w.line(".await?;");
    w.line("Ok(())");
    w.dedent();
    w.line("}");
    w.blank();

    // --- get ---
    let all_col_names: Vec<&str> = cols.iter().map(|c| c.name.as_str()).collect();
    let select_cols = all_col_names.join(", ");

    // Use conflict keys for the get query (unique lookup)
    let where_clauses: Vec<String> = config
        .conflict_keys
        .iter()
        .enumerate()
        .map(|(i, k)| format!("{k} = ${}", i + 1))
        .collect();

    let get_params: Vec<String> = config
        .conflict_keys
        .iter()
        .map(|k| format!("{k}: &str"))
        .collect();

    w.line(&format!(
        "pub async fn get(pool: &PgPool, {}) -> Result<Option<{row_name}>, sqlx::Error> {{",
        get_params.join(", ")
    ));
    w.indent();
    w.line(&format!(
        "sqlx::query_as::<_, {row_name}>(\"SELECT {} FROM {} WHERE {}\")",
        select_cols,
        config.table_name,
        where_clauses.join(" AND ")
    ));
    for key in config.conflict_keys {
        w.line(&format!(".bind({key})"));
    }
    w.line(".fetch_optional(pool)");
    w.line(".await");
    w.dedent();
    w.line("}");
    w.blank();

    // --- list (paginated by indexed_at cursor) ---
    w.line(&format!(
        "pub async fn list(pool: &PgPool, limit: i64, cursor: Option<&str>) -> Result<Vec<{row_name}>, sqlx::Error> {{"
    ));
    w.indent();
    w.line("if let Some(cursor_ts) = cursor {");
    w.indent();
    w.line(&format!(
        "sqlx::query_as::<_, {row_name}>(\"SELECT {} FROM {} WHERE indexed_at < $1::timestamptz ORDER BY indexed_at DESC LIMIT $2\")",
        select_cols,
        config.table_name,
    ));
    w.line(".bind(cursor_ts)");
    w.line(".bind(limit)");
    w.line(".fetch_all(pool)");
    w.line(".await");
    w.dedent();
    w.line("} else {");
    w.indent();
    w.line(&format!(
        "sqlx::query_as::<_, {row_name}>(\"SELECT {} FROM {} ORDER BY indexed_at DESC LIMIT $1\")",
        select_cols, config.table_name,
    ));
    w.line(".bind(limit)");
    w.line(".fetch_all(pool)");
    w.line(".await");
    w.dedent();
    w.line("}");
    w.dedent();
    w.line("}");
    w.blank();

    Ok(w.finish())
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
    use panproto_gat::Name;
    schema
        .required
        .get(&Name::from(body_id))
        .map(|reqs| {
            reqs.iter()
                .any(|e| e.name.as_ref().map(|n| n.as_str()) == Some(field_name))
        })
        .unwrap_or(false)
}

fn lexicon_kind_to_db_types(kind: &panproto_gat::Name, field_name: &str) -> (String, String) {
    // DateTime fields
    if field_name.ends_with("At") || field_name == "createdAt" || field_name == "completedAt" {
        if kind.as_str() == "string" {
            return ("DateTime<Utc>".into(), "TIMESTAMPTZ".into());
        }
    }
    match kind.as_str() {
        "string" => ("String".into(), "TEXT".into()),
        "integer" => ("i64".into(), "BIGINT".into()),
        "number" => ("f64".into(), "DOUBLE PRECISION".into()),
        "boolean" => ("bool".into(), "BOOLEAN".into()),
        "bytes" => ("Vec<u8>".into(), "BYTEA".into()),
        "blob" => ("serde_json::Value".into(), "JSONB".into()),
        "array" => ("serde_json::Value".into(), "JSONB".into()),
        "object" => ("serde_json::Value".into(), "JSONB".into()),
        "ref" => ("String".into(), "TEXT".into()),
        "union" => ("serde_json::Value".into(), "JSONB".into()),
        "unknown" => ("serde_json::Value".into(), "JSONB".into()),
        "token" => ("String".into(), "TEXT".into()),
        "cid-link" => ("String".into(), "TEXT".into()),
        _ => ("serde_json::Value".into(), "JSONB".into()),
    }
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

fn snake_to_camel(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;
    for (i, c) in s.chars().enumerate() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_uppercase().next().unwrap_or(c));
            capitalize_next = false;
        } else if i == 0 {
            result.push(c); // keep first char lowercase
        } else {
            result.push(c);
        }
    }
    result
}
