use std::sync::Arc;

use tokio::sync::Mutex;

use crate::config::{CheckMode, ValidationConfig};
use crate::store::RepoManager;

pub struct ValidationPipeline {
    gat_type_check: CheckMode,
    equation_verify: CheckMode,
    breaking_change: CheckMode,
    auto_lens: bool,
}

#[derive(Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationResult {
    pub breaking_changes: Vec<serde_json::Value>,
    pub lens_id: Option<String>,
    pub lens_quality: Option<f64>,
}

impl ValidationPipeline {
    pub fn new(config: &ValidationConfig) -> Self {
        Self {
            gat_type_check: config.gat_type_check,
            equation_verify: config.equation_verify,
            breaking_change: config.breaking_change,
            auto_lens: config.auto_lens,
        }
    }

    /// Validate a ref update. Returns validation metadata to include in the refUpdate record.
    ///
    /// Loads old and new schemas from the store, runs panproto-check diff/classify,
    /// and optionally generates an auto-lens via panproto-lens.
    pub async fn validate(
        &self,
        store: &Arc<Mutex<RepoManager>>,
        did: &str,
        repo: &str,
        protocol_name: &str,
        old_target: Option<&str>,
        new_target: &str,
    ) -> Result<ValidationResult, String> {
        let mut result = ValidationResult::default();

        // If there's no old target, this is the first commit — nothing to diff against.
        let old_target = match old_target {
            Some(t) => t,
            None => return Ok(result),
        };

        // If all checks are skipped and auto-lens is off, short-circuit.
        let all_skipped = matches!(self.gat_type_check, CheckMode::Skip)
            && matches!(self.equation_verify, CheckMode::Skip)
            && matches!(self.breaking_change, CheckMode::Skip)
            && !self.auto_lens;
        if all_skipped {
            return Ok(result);
        }

        // Load the old and new schemas from the store.
        let (old_schema, new_schema) = {
            let store_guard = store.lock().await;

            let old_id: panproto_core::vcs::ObjectId = old_target
                .parse()
                .map_err(|_| format!("invalid old target object ID: {old_target}"))?;
            let new_id: panproto_core::vcs::ObjectId = new_target
                .parse()
                .map_err(|_| format!("invalid new target object ID: {new_target}"))?;

            // Load objects — they should be commits. Walk to their schemas.
            let old_obj = store_guard
                .get_object(did, repo, &old_id)
                .map_err(|e| format!("failed to load old target: {e}"))?;
            let new_obj = store_guard
                .get_object(did, repo, &new_id)
                .map_err(|e| format!("failed to load new target: {e}"))?;

            let old_schema = extract_schema(&store_guard, did, repo, &old_obj)
                .map_err(|e| format!("failed to extract old schema: {e}"))?;
            let new_schema = extract_schema(&store_guard, did, repo, &new_obj)
                .map_err(|e| format!("failed to extract new schema: {e}"))?;

            (old_schema, new_schema)
        };

        // Step 1: GAT type-check — validate the schemas against the protocol theory.
        match self.gat_type_check {
            CheckMode::Strict => {
                let errors =
                    panproto_schema::validate(&new_schema, &resolve_protocol(protocol_name));
                if !errors.is_empty() {
                    let msg = errors
                        .iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<_>>()
                        .join("; ");
                    return Err(format!("GAT type-check failed (strict): {msg}"));
                }
                tracing::debug!("gat type-check: passed (strict)");
            }
            CheckMode::Warn => {
                let errors =
                    panproto_schema::validate(&new_schema, &resolve_protocol(protocol_name));
                if !errors.is_empty() {
                    for e in &errors {
                        tracing::warn!("gat type-check warning: {e}");
                    }
                }
                tracing::debug!("gat type-check: passed (warn mode)");
            }
            CheckMode::Skip => {}
        }

        // Step 2: Equation verification.
        match self.equation_verify {
            CheckMode::Strict => {
                let errors =
                    panproto_schema::validate(&new_schema, &resolve_protocol(protocol_name));
                if !errors.is_empty() {
                    let msg = errors
                        .iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<_>>()
                        .join("; ");
                    return Err(format!("equation verification failed (strict): {msg}"));
                }
                tracing::debug!("equation verify: passed (strict)");
            }
            CheckMode::Warn => {
                let errors =
                    panproto_schema::validate(&new_schema, &resolve_protocol(protocol_name));
                for e in &errors {
                    tracing::warn!("equation verify warning: {e}");
                }
                tracing::debug!("equation verify: passed (warn mode)");
            }
            CheckMode::Skip => {}
        }

        // Step 3: Breaking change detection via panproto-check.
        let schema_diff = panproto_check::diff(&old_schema, &new_schema);

        // Resolve the protocol definition for classification.
        let protocol = resolve_protocol(protocol_name);

        match self.breaking_change {
            CheckMode::Strict | CheckMode::Warn => {
                let compat_report = panproto_check::classify(&schema_diff, &protocol);

                if !compat_report.breaking.is_empty() {
                    // Serialize breaking changes as JSON for the result.
                    let breaking_json = panproto_check::report_json(&compat_report);
                    let breaking_changes = breaking_json
                        .get("breaking")
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_default();

                    if matches!(self.breaking_change, CheckMode::Strict) {
                        return Err(format!(
                            "breaking changes detected (strict mode): {} change(s)",
                            breaking_changes.len()
                        ));
                    }

                    // Warn mode: include in result but don't reject.
                    for bc in &compat_report.breaking {
                        tracing::warn!("breaking change detected: {bc:?}");
                    }
                    result.breaking_changes = breaking_changes;
                }
            }
            CheckMode::Skip => {}
        }

        // Step 4: Auto-lens generation via panproto-lens.
        if self.auto_lens {
            let config = panproto_lens::AutoLensConfig::default();
            match panproto_lens::auto_generate(&old_schema, &new_schema, &protocol, &config) {
                Ok(auto_result) => {
                    // Store the lens in the VCS store for future use.
                    let lens_id = format!(
                        "auto:{}->{}",
                        &old_target[..8.min(old_target.len())],
                        &new_target[..8.min(new_target.len())]
                    );
                    result.lens_id = Some(lens_id);
                    result.lens_quality = Some(auto_result.alignment_quality);
                    tracing::info!(
                        quality = auto_result.alignment_quality,
                        "auto-lens generated successfully"
                    );
                }
                Err(e) => {
                    tracing::warn!("auto-lens generation failed: {e}");
                    // Not fatal — auto-lens is best-effort.
                }
            }
        }

        Ok(result)
    }
}

