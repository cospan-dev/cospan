//! Cospan Lexicon → Database schema projections as panproto migrations.
//!
//! Each record type's database projection is expressed as a panproto Migration
//! with FieldTransforms that handle:
//! - AT-URI decomposition (split URI into did + name columns)
//! - Field renames (camelCase → snake_case, `did` → `member_did`)
//! - Default values (state = "open", counters = 0)
//! - Nested field extraction (avatar.ref.$link → avatar_cid)
//!
//! These are compiled at codegen time and applied at runtime via
//! `lift_wtype_sigma()`.

use std::collections::HashMap;
use std::sync::Arc;

use panproto_expr::{BuiltinOp, Expr};
use panproto_gat::{CoercionClass, Name};
use panproto_inst::FieldTransform;
use panproto_inst::value::Value;

/// Build the field transforms for a record type's database projection.
/// Returns transforms keyed by the schema vertex they apply to.
pub fn db_transforms(nsid: &str) -> HashMap<Name, Vec<FieldTransform>> {
    let mut transforms = HashMap::new();
    let body_vertex = record_body_vertex(nsid);

    match nsid {
        // All target keys are camelCase to match #[serde(rename_all = "camelCase")]
        // on the Row structs, so serde can deserialize the panproto output directly.
        "dev.cospan.repo" => {
            transforms.insert(
                Name::from(body_vertex),
                vec![
                    at_uri_extract_did("node", "nodeDid"),
                    add_field_str("nodeUrl", ""),
                    add_field_int("starCount", 0),
                    add_field_int("forkCount", 0),
                    add_field_int("openIssueCount", 0),
                    add_field_int("openMrCount", 0),
                    add_field_str("source", "pds"),
                    drop_field("node"),
                ],
            );
        }
        "dev.cospan.vcs.refUpdate" => {
            transforms.insert(
                Name::from(body_vertex),
                vec![
                    at_uri_extract_did("repo", "repoDid"),
                    at_uri_extract_name("repo", "repoName"),
                    compute_array_len("breakingChanges", "breakingChangeCount"),
                    drop_field("repo"),
                    drop_field("breakingChanges"),
                ],
            );
        }
        "dev.cospan.repo.issue" => {
            transforms.insert(
                Name::from(body_vertex),
                vec![
                    at_uri_extract_did("repo", "repoDid"),
                    at_uri_extract_name("repo", "repoName"),
                    add_field_str("state", "open"),
                    add_field_int("commentCount", 0),
                    drop_field("repo"),
                    drop_field("schemaRefs"),
                    drop_field("labels"),
                    drop_field("mentions"),
                    drop_field("references"),
                ],
            );
        }
        "dev.cospan.repo.issue.comment" => {
            transforms.insert(
                Name::from(body_vertex),
                vec![
                    rename_field("issue", "issueUri"),
                    drop_field("schemaRefs"),
                    drop_field("mentions"),
                ],
            );
        }
        "dev.cospan.repo.issue.state" => {
            transforms.insert(
                Name::from(body_vertex),
                vec![rename_field("issue", "issueUri")],
            );
        }
        "dev.cospan.repo.pull" => {
            transforms.insert(
                Name::from(body_vertex),
                vec![
                    at_uri_extract_did("repo", "repoDid"),
                    at_uri_extract_name("repo", "repoName"),
                    add_field_str("state", "open"),
                    add_field_int("commentCount", 0),
                    drop_field("repo"),
                    drop_field("mergePreview"),
                    drop_field("mentions"),
                    drop_field("references"),
                ],
            );
        }
        "dev.cospan.repo.pull.comment" => {
            transforms.insert(
                Name::from(body_vertex),
                vec![
                    rename_field("pull", "pullUri"),
                    drop_field("schemaRefs"),
                    drop_field("mentions"),
                ],
            );
        }
        "dev.cospan.repo.pull.state" => {
            transforms.insert(
                Name::from(body_vertex),
                vec![rename_field("pull", "pullUri")],
            );
        }
        "dev.cospan.actor.profile" => {
            transforms.insert(
                Name::from(body_vertex),
                vec![
                    path_extract("avatar", vec!["ref", "$link"], "avatarCid"),
                    drop_field("avatar"),
                    drop_field("links"),
                ],
            );
        }
        "dev.cospan.label.definition" => {
            transforms.insert(
                Name::from(body_vertex),
                vec![
                    at_uri_extract_did("repo", "repoDid"),
                    at_uri_extract_name("repo", "repoName"),
                    drop_field("repo"),
                ],
            );
        }
        "dev.cospan.org" => {
            transforms.insert(
                Name::from(body_vertex),
                vec![
                    path_extract("avatar", vec!["ref", "$link"], "avatarCid"),
                    drop_field("avatar"),
                ],
            );
        }
        "dev.cospan.org.member" => {
            transforms.insert(
                Name::from(body_vertex),
                vec![
                    rename_field("org", "orgUri"),
                    rename_field("member", "memberDid"),
                ],
            );
        }
        "dev.cospan.repo.collaborator" => {
            transforms.insert(
                Name::from(body_vertex),
                vec![
                    at_uri_extract_did("repo", "repoDid"),
                    at_uri_extract_name("repo", "repoName"),
                    rename_field("did", "memberDid"),
                    drop_field("repo"),
                ],
            );
        }
        "dev.cospan.repo.dependency" => {
            transforms.insert(
                Name::from(body_vertex),
                vec![
                    at_uri_extract_did("sourceRepo", "sourceRepoDid"),
                    at_uri_extract_name("sourceRepo", "sourceRepoName"),
                    at_uri_extract_did("targetRepo", "targetRepoDid"),
                    at_uri_extract_name("targetRepo", "targetRepoName"),
                    drop_field("sourceRepo"),
                    drop_field("targetRepo"),
                ],
            );
        }
        "dev.cospan.pipeline" => {
            transforms.insert(
                Name::from(body_vertex),
                vec![
                    at_uri_extract_did("repo", "repoDid"),
                    at_uri_extract_name("repo", "repoName"),
                    path_extract("algebraicChecks", vec!["gatTypeCheck"], "gatTypeCheck"),
                    path_extract(
                        "algebraicChecks",
                        vec!["equationVerification"],
                        "equationVerification",
                    ),
                    path_extract("algebraicChecks", vec!["lensLawCheck"], "lensLawCheck"),
                    path_extract(
                        "algebraicChecks",
                        vec!["breakingChangeCheck"],
                        "breakingChangeCheck",
                    ),
                    drop_field("repo"),
                    drop_field("algebraicChecks"),
                    drop_field("workflows"),
                ],
            );
        }
        // Simple records with no transforms needed
        _ => {}
    }

    transforms
}

