-- Projected from dev.cospan.actor.profile Lexicon
CREATE TABLE actor_profiles (
    did          TEXT PRIMARY KEY,
    bluesky      TEXT NOT NULL,
    display_name TEXT,
    description  TEXT,
    avatar_cid   TEXT,
    indexed_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
