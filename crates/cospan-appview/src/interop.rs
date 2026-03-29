//! Tangled → Cospan interop via pre-compiled panproto morphisms.
//!
//! Morphisms are defined explicitly and compiled at codegen time
//! (see cospan-codegen/src/tangled_interop.rs). The compiled migrations
//! are serialized to generated/interop/compiled_morphisms.json.
//!
//! At runtime, this module loads the compiled migrations and applies
//! them to incoming Tangled Jetstream records using panproto's
//! `lift_wtype_sigma()`.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use panproto_inst::CompiledMigration;
use panproto_mig::lift_wtype_sigma;
use panproto_schema::Schema;

/// A pre-compiled interop mapping loaded from codegen output.
#[derive(serde::Deserialize)]
pub struct CompiledInterop {
    pub tangled_nsid: String,
    pub cospan_nsid: String,
    pub tangled_schema: Schema,
    pub cospan_schema: Schema,
    pub compiled: CompiledMigration,
    pub quality_report: String,
}

/// Registry of pre-compiled Tangled → Cospan morphisms.
pub struct TangledInterop {
    mappings: HashMap<String, CompiledInterop>,
}

impl TangledInterop {
    /// Create an empty registry (no morphisms loaded).
    pub fn empty() -> Self {
        Self {
            mappings: HashMap::new(),
        }
    }

    /// Load pre-compiled morphisms from the codegen output file.
    pub fn load(lexicons_dir: &Path) -> Result<Self> {
        // The compiled morphisms are at generated/interop/compiled_morphisms.json
        // relative to the workspace root. lexicons_dir is packages/lexicons/,
        // so workspace root is two levels up.
        let workspace_root = lexicons_dir
            .parent()
            .and_then(|p| p.parent())
            .unwrap_or(lexicons_dir);
        let morphisms_path = workspace_root.join("generated/interop/compiled_morphisms.json");

        if !morphisms_path.exists() {
            anyhow::bail!(
                "compiled morphisms not found at {}. Run `cargo run -p cospan-codegen` first.",
                morphisms_path.display()
            );
        }

        let json = std::fs::read_to_string(&morphisms_path)
            .with_context(|| format!("reading {}", morphisms_path.display()))?;
        let interops: Vec<CompiledInterop> = serde_json::from_str(&json)
            .with_context(|| "deserializing compiled morphisms")?;

        let mut mappings = HashMap::new();
        for interop in interops {
            tracing::info!(
                tangled = %interop.tangled_nsid,
                cospan = %interop.cospan_nsid,
                quality = %interop.quality_report,
                "loaded compiled interop morphism"
            );
            mappings.insert(interop.tangled_nsid.clone(), interop);
        }

        tracing::info!(count = mappings.len(), "loaded tangled interop morphisms");
        Ok(Self { mappings })
    }

    /// Transform a Tangled JSON record to Cospan JSON using the pre-compiled
    /// morphism. Returns None if no morphism exists for this NSID.
    pub fn transform(
        &self,
        tangled_nsid: &str,
        record: &serde_json::Value,
    ) -> Option<Result<serde_json::Value>> {
        let mapping = self.mappings.get(tangled_nsid)?;
        Some(apply_lift(mapping, record))
    }
}

/// Apply the pre-compiled morphism: parse → lift → emit.
fn apply_lift(
    mapping: &CompiledInterop,
    record: &serde_json::Value,
) -> Result<serde_json::Value> {
    // Parse the Tangled JSON into a panproto WInstance
    let instance = panproto_inst::parse::parse_json(
        &mapping.tangled_schema,
        &mapping.tangled_nsid,
        record,
    )
    .map_err(|e| anyhow::anyhow!("parse {}: {e:?}", mapping.tangled_nsid))?;

    // Lift through the pre-compiled morphism (Sigma for field renames)
    let lifted = lift_wtype_sigma(
        &mapping.compiled,
        &mapping.cospan_schema,
        &instance,
    )
    .map_err(|e| anyhow::anyhow!("lift {} → {}: {e:?}", mapping.tangled_nsid, mapping.cospan_nsid))?;

    // Emit back to JSON in the Cospan schema shape
    Ok(panproto_inst::parse::to_json(&mapping.cospan_schema, &lifted))
}
