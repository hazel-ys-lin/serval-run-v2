pub mod api;
pub mod collection;
pub mod environment;
pub mod project;
pub mod report;
pub mod response;
pub mod scenario;
pub mod user;

pub use api::ApiRepository;
pub use collection::CollectionRepository;
pub use environment::EnvironmentRepository;
pub use project::ProjectRepository;
pub use report::ReportRepository;
pub use response::ResponseRepository;
pub use scenario::ScenarioRepository;
pub use user::UserRepository;

use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::error::AppResult;

/// Base repository trait for common CRUD operations
#[async_trait]
pub trait Repository<T>
where
    T: Send + Sync,
{
    /// Find entity by ID
    async fn find_by_id(db: &DatabaseConnection, id: Uuid) -> AppResult<T>;

    /// Delete entity by ID
    async fn delete(db: &DatabaseConnection, id: Uuid) -> AppResult<()>;

    /// List entities with pagination
    async fn list(db: &DatabaseConnection, limit: u64, offset: u64) -> AppResult<Vec<T>>;

    /// Count total entities
    async fn count(db: &DatabaseConnection) -> AppResult<u64>;
}
