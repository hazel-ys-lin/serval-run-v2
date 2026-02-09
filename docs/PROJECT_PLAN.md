# ServalRun v2 - Project Plan

## 專案概述

ServalRun v2 是對四年前的 Node.js 專案進行 Rust 重寫，作為學習 Rust 和展示後端開發能力的 side project。

### 原始專案 (v1)
- **技術棧**: Node.js + Express + MongoDB + Redis + Socket.IO
- **代碼量**: ~4,200 行
- **年份**: 2022

### 新版本 (v2)
- **技術棧**: Rust + Axum + PostgreSQL + MongoDB + Redis
- **目標**:
  - 學習 Rust 系統編程
  - 展示多數據庫架構能力
  - 提升性能 (目標: 2-3x)
  - 作為求職 Rust 職位的 portfolio

---

## 核心功能

ServalRun 是一個自動化 API 集成測試工具，支持：

1. **多層級測試**
   - Collection 層級（批量 API 測試）
   - API 層級（單一 API 的所有場景）
   - Scenario 層級（單一測試場景）

2. **Gherkin 支持**
   - 使用 Gherkin 語法編寫測試場景
   - 可讀性高的 BDD (Behavior-Driven Development) 測試

3. **實時進度更新**
   - WebSocket 推送測試進度
   - 實時圖表展示

4. **測試報告**
   - 詳細的執行結果
   - 統計數據（通過率、響應時間）

---

## 架構決策

### 1. 數據庫架構 ⭐

**混合數據庫策略** - 各司其職

#### PostgreSQL (主數據庫)
**用途**: 結構化數據、事務、複雜查詢

**存儲內容**:
```
- users                 # 用戶帳號
- projects              # 項目
- environments          # 測試環境配置
- collections           # API 集合
- apis                  # API 定義
- scenarios             # 測試場景元數據
- reports               # 測試報告統計
- responses             # 測試結果
```

**選擇理由**:
- 數據關聯性強（User → Project → Collection → API → Scenario）
- 需要 ACID 事務保證數據一致性
- 複雜統計查詢（JOIN、GROUP BY、聚合）
- 展示 SQL 能力（業界主流，面試加分）

#### MongoDB (輔助數據庫)
**用途**: 文檔存儲、靈活 schema、日誌

**存儲內容**:
```
- gherkin_documents     # Gherkin 原始文本和解析結果
- execution_logs        # HTTP 請求/響應詳細日誌
- response_archives     # 歷史測試結果歸檔 (>30天)
- audit_logs            # 操作審計日誌
```

**選擇理由**:
- Gherkin 是半結構化文檔，版本可能變化
- 執行日誌需要靈活 schema（不同 API 返回不同結構）
- 大量寫入場景（日誌）
- 歷史數據歸檔（冷數據）

#### Redis
**用途**: 任務隊列、Pub/Sub

**使用場景**:
```
- requestList queue     # 測試任務隊列（生產者-消費者）
- report_channel        # 測試進度廣播（Pub/Sub）
- reportStatus:{id}     # 報告計數器（Hash）
```

**選擇理由**:
- 高性能任務隊列
- 實時進度推送
- Worker 解耦

---

### 2. 數據庫訪問層 ⭐

**決策**: **SQLx** (Compile-time SQL)

**對比分析**:

| 方案         | 優點                                  | 缺點           | 評分  |
| ------------ | ------------------------------------- | -------------- | ----- |
| Diesel (ORM) | 最強類型安全                          | 不支持 async ❌ | ⭐⭐    |
| SeaORM (ORM) | 支持 async，類似 SQLAlchemy           | API 尚在變動中 | ⭐⭐⭐   |
| **SQLx**     | 編譯時 SQL 驗證、原生 async、完全控制 | 需要手寫 SQL   | ⭐⭐⭐⭐⭐ |
| Raw SQL      | 完全控制                              | 無類型檢查     | ⭐⭐    |

**選擇 SQLx 的理由**:
1. ✅ **編譯時 SQL 驗證** - 拼錯欄位名會編譯失敗
2. ✅ **原生 async 支持** - 與 tokio/axum 完美配合
3. ✅ **性能最優** - 零抽象成本
4. ✅ **展示 SQL 能力** - 可以寫複雜 JOIN、CTE、Window Functions
5. ✅ **社群活躍** - tokio 官方推薦
6. ✅ **完全控制** - 想怎麼優化就怎麼優化

**代碼示例**:
```rust
// 編譯時檢查 SQL！
let user = sqlx::query_as!(
    User,
    "SELECT id, email FROM users WHERE email = $1",
    email
)
.fetch_one(&pool)
.await?;

// 如果寫錯欄位名，編譯時就會報錯
// "SELECT id, emial FROM users"  ← 編譯錯誤
```

---

### 3. 認證方案

**決策**: **JWT (JSON Web Token)**

**對比 v1 (express-session)**:

