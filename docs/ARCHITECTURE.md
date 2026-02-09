# Architecture Documentation

## Implementation Status

| Component | Status | Notes |
|-----------|--------|-------|
| **Multi-Database** | ✅ Done | PostgreSQL + MongoDB + Redis |
| **Core CRUD** | ✅ Done | 8 entities with full CRUD |
| **Authentication** | ✅ Done | JWT + Argon2 |
| **Gherkin Parser** | ✅ Done | BDD test support |
| **Test Execution** | ✅ Done | Sync + Async modes |
| **Job Queue (DI)** | ✅ Done | Redis + InMemory implementations |
| **Worker** | ✅ Done | Background processing |
| **WebSocket** | ❌ Planned | Phase 4 |

## Overview

ServalRun v2 is built using a **hybrid multi-database architecture** that leverages the strengths of PostgreSQL, MongoDB, and Redis to create a high-performance API testing platform.

## System Architecture

### High-Level Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         Client (Browser/API)                    │
└────────────────────────────┬────────────────────────────────────┘
                             │
                             │ HTTP/WebSocket
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                       Axum API Server                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐           │
│  │  Handlers    │  │  Middleware  │  │  Services    │           │
│  │  (HTTP)      │  │  (Auth, Log) │  │  (Business)  │           │
│  └──────────────┘  └──────────────┘  └──────────────┘           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐           │
│  │ WebSocket    │  │ Repositories │  │  Models      │           │
│  │  (Progress)  │  │  (Data)      │  │  (Structs)   │           │
│  └──────────────┘  └──────────────┘  └──────────────┘           │
└────────┬────────────────────┬────────────────────┬──────────────┘
         │                    │                    │
         │                    │                    │
         ▼                    ▼                    ▼
┌────────────────┐   ┌────────────────┐   ┌────────────────┐
│  PostgreSQL    │   │   MongoDB      │   │     Redis      │
│                │   │                │   │                │
│ - Users        │   │ - Gherkin Docs │   │ - Task Queue   │
│ - Projects     │   │ - Exec Logs    │   │ - Pub/Sub      │
│ - APIs         │   │ - Archives     │   │ - Counters     │
│ - Reports      │   │                │   │                │
└────────────────┘   └────────────────┘   └───┬────────────┘
                                              │
                                              │ BRPOP
                                              ▼
                                     ┌────────────────┐
                                     │  Worker        │
                                     │                │
                                     │ - HTTP Exec    │
                                     │ - Result Save  │
                                     │ - Progress Pub │
                                     └────────────────┘
```

## Data Flow

### 1. User Creates a Test Scenario

```
User → API Server → Gherkin Parser
                   ↓
        ┌──────────┴──────────┐
        ▼                     ▼
   PostgreSQL            MongoDB
   (metadata)         (raw gherkin)
```

**Steps:**
1. User submits Gherkin code
2. Server parses with `gherkin` crate
3. Save metadata to PostgreSQL (`scenarios` table)
4. Save raw document to MongoDB (`gherkin_documents` collection)
5. Return scenario ID to user

### 2. User Runs a Test

```
User → API Server
       ↓
    Create Report (PostgreSQL)
       ↓
    Create Response Records (PostgreSQL)
       ↓
    Build Test Job
       ↓
    Push to Redis Queue (RPUSH)
       ↓
    Return Report ID
```

### 3. Worker Executes Test

```
Worker (blocking on Redis)
  ↓
BRPOP "requestList" → Get Job
  ↓
For each test example:
  ├─ Execute HTTP Request (reqwest)
  ├─ Update Response (PostgreSQL)
  ├─ Save Execution Log (MongoDB)
  ├─ Publish Progress (Redis Pub/Sub)
  └─ Check if Report Complete
      ↓
  Calculate Statistics (PostgreSQL)
      ↓
  Mark Report as Finished
```

### 4. Real-time Progress Update

```
Worker
  ↓
Redis PUBLISH "report_channel"
  ↓
API Server (subscribed)
  ↓
WebSocket emit to client
  ↓
Client updates UI
```

## Database Design

### PostgreSQL Schema

#### Core Tables

**users**
```sql
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    name VARCHAR(255),
    job_title VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

**projects**
```sql
CREATE TABLE projects (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(user_id, name)
);
```

**environments**
```sql
CREATE TABLE environments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    domain_name VARCHAR(500) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(project_id, title)
);
```

**collections**
```sql
CREATE TABLE collections (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(project_id, name)
);
```

**apis**
```sql
CREATE TABLE apis (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    collection_id UUID NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    http_method VARCHAR(10) NOT NULL,
    endpoint VARCHAR(500) NOT NULL,
    severity SMALLINT DEFAULT 0,
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(collection_id, name)
);
```

**scenarios**
```sql
CREATE TABLE scenarios (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    api_id UUID NOT NULL REFERENCES apis(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    tags TEXT[],
    steps JSONB NOT NULL,      -- Parsed Gherkin steps (keyword, text, docString, dataTable)
    examples JSONB NOT NULL,   -- Test examples with expected_status_code
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(api_id, title)
);
```

**reports**
```sql
CREATE TABLE reports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    environment_id UUID NOT NULL REFERENCES environments(id),
    collection_id UUID REFERENCES collections(id),
    report_level SMALLINT NOT NULL,       -- 0: scenario, 1: api/collection
    report_type VARCHAR(50),              -- scenario, api, collection
    finished BOOLEAN DEFAULT FALSE,
    calculated BOOLEAN DEFAULT FALSE,
    pass_rate DECIMAL(5, 2),
    response_count INT DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    finished_at TIMESTAMPTZ
);
```

**responses**
```sql
CREATE TABLE responses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    report_id UUID NOT NULL REFERENCES reports(id) ON DELETE CASCADE,
    api_id UUID NOT NULL REFERENCES apis(id),
    scenario_id UUID NOT NULL REFERENCES scenarios(id),
    example_index INT NOT NULL,
    response_status SMALLINT,
    response_data JSONB,
    pass BOOLEAN NOT NULL,
    error_message TEXT,
    request_time TIMESTAMPTZ NOT NULL,
    request_duration_ms INT
);
```

#### Key Indexes

```sql
-- User lookup
CREATE INDEX idx_users_email ON users(email);

-- Project queries
CREATE INDEX idx_projects_user_id ON projects(user_id);
CREATE INDEX idx_environments_project_id ON environments(project_id);
CREATE INDEX idx_collections_project_id ON collections(project_id);

-- API hierarchy
CREATE INDEX idx_apis_collection_id ON apis(collection_id);
CREATE INDEX idx_scenarios_api_id ON scenarios(api_id);

-- Report queries
CREATE INDEX idx_reports_project_id ON reports(project_id);
CREATE INDEX idx_reports_finished ON reports(finished) WHERE finished = FALSE;
CREATE INDEX idx_reports_created_at ON reports(created_at DESC);

-- Response queries
CREATE INDEX idx_responses_report_id ON responses(report_id);
CREATE INDEX idx_responses_pass ON responses(pass);

-- JSONB indexes
CREATE INDEX idx_scenarios_steps ON scenarios USING GIN(steps);
CREATE INDEX idx_scenarios_tags ON scenarios USING GIN(tags);
```

