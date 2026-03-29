//! Per-record-type configuration for database projection.
//!
//! Encodes the denormalization decisions that go beyond raw Lexicon projection:
//! AT-URI decomposition, aggregation counters, flattened sub-objects,
//! field renames, type overrides, etc.

/// Extra column not present in the Lexicon but needed in the database.
#[derive(Clone)]
pub struct ExtraColumn {
    pub name: &'static str,
    pub rust_type: &'static str,
    pub sql_type: &'static str,
    pub optional: bool,
    /// Whether to exclude from INSERT (auto-managed by DB, e.g. counters).
    /// Columns with this flag are not included in upsert() bind params.
    pub exclude_from_insert: bool,
}

/// How to decompose an AT-URI field into separate columns.
#[derive(Clone)]
pub struct UriDecomposition {
    /// The Lexicon field name (camelCase) containing the AT-URI.
    pub source_field: &'static str,
    /// Column name for the DID portion.
    pub did_column: &'static str,
    /// Column name for the rkey/name portion.
    pub name_column: &'static str,
}

/// Store an AT-URI field as a single string column (renamed).
#[derive(Clone)]
pub struct UriStorage {
    /// The Lexicon field name (camelCase).
    pub source_field: &'static str,
    /// Column name in the database.
    pub column_name: &'static str,
}

/// Rename a Lexicon field in the database.
#[derive(Clone)]
pub struct FieldRename {
    /// The Lexicon field name (camelCase).
    pub source_field: &'static str,
    /// The column name in the database (snake_case).
    pub column_name: &'static str,
    /// Override the Rust type (if None, use auto-detected type).
    pub rust_type: Option<&'static str>,
}

/// Override the auto-detected type for a Lexicon field.
#[derive(Clone)]
pub struct TypeOverride {
    /// The Lexicon field name (camelCase).
    pub source_field: &'static str,
    /// The Rust type to use.
    pub rust_type: &'static str,
    /// The SQL type to use.
    pub sql_type: &'static str,
}

/// Configuration for a single record type's database projection.
pub struct RecordConfig {
    pub nsid: &'static str,
    pub table_name: &'static str,
    pub row_struct_name: &'static str,
    /// Columns for ON CONFLICT clause.
    pub conflict_keys: &'static [&'static str],
    /// AT-URI fields to decompose into (did, name) pairs.
    pub uri_decompositions: &'static [UriDecomposition],
    /// AT-URI fields to store as full URI strings (renamed).
    pub uri_storages: &'static [UriStorage],
    /// Fields to rename in the database.
    pub field_renames: &'static [FieldRename],
    /// Type overrides for Lexicon fields.
    pub type_overrides: &'static [TypeOverride],
    /// Extra denormalized columns.
    pub extra_columns: &'static [ExtraColumn],
    /// Lexicon fields to skip in the row (stored as JSONB or not indexed).
    pub skip_fields: &'static [&'static str],
    /// Whether this record has an auto-increment primary key.
    pub has_serial_id: bool,
    /// Whether to include the standard ATProto `did` column.
    /// Some records (like ref_update) don't store the record owner DID.
    pub include_did: bool,
    /// Whether to include the standard ATProto `rkey` column.
    /// Some records (like actor.profile with key "literal:self") don't store rkey.
    pub include_rkey: bool,
}

const URI_DECOMP_REPO: UriDecomposition = UriDecomposition {
    source_field: "repo",
    did_column: "repo_did",
    name_column: "repo_name",
};

const URI_DECOMP_NODE: UriDecomposition = UriDecomposition {
    source_field: "node",
    did_column: "node_did",
    name_column: "node_url",
};

const URI_DECOMP_SOURCE_REPO: UriDecomposition = UriDecomposition {
    source_field: "sourceRepo",
    did_column: "source_repo_did",
    name_column: "source_repo_name",
};

const URI_DECOMP_TARGET_REPO: UriDecomposition = UriDecomposition {
    source_field: "targetRepo",
    did_column: "target_repo_did",
    name_column: "target_repo_name",
};

