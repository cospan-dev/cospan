-- Fork jobs track the async git-copy work that runs after a user
-- initiates a fork via POST /xrpc/dev.cospan.repo.fork. The PDS
-- record and DB row are created synchronously; the git object copy
-- happens in a background task and updates this table.
--
-- States:
--   pending    — queued, not yet started
--   running    — git copy in progress
--   completed  — all refs copied successfully
--   failed     — copy aborted; see last_error

CREATE TABLE fork_jobs (
    id              UUID PRIMARY KEY,
    -- The forked repo (destination) — FK to repos (did, rkey).
    did             TEXT NOT NULL,
    rkey            TEXT NOT NULL,
    name            TEXT NOT NULL,
    -- The source repo AT-URI the user forked from.
    source_repo_uri TEXT NOT NULL,
    -- The git URL we're copying from.
    source_git_url  TEXT NOT NULL,
    -- The git URL we're copying to (destination node's git-receive-pack).
    dest_git_url    TEXT NOT NULL,
    state           TEXT NOT NULL DEFAULT 'pending',
    refs_copied     INT NOT NULL DEFAULT 0,
    last_error      TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at      TIMESTAMPTZ,
    completed_at    TIMESTAMPTZ
);

CREATE INDEX idx_fork_jobs_state ON fork_jobs (state) WHERE state IN ('pending', 'running');
CREATE INDEX idx_fork_jobs_did_rkey ON fork_jobs (did, rkey);