### MongoDB Collections

#### gherkin_documents

```javascript
{
  _id: ObjectId("..."),
  scenario_id: "12345",  // PostgreSQL scenario.id
  raw_gherkin: `
Feature: User Authentication
  Scenario: Sign in
    Given I am a user
    When I sign in
    Then I should see status 200
  `,
  parsed_steps: [
    { keyword: "Given", text: "I am a user" },
    { keyword: "When", text: "I sign in" },
    { keyword: "Then", text: "I should see status 200" }
  ],
  examples: [
    { email: "test@example.com", password: "123456", status: 200 }
  ],
  version: 1,
  created_at: ISODate("2025-01-20T00:00:00Z"),
  updated_at: ISODate("2025-01-20T00:00:00Z")
}
```

**Indexes:**
```javascript
db.gherkin_documents.createIndex({ scenario_id: 1 }, { unique: true })
db.gherkin_documents.createIndex({ created_at: -1 })
```

#### execution_logs

```javascript
{
  _id: ObjectId("..."),
  response_id: "67890",  // PostgreSQL response.id
  request: {
    method: "POST",
    url: "https://api.example.com/auth/signin",
    headers: { "Content-Type": "application/json" },
    body: { email: "test@example.com", password: "123456" },
    sent_at: ISODate("2025-01-20T10:00:00.123Z")
  },
  response: {
    status: 200,
    headers: { "Content-Type": "application/json" },
    body: { token: "eyJ...", user: { id: 1, email: "test@example.com" } },
    received_at: ISODate("2025-01-20T10:00:00.456Z")
  },
  timing: {
    dns_ms: 5,
    tcp_ms: 10,
    tls_ms: 20,
    first_byte_ms: 150,
    total_ms: 333
  },
  error: null,
  created_at: ISODate("2025-01-20T10:00:00.456Z")
}
```

**Indexes:**
```javascript
db.execution_logs.createIndex({ response_id: 1 })
db.execution_logs.createIndex({ created_at: -1 })
db.execution_logs.createIndex({ "response.status": 1 })
```

#### response_archives

```javascript
{
  _id: ObjectId("..."),
  archived_at: ISODate("2025-02-20T00:00:00Z"),
  original_response_id: 12345,
  report_id: 100,
  api_id: 50,
  scenario_id: 200,
  // ... full response data
}
```

**Indexes:**
```javascript
db.response_archives.createIndex({ archived_at: -1 })
db.response_archives.createIndex({ original_response_id: 1 })
db.response_archives.createIndex({ report_id: 1 })
```

### Redis Data Structures

#### Task Queue

```
Key: "requestList"
Type: List (RPUSH/BRPOP)

Value (JSON):
{
  "report_id": 123,
  "test_config": {
    "method": "POST",
    "domain": "https://api.example.com",
    "endpoint": "/auth/signin",
    "headers": { "Content-Type": "application/json" },
    "timeout": 30
  },
  "test_data": [
    {
      "response_id": 456,
      "api_id": 50,
      "scenario_id": 200,
      "example_index": 0,
      "params": { "email": "test@example.com", "password": "123456" },
      "expected_status": 200
    }
  ]
}
```

#### Report Status Counter

```
Key: "reportStatus:{report_id}"
Type: Hash

Fields:
{
  "success": "10",
  "fail": "2"
}

Commands:
HINCRBY reportStatus:123 success 1
HINCRBY reportStatus:123 fail 1
HGETALL reportStatus:123
```

#### Progress Channel

```
Channel: "report_channel"
Type: Pub/Sub

Message (JSON):
{
  "report_id": 123,
  "success": 10,
  "fail": 2,
  "timestamp": "2025-01-20T10:00:00Z"
}
```

## Queue Architecture (Dependency Injection) ⭐ **已實作**

### Design Pattern

v2 採用 trait-based 依賴注入，實現可測試的 Queue 系統：

```rust
#[async_trait]
pub trait JobQueue: Send + Sync {
    async fn enqueue(&self, job: TestJob) -> AppResult<Uuid>;
    async fn dequeue(&self, timeout: u64) -> AppResult<Option<TestJob>>;
    async fn get_job(&self, job_id: Uuid) -> AppResult<Option<TestJob>>;
    async fn complete_job(&self, job_id: Uuid, result: JobResult) -> AppResult<()>;
    async fn fail_job(&self, job_id: Uuid, error: String, retryable: bool) -> AppResult<()>;
    async fn list_jobs_by_user(&self, user_id: Uuid, limit: usize) -> AppResult<Vec<TestJob>>;
    async fn cancel_job(&self, job_id: Uuid) -> AppResult<()>;
    async fn requeue(&self, job_id: Uuid) -> AppResult<()>;
    async fn queue_length(&self) -> AppResult<u64>;
}
```

### Implementations

| 實作 | 用途 | 儲存機制 |
|------|------|----------|
| `RedisQueue` | 生產環境 | Redis List + Hash |
| `InMemoryQueue` | 單元測試 | `Arc<Mutex<VecDeque>>` + `Notify` |

### AppState Integration

```rust
pub struct AppState {
    pub db: DatabaseConnection,        // SeaORM
    pub pg_pool: PgPool,               // SQLx (migrations)
    pub mongo_client: MongoClient,
    pub redis: RedisConnectionManager,
    pub job_queue: Arc<dyn JobQueue>,  // DI: 可替換實作
    pub config: Config,
}
```

### Redis Keys (Production)

| Key Pattern | Type | 用途 |
|-------------|------|------|
| `serval:jobs:queue` | List | FIFO 待處理佇列 |
| `serval:jobs:{id}` | String (JSON) | Job 完整資料 |
| `serval:jobs:by_user:{uid}` | Set | 使用者的所有 job IDs |

### Job Status Lifecycle

```
Pending → Running → Completed
                  ↘ Failed → (retry) → Pending
                          ↘ Dead (max retries exceeded)
                  ↘ Cancelled (user cancelled)
```

---

## Code Architecture

### Layer Separation

```
┌─────────────────────────────────────┐
│         Handlers (HTTP)             │  ← HTTP request/response
│  - auth.rs, project.rs, job.rs      │
└─────────────┬───────────────────────┘
              │ calls
              ▼
┌─────────────────────────────────────┐
│       Services (Business Logic)     │  ← Business rules
│  - auth.rs, gherkin.rs              │
│  - test_runner.rs                   │
└─────────────┬───────────────────────┘
              │
     ┌────────┴────────┐
     │ calls           │ enqueue
     ▼                 ▼
┌──────────────┐  ┌──────────────────┐
│ Repositories │  │    JobQueue      │  ← DI: Redis/InMemory
│  (SeaORM)    │  │  (trait object)  │
└──────┬───────┘  └────────┬─────────┘
       │                   │
       │ queries           │ dequeue
       ▼                   ▼
┌──────────────┐  ┌──────────────────┐
│ PostgreSQL   │  │     Worker       │
│ MongoDB      │  │  (executor.rs)   │
│ Redis        │  │  result_handler  │
└──────────────┘  └──────────────────┘
```