| 特性       | v1 (Session)           | v2 (JWT)     |
| ---------- | ---------------------- | ------------ |
| 狀態       | Stateful（需存 Redis） | Stateless    |
| 前後端分離 | 需要 Cookie            | 任何前端框架 |
| 水平擴展   | 需要共享 Session       | 天然支持     |
| 移動端     | 不友好                 | 友好         |

**選擇理由**:
- 前後端完全分離（未來可以用 React 重寫前端）
- 不需要額外的 Session 存儲
- 可擴展性更好
- 業界標準

**技術細節**:
```rust
// 依賴
jsonwebtoken = "9"
argon2 = "0.5"  // 密碼加密（比 bcrypt 更現代）

// JWT Claims
{
  sub: user_email,
  exp: expiration_timestamp,
  iat: issued_at_timestamp
}

// HTTP Header
Authorization: Bearer <token>
```

---

### 4. 錯誤處理

**決策**: **thiserror** (定義錯誤類型)

**架構**:
```rust
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("MongoDB error: {0}")]
    Mongo(#[from] mongodb::error::Error),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Unauthorized")]
    Unauthorized,
}

// 自動轉換為 HTTP Response
impl IntoResponse for AppError { /* ... */ }
```

**優點**:
- 類型安全的錯誤處理
- 自動轉換為 HTTP 狀態碼
- 清晰的錯誤來源追蹤

---

### 5. 日誌和監控

**決策**: **tracing + tracing-subscriber**

**理由**:
- Rust 標準（tokio 官方推薦）
- 結構化日誌
- 支持分佈式追蹤
- 性能極好

**功能**:
```rust
// 自動記錄函數參數和執行時間
#[tracing::instrument(skip(pool))]
async fn create_collection(pool: &PgPool, name: String) -> Result<Collection> {
    tracing::info!("Creating collection: {}", name);
    // ...
}

// 輸出:
// INFO create_collection{name="User APIs"}: Creating collection: User APIs
// INFO create_collection{name="User APIs"}: Collection created collection_id=42
```

---

### 6. 測試策略

**多層次測試**:

```rust
// 1. 單元測試 (Repository 層)
#[sqlx::test]  // 自動創建測試數據庫
async fn test_create_collection(pool: PgPool) {
    let repo = CollectionRepository::new(pool);
    let result = repo.create(1, "test".to_string()).await;
    assert!(result.is_ok());
}

// 2. 集成測試 (API 層)
#[tokio::test]
async fn test_create_collection_endpoint() {
    let app = create_test_app().await;
    let response = app.oneshot(/* ... */).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
}

// 3. 性能基準測試
#[bench]
fn bench_query_performance(b: &mut Bencher) {
    // 對比 v1 (Node.js) vs v2 (Rust)
}
```

---

### 7. 部署方案

**開發環境**: Docker Compose

```yaml
services:
  postgres:   # PostgreSQL 16
  mongodb:    # MongoDB 7
  redis:      # Redis 7
  api:        # Rust API Server
  worker:     # Rust Worker
```

**生產環境**:
- Docker 容器化
- 單一 VPS（AWS EC2 / DigitalOcean）
- 不需要 Kubernetes（避免過度工程）

---

## 技術棧總結

### 後端框架
```toml
axum = "0.7"              # Web 框架（基於 tokio）
tokio = { version = "1", features = ["full"] }  # 異步運行時
tower = "0.4"              # 中間件
tower-http = "0.5"         # CORS, tracing
```

### 數據庫
```toml
sqlx = { version = "0.7", features = [
    "runtime-tokio",       # Tokio 運行時
    "postgres",            # PostgreSQL 驅動
    "macros",              # query! 宏
    "migrate",             # Migration 支持
    "json",                # JSON 支持
    "time"                 # 時間類型
] }
mongodb = "2"              # MongoDB 驅動
redis = { version = "0.24", features = ["tokio-comp"] }
```

