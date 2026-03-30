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
#[allow(dead_code)]
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
#[allow(dead_code)]
pub struct UriStorage {
    /// The Lexicon field name (camelCase).
    pub source_field: &'static str,
    /// Column name in the database.
    pub column_name: &'static str,
}

/// Rename a Lexicon field in the database.
#[derive(Clone)]
#[allow(dead_code)]
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

/// A database index definition.
#[derive(Clone)]
pub struct IndexConfig {
    /// Index name (e.g., "idx_repos_node_did").
    pub name: &'static str,
    /// Column expressions (e.g., &["node_did"] or &["created_at DESC"]).
    pub columns: &'static [&'static str],
    /// Whether this is a UNIQUE index.
    pub unique: bool,
    /// Optional WHERE clause for partial indexes.
    pub where_clause: Option<&'static str>,
    /// Optional USING method (e.g., "GIN").
    pub using: Option<&'static str>,
    /// Optional raw expression to use instead of columns (e.g., for GIN indexes).
    pub raw_expression: Option<&'static str>,
}

/// A foreign key constraint.
#[derive(Clone)]
pub struct ForeignKeyConfig {
    /// Local columns participating in the FK.
    pub columns: &'static [&'static str],
    /// Referenced table.
    pub ref_table: &'static str,
    /// Referenced columns.
    pub ref_columns: &'static [&'static str],
}

