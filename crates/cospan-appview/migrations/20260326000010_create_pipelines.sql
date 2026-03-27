-- Pipelines and dependencies
-- Projected from dev.cospan.pipeline, dev.cospan.repo.dependency

CREATE TABLE pipelines (
    did              TEXT NOT NULL,
    rkey             TEXT NOT NULL,
    repo_did         TEXT NOT NULL,
    repo_name        TEXT NOT NULL,
    commit_id        TEXT NOT NULL,
    ref_name         TEXT,
    status           TEXT NOT NULL DEFAULT 'pending',
    gat_type_check   TEXT,
    equation_verification TEXT,
    lens_law_check   TEXT,
    breaking_change_check TEXT,
    created_at       TIMESTAMPTZ NOT NULL,
    completed_at     TIMESTAMPTZ,
    indexed_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (did, rkey),
    FOREIGN KEY (repo_did, repo_name) REFERENCES repos(did, name)
);

CREATE INDEX idx_pipelines_repo ON pipelines (repo_did, repo_name, created_at DESC);
CREATE INDEX idx_pipelines_commit ON pipelines (repo_did, repo_name, commit_id);
CREATE INDEX idx_pipelines_status ON pipelines (status);

CREATE TABLE dependencies (
    did              TEXT NOT NULL,
    rkey             TEXT NOT NULL,
    source_repo_did  TEXT NOT NULL,
    source_repo_name TEXT NOT NULL,
    target_repo_did  TEXT NOT NULL,
    target_repo_name TEXT NOT NULL,
    morphism_id      TEXT NOT NULL,
    source_protocol  TEXT,
    target_protocol  TEXT,
    description      TEXT,
    created_at       TIMESTAMPTZ NOT NULL,
    indexed_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (did, rkey),
    FOREIGN KEY (source_repo_did, source_repo_name) REFERENCES repos(did, name),
    FOREIGN KEY (target_repo_did, target_repo_name) REFERENCES repos(did, name)
);

CREATE INDEX idx_dependencies_source ON dependencies (source_repo_did, source_repo_name);
CREATE INDEX idx_dependencies_target ON dependencies (target_repo_did, target_repo_name);
