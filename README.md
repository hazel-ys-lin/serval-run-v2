# ServalRun v2

A modern rewrite of [ServalRun](https://github.com/hazel-ys-lin/serval-run) in Rust -- an automated API integration testing platform with multi-level test execution and background job processing.

> **Current status**: `v0.1.0` -- REST API + worker baseline. Now pivoting toward a CLI-first, spec-anchored execution layer (see [Direction](#direction) below).

## Direction

Evolving from a REST-only testing platform into a **source-agnostic spec execution layer** -- a CLI tool that runs, mocks, verifies, and feeds API specs to AI agents, regardless of where the spec came from (Gherkin, OpenAPI, AsyncAPI, etc.).

### v0.x -- pivot phase

| Tag | Milestone |
|-----|-----------|
| [x] `v0.1.0` | REST API + worker baseline (current) |
| [ ] `v0.2.0` | Lite mode: SQLite + in-memory queue, no Docker required |
| [ ] `v0.3.0` | CLI primary interface (`servalrun run / diff / mock / spec`) |
| [ ] `v0.4.0` | Specs-as-code: `.feature` files in git, DB as cache |
| [ ] `v0.5.0` | Mock server mode (one spec serves as mock + impl target + test) |
| [ ] `v0.6.0` | Agent eval harness (compressed spec format + structured diff for Claude Code etc.) |

### v1.0.0 -- target

A stable, source-agnostic spec execution layer:
- Reads any common spec format (Gherkin, OpenAPI 3.x, AsyncAPI)
- Three deployment contexts share the same CLI: local dev, CI/CD, agent loops
- Generates language-neutral test fixtures (consumed by Claude Code or similar to write language-specific tests)
- No code generation, no visualisation, no authoring UI -- those belong to upstream tools

See [CHANGELOG.md](CHANGELOG.md) for release notes.

## v1 → v2 Comparison

| Category | v1 (Node.js) | v2 (Rust) |
|---|---|---|
| **Language** | JavaScript (Node.js) | Rust |
| **Framework** | Express.js v4 | Axum 0.8 + Tokio |
| **Architecture** | Full-stack (Pug templates + REST) | REST API only |
| **Primary DB** | MongoDB only | PostgreSQL (structured) + MongoDB (documents) |
| **ORM / ODM** | Mongoose v6 | SeaORM 1.1 + SQLx 0.8 |
| **Auth** | Session-based (express-session) | JWT (stateless) |
| **Password hashing** | bcryptjs (cost 8) | Argon2id (stronger) |
| **Job queue** | Redis list — 2 states (queued / done) | Redis — 6 states (Pending → Running → Completed / Failed / Dead / Cancelled) + retry |
| **Worker shutdown** | `while(true)` loop, no cleanup | `tokio::select!` + graceful shutdown |
| **Real-time updates** | Redis pub/sub + Socket.IO | — (async job polling) |
| **Rate limiting** | None | 5 req/s (auth) / 25 req/s (general) via tower_governor |
| **API documentation** | None | OpenAPI / Swagger UI (utoipa) |
| **Input validation** | express-validator | Custom validators + length limits on all fields |
| **Error handling** | `try/catch`, raw errors exposed | Typed `AppError` enum, internal errors never leak |
| **Test coverage** | 0 (test files commented out) | 17 unit + 125+ integration tests |
| **CI/CD** | None | GitHub Actions (fmt → clippy → test) |
| **Docker** | None | Multi-stage Dockerfile + docker-compose |
| **Type safety** | Runtime (JavaScript) | Compile-time (Rust) |

## Tech Stack

- **Rust** + **Axum** + **Tokio** -- async web framework and runtime
- **PostgreSQL** (SeaORM + SQLx) -- structured data, migrations, 8 entity tables
- **MongoDB** -- document storage for Gherkin docs and execution logs
- **Redis** -- job queue backend with trait-based DI
- **JWT** (jsonwebtoken + Argon2) -- stateless authentication
- **OpenAPI/Swagger UI** (utoipa) -- auto-generated API documentation

## Features

- **8 CRUD entities** -- Users, Projects, Collections, Environments, APIs, Scenarios, Reports, Responses
- **Gherkin BDD support** -- parse and create scenarios from `.feature` syntax
- **Multi-level test execution** -- run tests at Scenario, API, or Collection level
- **Sync and async modes** -- immediate results or background job queue
- **Background worker** -- separate binary (`cargo run --bin worker`) with graceful shutdown
- **Job queue with DI** -- `JobQueue` trait with Redis (production) and InMemory (testing) implementations
- **Ownership isolation** -- all resources scoped to authenticated user
- **45 API endpoints** -- full CRUD + test execution + job management + reports
- **Integration tests** -- 8 test suites covering all endpoint groups

## Getting Started

### Prerequisites

- Rust 1.75+
- Docker & Docker Compose -- *only required for full mode*

ServalRun runs in two modes. Pick one based on what you're doing.

### Quick Start -- Lite mode (no docker, recommended for local dev)

Single-process server backed by SQLite and an in-memory job queue. Boots in seconds, leaves no containers behind.

```bash
SERVAL_MODE=lite \
DATABASE_URL=sqlite:./serval.db \
JWT_SECRET=$(openssl rand -hex 32) \
cargo run --bin server
```

The API server runs at `http://localhost:3000`. The `/health` endpoint reports `mongodb` and `redis` as `"not_configured"` -- this is expected.

Caveats:
- MongoDB writes (Gherkin docs, execution logs) are skipped silently. These are already non-fatal in full mode, so behaviour matches.
- The background `worker` binary is **not** supported in lite mode -- the in-memory queue is process-local, so sync test execution only.
- See [CHANGELOG.md](CHANGELOG.md) for other lite-mode caveats.

### Quick Start -- Full mode (Postgres + MongoDB + Redis)

Original v0.1.0 shape; required for shared / team deployments, async test execution, and Mongo-backed log storage.

```bash
# Start the database stack
docker-compose up -d

# Copy environment variables
cp .env.example .env

# Run database migrations and start API server (default mode is full)
cargo run --bin server

# In another terminal, start the background worker
cargo run --bin worker
```

The API server runs at `http://localhost:3000` with Swagger UI at `http://localhost:3000/swagger-ui/`.

### `servalrun` CLI

A `servalrun` CLI is being built up alongside the server (Phase 1 of the v0.x roadmap). Current surface is small; more subcommands land each PR.

```bash
# Build and run from the workspace
cargo run --bin servalrun -- --help
cargo run --bin servalrun -- --version

# Health-check whichever server you point at (defaults to localhost:3000)
cargo run --bin servalrun -- status
cargo run --bin servalrun -- status --server http://localhost:3000 --json
SERVAL_SERVER=http://staging.example.com cargo run --bin servalrun -- status

# Install as a real binary on PATH
cargo install --path . --bin servalrun
servalrun status
```

Exit codes are stable across subcommands: `0` ok, `1` test/spec assertion failed, `2` system error (network / auth / server down), `3` bad input.

### Development

```bash
# Library tests only (lite-mode + SQLite smoke tests; no docker required)
cargo test --lib

# Full integration test suite (needs the docker stack up)
docker-compose up -d
cargo test --test '*'

# Run with debug logging
RUST_LOG=serval_run=debug cargo run --bin server

# Format and lint
cargo fmt && cargo clippy
```

### Environment Variables

See [.env.example](.env.example) for all configuration options:

| Variable | Default | Required in | Description |
|----------|---------|-------------|-------------|
| `SERVAL_MODE` | `full` | -- | `full` or `lite`. Unknown values fall back to `full` |
| `DATABASE_URL` | -- | both modes | `postgres://...` for full mode, `sqlite:...` for lite mode |
| `MONGODB_URL` | -- | full only | MongoDB connection string (ignored in lite) |
| `REDIS_URL` | -- | full only | Redis connection string (ignored in lite) |
| `JWT_SECRET` | -- | both modes | Secret key for JWT signing |
| `JWT_EXPIRATION_HOURS` | `24` | both modes | Token expiration time |
| `HOST` | `0.0.0.0` | both modes | Server bind address |
| `PORT` | `3000` | both modes | Server port |

## API Overview

| Group | Endpoints | Description |
|-------|-----------|-------------|
| Auth | `POST /api/auth/register`, `login`, `GET me`, `PUT me` | Registration, login, profile |
| Projects | CRUD under `/api/projects` | Project management |
| Collections | Nested under projects, direct access by ID | API grouping |
| Environments | Nested under projects, direct access by ID | Domain/base URL config |
| APIs | Nested under collections, direct access by ID | HTTP endpoint definitions |
| Scenarios | Nested under APIs, direct access by ID | Test cases with Gherkin support |
| Test Execution | `POST /api/{scenarios,apis,collections}/{id}/run` | Run tests at 3 levels |
| Jobs | `/api/jobs` -- list, status, cancel, requeue, stats | Background job management |
| Reports | Nested under projects, direct access by ID | Test results and details |

All protected endpoints require `Authorization: Bearer <token>` header.

## Project Structure

```
src/
  main.rs                 # API server entry point
  lib.rs                  # Library crate, router definition
  config.rs               # Environment-based configuration
  error.rs                # AppError type with HTTP status mapping
  state.rs                # AppState (DB connections + job queue)
  entity/                 # SeaORM entities (8 models)
  models/                 # Domain models and request/response types
  repositories/           # Data access layer (8 repositories)
  services/               # Business logic (auth, gherkin, test_runner)
  handlers/               # HTTP handlers (10 modules)
  middlewares/            # JWT auth middleware
  queue/                  # JobQueue trait + Redis/InMemory implementations
  worker/
    main.rs               # Worker binary entry point
    executor.rs           # Job execution logic
    result_handler.rs     # Save test results to DB
migrations/               # 8 SQL migration files
tests/                    # Integration tests (8 test suites)
docker-compose.yml        # PostgreSQL, MongoDB, Redis
```

## Architecture

See [ARCHITECTURE.md](docs/ARCHITECTURE.md) for detailed system design, database schema, data flows, and design decisions.

## Author

**Hazel Lin** -- [GitHub](https://github.com/hazel-ys-lin) | [LinkedIn](https://www.linkedin.com/in/hazel-lin-yi-sin/)

## License

MIT