### Example: Create Collection Flow

```rust
// 1. Handler (HTTP layer)
pub async fn create_collection(
    State(service): State<Arc<CollectionService>>,
    Json(req): Json<CreateCollectionRequest>,
) -> Result<Json<Collection>, AppError> {
    let collection = service.create(req.project_id, req.name).await?;
    Ok(Json(collection))
}

// 2. Service (Business logic layer)
impl CollectionService {
    pub async fn create(
        &self,
        project_id: i64,
        name: String,
    ) -> Result<Collection> {
        // Business validation
        if name.is_empty() {
            return Err(Error::Validation("Name cannot be empty"));
        }

        // Transaction
        let mut tx = self.pool.begin().await?;

        // Check project exists
        self.project_repo.find_by_id(&mut tx, project_id).await?;

        // Create collection
        let collection = self.collection_repo
            .create(&mut tx, project_id, name)
            .await?;

        tx.commit().await?;
        Ok(collection)
    }
}

// 3. Repository (Data access layer)
impl CollectionRepository {
    pub async fn create(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        project_id: i64,
        name: String,
    ) -> Result<Collection> {
        let collection = sqlx::query_as!(
            Collection,
            r#"
            INSERT INTO collections (project_id, name)
            VALUES ($1, $2)
            RETURNING id, project_id, name, created_at, updated_at
            "#,
            project_id,
            name
        )
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| {
            if e.to_string().contains("collections_project_id_name_key") {
                Error::Conflict("Collection name already exists")
            } else {
                Error::Database(e)
            }
        })?;

        Ok(collection)
    }
}
```

## Authentication Flow

```
┌────────┐                ┌────────┐                ┌────────┐
│ Client │                │  API   │                │  DB    │
└───┬────┘                └───┬────┘                └───┬────┘
    │                         │                         │
    │  POST /auth/signin      │                         │
    ├────────────────────────>│                         │
    │                         │                         │
    │                         │  Query user by email    │
    │                         ├────────────────────────>│
    │                         │                         │
    │                         │  Return user + hash     │
    │                         │<────────────────────────┤
    │                         │                         │
    │                         │ Verify password (argon2)│
    │                         │                         │
    │                         │ Generate JWT            │
    │                         │                         │
    │  { token: "eyJ..." }    │                         │
    │<────────────────────────┤                         │
    │                         │                         │
    │  GET /api/projects      │                         │
    │  Authorization: Bearer  │                         │
    ├────────────────────────>│                         │
    │                         │                         │
    │                         │ Verify JWT              │
    │                         │ Extract user email      │
    │                         │                         │
    │                         │  Query projects         │
    │                         ├────────────────────────>│
    │                         │                         │
    │  { projects: [...] }    │                         │
    │<────────────────────────┤                         │
```

## WebSocket Real-time Updates ⭐ **規劃中 (Phase 4)**

> ⚠️ **注意**: 此功能尚未實作，以下為設計規劃文件。
> 目前測試進度查詢透過 polling `GET /api/jobs/{id}` 實現。

### Architecture Overview

WebSocket 實時進度更新是 ServalRun v2 的**殺手級功能**，展示了完整的分佈式系統設計能力。

```
┌─────────────────────────────────────────────────────────────────┐
│                         完整數據流                                │
└─────────────────────────────────────────────────────────────────┘

Client (Browser)
    │
    │ 1. HTTP POST /api/collections/123/run
    ├──────────────────────────────────────────> API Server
    │                                             │
    │                                             │ 2. Create Report (PostgreSQL)
    │                                             │ 3. Push to Redis Queue
    │                                             │
    │ 4. Return report_id                         │
    │<────────────────────────────────────────────┤
    │                                             │
    │ 5. WebSocket Connect (ws://...api/ws)       │
    ├──────────────────────────────────────────>  │
    │                                             │
    │                                             │ 6. SUBSCRIBE "report-channel"
    │                                             ├─────────────> Redis Pub/Sub
    │                                             │                    ▲
    │                                             │                    │
    │                                    Worker   │                    │
    │                                      │      │                    │
    │                                      │ 7. BRPOP queue            │
    │                                      │      │                    │
    │                                      │ 8. Execute HTTP test      │
    │                                      │      │                    │
    │                                      │ 9. HINCRBY reportStatus   │
    │                                      │      │                    │
    │                                      │ 10. PUBLISH progress      │
    │                                      └──────┴───────────────────-┘
    │                                             │
    │                                             │ 11. Receive from Pub/Sub
    │                                             │ 12. Filter by user
    │                                             │ 13. Find WebSocket connection
    │                                             │
    │ 14. WS Message                              │
    │<────────────────────────────────────────────┤
    │ { report_id: 123, success: 5, fail: 1 }     │
    │                                             │
    │ 15. Update progress bar                     │
    │                                             │
```

### Sequence Diagram

```
┌────────┐    ┌────────┐    ┌─────────┐    ┌────────┐    ┌────────┐
│ Client │    │  API   │    │  Redis  │    │ Worker │    │  PG DB │
└───┬────┘    └───┬────┘    └────┬────┘    └───┬────┘    └───┬────┘
    │             │              │             │             │
    │ POST /run   │              │             │             │
    ├────────────>│              │             │             │
    │             │ INSERT report│             │             │
    │             ├──────────────┼─────────────┼────────────>│
    │             │              │             │             │
    │             │ RPUSH queue  │             │             │
    │             ├─────────────>│             │             │
    │             │              │             │             │
    │ report_id   │              │             │             │
    │<────────────┤              │             │             │
    │             │              │             │             │
    │ WS Connect  │              │             │             │
    ├────────────>│              │             │             │
    │             │ SUBSCRIBE    │             │             │
    │             ├─────────────>│             │             │
    │             │              │             │             │
    │             │              │ BRPOP       │             │
    │             │              │<────────────┤             │
    │             │              │             │             │
    │             │              │       Execute HTTP test   │
    │             │              │             │             │
    │             │              │ HINCRBY     │             │
    │             │              │<────────────┤             │
    │             │              │             │             │
    │             │              │ PUBLISH     │             │
    │             │              │<────────────┤             │
    │             │              │             │             │
    │             │ Message      │             │             │
    │             │<─────────────┤             │             │
    │             │              │             │             │
    │ WS emit     │              │             │             │
    │<────────────┤              │             │             │
    │ progress    │              │             │             │
```

### Key Design Decisions