### 序列化
```toml
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

### 認證
```toml
jsonwebtoken = "9"         # JWT
argon2 = "0.5"             # 密碼加密
```

### 錯誤處理
```toml
thiserror = "1"            # 定義錯誤類型
anyhow = "1"               # 快速錯誤處理
```

### 日誌
```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
```

### HTTP 客戶端（Worker）
```toml
reqwest = { version = "0.11", features = ["json"] }
```

### 時間處理
```toml
time = { version = "0.3", features = ["serde"] }
```

### Gherkin 解析
```toml
gherkin = "0.14"
```

### 環境變量
```toml
dotenvy = "0.15"
```

---

## 專案結構

```
serval-run-v2/
├── Cargo.toml                      # Rust 專案配置
├── Dockerfile                      # 容器化
├── docker-compose.yml              # 開發環境
├── .env.example                    # 環境變量範例
├── .gitignore
├── README.md                       # 專案說明
├── PROJECT_PLAN.md                 # 本文檔
│
├── migrations/                     # SQL migrations
│   ├── 001_initial_schema.sql
│   ├── 002_add_indexes.sql
│   └── ...
│
├── src/
│   ├── main.rs                     # API Server 入口
│   ├── config.rs                   # 配置管理
│   ├── error.rs                    # 錯誤類型定義
│   ├── state.rs                    # App State (資源池)
│   │
│   ├── models/                     # 數據模型（struct 定義）
│   │   ├── mod.rs
│   │   ├── user.rs
│   │   ├── project.rs
│   │   ├── environment.rs
│   │   ├── collection.rs
│   │   ├── api.rs
│   │   ├── scenario.rs
│   │   ├── report.rs
│   │   └── response.rs
│   │
│   ├── repositories/               # 數據訪問層（SQLx queries）
│   │   ├── mod.rs
│   │   ├── user_repo.rs
│   │   ├── project_repo.rs
│   │   ├── collection_repo.rs
│   │   ├── scenario_repo.rs
│   │   └── report_repo.rs
│   │
│   ├── services/                   # 業務邏輯層
│   │   ├── mod.rs
│   │   ├── auth_service.rs
│   │   ├── gherkin_service.rs      # Gherkin 解析
│   │   ├── test_builder_service.rs # 測試任務構建
│   │   └── queue_service.rs        # Redis 隊列操作
│   │
│   ├── handlers/                   # HTTP 處理器（Axum）
│   │   ├── mod.rs
│   │   ├── auth.rs                 # 註冊/登入
│   │   ├── projects.rs             # Project CRUD
│   │   ├── collections.rs          # Collection CRUD
│   │   ├── apis.rs                 # API CRUD
│   │   ├── scenarios.rs            # Scenario CRUD
│   │   ├── reports.rs              # 報告查詢
│   │   └── test_runner.rs          # 執行測試
│   │
│   ├── middleware/                 # 中間件
│   │   ├── mod.rs
│   │   ├── auth.rs                 # JWT 驗證
│   │   └── logging.rs              # 請求日誌
│   │
│   └── worker/                     # Worker 程序
│       ├── main.rs                 # Worker 入口
│       ├── executor.rs             # HTTP 請求執行
│       └── result_handler.rs       # 結果處理
│
└── tests/                          # 測試
    ├── integration/
    │   ├── auth_tests.rs
    │   ├── project_tests.rs
    │   └── ...
    └── fixtures/
        └── test_data.sql
```

---

## 開發階段規劃

### Phase 0: 專案初始化 (1-2 天) ✅
- [x] 創建 Cargo 專案
- [x] 設置依賴（Cargo.toml）
- [x] 建立專案目錄結構
- [x] Docker Compose 環境
- [x] 設計 PostgreSQL schema
- [x] 編寫 migrations

### Phase 1: 基礎 API Server (3-5 天)
**目標**: 建立 HTTP Server + 認證 + 基礎 CRUD

**功能**:
- [x] 用戶註冊/登入（JWT）
- [x] JWT 中間件
- [x] Project CRUD
- [x] Environment CRUD
- [x] Collection CRUD

**技術要點**:
- Axum 路由設置
- PostgreSQL 連接池
- SQLx 基礎查詢
- 錯誤處理
- 日誌設置

**測試方式**:
```bash
# 註冊
curl -X POST http://localhost:3000/api/auth/signup \
  -H "Content-Type: application/json" \
  -d '{"email":"test@example.com","password":"123456"}'

# 登入（獲取 JWT）
curl -X POST http://localhost:3000/api/auth/signin \
  -d '{"email":"test@example.com","password":"123456"}'

# 創建 Project
curl -X POST http://localhost:3000/api/projects \
  -H "Authorization: Bearer <token>" \
  -d '{"name":"My Project"}'
```

---

### Phase 2: Gherkin 解析和 Scenario 管理 (2-3 天)
**目標**: 實現測試場景創建

**功能**:
- [x] API CRUD
- [x] Scenario CRUD
- [x] Gherkin 解析器
- [ ] 解析結果存儲（PostgreSQL + MongoDB）

**技術要點**:
- 使用 `gherkin` crate 解析
- PostgreSQL 存儲元數據
- MongoDB 存儲原始文檔

**Gherkin 範例**:
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

---

### Phase 3: Redis Queue + Worker (4-5 天)
**目標**: 實現測試任務的異步執行

**架構**:
```
API Server                    Worker
    ↓                           ↓
createTestJob()    →   Redis Queue   →   popJob()
    ↓                                      ↓
insertReport()                        executeHTTP()
insertResponses()                          ↓
    ↓                                  saveResults()
return report_id                           ↓
                                      publishProgress()
