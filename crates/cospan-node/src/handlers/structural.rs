//! Panproto-powered structural diff for any file type panproto knows how
//! to parse.
//!
//! This is what makes Cospan a *schematic* version control system and not a
//! textual one. For every changed blob, we:
//!
//! 1. Parse both sides into a panproto `Schema` via
//!    [`panproto_parse::ParserRegistry::parse_file`]: the generic
//!    tree-sitter walker covers 248 programming languages with one code
//!    path.
//! 2. Fall back to content-based protocol detection for structured
//!    configuration files that tree-sitter can't classify by extension
//!    alone: ATProto lexicons (`{ "lexicon": 1, "id": ..., "defs": ... }`),
//!    JSON/YAML schemas, OpenAPI, Avro, etc. These go through
//!    `panproto-protocols`.
//! 3. Run [`panproto_check::diff`] to produce a `SchemaDiff` recording
//!    every added/removed/modified vertex, edge, constraint, and NSID.
//! 4. Run [`panproto_check::classify`] to split the diff into
//!    breaking vs non-breaking changes against the schema's protocol.
//!
//! The result is rendered directly by the frontend: no line-level
//! diff, but a tree view: "removed vertex X", "added edge A→B",
//! "constraint tightened on Y (breaking)", with a global
//! COMPATIBLE / BREAKING verdict and counts for each class.
//!
//! Everything that can be computed via panproto IS. This module does
//! zero hand-coded JSON traversal of schemas: it just parses through
//! panproto and forwards the result.

use panproto_check::{BreakingChange, CompatReport, NonBreakingChange, SchemaDiff};
use panproto_check::scope::{report_by_scope, report_scope_json};
use panproto_parse::ParserRegistry;
use panproto_protocols::web_document::atproto;
use panproto_schema::{Protocol, Schema};
use serde_json::{Value, json};

/// Top-level structural diff attached to each file entry in the
/// `diffCommits` response.
#[derive(Debug, Clone)]
pub struct StructuralDiff {
    /// The panproto protocol we classified the file as (e.g.
    /// `"typescript"`, `"rust"`, `"atproto-lexicon"`).
    pub protocol: String,
    pub report: CompatReport,
    pub raw_diff: SchemaDiff,
    pub old_schema: Schema,
    pub new_schema: Schema,
    pub old_vertex_count: usize,
    pub new_vertex_count: usize,
    pub old_edge_count: usize,
    pub new_edge_count: usize,
}

/// Attempt to compute a structural diff between two blob contents.
///
/// Returns `None` if neither side could be parsed: the frontend falls
/// back to the textual diff. If only one side parses (added/removed
/// file) we diff against an empty schema in the same protocol so the
/// result still has all the relevant adds or removes.
pub fn try_structural_diff(
    registry: &ParserRegistry,
    path: &str,
    old_bytes: Option<&[u8]>,
    new_bytes: Option<&[u8]>,
) -> Option<StructuralDiff> {
    let old_parsed = old_bytes.and_then(|b| parse_any(registry, path, b));
    let new_parsed = new_bytes.and_then(|b| parse_any(registry, path, b));

    let (old_schema, new_schema, protocol_name) = match (old_parsed, new_parsed) {
        (Some((old, p)), Some((new, _))) => (old, new, p),
        (Some((old, p)), None) => {
            let empty = empty_like(&old);
            (old, empty, p)
        }
        (None, Some((new, p))) => {
            let empty = empty_like(&new);
            (empty, new, p)
        }
        (None, None) => return None,
    };

    let raw_diff = panproto_check::diff(&old_schema, &new_schema);

    // Classify against the schema's own protocol. If we can't recover
    // the protocol we fall back to a conservative "all changes are
    // unclassified" report so the UI still renders the raw diff.
    let report = match resolve_protocol(&protocol_name) {
        Some(p) => panproto_check::classify(&raw_diff, &p),
        None => CompatReport {
            breaking: Vec::new(),
            non_breaking: Vec::new(),
            compatible: true,
        },
    };

    Some(StructuralDiff {
        protocol: protocol_name,
        report,
        raw_diff,
        old_vertex_count: old_schema.vertices.len(),
        new_vertex_count: new_schema.vertices.len(),
        old_edge_count: old_schema.edges.len(),
        new_edge_count: new_schema.edges.len(),
        old_schema,
        new_schema,
    })
}

