-- Firehose cursor: single-row table tracking Jetstream position
CREATE TABLE firehose_cursor (
    id         INTEGER PRIMARY KEY DEFAULT 1 CHECK (id = 1),
    cursor_us  BIGINT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Dead letter queue for permanent indexing failures
CREATE TABLE dlq_entries (
    id           TEXT PRIMARY KEY,
    collection   TEXT NOT NULL,
    rkey         TEXT NOT NULL,
    did          TEXT NOT NULL,
    operation    TEXT NOT NULL,
    error_detail JSONB NOT NULL,
    raw_record   JSONB,
    cursor_us    BIGINT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_dlq_collection ON dlq_entries (collection);
CREATE INDEX idx_dlq_created_at ON dlq_entries (created_at);
