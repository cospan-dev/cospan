# Changelog

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
