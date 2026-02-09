# ServalRun v2

![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)
![PostgreSQL](https://img.shields.io/badge/PostgreSQL-316192?style=for-the-badge&logo=postgresql&logoColor=white)
![MongoDB](https://img.shields.io/badge/MongoDB-4EA94B?style=for-the-badge&logo=mongodb&logoColor=white)
![Redis](https://img.shields.io/badge/redis-%23DD0031.svg?&style=for-the-badge&logo=redis&logoColor=white)

> A modern rewrite of ServalRun - an automated API integration testing tool

**ServalRun v2** is a complete rewrite of the original Node.js project in Rust, showcasing modern backend development practices and performance optimization.

## 🚀 Why Rewrite?

This project serves multiple purposes:
- **Learning Rust**: Deep dive into systems programming and async Rust
- **Performance**: Achieve 2-3x performance improvement over the Node.js version
- **Modern Architecture**: Showcase multi-database integration and microservices patterns

## ⚡ Performance Improvements (Goals)

| Metric                     | v1 (Node.js) | v2 (Rust)   | Improvement     |
| -------------------------- | ------------ | ----------- | --------------- |
| Single API Test            | ~150ms       | ~50ms       | **3x faster**   |
| Collection Test (100 APIs) | ~33s         | ~15s        | **2.2x faster** |
| Report Query               | ~1.7s        | ~150ms      | **11x faster**  |
| Memory Usage               | ~200MB       | ~50MB       | **4x less**     |
| Concurrent Requests        | ~1000 req/s  | ~5000 req/s | **5x more**     |

## 🏗️ Architecture

### Tech Stack

**Backend**
- **Rust** - Systems programming language
- **Axum** - Modern async web framework
- **Tokio** - Async runtime

**Databases**
- **PostgreSQL** - Structured data, transactions, complex queries
- **MongoDB** - Document storage, flexible schemas, logs
- **Redis** - Task queue, pub/sub

**Key Libraries**
- **SeaORM** - Async ORM for PostgreSQL
- **SQLx** - Compile-time SQL verification + migrations
- **utoipa** - OpenAPI/Swagger documentation
- **jsonwebtoken** - JWT authentication
- **tracing** - Structured logging
- **gherkin** - BDD test parser

### Multi-Database Strategy

```
┌─────────────┐
│ PostgreSQL  │ → User data, Projects, APIs, Test results (structured)
├─────────────┤
│  MongoDB    │ → Gherkin documents, Execution logs (flexible)
├─────────────┤
│   Redis     │ → Task queue, Real-time progress (fast)
└─────────────┘
```

**Why this approach?**
- **PostgreSQL**: Strong consistency, ACID transactions, complex JOINs
- **MongoDB**: Flexible schema for Gherkin documents and execution logs
- **Redis**: High-performance task queue and pub/sub for real-time updates

### System Architecture

```
┌──────────────┐         ┌──────────────┐
│  API Server  │────────▶│ PostgreSQL   │
│   (Axum)     │         │   (SQLx)     │
└──────┬───────┘         └──────────────┘
       │
       │ Push Task
       ▼
┌──────────────┐         ┌──────────────┐
│    Redis     │────────▶│   Worker     │
│    Queue     │  Pop    │  (Tokio)     │
└──────┬───────┘         └──────┬───────┘
       │                        │
       │ Pub/Sub                │ Save Logs
       ▼                        ▼
┌──────────────┐         ┌──────────────┐
│  WebSocket   │         │   MongoDB    │
│   (Axum)     │         │              │
└──────────────┘         └──────────────┘
```

## 🎯 Features

- **Multi-level Testing**
  - Collection level (batch API tests)
  - API level (all scenarios for one API)
  - Scenario level (single test case)

- **Gherkin Support**
  - Write tests in human-readable BDD format
  - Automatic parsing and validation

- **Real-time Updates**
  - WebSocket-based progress streaming
  - Live test result charts

- **Comprehensive Reports**
  - Detailed execution results
  - Statistics (pass rate, response time)
  - Historical data archival

## ✅ Implemented Features (v2)

### Core CRUD (8 entities)
- Users, Projects, Collections, Environments
- APIs, Scenarios, Reports, Responses
- Full ownership verification (user isolation)

### Authentication & Security
- JWT-based stateless authentication
- Argon2 password hashing
- Protected routes with middleware
- Token expiration handling

### Test Execution Engine
- **3 levels**: Scenario / API / Collection
- **2 modes**: Sync (immediate) / Async (background queue)
- HTTP client with configurable timeout
- Response validation (status code + JSON body matching)
- Placeholder substitution from Gherkin examples

### Job Queue System (DI Architecture)
- Abstract `JobQueue` trait for dependency injection
- Redis implementation (production)
- InMemory implementation (unit testing)
- Retry logic with configurable max_retries
- Job status tracking (Pending → Running → Completed/Failed)

### Background Worker
- Separate binary: `cargo run --bin worker`
- Graceful shutdown (SIGTERM/SIGINT)
- Result persistence to PostgreSQL

### API Documentation
- OpenAPI 3.0 specification
- Swagger UI at `/swagger-ui/`
- 44+ documented endpoints
- All request/response schemas

## 🚦 Getting Started

### Prerequisites

- Rust 1.75+ ([install](https://rustup.rs/))
- Docker & Docker Compose ([install](https://docs.docker.com/get-docker/))

### Quick Start

```bash
# Clone the repository
git clone https://github.com/hazel-ys-lin/serval-run-v2.git
cd serval-run-v2

# Start databases with Docker Compose
docker-compose up -d

# Copy environment variables
cp .env.example .env

# Run database migrations
cargo install sqlx-cli --features postgres
sqlx migrate run

# Start API server
cargo run --bin server

# In another terminal, start worker
cargo run --bin worker
```

The API will be available at `http://localhost:3000`

### Development

```bash
# Run with auto-reload
cargo install cargo-watch
cargo watch -x run

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run

# Format code
cargo fmt

# Lint
cargo clippy
```

## 📚 API Documentation

See [API_DOCS.md](./API_DOCS.md) for detailed API documentation.

### Quick Example

```bash
# Register
curl -X POST http://localhost:3000/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email":"test@example.com","password":"password123","name":"Test User"}'

# Login (get JWT)
curl -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"test@example.com","password":"password123"}'

# Create project (with JWT)
curl -X POST http://localhost:3000/api/projects \
  -H "Authorization: Bearer <your-jwt-token>" \
  -H "Content-Type: application/json" \
  -d '{"name":"My Project","description":"API testing project"}'

# Run test (async mode)
curl -X POST http://localhost:3000/api/scenarios/{id}/run \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"environment_id":"...","async_execution":true}'
```

### Swagger UI

Visit `http://localhost:3000/swagger-ui/` for interactive API documentation with all 44+ endpoints.

## 🧪 Testing with Gherkin

Write your tests in Gherkin format:

```gherkin
Feature: User Authentication

  Scenario: User signs in successfully
    Given I am a registered user
    When I enter valid credentials
    Then I should see status 200

  Examples:
    | email            | password | status |
    | test@example.com | 123456   | 200    |
    | test@example.com | wrong    | 401    |
```

## 📁 Project Structure

```
serval-run-v2/
├── src/
│   ├── main.rs              # API Server entry point
│   ├── config.rs            # Configuration management
│   ├── error.rs             # Error types and handling
│   ├── state.rs             # AppState (DB connections)
│   ├── entity/              # SeaORM entities (8 models)
│   ├── models/              # Domain models
│   ├── repositories/        # Data access layer (Repository pattern)
│   ├── services/            # Business logic
│   │   ├── auth.rs          # JWT authentication
│   │   ├── gherkin.rs       # Gherkin parsing
│   │   └── test_runner.rs   # Test execution engine
│   ├── handlers/            # HTTP handlers (10 modules)
│   ├── middlewares/         # Auth middleware
│   ├── queue/               # Job Queue (DI pattern)
│   │   ├── mod.rs           # JobQueue trait
│   │   ├── job.rs           # TestJob, JobStatus
│   │   ├── redis_queue.rs   # Redis implementation
│   │   └── memory_queue.rs  # InMemory implementation
│   └── worker/              # Background worker
│       ├── main.rs          # Worker binary entry
│       ├── executor.rs      # Job executor
│       └── result_handler.rs
├── migrations/              # 8 SQL migrations
├── Cargo.toml               # Rust dependencies
└── docker-compose.yml       # Development environment
```

## 📊 v1 vs v2 Comparison

| Feature | v1 (Node.js) | v2 (Rust) |
|---------|--------------|-----------|
| **Language** | Node.js + Express | Rust + Axum |
| **Database** | MongoDB only | PostgreSQL + MongoDB + Redis |
| **ORM** | Mongoose | SeaORM + SQLx |
| **Authentication** | Express Session (Stateful) | JWT (Stateless) |
| **Password Hashing** | bcrypt | Argon2 |
| **Queue** | Bull (Redis) | Custom JobQueue trait + Redis |
| **WebSocket** | Socket.IO | ❌ Planned (Phase 4) |
| **API Docs** | None | OpenAPI/Swagger UI |
| **Type Safety** | Runtime (JavaScript) | Compile-time (Rust) |
| **Testing** | Jest | cargo test (17+ tests) |

## 🎓 Learning Journey

This project demonstrates:

1. **Rust Fundamentals**
   - Ownership & borrowing
   - Error handling with `Result` and `?`
   - Async/await with Tokio
   - Traits and generics

2. **Backend Development**
   - RESTful API design
   - JWT authentication
   - Multi-database architecture
   - Background job processing

3. **Database Skills**
   - Complex SQL queries (JOINs, CTEs, Window Functions)
   - Database transactions
   - Query optimization with indexes
   - NoSQL document modeling

4. **System Design**
   - Producer-consumer pattern
   - Pub/sub messaging
   - Real-time communication (WebSocket)
   - Microservices architecture

5. **DevOps**
   - Docker containerization
   - Database migrations
   - Structured logging
   - Testing strategies

## 🔍 Hybrid Database Approach: SeaORM + SQLx

This project uses a **hybrid approach** combining the best of both worlds:

**SeaORM (Primary ORM)**
- ✅ Async ORM with Rust-native API
- ✅ Entity definitions with compile-time checking
- ✅ Fluent query builder for CRUD operations
- ✅ Relationship handling (belongs_to, has_many)

**SQLx (Migrations & Raw Queries)**
- ✅ Compile-time SQL verification for complex queries
- ✅ Database migrations management
- ✅ Full control when needed

**Example:**
```rust
// SeaORM for typical CRUD
let user = UserEntity::find_by_id(id)
    .filter(Column::UserId.eq(user_id))
    .one(&db)
    .await?;

// SQLx for migrations
sqlx::migrate!("./migrations").run(&pool).await?;
```

## 📊 Performance Benchmarks

(Coming soon - detailed benchmarks comparing v1 vs v2)

## 🛣️ Roadmap

- [x] Phase 0: Project planning and architecture design
- [x] Phase 1: Basic API server + JWT authentication + CRUD
- [x] Phase 2: Gherkin parsing + scenario management
- [x] Phase 3: Job Queue (DI architecture) + Worker implementation
- [ ] Phase 4: WebSocket real-time updates
- [ ] Phase 5: Performance optimization + comprehensive testing
- [ ] Frontend rewrite (React + TypeScript)

See [PROJECT_PLAN.md](./docs/PROJECT_PLAN.md) for detailed roadmap.

## 🤝 Contributing

This is primarily a learning project, but suggestions and feedback are welcome!

## 📝 License

MIT

## 👤 Author

**Hazel Lin**
- GitHub: [@hazel-ys-lin](https://github.com/hazel-ys-lin)
- LinkedIn: [hazel-lin-yi-sin](https://www.linkedin.com/in/hazel-lin-yi-sin/)
- Email: hazel.ys.lin@gmail.com

## 🙏 Acknowledgments

- Original ServalRun v1 project (2021)
- Rust community and ecosystem
- [Axum](https://github.com/tokio-rs/axum) framework
- [SQLx](https://github.com/launchbadge/sqlx) for compile-time SQL

---

**From Node.js to Rust - A Journey of Performance and Type Safety** 🚀