/// Parse a file via panproto, trying every entry point we know about.
/// Returns the first successful `(Schema, protocol_name)`.
pub(crate) fn parse_any(
    registry: &ParserRegistry,
    path: &str,
    bytes: &[u8],
) -> Option<(Schema, String)> {
    let p = std::path::Path::new(path);
    let lower = path.to_ascii_lowercase();

    // 1. For JSON/YAML files, try content-based protocol detection
    //    FIRST. Many structured schema formats (.json, .yaml) also
    //    have tree-sitter grammars, but the generic JSON/YAML AST
    //    gives syntactic vertices (object, pair, string) instead of
    //    the semantic vertices panproto-protocols knows about (record
    //    types, fields, constraints). Content-based detectors produce
    //    much richer schemas for schema files.
    if lower.ends_with(".json") || lower.ends_with(".yaml") || lower.ends_with(".yml") {
        if let Ok(text) = std::str::from_utf8(bytes) {
            if let Ok(json) = serde_json::from_str::<Value>(text) {
                if let Some(pair) = detect_json_protocol(&json) {
                    return Some(pair);
                }
            }
        }
    }

    // 2. Generic tree-sitter full-AST parsing: 248 languages, one
    //    code path. Uses the file extension to pick a grammar.
    if let Some(proto) = registry.detect_language(p) {
        if let Ok(schema) = registry.parse_file(p, bytes) {
            return Some((schema, proto.to_string()));
        }
    }

    // 3. Fallback: content-based detection for files without a
    //    recognized extension.
    if let Ok(text) = std::str::from_utf8(bytes) {
        if let Ok(json) = serde_json::from_str::<Value>(text) {
            if let Some(pair) = detect_json_protocol(&json) {
                return Some(pair);
            }
        }
    }

    None
}

/// Inspect a JSON value and dispatch to the matching
/// `panproto-protocols` parser. Order matters: we check for the most
/// specific marker fields first.
fn detect_json_protocol(json: &Value) -> Option<(Schema, String)> {
    // ATProto lexicon: { "lexicon": 1, "id": ..., "defs": ... }
    if json.get("lexicon").is_some()
        && json.get("id").is_some()
        && json.get("defs").is_some()
    {
        if let Ok(s) = atproto::parse_lexicon(json) {
            return Some((s, "atproto-lexicon".to_string()));
        }
    }

    // Avro: top-level "type" + "name" with an Avro-specific "fields"
    // or "symbols" array
    if json.get("type").is_some()
        && json.get("name").is_some()
        && (json.get("fields").is_some() || json.get("symbols").is_some())
    {
        if let Ok(s) = panproto_protocols::serialization::avro::parse_avsc(json) {
            return Some((s, "avro".to_string()));
        }
    }

    // k8s CustomResourceDefinition
    if json
        .get("apiVersion")
        .and_then(Value::as_str)
        .is_some_and(|v| v.starts_with("apiextensions.k8s.io"))
    {
        if let Ok(s) = panproto_protocols::config::k8s_crd::parse_k8s_crd_schema(json) {
            return Some((s, "k8s-crd".to_string()));
        }
    }

    // CloudFormation template
    if json.get("AWSTemplateFormatVersion").is_some() || json.get("Resources").is_some()
    {
        if let Ok(s) =
            panproto_protocols::config::cloudformation::parse_cfn_schema(json)
        {
            return Some((s, "cloudformation".to_string()));
        }
    }

    // MongoDB collection validator
    if json.get("bsonType").is_some() || json.get("$jsonSchema").is_some() {
        if let Ok(s) = panproto_protocols::database::mongodb::parse_mongodb_schema(json) {
            return Some((s, "mongodb".to_string()));
        }
    }

    None
}

/// Recover a `Protocol` definition from its name so we can classify
/// the diff. For protocols in panproto-protocols, returns the
/// explicit protocol. For tree-sitter-derived languages, constructs
/// a protocol from the extracted theory metadata so that all grammar
/// edges are classified as governed (removals are breaking).
pub(crate) fn resolve_protocol(name: &str) -> Option<Protocol> {
    // Lexicon is the most common case for Cospan's own repo.
    if name == "atproto-lexicon" || name == "dev.panproto.atproto-lexicon" {
        return Some(atproto::protocol());
    }
    if name == "raw-file" || name == "raw_file" {
        return Some(panproto_protocols::raw_file::protocol());
    }
    // For tree-sitter languages: build a protocol from the grammar's
    // theory metadata. This makes all named edge kinds governed, so
    // removing a field/method/parameter is classified as breaking.
    let registry = panproto_parse::ParserRegistry::new();
    if let Some(meta) = registry.theory_meta(name) {
        let edge_rules: Vec<panproto_schema::EdgeRule> = meta
            .edge_kinds
            .iter()
            .map(|ek| panproto_schema::EdgeRule {
                edge_kind: ek.clone(),
                src_kinds: vec![],
                tgt_kinds: vec![],
            })
            .collect();
        return Some(Protocol {
            name: name.to_string(),
            edge_rules,
            obj_kinds: meta.vertex_kinds.clone(),
            has_order: !meta.ordered_fields.is_empty(),
            has_coproducts: !meta.supertypes.is_empty(),
            ..Protocol::default()
        });
    }
    None
}