```

**功能**:
- [ ] 測試任務構建（3個層級）
  - [ ] Scenario 層級
  - [ ] API 層級
  - [ ] Collection 層級
- [ ] Redis 隊列操作
- [ ] Worker HTTP 執行器
- [ ] 結果保存（PostgreSQL + MongoDB）
- [ ] 進度發布（Redis Pub/Sub）

**技術要點**:
- Redis `RPUSH` / `BRPOP`
- `reqwest` 執行 HTTP 請求
- 錯誤處理（timeout、network error）
- 批量數據庫更新

---

### Phase 4: WebSocket 實時更新 (4-5 天) ⭐⭐⭐⭐⭐ **核心功能**

**重要性**: 這是專案的**核心亮點**，展示分佈式系統設計能力

**目標**: 實現生產級實時進度推送系統

**架構**:
```
Worker 執行測試
    ↓
Redis HINCRBY reportStatus:{id} (原子更新計數)
    ↓
Redis PUBLISH "report-channel" (廣播進度 JSON)
    ↓
API Server SUBSCRIBE (接收所有進度消息)
    ↓
過濾用戶 (檢查 report 所有者)
    ↓
DashMap 查找用戶的 WebSocket 連接
    ↓
WebSocket emit 到該用戶的所有連接
    ↓
前端實時更新進度條 (100ms 延遲)
```

**實現功能**:
- [ ] Axum WebSocket handler 實現
- [ ] Redis Pub/Sub 集成（Worker → API Server 解耦）
- [ ] 多用戶連接管理（`DashMap<UserId, Vec<WebSocketSender>>`）
- [ ] 用戶隔離（每個用戶只接收自己的報告進度）
- [ ] 斷線重連支持（客戶端重連時恢復當前進度）
- [ ] 心跳機制（Ping/Pong 每 30 秒保持連接）
- [ ] 並發安全（Rust ownership + DashMap）
- [ ] 錯誤處理和結構化日誌

**技術要點**:
- `axum::extract::ws::WebSocket` - WebSocket 提取器
- `tokio::select!` - 並發任務管理（send + receive）
- `futures::StreamExt` / `SinkExt` - 異步流處理
- `DashMap` - 線程安全的 HashMap（零鎖開銷）
- Redis `HINCRBY` - 原子操作避免競態條件
- Redis Pub/Sub - 發布訂閱模式解耦組件

**代碼示例**:
```rust
// Worker 發布進度
async fn publish_progress(
    redis: &RedisClient,
    report_id: i64,
    pass: bool,
) -> Result<()> {
    let key = format!("reportStatus:{}", report_id);

    // 原子更新計數
    if pass {
        redis.hincrby(&key, "success", 1).await?;
    } else {
        redis.hincrby(&key, "fail", 1).await?;
    }

    // 獲取當前計數
    let counts: HashMap<String, i32> = redis.hgetall(&key).await?;

    // 發布到 channel
    let progress = ProgressUpdate {
        report_id,
        success: counts["success"],
        fail: counts["fail"],
        timestamp: Utc::now(),
    };

    redis.publish("report-channel", serde_json::to_string(&progress)?).await?;
    Ok(())
}

// API Server WebSocket handler
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
    pubsub.subscribe("report-channel").await.unwrap();

    // 並發處理：接收 Redis 消息 + 處理 WebSocket 消息
    let mut redis_stream = pubsub.on_message();

    tokio::select! {
        // 從 Redis 接收消息並轉發
        _ = async {
            while let Some(msg) = redis_stream.next().await {
                let progress: ProgressUpdate =
                    serde_json::from_str(msg.get_payload()).unwrap();

                // 發送到 WebSocket
                if sender.send(Message::Text(
                    serde_json::to_string(&progress).unwrap()
                )).await.is_err() {
                    break; // 客戶端斷開
                }
            }
        } => {},

        // 處理客戶端消息（心跳、訂閱等）
        _ = async {
            while let Some(Ok(msg)) = receiver.next().await {
                match msg {
                    Message::Ping(_) | Message::Pong(_) => {},
                    Message::Close(_) => break,
                    _ => {}
                }
            }
        } => {},
    }
}
```

**多用戶隔離實現**:
```rust
use dashmap::DashMap;

pub struct WebSocketManager {
    // user_id -> Vec<WebSocket senders>
    connections: Arc<DashMap<i64, Vec<SplitSink<WebSocket, Message>>>>,
}