#### 1. Why Pub/Sub Instead of Direct Communication?

**Alternative 1: Direct WebSocket (不好)**
```
Worker → WebSocket → Client
❌ Worker 需要知道 WebSocket 連接
❌ 耦合度高，難以擴展
❌ Worker 重啟會斷開連接
```

**Alternative 2: Polling (不好)**
```
Client → API Server (every 2s) → Database
❌ 高延遲（最少 2 秒）
❌ 大量無用請求（99% 沒有新進度）
❌ 數據庫壓力大
```

**Chosen: Pub/Sub + WebSocket (最好)** ⭐
```
Worker → Redis Pub/Sub → API Server → WebSocket → Client
✅ 解耦：Worker 和 API Server 獨立
✅ 低延遲：< 100ms
✅ 可擴展：多個 Worker 並行
✅ 高效：只推送變化
```

#### 2. Multi-User Isolation

**Challenge**: 多個用戶同時測試，如何確保只接收自己的進度？

**Solution**:
```rust
// 1. Redis 中每個 report 有獨立的 Hash
reportStatus:123 -> { success: 5, fail: 1 }
reportStatus:456 -> { success: 10, fail: 0 }

// 2. 發布時包含 report_id
PUBLISH "report-channel" '{"report_id": 123, "success": 5, "fail": 1}'

// 3. API Server 檢查 report 所有權
async fn is_user_report(report_id: i64, user_id: i64) -> bool {
    let report = db.find_report(report_id).await?;
    report.user_id == user_id
}

// 4. 只發送給擁有者
if is_user_report(progress.report_id, user.id).await {
    ws_manager.send_to_user(user.id, progress).await;
}
```

#### 3. Concurrent Safety with DashMap

**Challenge**: 多個 WebSocket 連接並發讀寫，如何保證線程安全？

**v1 (Node.js) Solution**:
```javascript
global.usersMap = {};  // ❌ 全局變量，單線程安全
global.usersMap[userId] = socket.id;
```

**v2 (Rust) Solution**:
```rust
use dashmap::DashMap;

// ✅ 並發安全的 HashMap，零鎖開銷
pub struct WebSocketManager {
    connections: Arc<DashMap<i64, Vec<WebSocketSender>>>,
}

// 多線程同時調用也安全
impl WebSocketManager {
    pub async fn add_connection(&self, user_id: i64, sender: WebSocketSender) {
        self.connections.entry(user_id)
            .or_insert_with(Vec::new)
            .push(sender);
    }

    pub async fn broadcast_to_user(&self, user_id: i64, msg: &str) {
        if let Some(mut senders) = self.connections.get_mut(&user_id) {
            senders.retain_mut(|s| s.send(msg).is_ok());  // 移除斷開的連接
        }
    }
}
```

**DashMap 優勢**:
- 分片鎖設計（sharded locking）
- 比 `Mutex<HashMap>` 快 10x
- API 與 HashMap 類似
- Rust 的 ownership 保證線程安全

#### 4. Reconnection Support

**Challenge**: 網絡不穩定，WebSocket 斷開後如何恢復？

**Solution**:
```rust
// 客戶端重連時帶上 report_id
GET /api/ws?report_id=123

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsQuery>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| {
        handle_socket_with_recovery(socket, params.report_id)
    })
}

async fn handle_socket_with_recovery(
    socket: WebSocket,
    report_id: Option<i64>,
) {
    // 如果是重連，先發送當前進度
    if let Some(rid) = report_id {
        let current = redis.hgetall(format!("reportStatus:{}", rid)).await?;
        socket.send(serde_json::to_string(&current)?).await?;
    }

    // 繼續正常的 Pub/Sub 流程
    // ...
}
```

### Implementation Code

#### Worker: Publish Progress

```rust
// worker/executor.rs
pub async fn execute_test_and_publish(
    redis: &RedisClient,
    test: TestCase,
) -> Result<()> {
    // 執行 HTTP 請求
    let result = execute_http(test).await?;

    // 原子更新計數
    let key = format!("reportStatus:{}", test.report_id);
    if result.pass {
        redis.hincrby(&key, "success", 1).await?;
    } else {
        redis.hincrby(&key, "fail", 1).await?;
    }

    // 獲取當前計數
    let counts: HashMap<String, i32> = redis.hgetall(&key).await?;

    // 發布進度
    let progress = ProgressUpdate {
        report_id: test.report_id,
        success: counts["success"],
        fail: counts["fail"],
        total: test.total,
        timestamp: Utc::now().to_rfc3339(),
    };

    redis.publish("report-channel", serde_json::to_string(&progress)?).await?;

    tracing::info!(
        report_id = %test.report_id,
        progress = format!("{}/{}", counts["success"] + counts["fail"], test.total),
        "Progress published"
    );

    Ok(())
}
```

#### API Server: WebSocket Handler

```rust
// handlers/websocket.rs
use axum::extract::ws::{WebSocket, WebSocketUpgrade, Message};
use futures::{StreamExt, SinkExt};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    // 訂閱 Redis Pub/Sub
    let mut pubsub = state.redis.clone().into_pubsub();
    if let Err(e) = pubsub.subscribe("report-channel").await {
        tracing::error!("Failed to subscribe: {}", e);
        return;
    }

    tracing::info!("WebSocket connected");

    // 並發處理兩個任務
    tokio::select! {
        // Task 1: 從 Redis 接收消息並轉發到 WebSocket
        _ = async {
            let mut stream = pubsub.on_message();
            while let Some(msg) = stream.next().await {
                match serde_json::from_str::<ProgressUpdate>(msg.get_payload()) {
                    Ok(progress) => {
                        let json = serde_json::to_string(&progress).unwrap();
                        if sender.send(Message::Text(json)).await.is_err() {
                            tracing::info!("Client disconnected");
                            break;
                        }

                        tracing::debug!(
                            report_id = %progress.report_id,
                            "Forwarded progress to client"
                        );
                    }
                    Err(e) => {
                        tracing::error!("Failed to parse progress: {}", e);
                    }
                }
            }
        } => {},

        // Task 2: 處理客戶端消息（心跳）
        _ = async {
            while let Some(msg) = receiver.next().await {
                match msg {
                    Ok(Message::Ping(data)) => {
                        if sender.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Ok(Message::Close(_)) => {
                        tracing::info!("Client closed connection");
                        break;
                    }
                    Err(e) => {
                        tracing::error!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        } => {},
    }

    tracing::info!("WebSocket handler finished");
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProgressUpdate {
    pub report_id: i64,
    pub success: i32,
    pub fail: i32,
    pub total: i32,
    pub timestamp: String,
}
```

### Testing

#### Simple HTML Test Page

