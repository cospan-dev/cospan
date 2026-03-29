//! Tangled → Cospan interop morphisms.
//!
//! Each Tangled record type that maps to a Cospan database table gets a
//! `from_tangled_json()` method generated on the target Row type.
//! The morphism specifies how Tangled field names map to Cospan columns.

use anyhow::Result;
use panproto_protocols::emit::IndentWriter;

/// A field mapping from a Tangled Lexicon field to a Cospan Row column.
#[derive(Clone)]
pub enum FieldMapping {
    /// Direct copy: `rec.get("tangledField").as_str()` → column
    Direct {
        tangled_field: &'static str,
        required: bool,
    },
    /// Constant value
    Constant(&'static str),
    /// Custom extraction expression (raw Rust code)
    Custom(&'static str),
}

/// Morphism from a Tangled record to a Cospan Row type.
pub struct TangledMorphism {
    pub tangled_nsid: &'static str,
    pub target_row: &'static str,
    /// Ordered list of (column_name, mapping) pairs.
    /// Must match the Row struct field order.
    pub fields: Vec<(&'static str, FieldMapping)>,
}

pub fn all_tangled_morphisms() -> Vec<TangledMorphism> {
    vec![
        TangledMorphism {
            tangled_nsid: "sh.tangled.feed.star",
            target_row: "StarRow",
            fields: vec![
                ("did", FieldMapping::Custom("did.to_string()")),
                ("rkey", FieldMapping::Custom("rkey.to_string()")),
                ("created_at", FieldMapping::Custom("parse_datetime(rec, \"createdAt\")")),
                ("subject", FieldMapping::Direct { tangled_field: "subject", required: true }),
                ("indexed_at", FieldMapping::Custom("chrono::Utc::now()")),
            ],
        },
        TangledMorphism {
            tangled_nsid: "sh.tangled.graph.follow",
            target_row: "FollowRow",
            fields: vec![
                ("did", FieldMapping::Custom("did.to_string()")),
                ("rkey", FieldMapping::Custom("rkey.to_string()")),
                ("created_at", FieldMapping::Custom("parse_datetime(rec, \"createdAt\")")),
                ("subject", FieldMapping::Direct { tangled_field: "subject", required: true }),
                ("indexed_at", FieldMapping::Custom("chrono::Utc::now()")),
            ],
        },
        TangledMorphism {
            tangled_nsid: "sh.tangled.feed.reaction",
            target_row: "ReactionRow",
            fields: vec![
                ("did", FieldMapping::Custom("did.to_string()")),
                ("rkey", FieldMapping::Custom("rkey.to_string()")),
                ("created_at", FieldMapping::Custom("parse_datetime(rec, \"createdAt\")")),
                ("emoji", FieldMapping::Direct { tangled_field: "emoji", required: true }),
                ("subject", FieldMapping::Direct { tangled_field: "subject", required: true }),
                ("indexed_at", FieldMapping::Custom("chrono::Utc::now()")),
            ],
        },
        TangledMorphism {
            tangled_nsid: "sh.tangled.repo.issue",
            target_row: "IssueRow",
            fields: vec![
                ("did", FieldMapping::Custom("did.to_string()")),
                ("rkey", FieldMapping::Custom("rkey.to_string()")),
                ("repo_did", FieldMapping::Custom("parse_at_uri_did(rec, \"repo\")")),
                ("repo_name", FieldMapping::Custom("parse_at_uri_name(rec, \"repo\")")),
                ("body", FieldMapping::Direct { tangled_field: "body", required: false }),
                ("created_at", FieldMapping::Custom("parse_datetime(rec, \"createdAt\")")),
                ("title", FieldMapping::Direct { tangled_field: "title", required: true }),
                ("state", FieldMapping::Constant("\"open\".to_string()")),
                ("comment_count", FieldMapping::Constant("0")),
                ("indexed_at", FieldMapping::Custom("chrono::Utc::now()")),
            ],
        },
        TangledMorphism {
            tangled_nsid: "sh.tangled.repo.issue.state",
            target_row: "IssueStateRow",
            fields: vec![
                ("did", FieldMapping::Custom("did.to_string()")),
                ("rkey", FieldMapping::Custom("rkey.to_string()")),
                ("issue_uri", FieldMapping::Direct { tangled_field: "issue", required: true }),
                ("created_at", FieldMapping::Custom("parse_datetime(rec, \"createdAt\")")),
                ("reason", FieldMapping::Direct { tangled_field: "reason", required: false }),
                ("state", FieldMapping::Direct { tangled_field: "state", required: true }),
                ("indexed_at", FieldMapping::Custom("chrono::Utc::now()")),
            ],
        },
        TangledMorphism {
            tangled_nsid: "sh.tangled.repo.issue.comment",
            target_row: "IssueCommentRow",
            fields: vec![
                ("did", FieldMapping::Custom("did.to_string()")),
                ("rkey", FieldMapping::Custom("rkey.to_string()")),
                ("issue_uri", FieldMapping::Direct { tangled_field: "issue", required: true }),
                ("body", FieldMapping::Direct { tangled_field: "body", required: true }),
                ("created_at", FieldMapping::Custom("parse_datetime(rec, \"createdAt\")")),
                ("indexed_at", FieldMapping::Custom("chrono::Utc::now()")),
            ],
        },
        TangledMorphism {
            tangled_nsid: "sh.tangled.repo.pull",
            target_row: "PullRow",
            fields: vec![
                ("did", FieldMapping::Custom("did.to_string()")),
                ("rkey", FieldMapping::Custom("rkey.to_string()")),
                ("repo_did", FieldMapping::Custom("parse_at_uri_did(rec, \"repo\")")),
                ("repo_name", FieldMapping::Custom("parse_at_uri_name(rec, \"repo\")")),
                ("body", FieldMapping::Direct { tangled_field: "body", required: false }),
                ("created_at", FieldMapping::Custom("parse_datetime(rec, \"createdAt\")")),
                ("source_ref", FieldMapping::Direct { tangled_field: "sourceBranch", required: true }),
                ("source_repo", FieldMapping::Direct { tangled_field: "sourceRepo", required: false }),
                ("target_ref", FieldMapping::Direct { tangled_field: "targetBranch", required: true }),
                ("title", FieldMapping::Direct { tangled_field: "title", required: true }),
                ("state", FieldMapping::Constant("\"open\".to_string()")),
                ("comment_count", FieldMapping::Constant("0")),
                ("indexed_at", FieldMapping::Custom("chrono::Utc::now()")),
            ],
        },
        TangledMorphism {
            tangled_nsid: "sh.tangled.repo.pull.status",
            target_row: "PullStateRow",
            fields: vec![
                ("did", FieldMapping::Custom("did.to_string()")),
                ("rkey", FieldMapping::Custom("rkey.to_string()")),
                ("pull_uri", FieldMapping::Direct { tangled_field: "pull", required: true }),
                ("created_at", FieldMapping::Custom("parse_datetime(rec, \"createdAt\")")),
                ("merge_commit_id", FieldMapping::Direct { tangled_field: "mergeCommitId", required: false }),
                ("state", FieldMapping::Custom("{\
                    let s = rec.get(\"state\").and_then(|v| v.as_str()).unwrap_or(\"open\");\
                    match s { \"merged\" => \"merged\", \"closed\" => \"closed\", _ => \"open\" }.to_string()\
                }")),
                ("indexed_at", FieldMapping::Custom("chrono::Utc::now()")),
            ],
        },
        TangledMorphism {
            tangled_nsid: "sh.tangled.repo.pull.comment",
            target_row: "PullCommentRow",
            fields: vec![
                ("did", FieldMapping::Custom("did.to_string()")),
                ("rkey", FieldMapping::Custom("rkey.to_string()")),
                ("pull_uri", FieldMapping::Direct { tangled_field: "pull", required: true }),
                ("body", FieldMapping::Direct { tangled_field: "body", required: true }),
                ("created_at", FieldMapping::Custom("parse_datetime(rec, \"createdAt\")")),
                ("review_decision", FieldMapping::Direct { tangled_field: "reviewDecision", required: false }),
                ("indexed_at", FieldMapping::Custom("chrono::Utc::now()")),
            ],
        },
        TangledMorphism {
            tangled_nsid: "sh.tangled.repo.collaborator",
            target_row: "CollaboratorRow",
            fields: vec![
                ("did", FieldMapping::Custom("did.to_string()")),
                ("rkey", FieldMapping::Custom("rkey.to_string()")),
                ("repo_did", FieldMapping::Custom("parse_at_uri_did(rec, \"repo\")")),
                ("repo_name", FieldMapping::Custom("parse_at_uri_name(rec, \"repo\")")),
                // Tangled uses "subject" for the collaborator DID, Cospan uses "did" → "member_did"
                ("member_did", FieldMapping::Direct { tangled_field: "subject", required: true }),
                ("created_at", FieldMapping::Custom("parse_datetime(rec, \"createdAt\")")),
                ("role", FieldMapping::Constant("\"contributor\".to_string()")),
                ("indexed_at", FieldMapping::Custom("chrono::Utc::now()")),
            ],
        },
        TangledMorphism {
            tangled_nsid: "sh.tangled.knot",
            target_row: "NodeRow",
            fields: vec![
                ("did", FieldMapping::Custom("did.to_string()")),
                ("rkey", FieldMapping::Custom("rkey.to_string()")),
                ("created_at", FieldMapping::Custom("parse_datetime(rec, \"createdAt\")")),
                ("public_endpoint", FieldMapping::Custom("{\
                    let hostname = rec.get(\"hostname\").and_then(|v| v.as_str()).unwrap_or(\"\");\
                    if hostname.is_empty() { None } else { Some(format!(\"https://{hostname}\")) }\
                }")),
                ("indexed_at", FieldMapping::Custom("chrono::Utc::now()")),
            ],
        },
        // sh.tangled.spindle also maps to NodeRow but uses from_tangled_spindle_json
        // to avoid duplicate impl with sh.tangled.knot
        TangledMorphism {
            tangled_nsid: "sh.tangled.spindle",
            target_row: "NodeRow",
            fields: vec![], // handled specially below
        },
        TangledMorphism {
            tangled_nsid: "sh.tangled.actor.profile",
            target_row: "ActorProfileRow",
            fields: vec![
                ("did", FieldMapping::Custom("did.to_string()")),
                // Tangled bluesky is a boolean; we store the handle or empty string
                ("bluesky", FieldMapping::Custom("{\
                    match rec.get(\"bluesky\") {\
                        Some(serde_json::Value::Bool(true)) => did.to_string(),\
                        Some(serde_json::Value::String(s)) => s.clone(),\
                        _ => String::new(),\
                    }\
                }")),
                ("description", FieldMapping::Direct { tangled_field: "description", required: false }),
                ("display_name", FieldMapping::Custom("None")), // Tangled profiles don't have displayName
                ("avatar_cid", FieldMapping::Custom("\
                    rec.get(\"avatar\")\
                    .and_then(|v| v.get(\"ref\"))\
                    .and_then(|v| v.get(\"$link\"))\
                    .and_then(|v| v.as_str())\
                    .map(String::from)")),
                ("indexed_at", FieldMapping::Custom("chrono::Utc::now()")),
            ],
        },
        TangledMorphism {
            tangled_nsid: "sh.tangled.repo",
            target_row: "RepoRow",
            fields: vec![
                ("did", FieldMapping::Custom("did.to_string()")),
                ("rkey", FieldMapping::Custom("rkey.to_string()")),
                // Tangled uses "knot" (hostname), not AT-URI node
                ("node_did", FieldMapping::Custom("{\
                    let knot = rec.get(\"knot\").and_then(|v| v.as_str()).unwrap_or(\"\");\
                    if knot.is_empty() { String::new() } else { format!(\"did:web:{knot}\") }\
                }")),
                ("node_url", FieldMapping::Custom("{\
                    let knot = rec.get(\"knot\").and_then(|v| v.as_str()).unwrap_or(\"\");\
                    if knot.is_empty() { String::new() } else { format!(\"https://{knot}\") }\
                }")),
                ("created_at", FieldMapping::Custom("parse_datetime(rec, \"createdAt\")")),
                ("default_branch", FieldMapping::Constant("\"main\".to_string()")),
                ("description", FieldMapping::Direct { tangled_field: "description", required: false }),
                ("name", FieldMapping::Direct { tangled_field: "name", required: true }),
                ("protocol", FieldMapping::Constant("\"git\".to_string()")),
                ("source_repo", FieldMapping::Custom("None")),
                ("visibility", FieldMapping::Constant("\"public\".to_string()")),
                ("star_count", FieldMapping::Constant("0")),
                ("fork_count", FieldMapping::Constant("0")),
                ("open_issue_count", FieldMapping::Constant("0")),
                ("open_mr_count", FieldMapping::Constant("0")),
                ("source", FieldMapping::Constant("\"tangled\".to_string()")),
                ("source_uri", FieldMapping::Custom("Some(format!(\"at://{did}/sh.tangled.repo/{rkey}\"))")),
                ("indexed_at", FieldMapping::Custom("chrono::Utc::now()")),
            ],
        },
        TangledMorphism {
            tangled_nsid: "sh.tangled.knot.member",
            target_row: "OrgMemberRow",
            fields: vec![
                ("did", FieldMapping::Custom("did.to_string()")),
                ("rkey", FieldMapping::Custom("rkey.to_string()")),
                ("org_uri", FieldMapping::Direct { tangled_field: "knot", required: true }),
                ("member_did", FieldMapping::Direct { tangled_field: "member", required: true }),
                ("created_at", FieldMapping::Custom("parse_datetime(rec, \"createdAt\")")),
                ("role", FieldMapping::Direct { tangled_field: "role", required: true }),
                ("indexed_at", FieldMapping::Custom("chrono::Utc::now()")),
            ],
        },
        TangledMorphism {
            tangled_nsid: "sh.tangled.spindle.member",
            target_row: "OrgMemberRow",
            fields: vec![
                ("did", FieldMapping::Custom("did.to_string()")),
                ("rkey", FieldMapping::Custom("rkey.to_string()")),
                ("org_uri", FieldMapping::Direct { tangled_field: "spindle", required: true }),
                ("member_did", FieldMapping::Direct { tangled_field: "member", required: true }),
                ("created_at", FieldMapping::Custom("parse_datetime(rec, \"createdAt\")")),
                ("role", FieldMapping::Direct { tangled_field: "role", required: true }),
                ("indexed_at", FieldMapping::Custom("chrono::Utc::now()")),
            ],
        },
        TangledMorphism {
            tangled_nsid: "sh.tangled.label.definition",
            target_row: "LabelRow",
            fields: vec![
                ("did", FieldMapping::Custom("did.to_string()")),
                ("rkey", FieldMapping::Custom("rkey.to_string()")),
                // Tangled labels don't have a repo AT-URI directly, infer from context
                ("repo_did", FieldMapping::Custom("did.to_string()")),
                ("repo_name", FieldMapping::Constant("String::new()")),
                ("color", FieldMapping::Custom("rec.get(\"color\").and_then(|v| v.as_str()).unwrap_or(\"#6b7280\").to_string()")),
                ("created_at", FieldMapping::Custom("parse_datetime(rec, \"createdAt\")")),
                ("description", FieldMapping::Custom("rec.get(\"name\").and_then(|v| v.as_str()).map(String::from)")),
                ("name", FieldMapping::Direct { tangled_field: "name", required: true }),
                ("indexed_at", FieldMapping::Custom("chrono::Utc::now()")),
            ],
        },
        TangledMorphism {
            tangled_nsid: "sh.tangled.pipeline",
            target_row: "PipelineRow",
            fields: vec![
                ("did", FieldMapping::Custom("did.to_string()")),
                ("rkey", FieldMapping::Custom("rkey.to_string()")),
                // Extract repo info from triggerMetadata.repo
                ("repo_did", FieldMapping::Custom("rec.get(\"triggerMetadata\").and_then(|t| t.get(\"repo\")).and_then(|r| r.get(\"did\")).and_then(|v| v.as_str()).unwrap_or(\"\").to_string()")),
                ("repo_name", FieldMapping::Custom("rec.get(\"triggerMetadata\").and_then(|t| t.get(\"repo\")).and_then(|r| r.get(\"repo\")).and_then(|v| v.as_str()).unwrap_or(\"\").to_string()")),
                // Extract commit ID from push trigger data
                ("commit_id", FieldMapping::Custom("rec.get(\"triggerMetadata\").and_then(|t| t.get(\"push\")).and_then(|p| p.get(\"newSha\")).and_then(|v| v.as_str()).unwrap_or(\"\").to_string()")),
                ("completed_at", FieldMapping::Custom("None")),
                ("created_at", FieldMapping::Custom("chrono::Utc::now()")),
                ("ref_name", FieldMapping::Custom("rec.get(\"triggerMetadata\").and_then(|t| t.get(\"push\")).and_then(|p| p.get(\"ref\")).and_then(|v| v.as_str()).map(String::from)")),
                ("status", FieldMapping::Constant("\"pending\".to_string()")),
                ("gat_type_check", FieldMapping::Constant("Some(\"skipped\".to_string())")),
                ("equation_verification", FieldMapping::Constant("Some(\"skipped\".to_string())")),
                ("lens_law_check", FieldMapping::Constant("Some(\"skipped\".to_string())")),
                ("breaking_change_check", FieldMapping::Constant("Some(\"skipped\".to_string())")),
                ("indexed_at", FieldMapping::Custom("chrono::Utc::now()")),
            ],
        },
        TangledMorphism {
            tangled_nsid: "sh.tangled.git.refUpdate",
            target_row: "RefUpdateRow",
            fields: vec![
                ("id", FieldMapping::Constant("0")),
                // No did column for ref_updates (include_did: false)
                ("rkey", FieldMapping::Custom("rkey.to_string()")),
                // Tangled uses repoDid/repoName directly instead of AT-URI
                ("repo_did", FieldMapping::Direct { tangled_field: "repoDid", required: true }),
                ("repo_name", FieldMapping::Direct { tangled_field: "repoName", required: true }),
                ("commit_count", FieldMapping::Constant("1")),
                ("committer_did", FieldMapping::Direct { tangled_field: "committerDid", required: true }),
                ("created_at", FieldMapping::Custom("chrono::Utc::now()")),
                ("lens_id", FieldMapping::Custom("None")),
                ("lens_quality", FieldMapping::Custom("None")),
                ("migration_id", FieldMapping::Custom("None")),
                ("new_target", FieldMapping::Direct { tangled_field: "newSha", required: true }),
                ("old_target", FieldMapping::Direct { tangled_field: "oldSha", required: false }),
                ("protocol", FieldMapping::Constant("\"git\".to_string()")),
                ("ref_name", FieldMapping::Direct { tangled_field: "ref", required: true }),
                ("breaking_change_count", FieldMapping::Constant("0")),
                ("indexed_at", FieldMapping::Custom("chrono::Utc::now()")),
            ],
        },
    ]
}

/// Emit `from_tangled_json()` methods for all morphisms.
pub fn emit_tangled_from_json(morphisms: &[TangledMorphism]) -> Result<String> {
    let mut w = IndentWriter::new("    ");

    // Track which row types already have from_tangled_json to avoid duplicate impls
    let mut seen_rows = std::collections::HashSet::new();

    for m in morphisms {
        if m.fields.is_empty() {
            // Skip morphisms without field mappings (handled by another morphism)
            w.line(&format!("// {} → {} (uses same from_tangled_json as knot)", m.tangled_nsid, m.target_row));
            w.blank();
            continue;
        }
        if !seen_rows.insert(m.target_row) {
            // Skip duplicate implementations for the same target row
            w.line(&format!("// {} → {} (skipped: duplicate impl)", m.tangled_nsid, m.target_row));
            w.blank();
            continue;
        }
        w.line(&format!("// {} → {}", m.tangled_nsid, m.target_row));
        w.line(&format!("impl {} {{", m.target_row));
        w.indent();
        w.line(&format!(
            "/// Deserialize a {} Jetstream record into a Cospan row.",
            m.tangled_nsid
        ));
        w.line("#[allow(unused_variables)]");
        w.line("pub fn from_tangled_json(did: &str, rkey: &str, rec: &serde_json::Value) -> Self {");
        w.indent();
        w.line("Self {");
        w.indent();

        for (col_name, mapping) in &m.fields {
            match mapping {
                FieldMapping::Direct { tangled_field, required: true } => {
                    w.line(&format!(
                        "{col_name}: rec.get(\"{tangled_field}\").and_then(|v| v.as_str()).unwrap_or(\"\").to_string(),"
                    ));
                }
                FieldMapping::Direct { tangled_field, required: false } => {
                    w.line(&format!(
                        "{col_name}: rec.get(\"{tangled_field}\").and_then(|v| v.as_str()).map(String::from),"
                    ));
                }
                FieldMapping::Constant(val) => {
                    w.line(&format!("{col_name}: {val},"));
                }
                FieldMapping::Custom(expr) => {
                    w.line(&format!("{col_name}: {expr},"));
                }
            }
        }

        w.dedent();
        w.line("}");
        w.dedent();
        w.line("}");
        w.dedent();
        w.line("}");
        w.blank();
    }

    Ok(w.finish())
}