/// A column default value for DDL generation.
#[derive(Clone)]
pub struct ColumnDefault {
    /// Column name (snake_case).
    pub column: &'static str,
    /// SQL DEFAULT expression.
    pub expression: &'static str,
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
    /// Database indexes for this table.
    pub indexes: &'static [IndexConfig],
    /// Foreign key constraints.
    pub foreign_keys: &'static [ForeignKeyConfig],
    /// Column defaults for DDL generation.
    pub column_defaults: &'static [ColumnDefault],
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
            indexes: &[IndexConfig {
                name: "idx_nodes_endpoint",
                columns: &["public_endpoint"],
                unique: false,
                where_clause: Some("public_endpoint IS NOT NULL"),
                using: None,
                raw_expression: None,
            }],
            foreign_keys: &[],
            column_defaults: &[],
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
            indexes: &[],
            foreign_keys: &[],
            column_defaults: &[],
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
            indexes: &[
                IndexConfig {
                    name: "idx_repos_did_rkey",
                    columns: &["did", "rkey"],
                    unique: true,
                    where_clause: None,
                    using: None,
                    raw_expression: None,
                },
                IndexConfig {
                    name: "idx_repos_node_did",
                    columns: &["node_did"],
                    unique: false,
                    where_clause: None,
                    using: None,
                    raw_expression: None,
                },
                IndexConfig {
                    name: "idx_repos_protocol",
                    columns: &["protocol"],
                    unique: false,
                    where_clause: None,
                    using: None,
                    raw_expression: None,
                },
                IndexConfig {
                    name: "idx_repos_created_at",
                    columns: &["created_at DESC"],
                    unique: false,
                    where_clause: None,
                    using: None,
                    raw_expression: None,
                },
                IndexConfig {
                    name: "idx_repos_source",
                    columns: &["source"],
                    unique: false,
                    where_clause: Some("source != 'cospan'"),
                    using: None,
                    raw_expression: None,
                },
                IndexConfig {
                    name: "idx_repos_search",
                    columns: &[],
                    unique: false,
                    where_clause: None,
                    using: Some("GIN"),
                    raw_expression: Some(
                        "to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, ''))",
                    ),
                },
            ],
            foreign_keys: &[ForeignKeyConfig {
                columns: &["node_did"],
                ref_table: "nodes",
                ref_columns: &["did"],
            }],
            column_defaults: &[
                ColumnDefault {
                    column: "default_branch",
                    expression: "'main'",
                },
                ColumnDefault {
                    column: "visibility",
                    expression: "'public'",
                },
                ColumnDefault {
                    column: "star_count",
                    expression: "0",
                },
                ColumnDefault {
                    column: "fork_count",
                    expression: "0",
                },
                ColumnDefault {
                    column: "open_issue_count",
                    expression: "0",
                },
                ColumnDefault {
                    column: "open_mr_count",
                    expression: "0",
                },
                ColumnDefault {
                    column: "source",
                    expression: "'cospan'",
                },
            ],
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
            indexes: &[
                IndexConfig {
                    name: "idx_ref_updates_rkey",
                    columns: &["committer_did", "rkey"],
                    unique: true,
                    where_clause: None,
                    using: None,
                    raw_expression: None,
                },
                IndexConfig {
                    name: "idx_ref_updates_repo",
                    columns: &["repo_did", "repo_name", "created_at DESC"],
                    unique: false,
                    where_clause: None,
                    using: None,
                    raw_expression: None,
                },
                IndexConfig {
                    name: "idx_ref_updates_committer",
                    columns: &["committer_did", "created_at DESC"],
                    unique: false,
                    where_clause: None,
                    using: None,
                    raw_expression: None,
                },
                IndexConfig {
                    name: "idx_ref_updates_breaking",
                    columns: &["repo_did", "repo_name"],
                    unique: false,
                    where_clause: Some("breaking_change_count > 0"),
                    using: None,
                    raw_expression: None,
                },
            ],
            foreign_keys: &[ForeignKeyConfig {
                columns: &["repo_did", "repo_name"],
                ref_table: "repos",
                ref_columns: &["did", "name"],
            }],
            column_defaults: &[
                ColumnDefault {
                    column: "breaking_change_count",
                    expression: "0",
                },
                ColumnDefault {
                    column: "commit_count",
                    expression: "0",
                },
            ],
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
            indexes: &[
                IndexConfig {
                    name: "idx_issues_repo",
                    columns: &["repo_did", "repo_name", "created_at DESC"],
                    unique: false,
                    where_clause: None,
                    using: None,
                    raw_expression: None,
                },
                IndexConfig {
                    name: "idx_issues_state",
                    columns: &["state"],
                    unique: false,
                    where_clause: None,
                    using: None,
                    raw_expression: None,
                },
                IndexConfig {
                    name: "idx_issues_created_at",
                    columns: &["created_at DESC"],
                    unique: false,
                    where_clause: None,
                    using: None,
                    raw_expression: None,
                },
                IndexConfig {
                    name: "idx_issues_repo_state",
                    columns: &["repo_did", "repo_name", "state"],
                    unique: false,
                    where_clause: None,
                    using: None,
                    raw_expression: None,
                },
            ],
            foreign_keys: &[ForeignKeyConfig {
                columns: &["repo_did", "repo_name"],
                ref_table: "repos",
                ref_columns: &["did", "name"],
            }],
            column_defaults: &[
                ColumnDefault {
                    column: "state",
                    expression: "'open'",
                },
                ColumnDefault {
                    column: "comment_count",
                    expression: "0",
                },
            ],
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
            indexes: &[IndexConfig {
                name: "idx_issue_comments_issue",
                columns: &["issue_uri", "created_at ASC"],
                unique: false,
                where_clause: None,
                using: None,
                raw_expression: None,
            }],
            foreign_keys: &[],
            column_defaults: &[],
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
            indexes: &[IndexConfig {
                name: "idx_issue_states_issue",
                columns: &["issue_uri", "created_at DESC"],
                unique: false,
                where_clause: None,
                using: None,
                raw_expression: None,
            }],
            foreign_keys: &[],
            column_defaults: &[],
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
            indexes: &[
                IndexConfig {
                    name: "idx_pulls_repo",
                    columns: &["repo_did", "repo_name", "created_at DESC"],
                    unique: false,
                    where_clause: None,
                    using: None,
                    raw_expression: None,
                },
                IndexConfig {
                    name: "idx_pulls_state",
                    columns: &["state"],
                    unique: false,
                    where_clause: None,
                    using: None,
                    raw_expression: None,
                },
                IndexConfig {
                    name: "idx_pulls_created_at",
                    columns: &["created_at DESC"],
                    unique: false,
                    where_clause: None,
                    using: None,
                    raw_expression: None,
                },
                IndexConfig {
                    name: "idx_pulls_repo_state",
                    columns: &["repo_did", "repo_name", "state"],
                    unique: false,
                    where_clause: None,
                    using: None,
                    raw_expression: None,
                },
            ],
            foreign_keys: &[ForeignKeyConfig {
                columns: &["repo_did", "repo_name"],
                ref_table: "repos",
                ref_columns: &["did", "name"],
            }],
            column_defaults: &[
                ColumnDefault {
                    column: "state",
                    expression: "'open'",
                },
                ColumnDefault {
                    column: "comment_count",
                    expression: "0",
                },
            ],
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
            indexes: &[IndexConfig {
                name: "idx_pull_comments_pull",
                columns: &["pull_uri", "created_at ASC"],
                unique: false,
                where_clause: None,
                using: None,
                raw_expression: None,
            }],
            foreign_keys: &[],
            column_defaults: &[],
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
            indexes: &[IndexConfig {
                name: "idx_pull_states_pull",
                columns: &["pull_uri", "created_at DESC"],
                unique: false,
                where_clause: None,
                using: None,
                raw_expression: None,
            }],
            foreign_keys: &[],
            column_defaults: &[],
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
            indexes: &[
                IndexConfig {
                    name: "idx_stars_subject",
                    columns: &["subject"],
                    unique: false,
                    where_clause: None,
                    using: None,
                    raw_expression: None,
                },
                IndexConfig {
                    name: "idx_stars_did",
                    columns: &["did", "created_at DESC"],
                    unique: false,
                    where_clause: None,
                    using: None,
                    raw_expression: None,
                },
                IndexConfig {
                    name: "idx_stars_did_subject",
                    columns: &["did", "subject"],
                    unique: true,
                    where_clause: None,
                    using: None,
                    raw_expression: None,
                },
            ],
            foreign_keys: &[],
            column_defaults: &[],
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
            indexes: &[
                IndexConfig {
                    name: "idx_reactions_subject",
                    columns: &["subject"],
                    unique: false,
                    where_clause: None,
                    using: None,
                    raw_expression: None,
                },
                IndexConfig {
                    name: "idx_reactions_did",
                    columns: &["did", "created_at DESC"],
                    unique: false,
                    where_clause: None,
                    using: None,
                    raw_expression: None,
                },
            ],
            foreign_keys: &[],
            column_defaults: &[],
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
            indexes: &[
                IndexConfig {
                    name: "idx_follows_subject",
                    columns: &["subject"],
                    unique: false,
                    where_clause: None,
                    using: None,
                    raw_expression: None,
                },
                IndexConfig {
                    name: "idx_follows_did",
                    columns: &["did", "created_at DESC"],
                    unique: false,
                    where_clause: None,
                    using: None,
                    raw_expression: None,
                },
                IndexConfig {
                    name: "idx_follows_did_subject",
                    columns: &["did", "subject"],
                    unique: true,
                    where_clause: None,
                    using: None,
                    raw_expression: None,
                },
            ],
            foreign_keys: &[],
            column_defaults: &[],
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
            indexes: &[IndexConfig {
                name: "idx_labels_repo",
                columns: &["repo_did", "repo_name"],
                unique: false,
                where_clause: None,
                using: None,
                raw_expression: None,
            }],
            foreign_keys: &[ForeignKeyConfig {
                columns: &["repo_did", "repo_name"],
                ref_table: "repos",
                ref_columns: &["did", "name"],
            }],
            column_defaults: &[],
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
            indexes: &[IndexConfig {
                name: "idx_orgs_name",
                columns: &["name"],
                unique: false,
                where_clause: None,
                using: None,
                raw_expression: None,
            }],
            foreign_keys: &[],
            column_defaults: &[],
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
            indexes: &[
                IndexConfig {
                    name: "idx_org_members_org",
                    columns: &["org_uri"],
                    unique: false,
                    where_clause: None,
                    using: None,
                    raw_expression: None,
                },
                IndexConfig {
                    name: "idx_org_members_member",
                    columns: &["member_did"],
                    unique: false,
                    where_clause: None,
                    using: None,
                    raw_expression: None,
                },
            ],
            foreign_keys: &[],
            column_defaults: &[],
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
            indexes: &[
                IndexConfig {
                    name: "idx_collaborators_repo",
                    columns: &["repo_did", "repo_name"],
                    unique: false,
                    where_clause: None,
                    using: None,
                    raw_expression: None,
                },
                IndexConfig {
                    name: "idx_collaborators_member",
                    columns: &["member_did"],
                    unique: false,
                    where_clause: None,
                    using: None,
                    raw_expression: None,
                },
            ],
            foreign_keys: &[ForeignKeyConfig {
                columns: &["repo_did", "repo_name"],
                ref_table: "repos",
                ref_columns: &["did", "name"],
            }],
            column_defaults: &[],
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
            indexes: &[
                IndexConfig {
                    name: "idx_dependencies_source",
                    columns: &["source_repo_did", "source_repo_name"],
                    unique: false,
                    where_clause: None,
                    using: None,
                    raw_expression: None,
                },
                IndexConfig {
                    name: "idx_dependencies_target",
                    columns: &["target_repo_did", "target_repo_name"],
                    unique: false,
                    where_clause: None,
                    using: None,
                    raw_expression: None,
                },
            ],
            foreign_keys: &[
                ForeignKeyConfig {
                    columns: &["source_repo_did", "source_repo_name"],
                    ref_table: "repos",
                    ref_columns: &["did", "name"],
                },
                ForeignKeyConfig {
                    columns: &["target_repo_did", "target_repo_name"],
                    ref_table: "repos",
                    ref_columns: &["did", "name"],
                },
            ],
            column_defaults: &[],
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
            indexes: &[
                IndexConfig {
                    name: "idx_pipelines_repo",
                    columns: &["repo_did", "repo_name", "created_at DESC"],
                    unique: false,
                    where_clause: None,
                    using: None,
                    raw_expression: None,
                },
                IndexConfig {
                    name: "idx_pipelines_commit",
                    columns: &["repo_did", "repo_name", "commit_id"],
                    unique: false,
                    where_clause: None,
                    using: None,
                    raw_expression: None,
                },
                IndexConfig {
                    name: "idx_pipelines_status",
                    columns: &["status"],
                    unique: false,
                    where_clause: None,
                    using: None,
                    raw_expression: None,
                },
            ],
            foreign_keys: &[ForeignKeyConfig {
                columns: &["repo_did", "repo_name"],
                ref_table: "repos",
                ref_columns: &["did", "name"],
            }],
            column_defaults: &[ColumnDefault {
                column: "status",
                expression: "'pending'",
            }],
        },
    ]
}

/// Look up the record config for a given NSID.
pub fn config_for_nsid(nsid: &str) -> Option<&'static RecordConfig> {
    static CONFIGS: std::sync::OnceLock<Vec<RecordConfig>> = std::sync::OnceLock::new();
    let configs = CONFIGS.get_or_init(all_record_configs);
    configs.iter().find(|c| c.nsid == nsid)
}
