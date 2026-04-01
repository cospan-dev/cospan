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
    for vid in src.vertices.keys() {
        if tgt.has_vertex(vid) {
            vertex_map.insert(vid.clone(), vid.clone());
        }
    }
    let mut edge_map = HashMap::new();
    for edge in src.edges.keys() {
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

/// Build a morphism that renames NSID prefixes (mapping all shared field names)
/// and additionally applies explicit field renames.
///
/// `renames` is a list of `(src_field, tgt_field)` pairs for property names
/// that differ between the two schemas (e.g., `("knot", "node")`).
/// Fields with the same name are mapped automatically.
fn renamed_morphism(
    src: &Schema,
    tgt: &Schema,
    src_nsid: &str,
    tgt_nsid: &str,
    renames: &[(&str, &str)],
) -> Migration {
    let src_body = format!("{src_nsid}:body");
    let tgt_body = format!("{tgt_nsid}:body");

    let mut vertex_map: HashMap<Name, Name> = HashMap::new();
    let mut edge_map: HashMap<Edge, Edge> = HashMap::new();

    // Map the record vertex and body vertex
    if src.has_vertex(&Name::from(src_nsid.to_string())) {
        vertex_map.insert(
            Name::from(src_nsid.to_string()),
            Name::from(tgt_nsid.to_string()),
        );
    }
    if src.has_vertex(&Name::from(src_body.clone())) {
        vertex_map.insert(Name::from(src_body.clone()), Name::from(tgt_body.clone()));
    }

    // Build a lookup of renamed fields: src_field -> tgt_field
    let rename_map: HashMap<&str, &str> = renames.iter().copied().collect();

    // Walk source vertices to find body.X vertices and map them
    for vid in src.vertices.keys() {
        let vid_str = vid.to_string();
        if let Some(suffix) = vid_str.strip_prefix(&format!("{src_body}.")) {
            // Check if this is a top-level field (no further dots) or a sub-object field
            let top_field = suffix.split('.').next().unwrap_or(suffix);
            let rest = suffix.strip_prefix(top_field).unwrap_or("");

            let tgt_top = rename_map.get(top_field).copied().unwrap_or(top_field);
            let tgt_vid = format!("{tgt_body}.{tgt_top}{rest}");
            let tgt_name = Name::from(tgt_vid);
            if tgt.has_vertex(&tgt_name) {
                vertex_map.insert(vid.clone(), tgt_name);
            }
        }
    }

    // Map the record-schema edge (record → body)
    let src_rec_edge = Edge {
        src: Name::from(src_nsid.to_string()),
        tgt: Name::from(src_body.clone()),
        kind: Name::from("record-schema".to_string()),
        name: None,
    };
    let tgt_rec_edge = Edge {
        src: Name::from(tgt_nsid.to_string()),
        tgt: Name::from(tgt_body.clone()),
        kind: Name::from("record-schema".to_string()),
        name: None,
    };
    if src.edges.contains_key(&src_rec_edge) && tgt.edges.contains_key(&tgt_rec_edge) {
        edge_map.insert(src_rec_edge, tgt_rec_edge);
    }

    // Walk source edges to find property edges and map them
    for edge in src.edges.keys() {
        if edge.kind != "prop" {
            continue;
        }
        let src_str = edge.src.to_string();
        if !src_str.starts_with(&src_body) {
            continue;
        }

        // Determine the parent path relative to src_body
        let parent_suffix = src_str.strip_prefix(&src_body).unwrap_or("");
        // Map the edge name through renames if it's a top-level field
        let edge_name = edge
            .name
            .as_ref()
            .map(|n| n.to_string())
            .unwrap_or_default();
        let is_top_level = parent_suffix.is_empty();

        let tgt_edge_name = if is_top_level {
            rename_map
                .get(edge_name.as_str())
                .copied()
                .unwrap_or(&edge_name)
                .to_string()
        } else {
            edge_name.clone()
        };

        // Build the target parent path
        let tgt_parent = if parent_suffix.is_empty() {
            tgt_body.clone()
        } else {
            // Map each segment of the parent suffix
            let suffix = parent_suffix.strip_prefix('.').unwrap_or(parent_suffix);
            let top = suffix.split('.').next().unwrap_or(suffix);
            let rest = suffix.strip_prefix(top).unwrap_or("");
            let tgt_top = rename_map.get(top).copied().unwrap_or(top);
            format!("{tgt_body}.{tgt_top}{rest}")
        };

        // Build target edge
        let tgt_tgt_vid = if tgt_edge_name.is_empty() {
            tgt_parent.clone()
        } else {
            format!("{tgt_parent}.{tgt_edge_name}")
        };

        let tgt_edge = Edge {
            src: Name::from(tgt_parent),
            tgt: Name::from(tgt_tgt_vid),
            kind: edge.kind.clone(),
            name: if tgt_edge_name.is_empty() {
                None
            } else {
                Some(Name::from(tgt_edge_name))
            },
        };

        if tgt.edges.contains_key(&tgt_edge) {
            edge_map.insert(edge.clone(), tgt_edge);
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
        // Field names match across NSIDs — renamed_morphism maps the NSID-prefixed
        // vertex IDs while preserving field-level structure.
        InteropMorphism {
            tangled_nsid: "sh.tangled.feed.star",
            cospan_nsid: "dev.cospan.feed.star",
            build_migration: |src, tgt| {
                renamed_morphism(
                    src,
                    tgt,
                    "sh.tangled.feed.star",
                    "dev.cospan.feed.star",
                    &[],
                )
            },
        },
        InteropMorphism {
            tangled_nsid: "sh.tangled.graph.follow",
            cospan_nsid: "dev.cospan.graph.follow",
            build_migration: |src, tgt| {
                renamed_morphism(
                    src,
                    tgt,
                    "sh.tangled.graph.follow",
                    "dev.cospan.graph.follow",
                    &[],
                )
            },
        },
        InteropMorphism {
            tangled_nsid: "sh.tangled.feed.reaction",
            cospan_nsid: "dev.cospan.feed.reaction",
            build_migration: |src, tgt| {
                renamed_morphism(
                    src,
                    tgt,
                    "sh.tangled.feed.reaction",
                    "dev.cospan.feed.reaction",
                    &[],
                )
            },
        },
        InteropMorphism {
            tangled_nsid: "sh.tangled.repo.issue",
            cospan_nsid: "dev.cospan.repo.issue",
            build_migration: |src, tgt| {
                renamed_morphism(
                    src,
                    tgt,
                    "sh.tangled.repo.issue",
                    "dev.cospan.repo.issue",
                    &[],
                )
            },
        },
        InteropMorphism {
            tangled_nsid: "sh.tangled.repo.issue.comment",
            cospan_nsid: "dev.cospan.repo.issue.comment",
            build_migration: |src, tgt| {
                renamed_morphism(
                    src,
                    tgt,
                    "sh.tangled.repo.issue.comment",
                    "dev.cospan.repo.issue.comment",
                    &[],
                )
            },
        },
        InteropMorphism {
            tangled_nsid: "sh.tangled.repo.issue.state",
            cospan_nsid: "dev.cospan.repo.issue.state",
            build_migration: |src, tgt| {
                renamed_morphism(
                    src,
                    tgt,
                    "sh.tangled.repo.issue.state",
                    "dev.cospan.repo.issue.state",
                    &[],
                )
            },
        },
        InteropMorphism {
            tangled_nsid: "sh.tangled.repo.pull.comment",
            cospan_nsid: "dev.cospan.repo.pull.comment",
            build_migration: |src, tgt| {
                renamed_morphism(
                    src,
                    tgt,
                    "sh.tangled.repo.pull.comment",
                    "dev.cospan.repo.pull.comment",
                    &[],
                )
            },
        },
        InteropMorphism {
            tangled_nsid: "sh.tangled.repo.collaborator",
            cospan_nsid: "dev.cospan.repo.collaborator",
            build_migration: |src, tgt| {
                renamed_morphism(
                    src,
                    tgt,
                    "sh.tangled.repo.collaborator",
                    "dev.cospan.repo.collaborator",
                    &[("subject", "did")],
                )
            },
        },
        InteropMorphism {
            tangled_nsid: "sh.tangled.actor.profile",
            cospan_nsid: "dev.cospan.actor.profile",
            build_migration: |src, tgt| {
                renamed_morphism(
                    src,
                    tgt,
                    "sh.tangled.actor.profile",
                    "dev.cospan.actor.profile",
                    &[],
                )
            },
        },
        InteropMorphism {
            tangled_nsid: "sh.tangled.label.definition",
            cospan_nsid: "dev.cospan.label.definition",
            build_migration: |src, tgt| {
                renamed_morphism(
                    src,
                    tgt,
                    "sh.tangled.label.definition",
                    "dev.cospan.label.definition",
                    &[], // field names that overlap (name, color, description, createdAt) match
                )
            },
        },
        // ── Renamed-field morphisms ──────────────────────────────────
        InteropMorphism {
            tangled_nsid: "sh.tangled.repo",
            cospan_nsid: "dev.cospan.repo",
            build_migration: |src, tgt| {
                renamed_morphism(
                    src,
                    tgt,
                    "sh.tangled.repo",
                    "dev.cospan.repo",
                    &[("knot", "node")],
                    // name, description, createdAt match; defaultBranch/visibility only in target
                )
            },
        },
        InteropMorphism {
            tangled_nsid: "sh.tangled.repo.pull",
            cospan_nsid: "dev.cospan.repo.pull",
            build_migration: |src, tgt| {
                renamed_morphism(
                    src,
                    tgt,
                    "sh.tangled.repo.pull",
                    "dev.cospan.repo.pull",
                    &[],
                    // title, body, mentions, references, createdAt match directly;
                    // target/source are sub-objects with different structure — the
                    // renamed_morphism maps what overlaps automatically
                )
            },
        },
        InteropMorphism {
            tangled_nsid: "sh.tangled.git.refUpdate",
            cospan_nsid: "dev.cospan.vcs.refUpdate",
            build_migration: |src, tgt| {
                renamed_morphism(
                    src,
                    tgt,
                    "sh.tangled.git.refUpdate",
                    "dev.cospan.vcs.refUpdate",
                    &[("oldSha", "oldTarget"), ("newSha", "newTarget")],
                    // ref and committerDid match; repoDid/repoName → repo handled by DB transforms
                )
            },
        },
        InteropMorphism {
            tangled_nsid: "sh.tangled.pipeline",
            cospan_nsid: "dev.cospan.pipeline",
            build_migration: |src, tgt| {
                renamed_morphism(
                    src,
                    tgt,
                    "sh.tangled.pipeline",
                    "dev.cospan.pipeline",
                    &[],
                    // workflows match; triggerMetadata→repo/commitId handled by DB transforms
                )
            },
        },
        InteropMorphism {
            tangled_nsid: "sh.tangled.knot",
            cospan_nsid: "dev.cospan.node",
            build_migration: |src, tgt| {
                renamed_morphism(
                    src,
                    tgt,
                    "sh.tangled.knot",
                    "dev.cospan.node",
                    &[],
                    // createdAt matches; publicEndpoint only in target (optional)
                )
            },
        },
        // ── Tangled Spindle → Cospan Org ─────────────────────────────
        InteropMorphism {
            tangled_nsid: "sh.tangled.spindle",
            cospan_nsid: "dev.cospan.org",
            build_migration: |src, tgt| {
                renamed_morphism(
                    src,
                    tgt,
                    "sh.tangled.spindle",
                    "dev.cospan.org",
                    &[],
                    // createdAt matches; name/description only in target
                )
            },
        },
        InteropMorphism {
            tangled_nsid: "sh.tangled.spindle.member",
            cospan_nsid: "dev.cospan.org.member",
            build_migration: |src, tgt| {
                renamed_morphism(
                    src,
                    tgt,
                    "sh.tangled.spindle.member",
                    "dev.cospan.org.member",
                    &[("subject", "member"), ("instance", "org")],
                    // createdAt matches; role only in target
                )
            },
        },
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
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, index);
            } else if path.extension().is_some_and(|e| e == "json")
                && let Ok(content) = std::fs::read_to_string(&path)
                && let Ok(json) = serde_json::from_str::<serde_json::Value>(&content)
                && let Some(id) = json.get("id").and_then(|v| v.as_str())
            {
                index.insert(id.to_string(), path);
            }
        }
    }
    walk(lexicons_dir, &mut index);
    index
}