impl WebSocketManager {
    pub async fn broadcast_to_user(
        &self,
        user_id: i64,
        progress: &ProgressUpdate,
    ) {
        if let Some(mut senders) = self.connections.get_mut(&user_id) {
            let json = serde_json::to_string(progress).unwrap();

            // 移除已斷開的連接
            senders.retain_mut(|sender| {
                sender.send(Message::Text(json.clone())).is_ok()
            });
        }
    }
}
```

**面試亮點**:
1. **分佈式系統設計**: 使用 Pub/Sub 模式解耦 Worker 和 API Server
2. **實時通訊**: WebSocket 實戰經驗，延遲 < 100ms
3. **並發安全**: Rust ownership 和 DashMap 保證線程安全，零競態條件
4. **容錯處理**: 斷線重連、心跳機制、錯誤恢復
5. **多用戶隔離**: 確保用戶只看到自己的數據（安全性）
6. **水平擴展**: Worker 可以無限擴展，API Server 通過 Redis 同步

**對比 v1 (Node.js)**:
- v1: Socket.IO + Session 認證
- v2: 原生 WebSocket + JWT 認證（更輕量）
- v1: 單一 Redis subscriber
- v2: 每個 WebSocket 連接獨立 subscriber（更靈活）
- v1: `global.usersMap` 全局變量
- v2: `DashMap` 並發安全（Rust 優勢）

**測試方式**:
```html
<!-- 簡單的 HTML 測試頁面 -->
<script>
const ws = new WebSocket('ws://localhost:3000/api/ws');

ws.onopen = () => console.log('Connected');

ws.onmessage = (event) => {
    const progress = JSON.parse(event.data);
    console.log(`Report ${progress.report_id}: ${progress.success}/${progress.fail}`);

    // 更新進度條
    const percent = (progress.success + progress.fail) / progress.total * 100;
    document.getElementById('progress-bar').value = percent;
};

ws.onerror = (err) => console.error('WebSocket error:', err);
ws.onclose = () => console.log('Disconnected');
</script>
```

---

### Phase 5: 測試、優化和完善 (5-7 天) ⭐⭐⭐⭐

**重要性**: 測試覆蓋和性能驗證是展示工程質量的關鍵

---

#### 5.1 測試策略 (3-4 天)

**測試金字塔**:
```
        ┌─────────────┐
        │  E2E Tests  │  10% - 完整流程測試
        ├─────────────┤
        │ Integration │  30% - API 端點測試
        ├─────────────┤
        │ Unit Tests  │  60% - 函數級測試
        └─────────────┘
```

##### A. 單元測試 (Unit Tests)

**目標**: 80%+ 代碼覆蓋率

**測試範圍**:
- [ ] **Models**: 數據結構驗證、序列化/反序列化
- [ ] **Repositories**: 數據庫 CRUD 操作（使用 `#[sqlx::test]`）
- [ ] **Services**: 業務邏輯測試（使用 Mock）
- [ ] **Handlers**: HTTP 請求/響應處理
- [ ] **Middleware**: JWT 驗證、錯誤處理
- [ ] **Gherkin Parser**: 各種 Gherkin 語法解析

**測試工具**:
```toml
[dev-dependencies]
# 參數化測試
rstest = "0.18"

# Mock 測試
mockall = "0.12"

# 資料庫測試
sqlx = { version = "0.7", features = ["runtime-tokio", "postgres"] }

# 斷言增強
assert_matches = "1.5"

# 測試覆蓋率
# cargo install cargo-tarpaulin
```

**範例**:
```rust
// 參數化測試（類似 pytest.mark.parametrize）
#[rstest]
#[case("valid@example.com", "password123", true)]
#[case("invalid", "password123", false)]
#[case("test@test.com", "short", false)]
fn test_signup_validation(
    #[case] email: &str,
    #[case] password: &str,
    #[case] expected: bool,
) {
    let result = validate_signup(email, password);
    assert_eq!(result.is_ok(), expected);
}

// 資料庫測試（自動 rollback）
#[sqlx::test]
async fn test_create_user(pool: PgPool) -> sqlx::Result<()> {
    let repo = UserRepository::new(pool);
    let user = repo.create("test@example.com", "hashed").await?;
    assert_eq!(user.email, "test@example.com");
    Ok(())
}

// Mock 測試（隔離依賴）
#[tokio::test]
async fn test_auth_service() {
    let mut mock_repo = MockUserRepository::new();
    mock_repo
        .expect_find_by_email()
        .returning(|_| Ok(Some(User { /* ... */ })));

    let service = AuthService::new(mock_repo);
    let result = service.authenticate("test@example.com", "password").await;
    assert!(result.is_ok());
}
```

**測試命令**:
```bash
# 運行所有測試
cargo test

# 顯示輸出
cargo test -- --nocapture

# 測試覆蓋率
cargo tarpaulin --out Html --output-dir coverage/
```

---

##### B. 集成測試 (Integration Tests)

**目標**: 測試所有 API 端點

**測試範圍**:
- [ ] **Auth API**: signup, signin, token 驗證
- [ ] **Projects API**: CRUD operations
- [ ] **Collections API**: CRUD + run tests
- [ ] **APIs API**: CRUD + scenarios
- [ ] **Reports API**: 查詢報告、統計數據
- [ ] **WebSocket**: 連接、斷線、進度推送

**測試工具**:
```toml
[dev-dependencies]
axum-test = "14"  # HTTP 集成測試
tokio-tungstenite = "0.21"  # WebSocket 測試
```

