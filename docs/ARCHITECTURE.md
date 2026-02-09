# Architecture

## System Overview

ServalRun v2 is a two-binary system: an **API server** and a **background worker**, connected through a Redis job queue, with PostgreSQL as the primary data store and MongoDB for document storage.

```
                    ┌──────────────────┐
                    │     Client       │
                    └────────┬─────────┘
                             │ HTTP + JWT
                             ▼
┌────────────────────────────────────────────────────┐
│                  API Server (Axum)                  │
│                                                    │
│  Handlers → Services → Repositories                │
│                  ↓ (async jobs)                     │
│               JobQueue trait                        │
└──────┬──────────────┬──────────────┬───────────────┘
       │              │              │
       ▼              ▼              ▼
  PostgreSQL       MongoDB         Redis
  (8 tables)    (gherkin docs)   (job queue)
                                     │
                                     │ BRPOP
                                     ▼
                              ┌──────────────┐
                              │    Worker     │
                              │  (executor)   │
                              └──────────────┘
```

## Layer Architecture

```
Handlers (HTTP)      ← Parse requests, return responses
    │
Services             ← Business logic (auth, gherkin parsing, test execution)
    │
Repositories         ← Data access via SeaORM (one per entity)
    │
Entity               ← SeaORM model definitions
    │
PostgreSQL / MongoDB / Redis
```

### Handlers (`src/handlers/`)

10 handler modules corresponding to resource groups. Each handler:
- Extracts `AppState` and authenticated `Claims` from request
- Validates input
- Delegates to repositories or services
- Returns typed JSON responses

Modules: `auth`, `project`, `collection`, `environment`, `api`, `scenario`, `test_run`, `job`, `report`, `common`

### Services (`src/services/`)

- **AuthService** -- JWT token generation/validation, Argon2 password hashing
- **GherkinService** -- Parse `.feature` text into structured scenarios using the `gherkin` crate
- **TestRunner** -- Execute HTTP requests against target APIs, validate responses (status code + JSON body matching), support placeholder substitution from Gherkin examples

### Repositories (`src/repositories/`)

One repository per entity (8 total). Each implements:
- `find_by_id` / `find_by_id_and_user` (ownership check)
- `list_by_*` with pagination (limit/offset)
- `create` / `update` / `delete`

All queries go through SeaORM with the `DatabaseConnection` from `AppState`.

### Queue (`src/queue/`)

Trait-based dependency injection for the job queue:

```rust
#[async_trait]
pub trait JobQueue: Send + Sync {
    async fn enqueue(&self, job: TestJob) -> AppResult<Uuid>;
    async fn dequeue(&self, timeout_seconds: u64) -> AppResult<Option<TestJob>>;
    async fn get_job(&self, job_id: Uuid) -> AppResult<Option<TestJob>>;
    async fn update_status(&self, job_id: Uuid, status: JobStatus) -> AppResult<()>;
    async fn complete_job(&self, job_id: Uuid, result: JobResult) -> AppResult<()>;
    async fn fail_job(&self, job_id: Uuid, error: String, retryable: bool) -> AppResult<()>;
    async fn queue_length(&self) -> AppResult<u64>;
    async fn list_jobs_by_user(&self, user_id: Uuid, limit: u64) -> AppResult<Vec<TestJob>>;
    async fn requeue(&self, job_id: Uuid) -> AppResult<()>;
    async fn delete_job(&self, job_id: Uuid) -> AppResult<()>;
    async fn cancel_job(&self, job_id: Uuid) -> AppResult<()>;
}
```

Two implementations:
- **RedisQueue** -- production, uses Redis List (RPUSH/BRPOP) + Hash for job data
- **InMemoryQueue** -- testing, uses `Arc<Mutex<VecDeque>>` + `tokio::sync::Notify`

### Worker (`src/worker/`)

Separate binary (`cargo run --bin worker`) that:
1. Connects to all databases using the same `AppState`
2. Loops on `job_queue.dequeue(5)` (5-second blocking pop)
3. Executes tests via `JobExecutor` at the appropriate level (scenario/API/collection)
4. Saves results through `ResultHandler` to PostgreSQL
5. Supports graceful shutdown via SIGTERM/SIGINT

Job status lifecycle:
```
Pending → Running → Completed
                  → Failed (retryable → re-enqueue, or permanent)
                  → Cancelled
```