/// Build a Schema with the same protocol as the given one but all
/// graph contents cleared. Used to diff an added or removed file
/// against an empty baseline.
fn empty_like(schema: &Schema) -> Schema {
    let mut empty = schema.clone();
    empty.vertices.clear();
    empty.edges.clear();
    empty.hyper_edges.clear();
    empty.constraints.clear();
    empty.required.clear();
    empty.nsids.clear();
    empty.variants.clear();
    empty.orderings.clear();
    empty.recursion_points.clear();
    empty.spans.clear();
    empty.usage_modes.clear();
    empty.nominal.clear();
    empty.coercions.clear();
    empty.mergers.clear();
    empty.defaults.clear();
    empty.policies.clear();
    empty.outgoing.clear();
    empty.incoming.clear();
    empty.between.clear();
    empty
}

// ─── Vertex ID humanization ────────────────────────────────────────
//
// Raw panproto vertex IDs look like "src/auth.ts::User::email" or
// "dev.cospan.repo:body.protocol". Developers want to see
// "field `email` in `User`" or "field `protocol` in `body`".

/// Extract the nearest named scope and leaf name from a vertex ID.
///
/// Returns `(scope, leaf)` where both may be None if the ID is
/// entirely anonymous ($N segments only).
fn split_vertex_id(id: &str) -> (Option<&str>, Option<&str>) {
    // Tree-sitter style: "src/auth.ts::User::email" or "src/auth.ts::User::$7::$2"
    if id.contains("::") {
        let segments: Vec<&str> = id.split("::").collect();
        // Collect named (non-$N, non-file-path) segments
        let named: Vec<&str> = segments
            .iter()
            .filter(|s| {
                !s.starts_with('$')
                    && !s.is_empty()
                    && !s.contains('/')
                    && !s.contains('.')
            })
            .copied()
            .collect();
        return match named.len() {
            0 => (None, None),
            1 => (None, Some(named[0])),
            n => (Some(named[n - 2]), Some(named[n - 1])),
        };
    }
    // Lexicon style: "dev.cospan.repo:body.protocol"
    if let Some(colon_pos) = id.rfind(':') {
        let after = &id[colon_pos + 1..];
        let parts: Vec<&str> = after.split('.').filter(|s| !s.is_empty()).collect();
        return match parts.len() {
            0 => (None, None),
            1 => (None, Some(parts[0])),
            n => (Some(parts[n - 2]), Some(parts[n - 1])),
        };
    }
    // Bare name
    (None, Some(id))
}

/// Convert a raw vertex ID into a human-readable label.
///
/// Examples:
///   "src/auth.ts::User::email" -> "field `email` in `User`"
///   "src/auth.ts::User"        -> "`User`"
///   "dev.cospan.repo:body.protocol" -> "field `protocol` in `body`"
pub fn humanize_vertex(id: &str) -> String {
    let (scope, leaf) = split_vertex_id(id);
    match (scope, leaf) {
        (Some(s), Some(l)) if s != l => format!("`{l}` in `{s}`"),
        (_, Some(name)) => format!("`{name}`"),
        _ => id.to_string(),
    }
}

/// Human-readable label for an edge between two vertices.
fn humanize_edge(src: &str, tgt: &str, name: &Option<String>) -> String {
    let src_h = humanize_vertex(src);
    let tgt_h = humanize_vertex(tgt);
    match name {
        Some(n) if !n.starts_with('$') => format!("{src_h} -> {tgt_h} (via `{n}`)"),
        _ => format!("{src_h} -> {tgt_h}"),
    }
}

// ─── JSON serialization ────────────────────────────────────────────

