//! Load lens files via panproto-lens-dsl.
//!
//! Thin wrapper around panproto_lens_dsl for cospan-specific conventions
//! (db-projection vs interop lens IDs, table metadata in extensions).

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use panproto_lens_dsl::CompiledLens;

/// Type alias for backward compatibility.
pub type LensFile = CompiledLens;

/// Load and compile all lens files from a directory.
pub fn load_all_lenses(lenses_dir: &Path) -> Result<Vec<CompiledLens>> {
    if !lenses_dir.exists() {
        return Ok(Vec::new());
    }

    let result = panproto_lens_dsl::load_dir(lenses_dir)
        .with_context(|| format!("loading lenses from {}", lenses_dir.display()))?;

    for (path, err) in &result.errors {
        eprintln!("  warn: failed to load {}: {err}", path.display());
    }

    let resolver = |_id: &str| -> Option<CompiledLens> { None };
    let mut compiled = Vec::new();

    for doc in &result.documents {
        let body_vertex = format!("{}:body", doc.source);
        match panproto_lens_dsl::compile(doc, &body_vertex, &resolver) {
            Ok(c) => compiled.push(c),
            Err(e) => eprintln!("  warn: compile lens {}: {e}", doc.id),
        }
    }

    Ok(compiled)
}

/// DB projection lenses (id ending in ".db-projection").
pub fn db_projection_lenses(lenses: &[CompiledLens]) -> Vec<&CompiledLens> {
    lenses
        .iter()
        .filter(|l| l.id.ends_with(".db-projection"))
        .collect()
}

/// Interop lenses (id ending in ".interop").
#[allow(dead_code)]
pub fn interop_lenses(lenses: &[CompiledLens]) -> Vec<&CompiledLens> {
    lenses
        .iter()
        .filter(|l| l.id.ends_with(".interop"))
        .collect()
}

/// Find a compiled lens by source NSID.
pub fn find_by_source<'a>(lenses: &'a [CompiledLens], source: &str) -> Option<&'a CompiledLens> {
    lenses.iter().find(|l| l.source == source)
}

/// Extract table config from the lens's extensions.
pub fn table_config(lens: &CompiledLens) -> Option<TableConfig> {
    lens.extensions
        .get("table")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
}

/// DDL metadata from the "table" extension.
///
/// Fields are deserialized from a lens-config JSON document; clippy's
/// dead-code detector can't see those reads, hence the allow.
#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
pub struct TableConfig {
    pub name: String,
    pub row_struct: String,
    pub conflict_keys: Vec<String>,
    #[serde(default)]
    pub has_serial_id: bool,
    #[serde(default = "default_true")]
    pub include_did: bool,
    #[serde(default = "default_true")]
    pub include_rkey: bool,
    #[serde(default)]
    pub column_defaults: HashMap<String, String>,
    #[serde(default)]
    pub counter_fields: Vec<String>,
}

fn default_true() -> bool {
    true
}