## Application State

```rust
pub struct AppState {
    pub db: DatabaseConnection,        // SeaORM (primary queries)
    pub pg_pool: PgPool,               // SQLx (migrations)
    pub mongo_client: MongoClient,     // MongoDB
    pub redis: RedisConnectionManager, // Redis
    pub config: Config,                // Environment config
    pub job_queue: Arc<dyn JobQueue>,  // DI: swappable queue
}
```

On startup, `AppState::new()` connects to all three databases and runs SQLx migrations automatically.

## Database Design

### PostgreSQL (8 tables)

Entity hierarchy with cascading deletes:

```
users
  └── projects
        ├── environments
        ├── collections
        │     └── apis
        │           └── scenarios
        └── reports
              └── responses
```

Key design decisions:
- All IDs are UUID v4
- All tables have `created_at` / `updated_at` timestamps
- Unique constraints prevent duplicate names within parent scope (e.g., `UNIQUE(project_id, name)` on collections)
- Scenarios store parsed Gherkin as JSONB (`steps` and `examples` columns)
- Reports track `finished`, `calculated`, `pass_rate`, `response_count`
- Responses store individual test results with `pass`, `response_status`, `response_data` (JSONB), `request_duration_ms`

### MongoDB

Used for storing raw Gherkin documents and detailed execution logs -- data with flexible schemas that don't need relational joins.

### Redis

Used exclusively as the job queue backend. Key patterns:
- `serval:jobs:queue` (List) -- FIFO job queue
- `serval:jobs:{id}` (String/JSON) -- job data and status
- `serval:jobs:by_user:{uid}` (Set) -- user's job IDs for listing

## Authentication Flow

1. `POST /api/auth/register` -- hash password with Argon2, store user
2. `POST /api/auth/login` -- verify password, return JWT with `sub` (user email) and `exp`
3. Protected routes pass through `auth_middleware` which:
   - Extracts `Authorization: Bearer <token>` header
   - Validates and decodes JWT
   - Looks up user by email from claims
   - Injects user ID into request extensions

## Test Execution Flow

### Sync mode (immediate)

```
Client → POST /api/scenarios/{id}/run (async_execution: false)
       → Handler builds TestRunner
       → TestRunner executes HTTP requests
       → Results saved to PostgreSQL (report + responses)
       → Return TestRunResponse with results
```

### Async mode (background)

```
Client → POST /api/scenarios/{id}/run (async_execution: true)
       → Handler creates TestJob
       → job_queue.enqueue(job)
       → Return AsyncTestResponse with job_id

Worker → job_queue.dequeue()
       → JobExecutor runs tests
       → ResultHandler saves report + responses
       → job_queue.complete_job() with JobResult
```

## Error Handling

`AppError` enum maps to HTTP status codes:

| Error | HTTP Status |
|-------|-------------|
| `InvalidCredentials`, `InvalidToken`, `TokenExpired`, `Unauthorized` | 401 |
| `NotFound` | 404 |
| `Conflict` | 409 |
| `Validation` | 400 |
| `Database`, `Internal`, `Queue` | 500 |

Automatic conversions from `sqlx::Error`, `sea_orm::DbErr`, `argon2::Error`, and `jsonwebtoken::Error`.

## Testing

Integration tests use `axum-test` to spin up the full application router and test endpoints directly. Test infrastructure:

- `tests/common/app.rs` -- creates test `AppState` with `InMemoryQueue`
- `tests/common/factory.rs` -- helper functions to create test data
- 8 test suites: `auth_test`, `project_test`, `collection_test`, `environment_test`, `api_test`, `scenario_test`, `job_test`, `report_test`

## Key Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| axum | 0.8 | HTTP framework |
| sea-orm | 1.1 | ORM for PostgreSQL |
| sqlx | 0.8 | Migrations |
| mongodb | 3.5 | MongoDB driver |
| redis | 1.0 | Redis driver |
| jsonwebtoken | 9 | JWT auth |
| argon2 | 0.5 | Password hashing |
| utoipa | 5 | OpenAPI spec generation |
| gherkin | 0.15 | BDD test parser |
| reqwest | 0.13 | HTTP client (test runner) |
| axum-test | 18.7 | Integration testing |
