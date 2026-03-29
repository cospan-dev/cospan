//! Schema-driven record parsing via panproto.
//!
//! Parses Jetstream JSON records into panproto WInstances using the
//! Lexicon schema, then converts to JSON in the canonical Cospan shape.
//! This replaces all hand-coded `rec.get("field")` chains with
//! panproto's typed parsing infrastructure.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use panproto_inst::parse::{parse_json, to_json};
use panproto_protocols::web_document::atproto;
use panproto_schema::Schema;

/// Pre-loaded Lexicon schemas for all Cospan record types.
pub struct SchemaRegistry {
    /// NSID → parsed panproto Schema
    schemas: HashMap<String, Schema>,
}

impl SchemaRegistry {
    /// Load all Lexicon schemas from the lexicons directory.
    pub fn load(lexicons_dir: &Path) -> Result<Self> {
        let mut schemas = HashMap::new();

        fn walk(dir: &Path, schemas: &mut HashMap<String, Schema>) -> Result<()> {
            let entries = std::fs::read_dir(dir)
                .with_context(|| format!("reading {}", dir.display()))?;
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, schemas)?;
                } else if path.extension().is_some_and(|e| e == "json") {
                    let content = std::fs::read_to_string(&path)?;
                    let json: serde_json::Value = serde_json::from_str(&content)?;
                    if let Some(nsid) = json.get("id").and_then(|v| v.as_str()) {
                        // Only parse record types (not procedures)
                        let is_record = json
                            .pointer("/defs/main/type")
                            .and_then(|v| v.as_str())
                            == Some("record");
                        if is_record {
                            match atproto::parse_lexicon(&json) {
                                Ok(schema) => {
                                    schemas.insert(nsid.to_string(), schema);
                                }
                                Err(e) => {
                                    tracing::debug!(nsid, error = %e, "skipping unparseable lexicon");
                                }
                            }
                        }
                    }
                }
            }
            Ok(())
        }

        walk(lexicons_dir, &mut schemas)?;
        tracing::info!(count = schemas.len(), "loaded lexicon schemas");
        Ok(Self { schemas })
    }

    /// Parse a JSON record into a panproto WInstance, then emit as canonical JSON.
    ///
    /// This ensures the record is validated against the Lexicon schema and
    /// all field names are normalized. Returns None if the NSID has no schema.
    pub fn parse_record(
        &self,
        nsid: &str,
        record: &serde_json::Value,
    ) -> Option<Result<serde_json::Value>> {
        let schema = self.schemas.get(nsid)?;

        let result = parse_json(schema, nsid, record)
            .map(|instance| to_json(schema, &instance))
            .map_err(|e| anyhow::anyhow!("parse {nsid}: {e:?}"));

        Some(result)
    }

    /// Get the raw schema for an NSID (for use with lift operations).
    pub fn schema(&self, nsid: &str) -> Option<&Schema> {
        self.schemas.get(nsid)
    }
}
