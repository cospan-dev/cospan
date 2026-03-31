//! Schema-driven record transformation via pre-compiled panproto morphisms.
//!
//! All record transformations flow through panproto:
//!
//! **Cospan records**: parse_json → lift (DB projection transforms) → to_json
//! **Tangled records**: parse_json → lift (Tangled→Cospan + DB transforms) → to_json
//!
//! Morphisms and field transforms are compiled at codegen time and serialized.
//! This module loads them at startup and applies them at runtime.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use panproto_inst::CompiledMigration;
use panproto_mig::lift_wtype_sigma;
use panproto_schema::Schema;

/// A pre-compiled Tangled → Cospan morphism (includes DB projection).
#[derive(serde::Deserialize)]
pub struct CompiledInterop {
    pub tangled_nsid: String,
    pub cospan_nsid: String,
    pub tangled_schema: Schema,
    pub cospan_schema: Schema,
    pub compiled: CompiledMigration,
    pub quality_report: String,
}

/// A pre-compiled Cospan → Database projection.
#[derive(serde::Deserialize)]
pub struct CompiledDbProjection {
    pub nsid: String,
    pub schema: Schema,
    pub compiled: CompiledMigration,
}

/// Registry of all pre-compiled panproto morphisms.
pub struct RecordTransformer {
    /// Tangled NSID → compiled Tangled→Cospan+DB morphism
    tangled_morphisms: HashMap<String, CompiledInterop>,
    /// Cospan NSID → compiled Cospan→DB projection
    db_projections: HashMap<String, CompiledDbProjection>,
}

impl RecordTransformer {
    /// Create an empty transformer (no morphisms loaded).
    pub fn empty() -> Self {
        Self {
            tangled_morphisms: HashMap::new(),
            db_projections: HashMap::new(),
        }
    }

    /// Load pre-compiled morphisms from codegen output.
    pub fn load(lexicons_dir: &Path) -> Result<Self> {
        let workspace_root = lexicons_dir
            .parent()
            .and_then(|p| p.parent())
            .unwrap_or(lexicons_dir);
        let interop_dir = workspace_root.join("generated/interop");

        // Load Tangled morphisms
        let morphisms_path = interop_dir.join("compiled_morphisms.msgpack");
        let tangled_morphisms = if morphisms_path.exists() {
            let bytes = std::fs::read(&morphisms_path)
                .with_context(|| format!("reading {}", morphisms_path.display()))?;
            let interops: Vec<CompiledInterop> = rmp_serde::from_slice(&bytes)
                .with_context(|| "deserializing compiled morphisms")?;
            let mut map = HashMap::new();
            for interop in interops {
                tracing::info!(
                    tangled = %interop.tangled_nsid,
                    cospan = %interop.cospan_nsid,
                    quality = %interop.quality_report,
                    "loaded tangled morphism"
                );
                map.insert(interop.tangled_nsid.clone(), interop);
            }
            map
        } else {
            tracing::warn!("no compiled morphisms found, tangled interop disabled");
            HashMap::new()
        };

        // Load DB projections
        let projections_path = interop_dir.join("db_projections.msgpack");
        let db_projections = if projections_path.exists() {
            let bytes = std::fs::read(&projections_path)
                .with_context(|| format!("reading {}", projections_path.display()))?;
            let projections: Vec<CompiledDbProjection> =
                rmp_serde::from_slice(&bytes).with_context(|| "deserializing DB projections")?;
            let mut map = HashMap::new();
            for proj in projections {
                tracing::info!(nsid = %proj.nsid, "loaded DB projection");
                map.insert(proj.nsid.clone(), proj);
            }
            map
        } else {
            anyhow::bail!(
                "no DB projections found at {}. Run `cargo run -p cospan-codegen` first.",
                projections_path.display()
            );
        };

        tracing::info!(
            tangled = tangled_morphisms.len(),
            db = db_projections.len(),
            "loaded panproto morphisms"
        );

        Ok(Self {
            tangled_morphisms,
            db_projections,
        })
    }

    /// Transform a record through the appropriate panproto morphism.
    ///
    /// For Cospan records: applies DB projection (AT-URI decomposition, renames).
    /// For Tangled records: applies Tangled→Cospan morphism + DB projection.
    /// Returns None if no morphism exists for this NSID.
    pub fn transform(
        &self,
        collection: &str,
        record: &serde_json::Value,
    ) -> Option<Result<serde_json::Value>> {
        if collection.starts_with("sh.tangled.") {
            self.transform_tangled(collection, record)
        } else {
            self.transform_cospan(collection, record)
        }
    }

    fn transform_cospan(
        &self,
        nsid: &str,
        record: &serde_json::Value,
    ) -> Option<Result<serde_json::Value>> {
        let proj = self.db_projections.get(nsid)?;
        Some(apply_projection(&proj.schema, &proj.compiled, nsid, record))
    }

    fn transform_tangled(
        &self,
        tangled_nsid: &str,
        record: &serde_json::Value,
    ) -> Option<Result<serde_json::Value>> {
        let morphism = self.tangled_morphisms.get(tangled_nsid)?;
        let result = apply_morphism(morphism, record);
        if let Ok(ref json) = result {
            tracing::debug!(
                tangled = tangled_nsid,
                cospan = %morphism.cospan_nsid,
                has_repo_did = json.get("repoDid").is_some(),
                has_repo = json.get("repo").is_some(),
                keys = ?json.as_object().map(|o| o.keys().collect::<Vec<_>>()),
                "tangled transform output"
            );
        }
        Some(result)
    }
}

/// Apply a DB projection: parse → lift (with field transforms) → emit.
fn apply_projection(
    schema: &Schema,
    compiled: &CompiledMigration,
    nsid: &str,
    record: &serde_json::Value,
) -> Result<serde_json::Value> {
    let instance = panproto_inst::parse::parse_json(schema, nsid, record)
        .map_err(|e| anyhow::anyhow!("parse {nsid}: {e:?}"))?;

    let lifted = lift_wtype_sigma(compiled, schema, &instance)
        .map_err(|e| anyhow::anyhow!("project {nsid}: {e:?}"))?;

    Ok(panproto_inst::parse::to_json(schema, &lifted))
}

/// Apply a Tangled→Cospan morphism: parse → lift (rename + DB transforms) → emit.
fn apply_morphism(
    mapping: &CompiledInterop,
    record: &serde_json::Value,
) -> Result<serde_json::Value> {
    let instance =
        panproto_inst::parse::parse_json(&mapping.tangled_schema, &mapping.tangled_nsid, record)
            .map_err(|e| anyhow::anyhow!("parse {}: {e:?}", mapping.tangled_nsid))?;

    let lifted =
        lift_wtype_sigma(&mapping.compiled, &mapping.cospan_schema, &instance).map_err(|e| {
            anyhow::anyhow!(
                "lift {} → {}: {e:?}",
                mapping.tangled_nsid,
                mapping.cospan_nsid
            )
        })?;

    Ok(panproto_inst::parse::to_json(
        &mapping.cospan_schema,
        &lifted,
    ))
}