```html
<!DOCTYPE html>
<html>
<head>
    <title>WebSocket Test</title>
    <style>
        body { font-family: sans-serif; max-width: 600px; margin: 50px auto; }
        #status { padding: 10px; margin: 10px 0; border-radius: 4px; }
        .connected { background: #d4edda; color: #155724; }
        .disconnected { background: #f8d7da; color: #721c24; }
        progress { width: 100%; height: 30px; }
    </style>
</head>
<body>
    <h1>WebSocket Real-time Progress</h1>

    <div id="status" class="disconnected">Disconnected</div>

    <div>
        <strong>Report ID:</strong> <span id="report-id">-</span><br>
        <strong>Success:</strong> <span id="success">0</span><br>
        <strong>Failed:</strong> <span id="fail">0</span><br>
        <strong>Progress:</strong> <span id="current">0</span>/<span id="total">0</span>
    </div>

    <progress id="progress-bar" value="0" max="100"></progress>

    <div id="log" style="margin-top: 20px; padding: 10px; background: #f5f5f5; max-height: 300px; overflow-y: auto;"></div>

    <script>
        const ws = new WebSocket('ws://localhost:3000/api/ws');
        const status = document.getElementById('status');
        const log = document.getElementById('log');

        function addLog(msg) {
            const time = new Date().toLocaleTimeString();
            log.innerHTML += `[${time}] ${msg}<br>`;
            log.scrollTop = log.scrollHeight;
        }

        ws.onopen = () => {
            status.textContent = 'Connected';
            status.className = 'connected';
            addLog('✅ WebSocket connected');
        };

        ws.onmessage = (event) => {
            const progress = JSON.parse(event.data);

            // 更新 UI
            document.getElementById('report-id').textContent = progress.report_id;
            document.getElementById('success').textContent = progress.success;
            document.getElementById('fail').textContent = progress.fail;

            const current = progress.success + progress.fail;
            document.getElementById('current').textContent = current;
            document.getElementById('total').textContent = progress.total;

            // 更新進度條
            const percent = (current / progress.total) * 100;
            document.getElementById('progress-bar').value = percent;

            addLog(`📊 Progress: ${current}/${progress.total} (${percent.toFixed(1)}%)`);
        };

        ws.onerror = (error) => {
            addLog(`❌ Error: ${error}`);
        };

        ws.onclose = () => {
            status.textContent = 'Disconnected';
            status.className = 'disconnected';
            addLog('🔌 WebSocket disconnected');
        };
    </script>
</body>
</html>
```

### Performance Characteristics

| Metric          | Value         | Notes                                 |
| --------------- | ------------- | ------------------------------------- |
| **Latency**     | < 100ms       | From Worker publish to Client receive |
| **Throughput**  | 10,000+ msg/s | Redis Pub/Sub performance             |
| **Connections** | 10,000+       | Per API server instance               |
| **Memory**      | ~10KB         | Per WebSocket connection              |
| **CPU**         | Minimal       | Zero-copy message forwarding          |

### Comparison: v1 (Node.js) vs v2 (Rust)

| Aspect               | v1 (Node.js)       | v2 (Rust)                   |
| -------------------- | ------------------ | --------------------------- |
| **Library**          | Socket.IO          | Native WebSocket (Axum)     |
| **Protocol**         | Socket.IO protocol | Standard WebSocket          |
| **Bundle Size**      | ~200KB (client)    | ~10KB (client)              |
| **Auth**             | Session cookie     | JWT token                   |
| **Concurrency**      | Single-threaded    | Multi-threaded (tokio)      |
| **Memory**           | ~50MB baseline     | ~5MB baseline               |
| **Type Safety**      | Runtime            | Compile-time                |
| **Connection State** | `global.usersMap`  | `DashMap` (concurrent-safe) |

## Performance Optimizations

### 1. Connection Pooling

```rust
// PostgreSQL connection pool
let pool = PgPoolOptions::new()
    .max_connections(20)
    .min_connections(5)
    .acquire_timeout(Duration::from_secs(30))
    .connect(&database_url)
    .await?;

// MongoDB connection pool (built-in)
let client = Client::with_uri_str(&mongodb_url).await?;

// Redis connection pool
let redis = RedisClient::open(redis_url)?
    .get_multiplexed_async_connection_manager()
    .await?;
```

### 2. Batch Operations

```rust
// Bad: N queries
for response_id in response_ids {
    update_response(response_id).await?;
}

// Good: 1 batch query
sqlx::query!(
    "UPDATE responses SET pass = TRUE WHERE id = ANY($1)",
    &response_ids
)
.execute(&pool)
.await?;
```

### 3. Partial Indexes

```sql
-- Only index unfinished reports (most queries)
CREATE INDEX idx_reports_finished
ON reports(finished)
WHERE finished = FALSE;
```

## Testing Architecture

### Test Strategy Overview

ServalRun v2 採用**測試金字塔**策略，確保代碼質量和性能驗證。

```
        ┌─────────────────┐
        │   E2E Tests     │  10% - 完整業務流程
        │   (axum-test)   │
        ├─────────────────┤
        │ Integration     │  30% - API 端點測試
        │ (axum-test)     │
        ├─────────────────┤
        │  Unit Tests     │  60% - 函數級測試
        │ (rstest+sqlx)   │
        └─────────────────┘
```

**目標**:
- 代碼覆蓋率: **80%+**
- 所有 API 端點有集成測試
- 關鍵業務流程有 E2E 測試
- 性能對比數據完整（v1 vs v2）

---

### 1. Unit Tests (單元測試)

#### 1.1 測試範圍

| 層級               | 測試內容                  | 工具            | 範例                     |
| ------------------ | ------------------------- | --------------- | ------------------------ |
| **Models**         | 序列化/反序列化、驗證邏輯 | `#[test]`       | `test_user_validation()` |
| **Repositories**   | 數據庫 CRUD               | `#[sqlx::test]` | `test_create_user()`     |
| **Services**       | 業務邏輯                  | `mockall`       | `test_auth_service()`    |
| **Handlers**       | HTTP 處理                 | `axum-test`     | `test_signup_handler()`  |
| **Middleware**     | JWT、錯誤處理             | `#[test]`       | `test_jwt_validation()`  |
| **Gherkin Parser** | 語法解析                  | `rstest`        | `test_parse_scenario()`  |

#### 1.2 測試工具配置

```toml
[dev-dependencies]
# 參數化測試（類似 pytest.mark.parametrize）
rstest = "0.18"

# Mock 測試（類似 unittest.mock）
mockall = "0.12"

# 斷言增強
assert_matches = "1.5"

# 資料庫測試（自動 rollback）
sqlx = { version = "0.7", features = ["runtime-tokio", "postgres"] }

# 測試覆蓋率
# cargo install cargo-tarpaulin
```

#### 1.3 Repository 測試範例