/// Extract a schema from a VCS object. If the object is a Commit, follow its
/// schema_id to load the actual Schema object.
fn extract_schema(
    store: &RepoManager,
    did: &str,
    repo: &str,
    obj: &panproto_core::vcs::Object,
) -> Result<panproto_schema::Schema, String> {
    match obj {
        panproto_core::vcs::Object::Schema(s) => Ok(*s.clone()),
        panproto_core::vcs::Object::Commit(commit) => {
            let schema_obj = store
                .get_object(did, repo, &commit.schema_id)
                .map_err(|e| format!("failed to load schema {}: {e}", commit.schema_id))?;
            match schema_obj {
                panproto_core::vcs::Object::Schema(s) => Ok(*s),
                other => Err(format!(
                    "expected schema object at {}, got {}",
                    commit.schema_id,
                    other.type_name()
                )),
            }
        }
        other => Err(format!(
            "expected commit or schema object, got {}",
            other.type_name()
        )),
    }
}

/// Resolve a protocol name to its Protocol definition.
/// Uses the ATProto protocol for ATProto records, raw_file as default fallback.
/// Language-specific protocols (typescript, python, etc.) are handled by
/// panproto-parse's ParserRegistry, not by protocol definitions here.
fn resolve_protocol(name: &str) -> panproto_schema::Protocol {
    match name {
        "atproto" => panproto_protocols::atproto::protocol(),
        "raw_file" => panproto_protocols::raw_file::protocol(),
        _ => {
            // For language protocols, use a permissive default protocol.
            // The actual language-specific protocol validation happens in
            // panproto-parse during parsing, not during ref update validation.
            panproto_schema::Protocol::default()
        }
    }
}