// ---------------------------------------------------------------------------
// Expression builders
// ---------------------------------------------------------------------------

/// `split(replace(var(field), "at://", ""), "/")` → list, then head → DID
fn at_uri_extract_did(source_field: &str, target_field: &str) -> FieldTransform {
    // Expression: head(split(replace(source, "at://", ""), "/"))
    let expr = Expr::Builtin(
        BuiltinOp::Head,
        vec![Expr::Builtin(
            BuiltinOp::Split,
            vec![
                Expr::Builtin(
                    BuiltinOp::Replace,
                    vec![
                        Expr::Var(Arc::from(source_field)),
                        Expr::Lit(panproto_expr::Literal::Str("at://".into())),
                        Expr::Lit(panproto_expr::Literal::Str(String::new())),
                    ],
                ),
                Expr::Lit(panproto_expr::Literal::Str("/".into())),
            ],
        )],
    );
    FieldTransform::ComputeField {
        target_key: target_field.to_string(),
        expr,
        inverse: None,
        coercion_class: CoercionClass::Retraction,
    }
}

/// Extract name (3rd segment) from AT-URI: at://did/collection/name → name
fn at_uri_extract_name(source_field: &str, target_field: &str) -> FieldTransform {
    // Expression: index(split(replace(source, "at://", ""), "/"), 2)
    let expr = Expr::Index(
        Box::new(Expr::Builtin(
            BuiltinOp::Split,
            vec![
                Expr::Builtin(
                    BuiltinOp::Replace,
                    vec![
                        Expr::Var(Arc::from(source_field)),
                        Expr::Lit(panproto_expr::Literal::Str("at://".into())),
                        Expr::Lit(panproto_expr::Literal::Str(String::new())),
                    ],
                ),
                Expr::Lit(panproto_expr::Literal::Str("/".into())),
            ],
        )),
        Box::new(Expr::Lit(panproto_expr::Literal::Int(2))),
    );
    FieldTransform::ComputeField {
        target_key: target_field.to_string(),
        expr,
        inverse: None,
        coercion_class: CoercionClass::Retraction,
    }
}

fn rename_field(old: &str, new: &str) -> FieldTransform {
    FieldTransform::RenameField {
        old_key: old.to_string(),
        new_key: new.to_string(),
    }
}

fn drop_field(key: &str) -> FieldTransform {
    FieldTransform::DropField {
        key: key.to_string(),
    }
}

fn add_field_str(key: &str, value: &str) -> FieldTransform {
    FieldTransform::AddField {
        key: key.to_string(),
        value: Value::Str(value.to_string()),
    }
}

fn add_field_int(key: &str, value: i64) -> FieldTransform {
    FieldTransform::AddField {
        key: key.to_string(),
        value: Value::Int(value),
    }
}

/// Extract a value at a nested path and store in a new field.
fn path_extract(source_field: &str, path: Vec<&str>, target_field: &str) -> FieldTransform {
    // Navigate source_field.path[0].path[1]...
    let mut expr: Expr = Expr::Var(Arc::from(source_field));
    for segment in path {
        expr = Expr::Field(Box::new(expr), Arc::from(segment));
    }
    FieldTransform::ComputeField {
        target_key: target_field.to_string(),
        expr,
        inverse: None,
        coercion_class: CoercionClass::Retraction,
    }
}

/// Compute the length of a JSON array field and store as an integer.
fn compute_array_len(source_field: &str, target_field: &str) -> FieldTransform {
    // Expression: length(source_field)  — uses the list Length builtin
    let expr = Expr::Builtin(BuiltinOp::Length, vec![Expr::Var(Arc::from(source_field))]);
    FieldTransform::ComputeField {
        target_key: target_field.to_string(),
        expr,
        inverse: None,
        coercion_class: CoercionClass::Retraction,
    }
}

/// Get the record body vertex ID for a given NSID.
/// ATProto Lexicon schemas have the body under `{nsid}.record`.
fn record_body_vertex(nsid: &str) -> String {
    format!("{nsid}.record")
}
