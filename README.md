# Cospan

[![CI](https://github.com/cospan-dev/cospan/actions/workflows/ci.yml/badge.svg)](https://github.com/cospan-dev/cospan/actions/workflows/ci.yml)
[![License: AGPL-3.0](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)

**Decentralized code collaboration with structural version control.**

Cospan replaces git's text-based VCS with [panproto](https://github.com/panproto/panproto)-vcs, a mathematically-founded version control system where merges are categorical pushouts, diffs are structural, and every object is typed by a Generalized Algebraic Theory. Built on [ATProto](https://atproto.com/) for federated identity and data portability.

## What makes Cospan different

- **Structural diffs, not line diffs.** When two developers modify the same struct, Cospan sees structural schema modifications and computes provably commutative merges instead of textual three-way merge conflicts.
- **248 language support.** Full-AST parsing via [panproto-grammars](https://github.com/panproto/panproto) turns source files into typed schema graphs. Every function, class, statement, and expression is a vertex in the graph.
- **Cross-protocol schema tracking.** A Protobuf schema change can show which ATProto Lexicons and SQL tables downstream are affected, with auto-generated migration lenses for each hop.
- **Federated on ATProto.** Your identity is a DID. Your repos, issues, stars, and follows live in your PDS. Self-host a node on a Raspberry Pi or use the public AppView.
- **Tangled interop.** Subscribes to `sh.tangled.*` records and translates them via panproto lenses, so Tangled repos appear alongside native Cospan repos.

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                    Frontend                          │
│              SvelteKit 2 + Svelte 5                  │
│     31 routes, dark oklch theme, ATProto OAuth       │
└──────────────────────┬──────────────────────────────┘
                       │ XRPC
┌──────────────────────┴──────────────────────────────┐
│                    AppView                           │
│           Rust + Axum + PostgreSQL                   │
│  36 XRPC endpoints, Jetstream indexer, SSE, OAuth    │
└──────────────────────┬──────────────────────────────┘
                       │ XRPC
┌──────────────────────┴──────────────────────────────┐
│                     Node                             │
│            Rust + panproto-vcs FsStore               │
│  8 XRPC handlers, git smart HTTP, validation        │
└─────────────────────────────────────────────────────┘
```

**Source of truth:** 20 ATProto Lexicon schemas in `packages/lexicons/dev/cospan/`

**Codegen pipeline:** Lexicons → panproto `parse_lexicon()` → schema morphisms → SQL DDL + Rust types + TypeScript interfaces

## Getting started

### Prerequisites

- Rust 1.85+ (`rustup update stable`)
- Node.js 22+ and pnpm 9+
- PostgreSQL 17
- Docker (optional, for database)

### Quick start

```bash
# Clone
git clone https://github.com/cospan-dev/cospan.git
cd cospan

# Start PostgreSQL
docker compose up db redis -d

# Copy env
cp .env.example .env

# Build and run
cargo run -p cospan-node &    # Node on :3001
cargo run -p cospan-appview & # AppView on :3000

# Frontend
cd apps/web
pnpm install
pnpm build
PORT=3000 node build/index.js
```

### Using the CLI

Cospan repos are managed through panproto's `schema` CLI:

```bash
# Initialize a repo
schema init

# Add and commit
schema add .
schema commit -m "initial commit"

# Push to a cospan node
schema remote add origin cospan://node.cospan.dev/did:plc:xyz/my-project
schema push origin main

# Or use git transparently
git push cospan://node.cospan.dev/did:plc:xyz/my-project main
```

## Lexicon schemas

All 19 record types under `dev.cospan.*`:

| Record | Description |
|--------|-------------|
| `dev.cospan.node` | Node declaration |
| `dev.cospan.actor.profile` | User profile (links to Bluesky) |
| `dev.cospan.repo` | Repository with protocol type |
| `dev.cospan.vcs.refUpdate` | Ref update with algebraic metadata |
| `dev.cospan.repo.issue` | Issue with schema element references |
| `dev.cospan.repo.issue.comment` | Issue comment |
| `dev.cospan.repo.issue.state` | Issue state transitions |
| `dev.cospan.repo.pull` | Merge request with pushout preview |
| `dev.cospan.repo.pull.comment` | MR comment with review decisions |
| `dev.cospan.repo.pull.state` | MR state transitions |
| `dev.cospan.repo.dependency` | Cross-repo dependency via theory morphism |
| `dev.cospan.repo.collaborator` | Repo RBAC |
| `dev.cospan.feed.star` | Star |
| `dev.cospan.feed.reaction` | Reaction |
| `dev.cospan.graph.follow` | Follow |
| `dev.cospan.label.definition` | Label |
| `dev.cospan.org` | Organization |
| `dev.cospan.org.member` | Org membership |
| `dev.cospan.pipeline` | CI pipeline with algebraic checks |

## Project structure

```
cospan/
├── crates/
│   ├── cospan-node/        # panproto-vcs node server (Axum)
│   ├── cospan-appview/     # ATProto AppView (Axum + PostgreSQL)
│   └── cospan-codegen/     # Lexicon → SQL/Rust/TS codegen
├── apps/
│   └── web/                # SvelteKit frontend
├── packages/
│   ├── lexicons/           # dev.cospan.* Lexicon JSON schemas
│   └── types/              # Generated TypeScript types
├── docker-compose.yml
├── Caddyfile               # Production reverse proxy
└── Cargo.toml              # Rust workspace
```

## panproto integration

Cospan is a thin collaboration layer on top of panproto. It uses:

| Crate | Purpose |
|-------|---------|
| `panproto-vcs` | Content-addressed schema VCS (FsStore, commits, merges) |
| `panproto-parse` | Full-AST parsing for 248 languages via tree-sitter |
| `panproto-project` | Multi-file project assembly via schema coproduct |
| `panproto-git` | Bidirectional git ↔ panproto-vcs bridge |
| `panproto-check` | Breaking change detection and classification |
| `panproto-lens` | Auto-generated bidirectional migration lenses |
| `panproto-xrpc` | XRPC client for node push/pull/clone |
| `panproto-protocols` | ATProto Lexicon parsing and 77+ protocol definitions |
| `panproto-expr` | Expression language for structural search queries |
| `panproto-io` | Instance-level parse/emit for all protocols |

## Development

```bash
# Lint
cargo clippy --workspace -- -D warnings

# Format
cargo fmt --all

# Test (node + codegen, no database needed)
cargo test -p cospan-node -p cospan-codegen

# Test (appview, needs PostgreSQL)
cargo test -p cospan-appview

# Run codegen
cargo run -p cospan-codegen

# Frontend dev
cd apps/web && pnpm dev
```

## License

[AGPL-3.0-or-later](LICENSE)
