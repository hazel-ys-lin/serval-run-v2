# ServalRun v2 — Development Guidelines

## Project Structure

- **Framework**: Rust + Axum + SeaORM (PostgreSQL) + MongoDB + Redis
- **Binaries**: `server` (src/main.rs) and `worker` (src/worker/main.rs)
- **Tests**: Unit tests (17) + Integration tests (125+) in `tests/` directory

## Critical Rules

### Migrations
- **NEVER modify an already-executed migration file**, not even comments. `sqlx::migrate!` uses checksums — any change will break existing databases.
- If schema or comments need updating, create a new migration file (e.g., `009_fix_xxx.sql`).
- CI builds a fresh DB each time so won't catch checksum issues — always test locally too.

### SeaORM Queries
- **Do NOT use `select_only().column()` with `.one(db)`** — `.one(db)` deserializes into the full `Model` struct, which fails when columns are missing. Use `into_tuple()` if you truly need partial selects.
- First-level JOINs: use `.inner_join(TargetEntity)` (SeaORM auto-resolves the relation).
- Multi-level JOINs: use `.join(JoinType::InnerJoin, Relation::Xxx.def())` — requires `use sea_orm::RelationTrait`.
- Check `src/entity/*.rs` for exact `Relation` enum variant names (e.g., `Relation::Project` not `Relation::Projects`).

### Error Handling
- Database/internal errors must NOT leak raw details to API responses. Log the original error with `tracing::error!`, return a generic message.
- In binary startup code (`main`), use `.expect("descriptive message")` instead of `.unwrap()`.
- In handler/business logic, use `?` operator with the `AppError` type.

### Router Architecture
- `build_router()` returns `Router<()>` after `.with_state()`. Routes needing `State<AppState>` must be added BEFORE `.with_state()`.

## Testing

### Running Tests
```bash
cargo test              # all tests
cargo test --test api_test  # specific test file
```

### Test Infrastructure
- `InMemoryQueue` for tests, `RedisQueue` for production (trait: `JobQueue`).
- `TestApp` wraps `axum_test::TestServer` + `AppState`.
- `Factory` provides data builders: `create_user()`, `create_hierarchy()`, `create_api()`, etc.

### Common Test Issues
- If you see `Migration("migration N was previously applied but has been modified")`:
  Revert the migration file to its original content, or manually clean `_sqlx_migrations` table.
- `Cargo.lock` is in `.gitignore` — don't try to `git add` it.

## Code Style
- Run `cargo fmt` before `cargo clippy` (CI checks fmt first).
- Validation helpers: `validate_required()` and `validate_optional()` in `src/handlers/common.rs`.
- Ownership verification: use `OwnershipVerifier` in `src/repositories/ownership.rs` (shared across all repositories).
- MongoDB saves are non-fatal (wrapped in `if let Err` with warning logs).

## Dependencies
- `jsonwebtoken` uses v10 with `aws_lc_rs` feature flag for crypto backend.

## Learning Notes
- 每次檢討、修改或修 bug 時，若有學習價值，請 append 到 Heptabase 既有卡片：
  - 卡名：`Rust 學習筆記：Python/FastAPI 開發者的 Rust 之路`
  - card_id: `fd4f349b-9aaa-48e3-acc0-66716c004505`
  - whiteboard: `Rust` (id: `986b86a9-fb9d-4fa1-89c0-959ff1002412`)
- Append 指令：
  ```bash
  heptabase note append --card-id fd4f349b-9aaa-48e3-acc0-66716c004505 \
    --content "## <主題>\n\n<心得內容>"
  ```
- 筆記以 Python/FastAPI 開發者的角度撰寫，幫助理解 Rust/Axum 的對應概念。
- 不再使用本地 `RUST_LEARNING_NOTES.md`（.gitignore 規則保留以防誤建）。

## User Preferences
- Communicates in Traditional Chinese (繁體中文).