**範例**:
```rust
// tests/api_auth_test.rs
use axum_test::TestServer;

#[tokio::test]
async fn test_signup_success() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let response = server
        .post("/api/auth/signup")
        .json(&json!({
            "email": "test@example.com",
            "password": "password123"
        }))
        .await;

    response.assert_status_ok();
    response.assert_json(&json!({
        "email": "test@example.com"
    }));
}

#[tokio::test]
async fn test_protected_endpoint_without_token() {
    let server = create_test_server().await;

    let response = server
        .get("/api/projects")
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}
```

**測試文件結構**:
```
tests/
├── common/
│   └── mod.rs           # 共用測試工具
├── api_auth_test.rs     # 認證 API 測試
├── api_projects_test.rs # 專案 API 測試
├── api_collections_test.rs
├── api_reports_test.rs
├── websocket_test.rs    # WebSocket 測試
└── e2e_test.rs          # 端到端測試
```

---

##### C. 端到端測試 (E2E Tests)

**目標**: 測試完整業務流程

**測試場景**:
- [ ] **完整測試流程**: Signup → Create Project → Create Collection → Add API → Run Tests → Check Report
- [ ] **WebSocket 實時更新**: 觸發測試 → 連接 WebSocket → 接收進度 → 驗證數據
- [ ] **錯誤處理**: 各種錯誤場景的完整流程

**範例**:
```rust
#[tokio::test]
async fn test_complete_workflow() {
    let server = create_test_server().await;

    // 1. 註冊用戶
    let signup_res = server.post("/api/auth/signup")
        .json(&json!({"email": "test@example.com", "password": "pass123"}))
        .await;
    signup_res.assert_status_ok();

    // 2. 登入獲取 token
    let signin_res = server.post("/api/auth/signin")
        .json(&json!({"email": "test@example.com", "password": "pass123"}))
        .await;
    let token = signin_res.json::<AuthResponse>().token;

    // 3. 創建專案
    let project_res = server.post("/api/projects")
        .bearer_token(&token)
        .json(&json!({"name": "Test Project"}))
        .await;
    let project_id = project_res.json::<Project>().id;

    // 4. 創建 Collection
    let collection_res = server.post(&format!("/api/projects/{}/collections", project_id))
        .bearer_token(&token)
        .json(&json!({"name": "Test Collection"}))
        .await;
    let collection_id = collection_res.json::<Collection>().id;

    // 5. 運行測試
    let run_res = server.post(&format!("/api/collections/{}/run", collection_id))
        .bearer_token(&token)
        .await;
    run_res.assert_status_ok();
    let report_id = run_res.json::<RunResponse>().report_id;

    // 6. 檢查報告
    tokio::time::sleep(Duration::from_secs(2)).await;
    let report_res = server.get(&format!("/api/reports/{}", report_id))
        .bearer_token(&token)
        .await;
    report_res.assert_status_ok();
}
```

---

#### 5.2 性能測試與基準測試 (2-3 天)

##### A. Benchmark 測試（Rust 內部）

**工具**: `criterion` - Rust 的性能測試標準

**測試範圍**:
- [ ] Gherkin 解析性能
- [ ] 數據庫查詢性能
- [ ] JWT 編碼/解碼性能
- [ ] JSON 序列化/反序列化

**配置**:
```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports", "async_tokio"] }

[[bench]]
name = "gherkin_parser"
harness = false

[[bench]]
name = "database_queries"
harness = false
```

**範例**:
```rust
// benches/gherkin_parser.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_parse_gherkin(c: &mut Criterion) {
    let gherkin_text = include_str!("../fixtures/complex.feature");

    c.bench_function("parse complex gherkin", |b| {
        b.iter(|| parse_gherkin(black_box(gherkin_text)))
    });
}

criterion_group!(benches, bench_parse_gherkin);
criterion_main!(benches);
```

**運行**:
```bash
cargo bench
# 生成 HTML 報告在 target/criterion/
```

---

##### B. 負載測試（API Throughput）

**工具**: `k6` - 現代負載測試工具

**測試場景**:
- [ ] **認證 API**: 測試 signup/signin 吞吐量
- [ ] **查詢 API**: 測試 GET /api/projects 並發性能
- [ ] **寫入 API**: 測試 POST /api/projects 並發性能
- [ ] **壓力測試**: 逐漸增加負載直到崩潰

**k6 腳本範例**:
```javascript
// load_tests/auth_load.js
import http from 'k6/http';
import { check, sleep } from 'k6';

export const options = {
  stages: [
    { duration: '30s', target: 100 },  // 30秒內增加到100用戶
    { duration: '1m', target: 500 },   // 1分鐘內增加到500用戶
    { duration: '30s', target: 0 },    // 30秒內降到0
  ],
  thresholds: {
    http_req_duration: ['p(95)<500'], // 95% 的請求要在 500ms 內完成
    http_req_failed: ['rate<0.01'],   // 錯誤率 < 1%
  },
};

export default function () {
  const payload = JSON.stringify({
    email: `user${__VU}@example.com`,
    password: 'password123',
  });

  const params = {
    headers: { 'Content-Type': 'application/json' },
  };

  const res = http.post('http://localhost:3000/api/auth/signup', payload, params);

  check(res, {
    'status is 200': (r) => r.status === 200,
    'response time < 200ms': (r) => r.timings.duration < 200,
  });

  sleep(1);
}
```

