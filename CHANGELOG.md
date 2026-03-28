# Changelog

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