/// Compile all interop morphisms and serialize them.
pub fn compile_all_morphisms(lexicons_dir: &Path) -> Result<Vec<CompiledInterop>> {
    // Load lens files for field transforms
    let lenses_dir = lexicons_dir.parent().unwrap_or(lexicons_dir).join("lenses");
    let lenses = crate::lens_config::load_all_lenses(&lenses_dir).unwrap_or_default();

    let nsid_index = build_nsid_index(lexicons_dir);
    let morphisms = all_interop_morphisms();
    let mut results = Vec::new();

    for m in &morphisms {
        match compile_one(lexicons_dir, m, &nsid_index, &lenses) {
            Ok(compiled) => {
                println!("  Compiled interop: {} → {}", m.tangled_nsid, m.cospan_nsid);
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
    lenses: &[crate::lens_config::LensFile],
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
    let mut compiled = panproto_mig::compile(&tangled_schema, &cospan_schema, &migration)
        .map_err(|e| anyhow::anyhow!("compile: {e:?}"))?;

    // Field transforms must be keyed by SOURCE (Tangled) vertex names because
    // lift_wtype_sigma looks them up by `child_node.anchor` before remapping.
    // Both tangled_transforms and db_transforms return Cospan vertex keys,
    // so we remap them to the corresponding Tangled source vertex.
    let reverse_vertex_map: HashMap<Name, Name> = migration
        .vertex_map
        .iter()
        .map(|(src, tgt)| (tgt.clone(), src.clone()))
        .collect();

    // Inject field transforms from lens files (replaces db_projection.rs)
    let cospan_body = format!("{}:body", m.cospan_nsid);

    // Tangled interop lens (type coercions, semantic mappings)
    if let Some(interop_lens) = crate::lens_config::find_by_source(lenses, m.tangled_nsid) {
        let tangled_transforms =
            crate::lens_config::steps_to_value_transforms(&interop_lens.steps, &cospan_body);
        for (cospan_vertex, transforms) in tangled_transforms {
            let key = reverse_vertex_map
                .get(&cospan_vertex)
                .cloned()
                .unwrap_or(cospan_vertex);
            compiled
                .field_transforms
                .entry(key)
                .or_default()
                .extend(transforms);
        }
    }

    // DB projection lens (AT-URI decomposition, renames, defaults)
    if let Some(db_lens) = crate::lens_config::find_by_source(lenses, m.cospan_nsid) {
        let db_transforms =
            crate::lens_config::steps_to_value_transforms(&db_lens.steps, &cospan_body);
        for (cospan_vertex, transforms) in db_transforms {
            let key = reverse_vertex_map
                .get(&cospan_vertex)
                .cloned()
                .unwrap_or(cospan_vertex);
            compiled
                .field_transforms
                .entry(key)
                .or_default()
                .extend(transforms);
        }
    }

    Ok(CompiledInterop {
        tangled_nsid: m.tangled_nsid.to_string(),
        cospan_nsid: m.cospan_nsid.to_string(),
        tangled_schema,
        cospan_schema,
        compiled,
        quality_report,
    })
}

/// A compiled DB projection for a Cospan record type.
/// Used for direct Cospan records (not Tangled interop).
#[derive(serde::Serialize, serde::Deserialize)]
pub struct CompiledDbProjection {
    pub nsid: String,
    pub schema: Schema,
    pub compiled: panproto_inst::CompiledMigration,
}

/// Compile DB projections for all Cospan record types.
/// NSIDs are discovered from lens files (db-projection lenses).
pub fn compile_db_projections(lexicons_dir: &Path) -> Result<Vec<CompiledDbProjection>> {
    let lenses_dir = lexicons_dir.parent().unwrap_or(lexicons_dir).join("lenses");
    let lenses = crate::lens_config::load_all_lenses(&lenses_dir).unwrap_or_default();
    let db_lenses = crate::lens_config::db_projection_lenses(&lenses);

    let nsid_index = build_nsid_index(lexicons_dir);
    let mut results = Vec::new();

    for lens in &db_lenses {
        let nsid = &lens.source;
        let path = nsid_index
            .get(nsid.as_str())
            .cloned()
            .unwrap_or_else(|| nsid_to_path(lexicons_dir, nsid));

        let json_str = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let json: serde_json::Value = serde_json::from_str(&json_str)?;
        let schema = atproto::parse_lexicon(&json).with_context(|| format!("parsing {nsid}"))?;

        // Build identity migration (same schema in and out)
        let migration = identity_morphism(&schema, &schema);
        let mut compiled = panproto_mig::compile(&schema, &schema, &migration)
            .map_err(|e| anyhow::anyhow!("compile db projection for {nsid}: {e:?}"))?;

        // Inject field transforms from lens file (replaces db_projection.rs)
        let body_vertex = format!("{nsid}:body");
        let transforms = crate::lens_config::steps_to_value_transforms(&lens.steps, &body_vertex);
        for (vertex, field_transforms) in transforms {
            compiled
                .field_transforms
                .entry(vertex)
                .or_default()
                .extend(field_transforms);
        }

        println!("  Compiled DB projection: {nsid}");
        results.push(CompiledDbProjection {
            nsid: nsid.to_string(),
            schema,
            compiled,
        });
    }

    Ok(results)
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