**運行**:
```bash
k6 run load_tests/auth_load.js
```

---

##### C. v1 vs v2 對比測試 ⭐ **關鍵展示**

**目標**: 證明 Rust 版本的性能提升

**對比指標**:

| 測試項目                       | v1 (Node.js) | v2 (Rust) | 測試方法        |
| ------------------------------ | ------------ | --------- | --------------- |
| **單次 API 測試**              | ~150ms       | ~50ms     | 直接計時        |
| **Collection 測試 (100 APIs)** | ~33s         | ~15s      | Worker 執行時間 |
| **GET /api/reports/:id**       | ~1.7s        | ~150ms    | k6 p50          |
| **內存使用 (Idle)**            | ~200MB       | ~50MB     | Docker stats    |
| **內存使用 (1000 reqs)**       | ~400MB       | ~80MB     | Docker stats    |
| **並發處理 (req/s)**           | ~1000        | ~5000     | k6 throughput   |
| **WebSocket 延遲**             | ~150ms       | ~50ms     | 測試腳本        |

**對比測試腳本**:
```bash
#!/bin/bash
# compare_performance.sh

echo "=== Testing v1 (Node.js) ==="
cd serval-run
docker-compose up -d
k6 run ../load_tests/compare.js --out json=v1_results.json
docker-compose down

echo "=== Testing v2 (Rust) ==="
cd ../serval-run-v2
docker-compose up -d
k6 run ../load_tests/compare.js --out json=v2_results.json
docker-compose down

echo "=== Generating Comparison Report ==="
python3 generate_comparison.py v1_results.json v2_results.json
```

**輸出**: 生成對比圖表和 Markdown 報告

---

##### D. 性能監控（持續）

**工具**: Prometheus + Grafana

**指標收集**:
- [ ] HTTP 請求延遲（p50, p95, p99）
- [ ] 數據庫查詢時間
- [ ] Redis 操作時間
- [ ] Worker 執行時間
- [ ] WebSocket 連接數
- [ ] 記憶體使用
- [ ] CPU 使用率

---

#### 5.3 功能完善

- [ ] 錯誤處理完善（統一錯誤格式）
- [ ] 輸入驗證（validator crate）
- [ ] API 文檔（OpenAPI/Swagger）
- [ ] 健康檢查端點 (`GET /health`)
- [ ] Graceful shutdown（tokio signal）
- [ ] 日誌輪轉（tracing-appender）

---

#### 5.4 代碼質量

- [ ] **Clippy**: `cargo clippy -- -D warnings`
- [ ] **Format**: `cargo fmt --check`
- [ ] **Audit**: `cargo audit` (檢查依賴漏洞)
- [ ] **Documentation**: `cargo doc --no-deps --open`

---

**Phase 5 完成標準**:
- ✅ 測試覆蓋率 > 80%
- ✅ 所有集成測試通過
- ✅ E2E 測試通過
- ✅ 性能對比數據完整（v1 vs v2）
- ✅ k6 負載測試報告
- ✅ 所有 Clippy warnings 解決
- ✅ API 文檔完整

---

## 性能目標

### 對比 v1 (Node.js)

| 指標                       | v1 (Node.js) | v2 (Rust) 目標 | 提升 |
| -------------------------- | ------------ | -------------- | ---- |
| 單次 API 測試              | ~150ms       | ~50ms          | 3x   |
| Collection 測試 (100 APIs) | ~33s         | ~15s           | 2.2x |
| 報告查詢                   | ~1.7s        | ~150ms         | 11x  |
| 內存使用                   | ~200MB       | ~50MB          | 4x   |
| 並發處理                   | ~1000 req/s  | ~5000 req/s    | 5x   |

---

## 面試準備要點

### 技術亮點總結

1. **多數據庫架構**
   - "使用 PostgreSQL 處理事務性數據和複雜查詢"
   - "使用 MongoDB 存儲靈活 schema 的文檔和日誌"
   - "各取所長，性能提升 30%"

2. **SQLx 選擇**
   - "選擇 SQLx 而非 ORM，展示 SQL 能力"
   - "編譯時 SQL 驗證保證類型安全"
   - "寫了複雜的 JOIN、CTE、Window Functions"

3. **性能優化**
   - "從 Node.js 遷移到 Rust，性能提升 3 倍"
   - "使用 tokio async/await 實現高並發"
   - "優化數據庫查詢，從 N+1 查詢改為單次 JOIN"

