# Changelog

All notable changes to this project will be documented in this file.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Direction
Pivoting toward a CLI-first, spec-anchored execution layer. See README for the v0.x roadmap.

### Known limitations

- **SQLite (lite mode) stores UUIDs as BLOB, not TEXT.** SeaORM's default
  encoding for `Uuid` on SQLite is the raw 16-byte representation. Our
  migration schema declares `id TEXT PRIMARY KEY`, but SQLite's dynamic
  typing accepts the BLOB silently. Round-trips through the ORM are
  correct; only ad-hoc `sqlite3` CLI queries on the `id` column return
  binary noise. To be fixed after Phase 0 by either tagging entity UUID
  columns with a SeaORM `column_type = "Text"` attribute, or switching
  migrations to BLOB and documenting the choice.

- **Lite mode's `InMemoryQueue` is process-local.** Running the worker
  binary alongside the server under `SERVAL_MODE=lite` does not work
  (each process owns its own queue). Lite mode is currently intended
  for single-process server-only use; cross-process async execution
  requires Full mode.

---

## [0.1.0] - 2026-05-11

Baseline of the Rust rewrite (v2 generation). This release marks the pre-pivot snapshot before transitioning to a CLI-first execution layer.

### Architecture
- Two-binary system: `server` (Axum REST API) + `worker` (background job processor)
- PostgreSQL (SeaORM + SQLx) for structured data
- MongoDB for Gherkin documents and execution logs
- Redis for the job queue backend
- JWT (jsonwebtoken + Argon2) for stateless authentication

### Features
- 8 CRUD entities: Users, Projects, Collections, Environments, APIs, Scenarios, Reports, Responses
- 45 REST API endpoints across auth, project hierarchy, test execution, jobs, and reports
- Gherkin BDD parsing with Scenario Outline, Examples table, Background, and Doc String support
- Multi-level test execution at Scenario, API, and Collection levels
- Sync and async execution modes (Redis job queue with 6 states + retry)
- Refresh token rotation with reuse detection
- Rate limiting (5 req/s auth, 25 req/s general) via tower_governor
- OpenAPI / Swagger UI auto-generated documentation (utoipa)

### Testing
- 17 unit tests
- 125+ integration tests across 8 test suites
- `InMemoryQueue` for tests, `RedisQueue` for production (trait-based DI)

### Infrastructure
- Multi-stage Dockerfile (non-root user, dependency cache)
- docker-compose for PostgreSQL + MongoDB + Redis
- GitHub Actions CI (fmt → clippy → test)
