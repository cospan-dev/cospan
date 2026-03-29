//! Tangled → Cospan interop morphisms defined as panproto Migrations.
//!
//! Each morphism is an explicit vertex_map + edge_map specifying how
//! Tangled Lexicon fields map to Cospan Lexicon fields. These are compiled
//! at codegen time using `panproto_mig::compile()` and serialized for
//! the appview to load and apply at runtime via `lift_wtype_sigma()`.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use panproto_gat::Name;
use panproto_mig::Migration;
use panproto_protocols::web_document::atproto;
use panproto_schema::{Edge, Schema};

/// A pair of (tangled_nsid, cospan_nsid) with their explicit morphism.
pub struct InteropMorphism {
    pub tangled_nsid: &'static str,
    pub cospan_nsid: &'static str,
    /// Build the explicit Migration for this pair given the parsed schemas.
    pub build_migration: fn(&Schema, &Schema) -> Migration,
}

// migration_from_maps helper removed — use identity_morphism or build Migration directly

/// Build a morphism by matching vertices and edges with the same IDs
/// between source and target schemas.
fn identity_morphism(src: &Schema, tgt: &Schema) -> Migration {
    let mut vertex_map = HashMap::new();
    for (vid, _) in &src.vertices {
        if tgt.has_vertex(vid) {
            vertex_map.insert(vid.clone(), vid.clone());
        }
    }
    let mut edge_map = HashMap::new();
    for (edge, _) in &src.edges {
        if tgt.edges.contains_key(edge) {
            edge_map.insert(edge.clone(), edge.clone());
        }
    }
    Migration {
        vertex_map,
        edge_map,
        hyper_edge_map: HashMap::new(),
        label_map: HashMap::new(),
        resolver: HashMap::new(),
        hyper_resolver: HashMap::new(),
        expr_resolvers: HashMap::new(),
    }
}

/// All known Tangled → Cospan interop morphisms.
pub fn all_interop_morphisms() -> Vec<InteropMorphism> {
    vec![
        // These pairs have identical or near-identical Lexicon schemas.
        // field names match, so an identity morphism works.
        InteropMorphism {
            tangled_nsid: "sh.tangled.feed.star",
            cospan_nsid: "dev.cospan.feed.star",
            build_migration: |src, tgt| identity_morphism(src, tgt),
        },
        InteropMorphism {
            tangled_nsid: "sh.tangled.graph.follow",
            cospan_nsid: "dev.cospan.graph.follow",
            build_migration: |src, tgt| identity_morphism(src, tgt),
        },
        InteropMorphism {
            tangled_nsid: "sh.tangled.feed.reaction",
            cospan_nsid: "dev.cospan.feed.reaction",
            build_migration: |src, tgt| identity_morphism(src, tgt),
        },
        InteropMorphism {
            tangled_nsid: "sh.tangled.repo.issue",
            cospan_nsid: "dev.cospan.repo.issue",
            build_migration: |src, tgt| identity_morphism(src, tgt),
        },
        InteropMorphism {
            tangled_nsid: "sh.tangled.repo.issue.comment",
            cospan_nsid: "dev.cospan.repo.issue.comment",
            build_migration: |src, tgt| identity_morphism(src, tgt),
        },
        InteropMorphism {
            tangled_nsid: "sh.tangled.repo.issue.state",
            cospan_nsid: "dev.cospan.repo.issue.state",
            build_migration: |src, tgt| identity_morphism(src, tgt),
        },
        InteropMorphism {
            tangled_nsid: "sh.tangled.repo.pull.comment",
            cospan_nsid: "dev.cospan.repo.pull.comment",
            build_migration: |src, tgt| identity_morphism(src, tgt),
        },
        InteropMorphism {
            tangled_nsid: "sh.tangled.repo.collaborator",
            cospan_nsid: "dev.cospan.repo.collaborator",
            build_migration: |src, tgt| identity_morphism(src, tgt),
        },
        InteropMorphism {
            tangled_nsid: "sh.tangled.actor.profile",
            cospan_nsid: "dev.cospan.actor.profile",
            build_migration: |src, tgt| identity_morphism(src, tgt),
        },
        InteropMorphism {
            tangled_nsid: "sh.tangled.label.definition",
            cospan_nsid: "dev.cospan.label.definition",
            build_migration: |src, tgt| identity_morphism(src, tgt),
        },
        // TODO: These have different schemas that need explicit vertex/edge maps:
        // - sh.tangled.repo → dev.cospan.repo (knot → node, no defaultBranch/visibility)
        // - sh.tangled.repo.pull → dev.cospan.repo.pull (sourceBranch→sourceRef, targetBranch→targetRef)
        // - sh.tangled.git.refUpdate → dev.cospan.vcs.refUpdate (repoDid/repoName→repo, oldSha→oldTarget, newSha→newTarget)
        // - sh.tangled.pipeline → dev.cospan.pipeline (triggerMetadata→repo/commitId)
        // - sh.tangled.knot → dev.cospan.node (hostname→publicEndpoint)
    ]
}

