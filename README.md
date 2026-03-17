# ServalRun v2

A modern rewrite of [ServalRun](https://github.com/hazel-ys-lin/serval-run) in Rust -- an automated API integration testing platform with multi-level test execution and background job processing.

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
- Docker & Docker Compose

### Quick Start

```bash
# Start databases
docker-compose up -d

# Copy environment variables
cp .env.example .env

# Run database migrations and start API server
cargo run --bin server

# In another terminal, start the background worker
cargo run --bin worker
```

The API server runs at `http://localhost:3000` with Swagger UI at `http://localhost:3000/swagger-ui/`.

### Development

```bash
# Run tests (requires running databases)
cargo test

# Run with debug logging
RUST_LOG=serval_run=debug cargo run --bin server

# Format and lint
cargo fmt && cargo clippy
```

### Environment Variables

See [.env.example](.env.example) for all configuration options:

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | -- | PostgreSQL connection string |
| `MONGODB_URL` | -- | MongoDB connection string |
| `REDIS_URL` | -- | Redis connection string |
| `JWT_SECRET` | -- | Secret key for JWT signing |
| `JWT_EXPIRATION_HOURS` | `24` | Token expiration time |
| `HOST` | `0.0.0.0` | Server bind address |
| `PORT` | `3000` | Server port |

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