pub fn structural_diff_to_json(
    diff: &StructuralDiff,
    old_bytes: Option<&[u8]>,
    new_bytes: Option<&[u8]>,
) -> Value {
    // Compute scope-level report via panproto_check::scope.
    let scope_report = report_by_scope(
        &diff.raw_diff,
        &diff.old_schema,
        &diff.new_schema,
        old_bytes,
        new_bytes,
    );
    let scope_json = report_scope_json(&scope_report);
    // Filter out changes that reference only anonymous vertices ($N IDs).
    // These are tree-sitter internal AST nodes (syntax tokens, punctuation)
    // that carry no semantic meaning for developers.
    let breaking: Vec<Value> = diff
        .report
        .breaking
        .iter()
        .filter(|b| !is_anonymous_change_breaking(b))
        .map(breaking_json)
        .collect();
    let non_breaking: Vec<Value> = diff
        .report
        .non_breaking
        .iter()
        .filter(|nb| !is_anonymous_change_non_breaking(nb))
        .map(non_breaking_json)
        .collect();

    // Filter raw vertex lists to only named vertices
    let added_vertices: Vec<&String> = diff.raw_diff.added_vertices.iter().filter(|v| is_meaningful_vertex(v)).collect();
    let removed_vertices: Vec<&String> = diff.raw_diff.removed_vertices.iter().filter(|v| is_meaningful_vertex(v)).collect();

    let compatible = breaking.is_empty();

    json!({
        "protocol": diff.protocol,
        "compatible": compatible,
        "verdict": if compatible { "compatible" } else { "breaking" },
        "breakingCount": breaking.len(),
        "nonBreakingCount": non_breaking.len(),
        "oldVertexCount": diff.old_vertex_count,
        "newVertexCount": diff.new_vertex_count,
        "oldEdgeCount": diff.old_edge_count,
        "newEdgeCount": diff.new_edge_count,
        "addedVertices": added_vertices,
        "removedVertices": removed_vertices,
        "kindChanges": diff.raw_diff.kind_changes.iter()
            .filter(|kc| is_meaningful_vertex(&kc.vertex_id))
            .map(|kc| json!({
                "vertexId": kc.vertex_id,
                "oldKind": kc.old_kind,
                "newKind": kc.new_kind,
            })).collect::<Vec<_>>(),
        "addedEdges": diff.raw_diff.added_edges.iter()
            .filter(|e| is_meaningful_vertex(&e.src) || is_meaningful_vertex(&e.tgt))
            .map(edge_json).collect::<Vec<_>>(),
        "removedEdges": diff.raw_diff.removed_edges.iter()
            .filter(|e| is_meaningful_vertex(&e.src) || is_meaningful_vertex(&e.tgt))
            .map(edge_json).collect::<Vec<_>>(),
        "addedNsids": diff.raw_diff.added_nsids,
        "removedNsids": diff.raw_diff.removed_nsids,
        "changedNsids": diff.raw_diff.changed_nsids.iter().map(|(v, o, n)| json!({
            "vertexId": v, "oldNsid": o, "newNsid": n
        })).collect::<Vec<_>>(),
        "breakingChanges": breaking,
        "nonBreakingChanges": non_breaking,
        "scopeChanges": scope_json["scopes"],
        "namedElements": scope_json["named_elements"],
    })
}

/// Check if a vertex ID represents a meaningful named program element
/// (not an anonymous AST node like $N, and not just a file path).
fn is_meaningful_vertex(id: &str) -> bool {
    if id.contains("::") {
        // Tree-sitter style: "file.rs::FunctionName::field"
        // Must have at least one named segment that's not a file path
        id.split("::").any(|s| {
            !s.starts_with('$') && !s.is_empty() && !s.contains('/') && !s.contains('.')
        })
    } else if id.contains(':') && !id.contains("::") {
        // Lexicon-style: "dev.cospan.repo:body.field" - always meaningful
        true
    } else {
        // Bare ID: only meaningful if not $N and not a file path
        !id.starts_with('$') && !id.is_empty() && !id.contains('/') && !id.contains('.')
    }
}

