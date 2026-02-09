📝 你需要創建的文件清單
1. Cargo.toml (必須，Rust 專案配置文件)

[package]
name = "serval-run"
version = "2.0.0"
edition = "2021"

# 定義兩個 binary
[[bin]]
name = "server"
path = "src/main.rs"

[[bin]]
name = "worker"
path = "src/worker/main.rs"

[dependencies]
# ... 所有依賴（我列好了 45+ 個依賴）
作用: 類似 Node.js 的 package.json

2. .gitignore (必須)

/target/           # Rust 編譯產物（類似 node_modules）
Cargo.lock         # 依賴鎖定文件（library 不提交，binary 提交）
.env               # 環境變量
*.db               # SQLite 測試數據庫
.DS_Store
3. docker-compose.yml (開發環境必須)

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
作用: 一鍵啟動所有數據庫

4. .env.example (開發配置範例)

# Database
DATABASE_URL=postgres://serval:password@localhost:5432/serval_run
MONGODB_URL=mongodb://localhost:27017
REDIS_URL=redis://localhost:6379

# JWT
JWT_SECRET=your-secret-key-change-this
JWT_EXPIRATION_HOURS=24

# Server
HOST=0.0.0.0
PORT=3000

# Logging
RUST_LOG=serval_run=debug,sqlx=info
作用: 告訴其他開發者需要哪些環境變量

5. migrations/001_initial_schema.sql (數據庫 schema)

-- 創建所有表：users, projects, environments, collections, 
-- apis, scenarios, reports, responses
-- 創建所有索引
-- 創建 updated_at trigger
作用: SQLx 會自動執行這些 SQL 建表

6. src/main.rs (API Server 入口)

// 最簡單的 Hello World 版本
use axum::{Router, routing::get};

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(|| async { "Hello, ServalRun!" }));
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();
    
    axum::serve(listener, app).await.unwrap();
}
作用: 可以跑起來的最小 API server

7. src/worker/main.rs (Worker 入口)

// 最簡單的版本
#[tokio::main]
async fn main() {
    println!("Worker started");
    // TODO: 連接 Redis，處理任務
}
作用: Worker 程序入口（Phase 3 才會實作）

8. 目錄結構 (創建這些空文件夾和 mod.rs)

src/
├── main.rs                  ← 創建（API server）
├── config.rs                ← 創建（配置管理）
├── error.rs                 ← 創建（錯誤類型）
├── state.rs                 ← 創建（App State）
│
├── models/                  ← 創建資料夾
│   └── mod.rs              ← 創建（空文件）
│
├── repositories/            ← 創建資料夾
│   └── mod.rs              ← 創建（空文件）
│
├── services/                ← 創建資料夾
│   └── mod.rs              ← 創建（空文件）
│
├── handlers/                ← 創建資料夾
│   └── mod.rs              ← 創建（空文件）
│
├── middleware/              ← 創建資料夾
│   └── mod.rs              ← 創建（空文件）
│
└── worker/                  ← 創建資料夾
    └── main.rs             ← 創建（Worker 入口）
每個 mod.rs 的內容：


// 空文件，或者只有一行註解
// TODO: Implement this module
9. Dockerfile (可選，生產部署用)

FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin server

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/server /usr/local/bin/
CMD ["server"]
10. tests/ 資料夾（測試用）

tests/
├── integration/
│   └── .gitkeep
└── fixtures/
    └── .gitkeep
🚀 創建順序建議
Step 1: 基礎文件（必須）

cd /Users/hazel_lin/repos/serval-run-v2

# 1. 創建 Cargo.toml
# 2. 創建 .gitignore
# 3. 創建 docker-compose.yml
# 4. 創建 .env.example
Step 2: 源代碼結構

# 創建所有目錄
mkdir -p src/{models,repositories,services,handlers,middleware,worker}
mkdir -p migrations
mkdir -p tests/{integration,fixtures}

# 創建所有 mod.rs
touch src/models/mod.rs
touch src/repositories/mod.rs
touch src/services/mod.rs
touch src/handlers/mod.rs
touch src/middleware/mod.rs

# 創建入口文件
touch src/main.rs
touch src/worker/main.rs
touch src/config.rs
touch src/error.rs
touch src/state.rs
Step 3: 數據庫 Migrations

# 創建 migration 文件
touch migrations/001_initial_schema.sql
Step 4: 驗證

# 構建專案（會下載所有依賴）
cargo build

# 如果成功，應該看到：
# Compiling serval-run v2.0.0
# Finished dev [unoptimized + debuginfo] target(s) in X.XXs
📦 檔案大小參考
文件	大小	重要性
Cargo.toml	~1.5KB	⭐⭐⭐⭐⭐
.gitignore	~200B	⭐⭐⭐⭐⭐
docker-compose.yml	~1KB	⭐⭐⭐⭐⭐
.env.example	~500B	⭐⭐⭐⭐
migrations/001_initial_schema.sql	~5KB	⭐⭐⭐⭐⭐
src/main.rs	~500B	⭐⭐⭐⭐⭐
src/worker/main.rs	~100B	⭐⭐⭐
所有 mod.rs	~50B 每個	⭐⭐
⚠️ 注意事項
Cargo.toml 是最重要的

沒有它，cargo build 無法執行
依賴版本要寫對
migrations/ 是 SQLx 必須的

SQLx 在編譯時會連接數據庫驗證 SQL
需要先運行 sqlx migrate run
第一次 cargo build 會很慢

需要下載所有依賴（類似第一次 npm install）
可能需要 5-10 分鐘
之後就快了
src/main.rs 必須有 main 函數

否則 cargo build 會報錯
💡 建議工作流程

# 1. 創建基礎文件
vim Cargo.toml         # 複製我給的內容
vim .gitignore         # 複製我給的內容
vim docker-compose.yml # 複製我給的內容

# 2. 創建目錄結構
mkdir -p src/{models,repositories,services,handlers,middleware,worker}
mkdir -p migrations tests/{integration,fixtures}

# 3. 創建空 Rust 文件
touch src/main.rs src/worker/main.rs src/{config,error,state}.rs
touch src/{models,repositories,services,handlers,middleware}/mod.rs

# 4. 寫最簡單的 main.rs（我會給你代碼）

# 5. 試著構建
cargo build

# 6. 啟動數據庫
docker-compose up -d

# 7. 運行 migrationsc
cargo install sqlx-cli --features postgres
sqlx migrate run

# 8. 運行 server
cargo run --bin server

# 9. 測試
curl http://localhost:3000