# Changelog

## v0.4.0

### Panproto-powered codegen

- Enhanced cospan-codegen to generate sqlx-compatible Row types with denormalization config for all 19 record types
- Generated CRUD functions (upsert, get, delete, list) for all 19 record types
- Generated `from_json()` Jetstream record deserializers with AT-URI decomposition, field renames, and type overrides
- TypeScript interfaces now exported (`export interface`)
- Integrated panproto-check for Lexicon breaking change detection (`--check` mode)
- Added `schema-check` CI job for automated breaking change detection
- Schema baseline saved to `generated/sql/baseline.json` for diffing
- Removed unused `panproto-core` dependency from cospan-codegen
- Generated code written to `crates/cospan-appview/src/db/generated/` for direct appview integration
- Replaced 19 hand-written database modules with re-exports from generated code (keeping custom queries)
- Replaced manual JSON field extraction in consumer.rs with generated `from_json()` deserializers

### Tangled interop codegen

- Added 74 Tangled lexicon files under `packages/lexicons/sh/tangled/`
- Added `scripts/fetch-tangled-lexicons.sh` for pulling latest from tangled.org/tangled.org/core
- Generated 19 Tangled→Cospan interop morphisms with `from_tangled_json()` methods
- Replaced 15 manual Tangled Row constructions in consumer.rs with generated morphisms

## v0.3.2

- fix: include all 248 languages (was filtering out injection grammars)

## v0.3.1

- fix: profile avatar display, consistent page titles, UX polish

## v0.3.0

### Frontend UX revamp

- Mobile responsive header with hamburger menu (nav links, search, auth hidden on small screens)
- Horizontal scroll for tab bars and protocol filters on mobile
- Stacked layout for profile pages on small screens
- Consistent breadcrumb and tab bar across all repo sub-pages
- Consistent empty states with icons and CTAs on all list pages
- Fixed followers, following, and stars pages (API response key mapping)
- Homogeneous design patterns: consistent spacing, text sizes, card styles, and form layouts across all 31 routes
- Error page improvements
- Logo integrated into header and favicon (dark mode optimized with light strokes)

### Auth improvements

- Production OAuth with `token_endpoint_auth_method: "none"` for browser client
- Root URL added to `redirect_uris` for proper callback handling
- `transition:generic` scope for write permissions (create issues, stars, follows)

### Deployment fixes

- Pre-built Docker images on GHCR (no compilation on the server)
- Fixed Caddyfile syntax for production reverse proxy
- Fixed node container permissions for `/data` volume
- Fixed web container missing `@sveltejs/kit` runtime dependency
- Rust 1.88+ in Dockerfiles (MSRV for home, time crates)

## v0.2.4

- fix(auth): request `transition:generic` scope for write permissions

## v0.2.3

- feat: add logo to header and favicon (dark mode optimized with light strokes)

## v0.2.2

- fix(auth): add root URL to `redirect_uris` for browser OAuth client

## v0.2.1

- fix(auth): use `token_endpoint_auth_method: "none"` for browser OAuth client

## v0.2.0

### Tangled interop

- Complete interop for all 24 `sh.tangled.*` record types
- Translates stars, follows, reactions, issues, pulls, comments, state changes, repos, knots, spindles, profiles, labels, pipelines, collaborators, and git ref updates
- Tangled-only records (publicKey, string, artifact, label.op) logged and skipped
- All translated records tracked with `source: "tangled"` and `source_uri`

### panproto v0.20.0

- Upgraded to panproto v0.20.0 with `group-all` enabling all 248 tree-sitter language grammars
- Resolved C++ scanner linker issues on Linux (COMDAT section deduplication with `rust-lld`)
- Resolved unfetched grammar symbol errors (circom, fidl, postscript, prolog, qml)

### Database tests

- Fixed foreign key constraint violations in DB and XRPC test suites
- Tests now insert prerequisite node and repo records before child records

### Production deployment

- Added `docker-compose.prod.yml` with Caddy (auto-TLS), Postgres, Redis, node, appview, and frontend
- Added `Caddyfile.prod` for reverse proxy with automatic Let's Encrypt certificates
- Added `scripts/deploy.sh` for single-command deployment to a VPS
- Added `.env.production.example` with required configuration
- Updated Dockerfiles with build dependencies for tree-sitter grammars (cmake, g++, libssl)

## v0.1.0

Initial release. See [release notes](https://github.com/cospan-dev/cospan/releases/tag/v0.1.0).
