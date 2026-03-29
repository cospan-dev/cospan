//! Emit PostgreSQL CREATE TABLE DDL from panproto Schema + RecordConfig.
//!
//! This is the SQL codegen target, following the same pattern as Cassandra's
//! `emit_cql()` in panproto-protocols. It takes the projected column list
//! (from `columns_for_record`) and the RecordConfig metadata (indexes,
//! foreign keys, defaults, primary keys) to emit complete PostgreSQL DDL
//! that matches the hand-written migrations.

use std::fmt::Write as _;

use crate::emit_rows::columns_for_record;
use crate::record_config::RecordConfig;

/// Emit a complete PostgreSQL CREATE TABLE statement (plus indexes) for one record type.
///
/// The generated DDL includes:
/// - Column definitions with types, NOT NULL, and DEFAULT constraints
/// - PRIMARY KEY (inline for single-column, table-level for composite)
/// - FOREIGN KEY constraints
/// - CREATE INDEX / CREATE UNIQUE INDEX statements
pub fn emit_create_table(
    schema: &panproto_schema::Schema,
    nsid: &str,
    config: &RecordConfig,
) -> String {
    let cols = columns_for_record(schema, nsid, config);
    let table = config.table_name;
    let mut out = String::new();

    // -- Comment header
    let _ = writeln!(out, "-- Projected from {nsid} Lexicon");

    // -- CREATE TABLE
    let _ = writeln!(out, "CREATE TABLE {table} (");

    let col_count = cols.len();

    // Determine PK strategy:
    // - has_serial_id: PK is `id BIGSERIAL PRIMARY KEY` (inline), conflict_keys are a UNIQUE index
    // - single conflict_key: inline PRIMARY KEY on that column
    // - multiple conflict_keys: table-level PRIMARY KEY (col1, col2)
    let pk_columns: Vec<&str> = if config.has_serial_id {
        vec!["id"]
    } else {
        config.conflict_keys.to_vec()
    };
    let pk_is_inline = pk_columns.len() == 1;
    let needs_table_pk = !pk_is_inline;

    // Check if we need table-level constraints after the columns
    let composite_fks: Vec<_> = config
        .foreign_keys
        .iter()
        .filter(|fk| fk.columns.len() > 1)
        .collect();
    let has_table_constraints = needs_table_pk || !composite_fks.is_empty();

    for (i, col) in cols.iter().enumerate() {
        let mut parts = Vec::new();

        // Column name + type
        parts.push(format!("    {} {}", col.name, col.sql_type));

        // PRIMARY KEY inline for single-column PKs
        if pk_is_inline && pk_columns.contains(&col.name.as_str()) {
            parts.push("PRIMARY KEY".into());
        }

        // REFERENCES for single-column foreign keys
        for fk in config.foreign_keys {
            if fk.columns.len() == 1 && fk.columns[0] == col.name {
                parts.push(format!(
                    "REFERENCES {}({})",
                    fk.ref_table,
                    fk.ref_columns.join(", ")
                ));
            }
        }

        // NOT NULL (skip for inline PRIMARY KEY columns since PK implies NOT NULL,
        // and skip for BIGSERIAL which is implicitly NOT NULL)
        let is_inline_pk = pk_is_inline && pk_columns.contains(&col.name.as_str());
        if !col.optional && col.sql_type != "BIGSERIAL" && !is_inline_pk {
            parts.push("NOT NULL".into());
        }

        // DEFAULT
        if let Some(def) = find_default(config, &col.name) {
            parts.push(format!("DEFAULT {def}"));
        } else if col.name == "indexed_at" {
            parts.push("DEFAULT NOW()".into());
        }

        // Trailing comma logic
        let is_last_col = i == col_count - 1;
        let has_trailing = !is_last_col || has_table_constraints;

        let line = parts.join(" ");
        if has_trailing {
            let _ = writeln!(out, "{line},");
        } else {
            let _ = writeln!(out, "{line}");
        }
    }

    // Composite PRIMARY KEY
    if config.conflict_keys.len() > 1 {
        let pk_cols = config.conflict_keys.join(", ");
        let has_more = !config.foreign_keys.is_empty();
        if has_more {
            let _ = writeln!(out, "    PRIMARY KEY ({pk_cols}),");
        } else {
            let _ = writeln!(out, "    PRIMARY KEY ({pk_cols})");
        }
    }

    // FOREIGN KEY constraints (composite only; single-column FKs use REFERENCES inline)
    let composite_fks: Vec<_> = config
        .foreign_keys
        .iter()
        .filter(|fk| fk.columns.len() > 1)
        .collect();
    for (i, fk) in composite_fks.iter().enumerate() {
        let local = fk.columns.join(", ");
        let remote = fk.ref_columns.join(", ");
        let comma = if i + 1 < composite_fks.len() { "," } else { "" };
        let _ = writeln!(
            out,
            "    FOREIGN KEY ({local}) REFERENCES {}({remote}){comma}",
            fk.ref_table
        );
    }

    let _ = writeln!(out, ");");

    // -- CREATE INDEX statements
    if !config.indexes.is_empty() {
        let _ = writeln!(out);
    }
    for idx in config.indexes {
        emit_index(&mut out, table, idx);
    }

    out
}

/// Emit all record types' DDL into a single migration file.
pub fn emit_all_migrations(
    schemas: &[(String, panproto_schema::Schema)],
    configs: &[RecordConfig],
) -> String {
    let mut out = String::new();
    out.push_str("-- Generated by cospan-codegen from packages/lexicons/\n");
    out.push_str("-- Do not edit manually.\n\n");

    for config in configs {
        if let Some((_, schema)) = schemas.iter().find(|(nsid, _)| nsid == config.nsid) {
            let ddl = emit_create_table(schema, config.nsid, config);
            out.push_str(&ddl);
            out.push('\n');
        }
    }

    out
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn find_default<'a>(config: &'a RecordConfig, col_name: &str) -> Option<&'a str> {
    config
        .column_defaults
        .iter()
        .find(|d| d.column == col_name)
        .map(|d| d.expression)
}

fn emit_index(out: &mut String, table: &str, idx: &crate::record_config::IndexConfig) {
    let unique = if idx.unique { "UNIQUE " } else { "" };

    if let Some(using) = idx.using {
        // GIN or other non-btree index
        if let Some(raw) = idx.raw_expression {
            let _ = writeln!(
                out,
                "CREATE INDEX {name} ON {table} USING {using} (\n    {raw}\n);",
                name = idx.name,
            );
        } else {
            let cols = idx.columns.join(", ");
            let _ = writeln!(
                out,
                "CREATE {unique}INDEX {name} ON {table} USING {using} ({cols});",
                name = idx.name,
            );
        }
    } else {
        let cols = idx.columns.join(", ");
        let where_clause = idx
            .where_clause
            .map(|w| format!("\n    WHERE {w}"))
            .unwrap_or_default();
        let _ = writeln!(
            out,
            "CREATE {unique}INDEX {name} ON {table} ({cols}){where_clause};",
            name = idx.name,
        );
    }
}
