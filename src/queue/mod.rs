pub mod job;
pub mod memory_queue;
pub mod redis_queue;

pub use job::{JobResult, JobStatus, TestJob, TestJobConfig, TestJobType};
pub use memory_queue::InMemoryQueue;
pub use redis_queue::RedisQueue;

use async_trait::async_trait;
use uuid::Uuid;

use crate::error::AppResult;

/// Job queue trait for abstracting queue backends
/// Follows existing async_trait pattern from repositories
#[async_trait]
pub trait JobQueue: Send + Sync {
    /// Push a job onto the queue
    async fn enqueue(&self, job: TestJob) -> AppResult<Uuid>;

    /// Pop the next job from the queue (blocking with timeout)
    async fn dequeue(&self, timeout_seconds: u64) -> AppResult<Option<TestJob>>;

    /// Get job by ID
    async fn get_job(&self, job_id: Uuid) -> AppResult<Option<TestJob>>;

    /// Update job status
    async fn update_status(&self, job_id: Uuid, status: JobStatus) -> AppResult<()>;

    /// Update job with result (status + result data)
    async fn complete_job(&self, job_id: Uuid, result: JobResult) -> AppResult<()>;

    /// Mark job as failed with error message
    async fn fail_job(&self, job_id: Uuid, error: String, retryable: bool) -> AppResult<()>;

    /// Get queue length
    async fn queue_length(&self) -> AppResult<u64>;

    /// Get jobs by user (for listing)
    async fn list_jobs_by_user(&self, user_id: Uuid, limit: u64) -> AppResult<Vec<TestJob>>;

    /// Requeue failed job for retry
    async fn requeue(&self, job_id: Uuid) -> AppResult<()>;

    /// Delete a job
    async fn delete_job(&self, job_id: Uuid) -> AppResult<()>;

    /// Cancel a pending or running job
    async fn cancel_job(&self, job_id: Uuid) -> AppResult<()>;
}
