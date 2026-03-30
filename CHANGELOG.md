# Changelog

## v0.5.3

- All Row struct fields now have `#[serde(default)]` so panproto transforms that don't produce every field don't crash deserialization
- Added `BACKFILL_HOURS` env var: set to replay Jetstream history (up to ~72h) to backfill Tangled and Cospan records
- Restored original logo and favicon

## v0.5.0

### Reskin + Tangled interop

- Complete frontend redesign with DM Sans/DM Mono typography, deep indigo-black palette, dot-grid texture, and protocol-tinted repo cards
- Hero section with schema-first messaging and abstract cospan diagram
- Source tabs (All / Cospan / Tangled) and view tabs (Trending / Recent) on the landing page
- Multi-select language filter with keyboard support
- Tangled source tracking: repos from the firehose now get `source = "tangled"` with `source_uri`
- Source filter on repo list endpoint (`?source=tangled|cospan`)
- Repo import endpoint (`dev.cospan.repo.import`) for importing Tangled repos
- Fixed all Tangled morphisms: replaced `identity_morphism` with `renamed_morphism` for cross-NSID pairs
- Tangled type coercions via panproto expressions: boolean→string (bluesky), hostname→AT-URI (knot→did:web), field renames (subject→memberDid)
- Schema Health display on repo detail (GAT, Equations, Lens Laws, Breaking checks)
- Repo detail shows algebraic check results, lens quality, breaking change count
- Restyled all pages: profile, issues, PRs, search, feed, orgs, settings
- panproto-capabilities skill documenting all panproto expression/transform capabilities

## v0.4.0

### Panproto-native architecture

Every data layer is now powered by panproto — schemas, morphisms, field transforms, and instance parsing replace all hand-written string munging.

**Schema-driven record processing**
- All 130 Lexicon files (56 Cospan + 74 Tangled) parsed via `panproto_protocols::atproto::parse_lexicon()`
- Every incoming Jetstream record goes through `parse_json()` → `lift_wtype_sigma()` → `to_json()` → `serde_json::from_value()`
- DB projection field transforms (AT-URI decomposition, field renames, counter defaults, nested extraction) defined as panproto `FieldTransform` expressions (`ComputeField`, `RenameField`, `AddField`, `DropField`, `PathTransform`)
- 19 Cospan DB projections compiled at codegen time via `panproto_mig::compile()`

**Tangled interop via panproto morphisms**
- 17 Tangled→Cospan morphisms defined as explicit `Migration` vertex/edge maps
- Compiled at codegen time, serialized to msgpack, loaded at appview startup
- Applied at runtime via `lift_wtype_sigma()` — no string template code generation
- Added `scripts/fetch-tangled-lexicons.sh` to pull latest from tangled.org/tangled.org/core

**Generated code from Lexicons**
- sqlx-compatible Row types for all 19 record types
- CRUD functions (upsert, delete, get, list) per record type
- SQL DDL migrations generated from Schema vertices/edges/constraints
- 24 XRPC Input/Params types generated from Lexicon query/procedure definitions
- 36 new Lexicon files for all XRPC query and procedure endpoints
- TypeScript interfaces now exported (`export interface`)
- Breaking change detection via `panproto_check::diff()` + `classify()` (`--check` mode)

**Consumer dispatch**
- Generic dispatch table for simple records (upsert/delete with no side effects)
- Special-case arms only for records with business logic (counter updates, SSE events, state transitions)
- Centralized `at_uri` module replaces all inline AT-URI parsing

**Replaced ~2,700 lines of hand-written code** with panproto-powered codegen and runtime transforms.

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
