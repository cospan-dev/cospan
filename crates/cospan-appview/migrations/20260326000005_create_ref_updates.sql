-- Projected from dev.cospan.vcs.refUpdate Lexicon + denormalized counts
CREATE TABLE ref_updates (
    id                    BIGSERIAL PRIMARY KEY,
    repo_did              TEXT NOT NULL,
    repo_name             TEXT NOT NULL,
    rkey                  TEXT NOT NULL,
    committer_did         TEXT NOT NULL,
    ref_name              TEXT NOT NULL,
    old_target            TEXT,
    new_target            TEXT NOT NULL,
    protocol              TEXT NOT NULL,
    migration_id          TEXT,
    breaking_change_count INTEGER NOT NULL DEFAULT 0,
    lens_id               TEXT,
    lens_quality          REAL,
    commit_count          INTEGER NOT NULL DEFAULT 0,
    created_at            TIMESTAMPTZ NOT NULL,
    indexed_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    FOREIGN KEY (repo_did, repo_name) REFERENCES repos(did, name)
);

CREATE UNIQUE INDEX idx_ref_updates_rkey ON ref_updates (committer_did, rkey);
CREATE INDEX idx_ref_updates_repo ON ref_updates (repo_did, repo_name, created_at DESC);
CREATE INDEX idx_ref_updates_committer ON ref_updates (committer_did, created_at DESC);
CREATE INDEX idx_ref_updates_breaking ON ref_updates (repo_did, repo_name)
    WHERE breaking_change_count > 0;