pub fn all_record_configs() -> Vec<RecordConfig> {
    vec![
        RecordConfig {
            nsid: "dev.cospan.node",
            table_name: "nodes",
            row_struct_name: "NodeRow",
            conflict_keys: &["did"],
            uri_decompositions: &[],
            uri_storages: &[],
            field_renames: &[],
            type_overrides: &[],
            extra_columns: &[],
            skip_fields: &[],
            has_serial_id: false,
            include_did: true,
            include_rkey: true,
        },
        RecordConfig {
            nsid: "dev.cospan.actor.profile",
            table_name: "actor_profiles",
            row_struct_name: "ActorProfileRow",
            conflict_keys: &["did"],
            uri_decompositions: &[],
            uri_storages: &[],
            field_renames: &[],
            type_overrides: &[],
            extra_columns: &[ExtraColumn {
                name: "avatar_cid",
                rust_type: "Option<String>",
                sql_type: "TEXT",
                optional: true,
                exclude_from_insert: false,
            }],
            skip_fields: &["avatar", "links"],
            has_serial_id: false,
            include_did: true,
            include_rkey: false, // literal:self key — no rkey column in DB
        },
        RecordConfig {
            nsid: "dev.cospan.repo",
            table_name: "repos",
            row_struct_name: "RepoRow",
            conflict_keys: &["did", "name"],
            uri_decompositions: &[URI_DECOMP_NODE],
            uri_storages: &[],
            field_renames: &[],
            type_overrides: &[
                // These are NOT NULL DEFAULT in the migration, so use String not Option<String>
                TypeOverride {
                    source_field: "defaultBranch",
                    rust_type: "String",
                    sql_type: "TEXT",
                },
                TypeOverride {
                    source_field: "visibility",
                    rust_type: "String",
                    sql_type: "TEXT",
                },
            ],
            extra_columns: &[
                ExtraColumn {
                    name: "star_count",
                    rust_type: "i32",
                    sql_type: "INTEGER",
                    optional: false,
                    exclude_from_insert: true, // managed via increment/decrement
                },
                ExtraColumn {
                    name: "fork_count",
                    rust_type: "i32",
                    sql_type: "INTEGER",
                    optional: false,
                    exclude_from_insert: true,
                },
                ExtraColumn {
                    name: "open_issue_count",
                    rust_type: "i32",
                    sql_type: "INTEGER",
                    optional: false,
                    exclude_from_insert: true,
                },
                ExtraColumn {
                    name: "open_mr_count",
                    rust_type: "i32",
                    sql_type: "INTEGER",
                    optional: false,
                    exclude_from_insert: true,
                },
                ExtraColumn {
                    name: "source",
                    rust_type: "String",
                    sql_type: "TEXT",
                    optional: false,
                    exclude_from_insert: false, // meaningful data, not a counter
                },
                ExtraColumn {
                    name: "source_uri",
                    rust_type: "Option<String>",
                    sql_type: "TEXT",
                    optional: true,
                    exclude_from_insert: false,
                },
            ],
            skip_fields: &["node"],
            has_serial_id: false,
            include_did: true,
            include_rkey: true,
        },
        RecordConfig {
            nsid: "dev.cospan.vcs.refUpdate",
            table_name: "ref_updates",
            row_struct_name: "RefUpdateRow",
            conflict_keys: &["committer_did", "rkey"], // matches idx_ref_updates_rkey
            uri_decompositions: &[URI_DECOMP_REPO],
            uri_storages: &[],
            field_renames: &[],
            type_overrides: &[
                TypeOverride {
                    source_field: "lensQuality",
                    rust_type: "Option<f32>",
                    sql_type: "REAL",
                },
                TypeOverride {
                    source_field: "commitCount",
                    rust_type: "i32",
                    sql_type: "INTEGER",
                },
            ],
            extra_columns: &[ExtraColumn {
                name: "breaking_change_count",
                rust_type: "i32",
                sql_type: "INTEGER",
                optional: false,
                exclude_from_insert: true, // set by consumer from array length
            }],
            skip_fields: &["repo", "breakingChanges"],
            has_serial_id: true,
            include_did: false, // ref_updates don't store record owner DID
            include_rkey: true,
        },
        RecordConfig {
            nsid: "dev.cospan.repo.issue",
            table_name: "issues",
            row_struct_name: "IssueRow",
            conflict_keys: &["did", "rkey"],
            uri_decompositions: &[URI_DECOMP_REPO],
            uri_storages: &[],
            field_renames: &[],
            type_overrides: &[],
            extra_columns: &[
                ExtraColumn {
                    name: "state",
                    rust_type: "String",
                    sql_type: "TEXT",
                    optional: false,
                    exclude_from_insert: false, // initial state matters
                },
                ExtraColumn {
                    name: "comment_count",
                    rust_type: "i32",
                    sql_type: "INTEGER",
                    optional: false,
                    exclude_from_insert: true, // managed via increment/decrement
                },
            ],
            skip_fields: &["repo", "schemaRefs", "labels", "mentions", "references"],
            has_serial_id: false,
            include_did: true,
            include_rkey: true,
        },
        RecordConfig {
            nsid: "dev.cospan.repo.issue.comment",
            table_name: "issue_comments",
            row_struct_name: "IssueCommentRow",
            conflict_keys: &["did", "rkey"],
            uri_decompositions: &[],
            uri_storages: &[UriStorage {
                source_field: "issue",
                column_name: "issue_uri",
            }],
            field_renames: &[],
            type_overrides: &[],
            extra_columns: &[],
            skip_fields: &["issue", "schemaRefs", "mentions"],
            has_serial_id: false,
            include_did: true,
            include_rkey: true,
        },
        RecordConfig {
            nsid: "dev.cospan.repo.issue.state",
            table_name: "issue_states",
            row_struct_name: "IssueStateRow",
            conflict_keys: &["did", "rkey"],
            uri_decompositions: &[],
            uri_storages: &[UriStorage {
                source_field: "issue",
                column_name: "issue_uri",
            }],
            field_renames: &[],
            type_overrides: &[],
            extra_columns: &[],
            skip_fields: &["issue"],
            has_serial_id: false,
            include_did: true,
            include_rkey: true,
        },
        RecordConfig {
            nsid: "dev.cospan.repo.pull",
            table_name: "pulls",
            row_struct_name: "PullRow",
            conflict_keys: &["did", "rkey"],
            uri_decompositions: &[URI_DECOMP_REPO],
            uri_storages: &[],
            field_renames: &[],
            type_overrides: &[],
            extra_columns: &[
                ExtraColumn {
                    name: "state",
                    rust_type: "String",
                    sql_type: "TEXT",
                    optional: false,
                    exclude_from_insert: false,
                },
                ExtraColumn {
                    name: "comment_count",
                    rust_type: "i32",
                    sql_type: "INTEGER",
                    optional: false,
                    exclude_from_insert: true,
                },
            ],
            skip_fields: &["repo", "mergePreview", "mentions", "references"],
            has_serial_id: false,
            include_did: true,
            include_rkey: true,
        },
        RecordConfig {
            nsid: "dev.cospan.repo.pull.comment",
            table_name: "pull_comments",
            row_struct_name: "PullCommentRow",
            conflict_keys: &["did", "rkey"],
            uri_decompositions: &[],
            uri_storages: &[UriStorage {
                source_field: "pull",
                column_name: "pull_uri",
            }],
            field_renames: &[],
            type_overrides: &[],
            extra_columns: &[],
            skip_fields: &["pull", "schemaRefs", "mentions"],
            has_serial_id: false,
            include_did: true,
            include_rkey: true,
        },
        RecordConfig {
            nsid: "dev.cospan.repo.pull.state",
            table_name: "pull_states",
            row_struct_name: "PullStateRow",
            conflict_keys: &["did", "rkey"],
            uri_decompositions: &[],
            uri_storages: &[UriStorage {
                source_field: "pull",
                column_name: "pull_uri",
            }],
            field_renames: &[],
            type_overrides: &[],
            extra_columns: &[],
            skip_fields: &["pull"],
            has_serial_id: false,
            include_did: true,
            include_rkey: true,
        },
        RecordConfig {
            nsid: "dev.cospan.feed.star",
            table_name: "stars",
            row_struct_name: "StarRow",
            conflict_keys: &["did", "rkey"],
            uri_decompositions: &[],
            uri_storages: &[],
            field_renames: &[],
            type_overrides: &[],
            extra_columns: &[],
            skip_fields: &[],
            has_serial_id: false,
            include_did: true,
            include_rkey: true,
        },
        RecordConfig {
            nsid: "dev.cospan.feed.reaction",
            table_name: "reactions",
            row_struct_name: "ReactionRow",
            conflict_keys: &["did", "rkey"],
            uri_decompositions: &[],
            uri_storages: &[],
            field_renames: &[],
            type_overrides: &[],
            extra_columns: &[],
            skip_fields: &[],
            has_serial_id: false,
            include_did: true,
            include_rkey: true,
        },
        RecordConfig {
            nsid: "dev.cospan.graph.follow",
            table_name: "follows",
            row_struct_name: "FollowRow",
            conflict_keys: &["did", "rkey"],
            uri_decompositions: &[],
            uri_storages: &[],
            field_renames: &[],
            type_overrides: &[],
            extra_columns: &[],
            skip_fields: &[],
            has_serial_id: false,
            include_did: true,
            include_rkey: true,
        },
        RecordConfig {
            nsid: "dev.cospan.label.definition",
            table_name: "labels",
            row_struct_name: "LabelRow",
            conflict_keys: &["did", "rkey"],
            uri_decompositions: &[URI_DECOMP_REPO],
            uri_storages: &[],
            field_renames: &[],
            type_overrides: &[],
            extra_columns: &[],
            skip_fields: &["repo"],
            has_serial_id: false,
            include_did: true,
            include_rkey: true,
        },
        RecordConfig {
            nsid: "dev.cospan.org",
            table_name: "orgs",
            row_struct_name: "OrgRow",
            conflict_keys: &["did", "rkey"],
            uri_decompositions: &[],
            uri_storages: &[],
            field_renames: &[],
            type_overrides: &[],
            extra_columns: &[ExtraColumn {
                name: "avatar_cid",
                rust_type: "Option<String>",
                sql_type: "TEXT",
                optional: true,
                exclude_from_insert: false,
            }],
            skip_fields: &["avatar"],
            has_serial_id: false,
            include_did: true,
            include_rkey: true,
        },
        RecordConfig {
            nsid: "dev.cospan.org.member",
            table_name: "org_members",
            row_struct_name: "OrgMemberRow",
            conflict_keys: &["did", "rkey"],
            uri_decompositions: &[],
            uri_storages: &[UriStorage {
                source_field: "org",
                column_name: "org_uri",
            }],
            field_renames: &[FieldRename {
                source_field: "member",
                column_name: "member_did",
                rust_type: None,
            }],
            type_overrides: &[],
            extra_columns: &[],
            skip_fields: &["org", "member"],
            has_serial_id: false,
            include_did: true,
            include_rkey: true,
        },
        RecordConfig {
            nsid: "dev.cospan.repo.collaborator",
            table_name: "collaborators",
            row_struct_name: "CollaboratorRow",
            conflict_keys: &["did", "rkey"],
            uri_decompositions: &[URI_DECOMP_REPO],
            uri_storages: &[],
            // Lexicon has a `did` field (the collaborator) that collides with ATProto `did`
            field_renames: &[FieldRename {
                source_field: "did",
                column_name: "member_did",
                rust_type: None,
            }],
            type_overrides: &[],
            extra_columns: &[],
            skip_fields: &["repo", "did"],
            has_serial_id: false,
            include_did: true,
            include_rkey: true,
        },
        RecordConfig {
            nsid: "dev.cospan.repo.dependency",
            table_name: "dependencies",
            row_struct_name: "DependencyRow",
            conflict_keys: &["did", "rkey"],
            uri_decompositions: &[URI_DECOMP_SOURCE_REPO, URI_DECOMP_TARGET_REPO],
            uri_storages: &[],
            field_renames: &[],
            type_overrides: &[],
            extra_columns: &[],
            skip_fields: &["sourceRepo", "targetRepo"],
            has_serial_id: false,
            include_did: true,
            include_rkey: true,
        },
        RecordConfig {
            nsid: "dev.cospan.pipeline",
            table_name: "pipelines",
            row_struct_name: "PipelineRow",
            conflict_keys: &["did", "rkey"],
            uri_decompositions: &[URI_DECOMP_REPO],
            uri_storages: &[],
            field_renames: &[],
            type_overrides: &[],
            extra_columns: &[
                ExtraColumn {
                    name: "gat_type_check",
                    rust_type: "Option<String>",
                    sql_type: "TEXT",
                    optional: true,
                    exclude_from_insert: false,
                },
                ExtraColumn {
                    name: "equation_verification",
                    rust_type: "Option<String>",
                    sql_type: "TEXT",
                    optional: true,
                    exclude_from_insert: false,
                },
                ExtraColumn {
                    name: "lens_law_check",
                    rust_type: "Option<String>",
                    sql_type: "TEXT",
                    optional: true,
                    exclude_from_insert: false,
                },
                ExtraColumn {
                    name: "breaking_change_check",
                    rust_type: "Option<String>",
                    sql_type: "TEXT",
                    optional: true,
                    exclude_from_insert: false,
                },
            ],
            skip_fields: &["repo", "algebraicChecks", "workflows"],
            has_serial_id: false,
            include_did: true,
            include_rkey: true,
        },
    ]
}

/// Look up the record config for a given NSID.
pub fn config_for_nsid(nsid: &str) -> Option<&'static RecordConfig> {
    static CONFIGS: std::sync::OnceLock<Vec<RecordConfig>> = std::sync::OnceLock::new();
    let configs = CONFIGS.get_or_init(all_record_configs);
    configs.iter().find(|c| c.nsid == nsid)
}