```rust
// src/repositories/user_repo.rs
#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    // sqlx::test 會自動：
    // 1. 創建測試數據庫
    // 2. 運行 migrations
    // 3. 每個測試使用獨立事務
    // 4. 測試結束自動 rollback
    #[sqlx::test]
    async fn test_create_user(pool: PgPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool);

        let user = repo.create("test@example.com", "hashed_password").await?;

        assert_eq!(user.email, "test@example.com");
        assert!(user.id > 0);
        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_email(pool: PgPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool);

        // Arrange
        repo.create("test@example.com", "hashed_password").await?;

        // Act
        let found = repo.find_by_email("test@example.com").await?;

        // Assert
        assert!(found.is_some());
        assert_eq!(found.unwrap().email, "test@example.com");
        Ok(())
    }

    #[sqlx::test]
    async fn test_duplicate_email(pool: PgPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool);

        repo.create("test@example.com", "hashed_password").await?;

        // 應該返回錯誤（違反 UNIQUE 約束）
        let result = repo.create("test@example.com", "hashed_password").await;
        assert!(result.is_err());
        Ok(())
    }
}
```

#### 1.4 Service 測試範例（使用 Mock）

```rust
// src/services/auth_service.rs
use mockall::automock;

#[automock]
pub trait UserRepository {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>>;
}

pub struct AuthService<R: UserRepository> {
    user_repo: R,
}

impl<R: UserRepository> AuthService<R> {
    pub async fn authenticate(&self, email: &str, password: &str) -> Result<String> {
        let user = self.user_repo.find_by_email(email).await?
            .ok_or(AuthError::UserNotFound)?;

        // 驗證密碼...
        // 生成 JWT...
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_authenticate_success() {
        // Arrange
        let mut mock_repo = MockUserRepository::new();
        mock_repo
            .expect_find_by_email()
            .with(eq("test@example.com"))
            .times(1)
            .returning(|_| Ok(Some(User {
                id: 1,
                email: "test@example.com".into(),
                password_hash: "$2b$12$...".into(),
            })));

        let service = AuthService::new(mock_repo);

        // Act
        let result = service.authenticate("test@example.com", "password123").await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_authenticate_user_not_found() {
        let mut mock_repo = MockUserRepository::new();
        mock_repo
            .expect_find_by_email()
            .returning(|_| Ok(None));

        let service = AuthService::new(mock_repo);

        let result = service.authenticate("nonexistent@example.com", "password123").await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), AuthError::UserNotFound);
    }
}
```

#### 1.5 參數化測試範例

```rust
use rstest::rstest;

#[rstest]
#[case("valid@example.com", true)]
#[case("invalid", false)]
#[case("missing-at.com", false)]
#[case("@example.com", false)]
#[case("user@", false)]
fn test_email_validation(#[case] email: &str, #[case] expected: bool) {
    assert_eq!(is_valid_email(email), expected);
}

#[rstest]
#[case("password123", 11, true)]
#[case("short", 6, true)]
#[case("", 0, false)]
#[case("a", 1, false)]
fn test_password_length(
    #[case] password: &str,
    #[case] expected_len: usize,
    #[case] is_valid: bool,
) {
    assert_eq!(password.len(), expected_len);
    assert_eq!(is_valid_password(password), is_valid);
}
```

---

### 2. Integration Tests (集成測試)

#### 2.1 測試文件結構

```
serval-run-v2/
├── src/
└── tests/
    ├── common/
    │   ├── mod.rs           # 共用測試工具
    │   ├── fixtures.rs      # 測試數據
    │   └── helpers.rs       # 測試輔助函數
    ├── api_auth_test.rs     # 認證 API
    ├── api_projects_test.rs # 專案 API
    ├── api_collections_test.rs
    ├── api_reports_test.rs
    ├── websocket_test.rs
    └── e2e_test.rs
```

#### 2.2 測試工具配置

```toml
[dev-dependencies]
axum-test = "14"              # HTTP 集成測試
tokio-tungstenite = "0.21"    # WebSocket 測試
tower = { version = "0.4", features = ["util"] }
```

#### 2.3 Auth API 測試範例

```rust
// tests/api_auth_test.rs
use axum_test::TestServer;
use serde_json::json;

mod common;
use common::create_test_app;

#[tokio::test]
async fn test_signup_success() {
    // Arrange
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // Act
    let response = server
        .post("/api/auth/signup")
        .json(&json!({
            "email": "test@example.com",
            "password": "password123"
        }))
        .await;

    // Assert
    response.assert_status_ok();
    response.assert_json(&json!({
        "email": "test@example.com"
    }));
}

#[tokio::test]
async fn test_signup_duplicate_email() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // 第一次註冊
    server.post("/api/auth/signup")
        .json(&json!({
            "email": "test@example.com",
            "password": "password123"
        }))
        .await
        .assert_status_ok();

    // 第二次註冊（應該失敗）
    let response = server.post("/api/auth/signup")
        .json(&json!({
            "email": "test@example.com",
            "password": "password123"
        }))
        .await;

    response.assert_status(StatusCode::CONFLICT);
    response.assert_json(&json!({
        "error": "Email already exists"
    }));
}

#[tokio::test]
async fn test_signin_success() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // 先註冊
    server.post("/api/auth/signup")
        .json(&json!({
            "email": "test@example.com",
            "password": "password123"
        }))
        .await;

    // 登入
    let response = server.post("/api/auth/signin")
        .json(&json!({
            "email": "test@example.com",
            "password": "password123"
        }))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert!(body["token"].is_string());
}

#[tokio::test]
async fn test_protected_endpoint_without_token() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let response = server.get("/api/projects").await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_protected_endpoint_with_token() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // 註冊並登入獲取 token
    let token = common::get_test_token(&server).await;

    // 使用 token 訪問受保護端點
    let response = server
        .get("/api/projects")
        .add_header("Authorization", format!("Bearer {}", token))
        .await;

    response.assert_status_ok();
}
```

#### 2.4 共用測試工具

```rust
// tests/common/mod.rs
use axum::Router;
use sqlx::PgPool;
use serde_json::json;

pub async fn create_test_app() -> Router {
    // 創建測試數據庫連接
    let database_url = std::env::var("DATABASE_URL_TEST")
        .unwrap_or_else(|_| "postgres://localhost/serval_run_test".to_string());

    let pool = PgPool::connect(&database_url).await.unwrap();

    // 運行 migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .unwrap();

    // 創建應用
    create_app(pool, redis_client, mongo_client).await
}

pub async fn get_test_token(server: &TestServer) -> String {
    // 註冊
    server.post("/api/auth/signup")
        .json(&json!({
            "email": "test@example.com",
            "password": "password123"
        }))
        .await;

    // 登入
    let response = server.post("/api/auth/signin")
        .json(&json!({
            "email": "test@example.com",
            "password": "password123"
        }))
        .await;

    response.json::<serde_json::Value>()["token"]
        .as_str()
        .unwrap()
        .to_string()
}
```

---

### 3. E2E Tests (端到端測試)

#### 3.1 完整工作流測試

