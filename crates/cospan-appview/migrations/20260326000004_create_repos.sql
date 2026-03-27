-- Projected from dev.cospan.repo Lexicon + denormalized counters
CREATE TABLE repos (
    did              TEXT NOT NULL,
    rkey             TEXT NOT NULL,
    name             TEXT NOT NULL,
    description      TEXT,
    protocol         TEXT NOT NULL,
    node_did         TEXT NOT NULL REFERENCES nodes(did),
    node_url         TEXT NOT NULL,
    default_branch   TEXT NOT NULL DEFAULT 'main',
    visibility       TEXT NOT NULL DEFAULT 'public',
    source_repo      TEXT,
    star_count       INTEGER NOT NULL DEFAULT 0,
    fork_count       INTEGER NOT NULL DEFAULT 0,
    open_issue_count INTEGER NOT NULL DEFAULT 0,
    open_mr_count    INTEGER NOT NULL DEFAULT 0,
    source           TEXT NOT NULL DEFAULT 'cospan',
    source_uri       TEXT,
    created_at       TIMESTAMPTZ NOT NULL,
    indexed_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (did, name)
);

CREATE UNIQUE INDEX idx_repos_did_rkey ON repos (did, rkey);
CREATE INDEX idx_repos_node_did ON repos (node_did);
CREATE INDEX idx_repos_protocol ON repos (protocol);
CREATE INDEX idx_repos_created_at ON repos (created_at DESC);
CREATE INDEX idx_repos_source ON repos (source) WHERE source != 'cospan';
CREATE INDEX idx_repos_search ON repos USING GIN (
    to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, ''))
);
