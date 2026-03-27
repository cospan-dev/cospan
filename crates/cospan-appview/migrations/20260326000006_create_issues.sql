-- Issues, issue comments, and issue state changes
-- Projected from dev.cospan.repo.issue, dev.cospan.repo.issue.comment, dev.cospan.repo.issue.state

CREATE TABLE issues (
    did              TEXT NOT NULL,
    rkey             TEXT NOT NULL,
    repo_did         TEXT NOT NULL,
    repo_name        TEXT NOT NULL,
    title            TEXT NOT NULL,
    body             TEXT,
    state            TEXT NOT NULL DEFAULT 'open',
    comment_count    INTEGER NOT NULL DEFAULT 0,
    created_at       TIMESTAMPTZ NOT NULL,
    indexed_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (did, rkey),
    FOREIGN KEY (repo_did, repo_name) REFERENCES repos(did, name)
);

CREATE INDEX idx_issues_repo ON issues (repo_did, repo_name, created_at DESC);
CREATE INDEX idx_issues_state ON issues (state);
CREATE INDEX idx_issues_created_at ON issues (created_at DESC);
CREATE INDEX idx_issues_repo_state ON issues (repo_did, repo_name, state);

CREATE TABLE issue_comments (
    did              TEXT NOT NULL,
    rkey             TEXT NOT NULL,
    issue_uri        TEXT NOT NULL,
    body             TEXT NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL,
    indexed_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (did, rkey)
);

CREATE INDEX idx_issue_comments_issue ON issue_comments (issue_uri, created_at ASC);

CREATE TABLE issue_states (
    did              TEXT NOT NULL,
    rkey             TEXT NOT NULL,
    issue_uri        TEXT NOT NULL,
    state            TEXT NOT NULL,
    reason           TEXT,
    created_at       TIMESTAMPTZ NOT NULL,
    indexed_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (did, rkey)
);

CREATE INDEX idx_issue_states_issue ON issue_states (issue_uri, created_at DESC);