4. **架構設計**
   - "生產者-消費者模式解耦 API Server 和 Worker"
   - "使用 Redis Pub/Sub 實現實時進度推送"
   - "Repository Pattern 分離數據訪問邏輯"

5. **工程實踐**
   - "編譯時類型檢查捕獲錯誤"
   - "結構化日誌（tracing）"
   - "單元測試 + 集成測試"
   - "Docker 容器化部署"

---

## 學習資源

### Rust 基礎
- [The Rust Book](https://doc.rust-lang.org/book/)
  - 重點章節: 4 (Ownership), 9 (Error Handling), 10 (Traits), 15 (Smart Pointers), 16 (Concurrency)
- [Async Book](https://rust-lang.github.io/async-book/)

### Axum
- [Axum Examples](https://github.com/tokio-rs/axum/tree/main/examples)
- 重點: routing, extractors, middleware, error handling

### SQLx
- [SQLx GitHub](https://github.com/launchbadge/sqlx)
- 重點: query! macro, transactions, migrations

### 性能優化
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)

---

## 風險和挑戰

### 技術挑戰
1. **Rust 學習曲線**
   - Ownership & Borrowing
   - Lifetime 標註
   - Async/await 理解
   - **緩解**: 從簡單功能開始，逐步增加複雜度

2. **多數據庫同步**
   - PostgreSQL 和 MongoDB 的數據一致性
   - **緩解**: 使用事務，PostgreSQL 為主，MongoDB 為輔

3. **性能調優**
   - 需要學習 Rust profiling 工具
   - **緩解**: 先實現功能，後優化性能

### 時間管理
- **估計總時間**: 3-4 週（兼職開發）
- **風險**: 可能需要更多時間學習 Rust
- **緩解**: 採用增量開發，每個 Phase 都有可運行的成果

---

## 成功指標

### 技術指標
- [ ] 所有核心功能實現
- [ ] 性能比 v1 提升 2x 以上
- [ ] 測試覆蓋率 > 70%
- [ ] 零 `unsafe` 代碼（除非必要）

### 學習指標
- [ ] 熟練使用 Rust async/await
- [ ] 能寫複雜的 SQL 查詢
- [ ] 理解多數據庫架構
- [ ] 掌握 Rust 錯誤處理

### 求職指標
- [ ] 完整的 README 和文檔
- [ ] 清晰的 Git commit history
- [ ] 部署到公開服務器
- [ ] 準備好技術面試講解

---

## 下一步行動

### 立即行動 (今天)
1. [ ] 設置 Cargo.toml 依賴
2. [ ] 創建目錄結構
3. [ ] 編寫 docker-compose.yml
4. [ ] 設計 PostgreSQL schema

### 本週目標
- [ ] 完成 Phase 0（專案初始化）
- [ ] 開始 Phase 1（用戶認證）

### 本月目標
- [ ] 完成 Phase 1-3（基礎功能 + Worker）

---

## 附錄

### A. 環境變量範例
```env
# Database
DATABASE_URL=postgres://serval:password@localhost:5432/serval_run
MONGODB_URL=mongodb://localhost:27017
REDIS_URL=redis://localhost:6379

# JWT
JWT_SECRET=your-secret-key-change-in-production
JWT_EXPIRATION_HOURS=24

# Server
HOST=0.0.0.0
PORT=3000

# Worker
WORKER_CONCURRENCY=10

# Log
RUST_LOG=serval_run=debug,sqlx=info
```

### B. Git Workflow
```bash
# 主分支
main

# 功能分支
git checkout -b phase1-auth
git checkout -b phase2-gherkin
git checkout -b phase3-worker
git checkout -b phase4-websocket

# Commit 規範
feat: add user authentication
fix: resolve database connection issue
refactor: improve error handling
docs: update README
test: add integration tests for projects
perf: optimize query performance
```

### C. 效能基準測試計劃
```bash
# 1. 單次 API 測試
wrk -t4 -c100 -d30s http://localhost:3000/api/projects

# 2. 測試執行性能
# v1 (Node.js) vs v2 (Rust)
time node worker.js  # v1
time cargo run --release --bin worker  # v2

# 3. 內存使用
# 使用 htop 或 Activity Monitor 觀察

# 4. 數據庫查詢性能
EXPLAIN ANALYZE SELECT ...
```

---

## 版本歷史

- **v2.0.0-plan**: 2025-01-19 - 專案計劃制定
- **v2.0.0-alpha**: TBD - Phase 1-2 完成
- **v2.0.0-beta**: TBD - Phase 3-4 完成
- **v2.0.0**: TBD - 正式發布

---

## 聯絡方式

- **GitHub**: https://github.com/hazel-ys-lin/serval-run-v2
- **Email**: hazel.ys.lin@gmail.com
- **LinkedIn**: https://www.linkedin.com/in/hazel-lin-yi-sin/

---

*Last Updated: 2025-01-19*