```rust
// tests/e2e_test.rs
#[tokio::test]
async fn test_complete_workflow() {
    let server = create_test_server().await;

    // 1. 註冊用戶
    let signup_res = server.post("/api/auth/signup")
        .json(&json!({
            "email": "test@example.com",
            "password": "password123"
        }))
        .await;
    signup_res.assert_status_ok();

    // 2. 登入獲取 token
    let signin_res = server.post("/api/auth/signin")
        .json(&json!({
            "email": "test@example.com",
            "password": "password123"
        }))
        .await;
    let token = signin_res.json::<AuthResponse>().token;

    // 3. 創建專案
    let project_res = server.post("/api/projects")
        .bearer_token(&token)
        .json(&json!({"name": "Test Project"}))
        .await;
    let project_id = project_res.json::<Project>().id;

    // 4. 創建 Collection
    let collection_res = server
        .post(&format!("/api/projects/{}/collections", project_id))
        .bearer_token(&token)
        .json(&json!({"name": "Test Collection"}))
        .await;
    let collection_id = collection_res.json::<Collection>().id;

    // 5. 添加 API
    let api_res = server
        .post(&format!("/api/collections/{}/apis", collection_id))
        .bearer_token(&token)
        .json(&json!({
            "name": "Test API",
            "method": "GET",
            "url": "https://httpbin.org/get"
        }))
        .await;
    let api_id = api_res.json::<Api>().id;

    // 6. 添加 Scenario
    server
        .post(&format!("/api/apis/{}/scenarios", api_id))
        .bearer_token(&token)
        .json(&json!({
            "name": "Test Scenario",
            "gherkin": "Feature: Test\n  Scenario: Test\n    When I send GET request\n    Then status code should be 200"
        }))
        .await;

    // 7. 運行測試
    let run_res = server
        .post(&format!("/api/collections/{}/run", collection_id))
        .bearer_token(&token)
        .await;
    run_res.assert_status_ok();
    let report_id = run_res.json::<RunResponse>().report_id;

    // 8. 等待測試完成
    tokio::time::sleep(Duration::from_secs(3)).await;

    // 9. 檢查報告
    let report_res = server
        .get(&format!("/api/reports/{}", report_id))
        .bearer_token(&token)
        .await;
    report_res.assert_status_ok();

    let report = report_res.json::<Report>();
    assert!(report.finished);
    assert_eq!(report.success_count, 1);
    assert_eq!(report.fail_count, 0);
}
```

#### 3.2 WebSocket 測試

```rust
// tests/websocket_test.rs
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures::{StreamExt, SinkExt};

#[tokio::test]
async fn test_websocket_progress_updates() {
    let server = create_test_server().await;
    let token = get_test_token(&server).await;

    // 連接 WebSocket
    let ws_url = format!("ws://localhost:3000/api/ws?token={}", token);
    let (ws_stream, _) = connect_async(ws_url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    // 觸發測試
    let report_id = trigger_test(&server, &token).await;

    // 接收進度更新
    let mut received_updates = Vec::new();
    while let Some(Ok(msg)) = read.next().await {
        if let Message::Text(text) = msg {
            let progress: ProgressUpdate = serde_json::from_str(&text).unwrap();
            received_updates.push(progress);

            if progress.success + progress.fail >= progress.total {
                break;
            }
        }
    }

    // 驗證
    assert!(!received_updates.is_empty());
    assert_eq!(received_updates.last().unwrap().report_id, report_id);
}
```

---

### 4. Performance Testing (性能測試)

#### 4.1 Benchmark 測試（Criterion）

```rust
// benches/gherkin_parser.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_parse_simple_gherkin(c: &mut Criterion) {
    let gherkin = r#"
Feature: User Authentication
  Scenario: User logs in
    Given I am on the login page
    When I enter valid credentials
    Then I should see the dashboard
"#;

    c.bench_function("parse simple gherkin", |b| {
        b.iter(|| parse_gherkin(black_box(gherkin)))
    });
}

fn bench_parse_complex_gherkin(c: &mut Criterion) {
    let gherkin = include_str!("../fixtures/complex.feature");

    c.bench_function("parse complex gherkin with examples", |b| {
        b.iter(|| parse_gherkin(black_box(gherkin)))
    });
}

criterion_group!(benches, bench_parse_simple_gherkin, bench_parse_complex_gherkin);
criterion_main!(benches);
```

**運行**:
```bash
cargo bench
# 報告生成在 target/criterion/report/index.html
```

#### 4.2 負載測試（k6）

```javascript
// load_tests/api_throughput.js
import http from 'k6/http';
import { check, sleep } from 'k6';

export const options = {
  stages: [
    { duration: '30s', target: 100 },   // Ramp up to 100 users
    { duration: '1m', target: 500 },    // Stay at 500 users
    { duration: '30s', target: 1000 },  // Ramp up to 1000 users
    { duration: '1m', target: 1000 },   // Stay at 1000 users
    { duration: '30s', target: 0 },     // Ramp down
  ],
  thresholds: {
    http_req_duration: ['p(95)<500'],   // 95% < 500ms
    http_req_failed: ['rate<0.01'],     // Error rate < 1%
  },
};

const BASE_URL = 'http://localhost:3000';

export function setup() {
  // 註冊測試用戶並獲取 token
  const signup = http.post(`${BASE_URL}/api/auth/signup`, JSON.stringify({
    email: 'loadtest@example.com',
    password: 'password123',
  }), {
    headers: { 'Content-Type': 'application/json' },
  });

  const signin = http.post(`${BASE_URL}/api/auth/signin`, JSON.stringify({
    email: 'loadtest@example.com',
    password: 'password123',
  }), {
    headers: { 'Content-Type': 'application/json' },
  });

  return { token: signin.json('token') };
}

export default function (data) {
  const params = {
    headers: {
      'Authorization': `Bearer ${data.token}`,
      'Content-Type': 'application/json',
    },
  };

  // 測試獲取專案列表
  const res = http.get(`${BASE_URL}/api/projects`, params);

  check(res, {
    'status is 200': (r) => r.status === 200,
    'response time < 200ms': (r) => r.timings.duration < 200,
  });

  sleep(1);
}
```

**運行**:
```bash
k6 run load_tests/api_throughput.js
```

#### 4.3 v1 vs v2 性能對比

**對比腳本**:
```bash
#!/bin/bash
# scripts/compare_performance.sh

echo "🔥 Starting Performance Comparison: v1 (Node.js) vs v2 (Rust)"

# Test v1
echo "📊 Testing v1 (Node.js)..."
cd ../serval-run
docker-compose up -d
sleep 5
k6 run ../load_tests/compare.js --out json=../results/v1_results.json
docker-compose down

# Test v2
echo "📊 Testing v2 (Rust)..."
cd ../serval-run-v2
docker-compose up -d
sleep 5
k6 run ../load_tests/compare.js --out json=../results/v2_results.json
docker-compose down

# Generate report
echo "📈 Generating comparison report..."
python3 scripts/generate_comparison.py \
  ../results/v1_results.json \
  ../results/v2_results.json \
  --output ../results/comparison_report.md
```

