//! Load human-readable lens JSON files and convert to panproto ProtolensChain.
//!
//! Lens files use a concise DSL:
//!   { "remove_field": "fieldName" }           → combinators::remove_field
//!   { "rename_field": { "old": "x", "new": "y" } } → combinators::rename_field
//!   { "add_field": { "name": "x", "kind": "string", "default": "" } } → combinators::add_field
//!   { "add_field": { ..., "expr": "head(split(...))" } } → add_field + ComputeField
//!   { "apply_expr": { "field": "x", "expr": "match(...)" } } → ApplyExpr
//!   { "compute_field": { "target": "x", "expr": "concat(...)" } } → ComputeField
//!
//! Schema-level steps (remove, rename, add) → ProtolensChain via panproto combinators.
//! Value-level steps (expr, compute) → FieldTransforms via panproto expression parser.
//! Both come from the same JSON file; they're applied at different levels.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use panproto_gat::{CoercionClass, Name};
use panproto_inst::value::Value;
use panproto_inst::FieldTransform;
use panproto_lens::{combinators, ProtolensChain};
use serde::Deserialize;

/// A human-readable lens file.
#[derive(Debug, Deserialize)]
pub struct LensFile {
    #[serde(rename = "$type")]
    pub type_id: String,
    pub id: String,
    #[serde(default)]
    pub description: String,
    pub source: String,
    pub target: String,
    pub steps: Vec<LensStep>,
    #[serde(default)]
    pub table: Option<TableConfig>,
}

/// A single step in the lens pipeline.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum LensStep {
    RemoveField { remove_field: String },
    RenameField { rename_field: RenameSpec },
    AddField { add_field: AddFieldSpec },
    ApplyExpr { apply_expr: ApplyExprSpec },
    ComputeField { compute_field: ComputeFieldSpec },
}

#[derive(Debug, Deserialize)]
pub struct RenameSpec {
    pub old: String,
    pub new: String,
}

#[derive(Debug, Deserialize)]
pub struct AddFieldSpec {
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub default: serde_json::Value,
    #[serde(default)]
    pub expr: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ApplyExprSpec {
    pub field: String,
    pub expr: String,
}

#[derive(Debug, Deserialize)]
pub struct ComputeFieldSpec {
    pub target: String,
    pub expr: String,
}

/// DDL metadata (optional, only for db-projection lenses).
#[derive(Debug, Deserialize)]
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
    pub indexes: Vec<serde_json::Value>,
    #[serde(default)]
    pub foreign_keys: Vec<serde_json::Value>,
    #[serde(default)]
    pub column_defaults: HashMap<String, String>,
    #[serde(default)]
    pub counter_fields: Vec<String>,
}

fn default_true() -> bool { true }

/// Load all lens files from a directory.
pub fn load_all_lenses(lenses_dir: &Path) -> Result<Vec<LensFile>> {
    let mut lenses = Vec::new();
    if !lenses_dir.exists() {
        return Ok(lenses);
    }
    for entry in std::fs::read_dir(lenses_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            let json_str = std::fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?;
            let lens: LensFile = serde_json::from_str(&json_str)
                .with_context(|| format!("parsing {}", path.display()))?;
            lenses.push(lens);
        }
    }
    Ok(lenses)
}

/// Convert schema-level steps to a ProtolensChain using panproto combinators.
pub fn steps_to_protolens_chain(steps: &[LensStep], body_id: &str) -> ProtolensChain {
    let mut chains: Vec<ProtolensChain> = Vec::new();

    for step in steps {
        match step {
            LensStep::RemoveField { remove_field } => {
                let vertex_id = format!("{body_id}.{remove_field}");
                chains.push(combinators::remove_field(vertex_id));
            }
            LensStep::RenameField { rename_field } => {
                let field_vertex = format!("{body_id}.{}", rename_field.old);
                chains.push(combinators::rename_field(
                    body_id,
                    &*field_vertex,
                    &*rename_field.old,
                    &*rename_field.new,
                ));
            }
            LensStep::AddField { add_field } => {
                let vertex_id = format!("{body_id}.{}", add_field.name);
                let default = json_to_value(&add_field.default, &add_field.kind);
                chains.push(combinators::add_field(
                    body_id,
                    &*vertex_id,
                    &*add_field.kind,
                    default,
                ));
            }
            // Expression steps are value-level, not schema-level — handled by
            // steps_to_value_transforms, not by protolens combinators.
            LensStep::ApplyExpr { .. } | LensStep::ComputeField { .. } => {}
        }
    }

    combinators::pipeline(chains)
}