/// Serializable compiled interop result for a single NSID pair.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct CompiledInterop {
    pub tangled_nsid: String,
    pub cospan_nsid: String,
    pub tangled_schema: Schema,
    pub cospan_schema: Schema,
    pub compiled: panproto_inst::CompiledMigration,
    pub quality_report: String,
}

/// Build an index of NSID → file path by scanning all Lexicon JSON files.
fn build_nsid_index(lexicons_dir: &Path) -> HashMap<String, std::path::PathBuf> {
    let mut index = HashMap::new();
    fn walk(dir: &Path, index: &mut HashMap<String, std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, index);
            } else if path.extension().is_some_and(|e| e == "json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(id) = json.get("id").and_then(|v| v.as_str()) {
                            index.insert(id.to_string(), path);
                        }
                    }
                }
            }
        }
    }
    walk(lexicons_dir, &mut index);
    index
}

/// Compile all interop morphisms and serialize them.
pub fn compile_all_morphisms(lexicons_dir: &Path) -> Result<Vec<CompiledInterop>> {
    let nsid_index = build_nsid_index(lexicons_dir);
    let morphisms = all_interop_morphisms();
    let mut results = Vec::new();

    for m in &morphisms {
        match compile_one(lexicons_dir, m, &nsid_index) {
            Ok(compiled) => {
                println!(
                    "  Compiled interop: {} → {}",
                    m.tangled_nsid, m.cospan_nsid
                );
                results.push(compiled);
            }
            Err(e) => {
                eprintln!(
                    "  warn: interop {} → {}: {e}",
                    m.tangled_nsid, m.cospan_nsid
                );
            }
        }
    }

    Ok(results)
}

fn compile_one(
    lexicons_dir: &Path,
    m: &InteropMorphism,
    nsid_index: &HashMap<String, std::path::PathBuf>,
) -> Result<CompiledInterop> {
    let tangled_path = nsid_index
        .get(m.tangled_nsid)
        .cloned()
        .unwrap_or_else(|| nsid_to_path(lexicons_dir, m.tangled_nsid));
    let cospan_path = nsid_index
        .get(m.cospan_nsid)
        .cloned()
        .unwrap_or_else(|| nsid_to_path(lexicons_dir, m.cospan_nsid));

    let tangled_json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&tangled_path)
            .with_context(|| format!("reading {}", tangled_path.display()))?,
    )?;
    let cospan_json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&cospan_path)
            .with_context(|| format!("reading {}", cospan_path.display()))?,
    )?;

    let tangled_schema = atproto::parse_lexicon(&tangled_json)
        .with_context(|| format!("parsing {}", m.tangled_nsid))?;
    let cospan_schema = atproto::parse_lexicon(&cospan_json)
        .with_context(|| format!("parsing {}", m.cospan_nsid))?;

    // Build the explicit migration
    let migration = (m.build_migration)(&tangled_schema, &cospan_schema);

    let quality_report = format!(
        "vertex_map: {}/{} src vertices, edge_map: {}/{} src edges",
        migration.vertex_map.len(),
        tangled_schema.vertices.len(),
        migration.edge_map.len(),
        tangled_schema.edges.len(),
    );

    // Compile for runtime application
    let compiled = panproto_mig::compile(&tangled_schema, &cospan_schema, &migration)
        .map_err(|e| anyhow::anyhow!("compile: {e:?}"))?;

    Ok(CompiledInterop {
        tangled_nsid: m.tangled_nsid.to_string(),
        cospan_nsid: m.cospan_nsid.to_string(),
        tangled_schema,
        cospan_schema,
        compiled,
        quality_report,
    })
}

fn nsid_to_path(lexicons_dir: &Path, nsid: &str) -> std::path::PathBuf {
    let parts: Vec<&str> = nsid.split('.').collect();
    let mut path = lexicons_dir.to_path_buf();
    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            path.push(format!("{part}.json"));
        } else {
            path.push(part);
        }
    }
    path
}
