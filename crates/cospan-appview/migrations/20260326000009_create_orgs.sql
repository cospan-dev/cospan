-- Labels, orgs, org members, and collaborators
-- Projected from dev.cospan.label.definition, dev.cospan.org, dev.cospan.org.member, dev.cospan.repo.collaborator

CREATE TABLE labels (
    did              TEXT NOT NULL,
    rkey             TEXT NOT NULL,
    repo_did         TEXT NOT NULL,
    repo_name        TEXT NOT NULL,
    name             TEXT NOT NULL,
    color            TEXT NOT NULL,
    description      TEXT,
    created_at       TIMESTAMPTZ NOT NULL,
    indexed_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (did, rkey),
    FOREIGN KEY (repo_did, repo_name) REFERENCES repos(did, name)
);

CREATE INDEX idx_labels_repo ON labels (repo_did, repo_name);

CREATE TABLE orgs (
    did              TEXT NOT NULL,
    rkey             TEXT NOT NULL,
    name             TEXT NOT NULL,
    description      TEXT,
    avatar_cid       TEXT,
    created_at       TIMESTAMPTZ NOT NULL,
    indexed_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (did, rkey)
);

CREATE INDEX idx_orgs_name ON orgs (name);

CREATE TABLE org_members (
    did              TEXT NOT NULL,
    rkey             TEXT NOT NULL,
    org_uri          TEXT NOT NULL,
    member_did       TEXT NOT NULL,
    role             TEXT NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL,
    indexed_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (did, rkey)
);

CREATE INDEX idx_org_members_org ON org_members (org_uri);
CREATE INDEX idx_org_members_member ON org_members (member_did);

CREATE TABLE collaborators (
    did              TEXT NOT NULL,
    rkey             TEXT NOT NULL,
    repo_did         TEXT NOT NULL,
    repo_name        TEXT NOT NULL,
    member_did       TEXT NOT NULL,
    role             TEXT NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL,
    indexed_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (did, rkey),
    FOREIGN KEY (repo_did, repo_name) REFERENCES repos(did, name)
);

CREATE INDEX idx_collaborators_repo ON collaborators (repo_did, repo_name);
CREATE INDEX idx_collaborators_member ON collaborators (member_did);
