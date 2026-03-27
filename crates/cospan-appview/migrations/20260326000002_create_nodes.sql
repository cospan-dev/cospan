-- Projected from dev.cospan.node Lexicon
CREATE TABLE nodes (
    did             TEXT PRIMARY KEY,
    rkey            TEXT NOT NULL,
    public_endpoint TEXT,
    created_at      TIMESTAMPTZ NOT NULL,
    indexed_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_nodes_endpoint ON nodes (public_endpoint) WHERE public_endpoint IS NOT NULL;