/// Check if a breaking change should be hidden (references anonymous/internal vertices).
fn is_anonymous_change_breaking(b: &BreakingChange) -> bool {
    match b {
        BreakingChange::RemovedVertex { vertex_id } => !is_meaningful_vertex(vertex_id),
        BreakingChange::RemovedEdge { src, tgt, .. } => {
            // Filter if either end is anonymous (an edge from a file to $392 is noise)
            !is_meaningful_vertex(src) || !is_meaningful_vertex(tgt)
        }
        BreakingChange::KindChanged { vertex_id, .. } => !is_meaningful_vertex(vertex_id),
        BreakingChange::ConstraintTightened { vertex_id, .. } => !is_meaningful_vertex(vertex_id),
        BreakingChange::ConstraintAdded { vertex_id, .. } => !is_meaningful_vertex(vertex_id),
        _ => false,
    }
}

/// Check if a non-breaking change should be hidden.
fn is_anonymous_change_non_breaking(nb: &NonBreakingChange) -> bool {
    match nb {
        NonBreakingChange::AddedVertex { vertex_id } => !is_meaningful_vertex(vertex_id),
        NonBreakingChange::AddedEdge { src, tgt, .. } => {
            !is_meaningful_vertex(src) || !is_meaningful_vertex(tgt)
        }
        NonBreakingChange::ConstraintRelaxed { vertex_id, .. } => !is_meaningful_vertex(vertex_id),
        NonBreakingChange::ConstraintRemoved { vertex_id, .. } => !is_meaningful_vertex(vertex_id),
        _ => false,
    }
}

fn edge_json(e: &panproto_schema::Edge) -> Value {
    json!({
        "src": e.src,
        "tgt": e.tgt,
        "kind": format!("{:?}", e.kind),
        "name": e.name,
    })
}

fn breaking_json(b: &BreakingChange) -> Value {
    match b {
        BreakingChange::RemovedVertex { vertex_id } => json!({
            "kind": "RemovedVertex",
            "label": format!("Removed {}", humanize_vertex(vertex_id)),
            "vertexId": vertex_id,
        }),
        BreakingChange::RemovedEdge { src, tgt, kind, name } => json!({
            "kind": "RemovedEdge",
            "label": format!("Removed edge {}", humanize_edge(src, tgt, name)),
            "src": src, "tgt": tgt, "edgeKind": kind, "name": name,
        }),
        BreakingChange::KindChanged { vertex_id, old_kind, new_kind } => json!({
            "kind": "KindChanged",
            "label": format!("{}: kind changed ({old_kind} -> {new_kind})", humanize_vertex(vertex_id)),
            "vertexId": vertex_id, "oldKind": old_kind, "newKind": new_kind,
        }),
        BreakingChange::ConstraintTightened { vertex_id, sort, old_value, new_value } => json!({
            "kind": "ConstraintTightened",
            "label": format!("{}: {sort} tightened ({old_value} -> {new_value})", humanize_vertex(vertex_id)),
            "vertexId": vertex_id, "sort": sort, "oldValue": old_value, "newValue": new_value,
        }),
        BreakingChange::ConstraintAdded { vertex_id, sort, value } => json!({
            "kind": "ConstraintAdded",
            "label": format!("{}: added constraint {sort} = {value}", humanize_vertex(vertex_id)),
            "vertexId": vertex_id, "sort": sort, "value": value,
        }),
        other => json!({
            "kind": "Other",
            "label": format!("{other:?}"),
        }),
    }
}

fn non_breaking_json(nb: &NonBreakingChange) -> Value {
    match nb {
        NonBreakingChange::AddedVertex { vertex_id } => json!({
            "kind": "AddedVertex",
            "label": format!("Added {}", humanize_vertex(vertex_id)),
            "vertexId": vertex_id,
        }),
        NonBreakingChange::AddedEdge { src, tgt, kind, name } => json!({
            "kind": "AddedEdge",
            "label": format!("Added edge {}", humanize_edge(src, tgt, name)),
            "src": src, "tgt": tgt, "edgeKind": kind, "name": name,
        }),
        NonBreakingChange::ConstraintRelaxed { vertex_id, sort, old_value, new_value } => json!({
            "kind": "ConstraintRelaxed",
            "label": format!("{}: {sort} relaxed ({old_value} -> {new_value})", humanize_vertex(vertex_id)),
            "vertexId": vertex_id, "sort": sort, "oldValue": old_value, "newValue": new_value,
        }),
        NonBreakingChange::ConstraintRemoved { vertex_id, sort } => json!({
            "kind": "ConstraintRemoved",
            "label": format!("{}: removed constraint {sort}", humanize_vertex(vertex_id)),
            "vertexId": vertex_id, "sort": sort,
        }),
        other => json!({
            "kind": "Other",
            "label": format!("{other:?}"),
        }),
    }
}