**預期結果**:

| 指標          | v1 (Node.js) | v2 (Rust)   | 提升     |
| ------------- | ------------ | ----------- | -------- |
| P50 latency   | 150ms        | 50ms        | **3x**   |
| P95 latency   | 350ms        | 120ms       | **2.9x** |
| Throughput    | 1,000 req/s  | 5,000 req/s | **5x**   |
| Memory (idle) | 200MB        | 50MB        | **4x**   |
| Memory (load) | 400MB        | 80MB        | **5x**   |

---

### 5. 測試覆蓋率

**工具**: `cargo-tarpaulin`

```bash
# 安裝
cargo install cargo-tarpaulin

# 運行測試並生成覆蓋率報告
cargo tarpaulin --out Html --output-dir coverage/

# 打開報告
open coverage/index.html
```

**目標覆蓋率**:
- **Overall**: 80%+
- **Repositories**: 90%+
- **Services**: 85%+
- **Handlers**: 80%+

---

### 6. CI/CD 集成

```yaml
# .github/workflows/test.yml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:16
        env:
          POSTGRES_PASSWORD: postgres
        options: >-
          --health-cmd pg_isready
          --health-interval 10s

    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run tests
        run: cargo test --all-features

      - name: Check code coverage
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --out Xml

      - name: Upload coverage
        uses: codecov/codecov-action@v3

      - name: Run clippy
        run: cargo clippy -- -D warnings

      - name: Check formatting
        run: cargo fmt -- --check
```

---

## Summary

ServalRun v2 的測試架構展示了：

1. **完整的測試金字塔**: Unit → Integration → E2E
2. **高代碼覆蓋率**: 80%+ 目標
3. **性能驗證**: Criterion + k6 + v1 vs v2 對比
4. **持續集成**: GitHub Actions 自動化測試
5. **工程質量**: Clippy + rustfmt + cargo audit

這些測試不僅保證了代碼質量，更是**面試時的重點展示**，證明對軟體工程最佳實踐的深刻理解。

### 4. Async Parallelism

```rust
// Execute multiple HTTP requests in parallel
let futures: Vec<_> = test_examples
    .iter()
    .map(|example| execute_http_request(example))
    .collect();

let results = futures::future::join_all(futures).await;
```

## Error Handling Strategy

```rust
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("MongoDB error: {0}")]
    Mongo(#[from] mongodb::error::Error),

    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

// HTTP status code mapping
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),
            AppError::Validation(msg) => (StatusCode::BAD_REQUEST, msg),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string()),
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}
```

## Security Considerations

### 1. Password Hashing

```rust
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};

// Hash password (on signup)
let salt = SaltString::generate(&mut OsRng);
let argon2 = Argon2::default();
let password_hash = argon2
    .hash_password(password.as_bytes(), &salt)?
    .to_string();

// Verify password (on signin)
let parsed_hash = PasswordHash::new(&stored_hash)?;
argon2.verify_password(password.as_bytes(), &parsed_hash)?;
```

### 2. JWT Security

```rust
// Generate JWT with expiration
let expiration = Utc::now()
    .checked_add_signed(Duration::hours(24))
    .unwrap()
    .timestamp() as usize;

let claims = Claims {
    sub: user.email,
    exp: expiration,
    iat: Utc::now().timestamp() as usize,
};

// Use strong secret (from environment)
let token = encode(
    &Header::default(),
    &claims,
    &EncodingKey::from_secret(JWT_SECRET.as_ref())
)?;
```

### 3. SQL Injection Prevention

```rust
// SQLx prevents SQL injection via parameterized queries
sqlx::query_as!(
    User,
    "SELECT * FROM users WHERE email = $1",  // ← Safe: parameterized
    email  // ← Automatically escaped
)
```

### 4. Rate Limiting

```rust
// TODO: Implement with tower-governor
use tower_governor::{governor::GovernorConfig, GovernorLayer};

let governor_conf = Box::new(
    GovernorConfig::default()
        .per_second(10)  // 10 requests per second
        .burst_size(20)
);

let app = Router::new()
    .route("/api/auth/signin", post(signin))
    .layer(GovernorLayer { config: governor_conf });
```

## Monitoring and Observability

### Structured Logging

```rust
use tracing::{info, warn, error, instrument};

#[instrument(skip(pool))]
async fn create_collection(pool: &PgPool, name: String) -> Result<Collection> {
    info!(name = %name, "Creating collection");

    let collection = sqlx::query_as!(/* ... */)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            error!(error = ?e, "Failed to create collection");
            e
        })?;

    info!(collection_id = %collection.id, "Collection created successfully");
    Ok(collection)
}
```

### Metrics (Future)

```rust
// TODO: Integrate Prometheus metrics
// - Request count per endpoint
// - Response time percentiles (p50, p90, p99)
// - Database query duration
// - Worker queue length
// - Active WebSocket connections
```

## Deployment Architecture

### Development

```yaml
# docker-compose.yml
services:
  postgres:
    image: postgres:16-alpine
    ports: ["5432:5432"]

  mongodb:
    image: mongo:7
    ports: ["27017:27017"]

  redis:
    image: redis:7-alpine
    ports: ["6379:6379"]

  api:
    build: .
    ports: ["3000:3000"]
    depends_on: [postgres, mongodb, redis]

  worker:
    build: .
    command: cargo run --bin worker
    depends_on: [postgres, mongodb, redis]
```

### Production (Single VPS)

```
┌─────────────────────────────────────────┐
│              VPS (Ubuntu 24.04)         │
│                                         │
│  ┌────────────┐  ┌────────────┐         │
│  │ API Server │  │  Worker    │         │
│  │  (Docker)  │  │  (Docker)  │         │
│  └─────┬──────┘  └─────┬──────┘         │
│        │               │                │
│  ┌─────┴───────────────┴──────-┐        │
│  │     PostgreSQL (Docker)     │        │
│  └─────────────────────────────┘        │
│                                         │
│  ┌─────────────┐  ┌─────────────┐       │
│  │  MongoDB    │  │   Redis     │       │
│  │  (Docker)   │  │  (Docker)   │       │
│  └─────────────┘  └─────────────┘       │
│                                         │
│  ┌─────────────────────────────┐        │
│  │    Nginx (Reverse Proxy)    │        │
│  │    SSL/TLS (Let's Encrypt)  │        │
│  └─────────────────────────────┘        │
└─────────────────────────────────────────┘
```

## Future Improvements

### Phase 6+
- [ ] Metrics and monitoring (Prometheus + Grafana)
- [ ] Distributed tracing (OpenTelemetry)
- [ ] API rate limiting
- [ ] Request caching (Redis)
- [ ] Database replication (read replicas)
- [ ] Horizontal scaling (multiple API servers)
- [ ] Kubernetes deployment (if needed)

---

*Last Updated: 2025-01-19*
