-- Pull requests, pull comments, and pull state changes
-- Projected from dev.cospan.repo.pull, dev.cospan.repo.pull.comment, dev.cospan.repo.pull.state

CREATE TABLE pulls (
    did              TEXT NOT NULL,
    rkey             TEXT NOT NULL,
    repo_did         TEXT NOT NULL,
    repo_name        TEXT NOT NULL,
    title            TEXT NOT NULL,
    body             TEXT,
    target_ref       TEXT NOT NULL,
    source_ref       TEXT NOT NULL,
    source_repo      TEXT,
    state            TEXT NOT NULL DEFAULT 'open',
    comment_count    INTEGER NOT NULL DEFAULT 0,
    created_at       TIMESTAMPTZ NOT NULL,
    indexed_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (did, rkey),
    FOREIGN KEY (repo_did, repo_name) REFERENCES repos(did, name)
);

CREATE INDEX idx_pulls_repo ON pulls (repo_did, repo_name, created_at DESC);
CREATE INDEX idx_pulls_state ON pulls (state);
CREATE INDEX idx_pulls_created_at ON pulls (created_at DESC);
CREATE INDEX idx_pulls_repo_state ON pulls (repo_did, repo_name, state);

CREATE TABLE pull_comments (
    did              TEXT NOT NULL,
    rkey             TEXT NOT NULL,
    pull_uri         TEXT NOT NULL,
    body             TEXT NOT NULL,
    review_decision  TEXT,
    created_at       TIMESTAMPTZ NOT NULL,
    indexed_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (did, rkey)
);

CREATE INDEX idx_pull_comments_pull ON pull_comments (pull_uri, created_at ASC);

CREATE TABLE pull_states (
    did              TEXT NOT NULL,
    rkey             TEXT NOT NULL,
    pull_uri         TEXT NOT NULL,
    state            TEXT NOT NULL,
    merge_commit_id  TEXT,
    created_at       TIMESTAMPTZ NOT NULL,
    indexed_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (did, rkey)
);

CREATE INDEX idx_pull_states_pull ON pull_states (pull_uri, created_at DESC);