/// Extract value-level transforms (expressions) from lens steps.
///
/// These are injected into the CompiledMigration's field_transforms after
/// the schema-level chain is instantiated. Uses panproto_expr_parser to
/// parse expression strings from the lens JSON.
pub fn steps_to_value_transforms(
    steps: &[LensStep],
    body_vertex: &str,
) -> HashMap<Name, Vec<FieldTransform>> {
    let mut transforms: HashMap<Name, Vec<FieldTransform>> = HashMap::new();
    let key = Name::from(body_vertex);

    for step in steps {
        match step {
            LensStep::AddField { add_field } if add_field.expr.is_some() => {
                let expr_str = add_field.expr.as_ref().unwrap();
                if let Some(expr) = parse_expr(expr_str) {
                    transforms.entry(key.clone()).or_default().push(
                        FieldTransform::ComputeField {
                            target_key: add_field.name.clone(),
                            expr,
                            inverse: None,
                            coercion_class: CoercionClass::Projection,
                        },
                    );
                }
            }
            LensStep::ApplyExpr { apply_expr } => {
                if let Some(expr) = parse_expr(&apply_expr.expr) {
                    transforms.entry(key.clone()).or_default().push(
                        FieldTransform::ApplyExpr {
                            key: apply_expr.field.clone(),
                            expr,
                            inverse: None,
                            coercion_class: CoercionClass::Projection,
                        },
                    );
                }
            }
            LensStep::ComputeField { compute_field } => {
                if let Some(expr) = parse_expr(&compute_field.expr) {
                    transforms.entry(key.clone()).or_default().push(
                        FieldTransform::ComputeField {
                            target_key: compute_field.target.clone(),
                            expr,
                            inverse: None,
                            coercion_class: CoercionClass::Projection,
                        },
                    );
                }
            }
            _ => {}
        }
    }

    transforms
}

/// Parse a panproto expression string using panproto_expr_parser.
fn parse_expr(expr_str: &str) -> Option<panproto_expr::Expr> {
    let tokens = match panproto_expr_parser::tokenize(expr_str) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("  warn: failed to tokenize expression '{expr_str}': {e}");
            return None;
        }
    };
    match panproto_expr_parser::parse(&tokens) {
        Ok(expr) => Some(expr),
        Err(errors) => {
            for e in &errors {
                eprintln!("  warn: parse error in expression '{expr_str}': {e}");
            }
            None
        }
    }
}

/// Convert a JSON default value to a panproto Value.
fn json_to_value(json: &serde_json::Value, kind: &str) -> Value {
    match json {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::String(s) => Value::Str(s.clone()),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() { Value::Int(i) }
            else if let Some(f) = n.as_f64() { Value::Float(f) }
            else { Value::Int(0) }
        }
        serde_json::Value::Bool(b) => Value::Bool(*b),
        _ => match kind {
            "integer" => Value::Int(0),
            "number" | "float" => Value::Float(0.0),
            "boolean" => Value::Bool(false),
            _ => Value::Str(String::new()),
        },
    }
}

pub fn db_projection_lenses(lenses: &[LensFile]) -> Vec<&LensFile> {
    lenses.iter().filter(|l| l.id.ends_with(".db-projection")).collect()
}

pub fn interop_lenses(lenses: &[LensFile]) -> Vec<&LensFile> {
    lenses.iter().filter(|l| l.id.ends_with(".interop")).collect()
}

pub fn find_by_source<'a>(lenses: &'a [LensFile], source_nsid: &str) -> Option<&'a LensFile> {
    lenses.iter().find(|l| l.source == source_nsid)
}
