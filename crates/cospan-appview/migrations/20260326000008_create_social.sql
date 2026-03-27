-- Stars, follows, and reactions
-- Projected from dev.cospan.feed.star, dev.cospan.graph.follow, dev.cospan.feed.reaction

CREATE TABLE stars (
    did              TEXT NOT NULL,
    rkey             TEXT NOT NULL,
    subject          TEXT NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL,
    indexed_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (did, rkey)
);

CREATE INDEX idx_stars_subject ON stars (subject);
CREATE INDEX idx_stars_did ON stars (did, created_at DESC);
CREATE UNIQUE INDEX idx_stars_did_subject ON stars (did, subject);

CREATE TABLE follows (
    did              TEXT NOT NULL,
    rkey             TEXT NOT NULL,
    subject          TEXT NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL,
    indexed_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (did, rkey)
);

CREATE INDEX idx_follows_subject ON follows (subject);
CREATE INDEX idx_follows_did ON follows (did, created_at DESC);
CREATE UNIQUE INDEX idx_follows_did_subject ON follows (did, subject);

CREATE TABLE reactions (
    did              TEXT NOT NULL,
    rkey             TEXT NOT NULL,
    subject          TEXT NOT NULL,
    emoji            TEXT NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL,
    indexed_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (did, rkey)
);

CREATE INDEX idx_reactions_subject ON reactions (subject);
CREATE INDEX idx_reactions_did ON reactions (did, created_at DESC);
