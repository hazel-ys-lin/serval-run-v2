use async_trait::async_trait;
use redis::aio::ConnectionManager as RedisConnectionManager;
use redis::AsyncCommands;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::queue::{JobQueue, JobResult, JobStatus, TestJob};

/// Redis keys structure:
/// - serval:jobs:queue           - List for pending jobs (FIFO)
/// - serval:jobs:{id}            - String for job data (JSON)
/// - serval:jobs:by_user:{uid}   - Set of job IDs by user
const QUEUE_KEY: &str = "serval:jobs:queue";
const JOB_PREFIX: &str = "serval:jobs:";
const USER_PREFIX: &str = "serval:jobs:by_user:";

/// Redis-backed job queue implementation
#[derive(Clone)]
pub struct RedisQueue {
    conn: RedisConnectionManager,
}

impl RedisQueue {
    pub fn new(conn: RedisConnectionManager) -> Self {
        Self { conn }
    }

    fn job_key(id: Uuid) -> String {
        format!("{}{}", JOB_PREFIX, id)
    }

    fn user_key(user_id: Uuid) -> String {
        format!("{}{}", USER_PREFIX, user_id)
    }

    async fn save_job(&self, job: &TestJob) -> AppResult<()> {
        let mut conn = self.conn.clone();
        let job_json = serde_json::to_string(job)
            .map_err(|e| AppError::Internal(format!("Serialization error: {}", e)))?;

        let _: () = conn
            .set(Self::job_key(job.id), &job_json)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        Ok(())
    }
}

#[async_trait]
impl JobQueue for RedisQueue {
    async fn enqueue(&self, job: TestJob) -> AppResult<Uuid> {
        let mut conn = self.conn.clone();
        let job_id = job.id;
        let user_id = job.user_id;

        // Store job data
        self.save_job(&job).await?;

        // Add to queue
        let _: () = conn
            .rpush(QUEUE_KEY, job_id.to_string())
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        // Track by user
        let _: () = conn
            .sadd(Self::user_key(user_id), job_id.to_string())
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        tracing::info!(job_id = %job_id, "Job enqueued");

        Ok(job_id)
    }

    async fn dequeue(&self, timeout_seconds: u64) -> AppResult<Option<TestJob>> {
        let mut conn = self.conn.clone();

        // Blocking pop from queue
        let result: Option<(String, String)> = conn
            .blpop(QUEUE_KEY, timeout_seconds as f64)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        if let Some((_, job_id_str)) = result {
            let job_id = Uuid::parse_str(&job_id_str)
                .map_err(|e| AppError::Internal(format!("Invalid UUID: {}", e)))?;

            // Get job data
            if let Some(mut job) = self.get_job(job_id).await? {
                // Update status to running
                job.status = JobStatus::Running;
                job.started_at = Some(time::OffsetDateTime::now_utc());
                self.save_job(&job).await?;

                tracing::info!(job_id = %job_id, "Job dequeued and started");
                return Ok(Some(job));
            }
        }

        Ok(None)
    }

    async fn get_job(&self, job_id: Uuid) -> AppResult<Option<TestJob>> {
        let mut conn = self.conn.clone();

        let job_json: Option<String> = conn
            .get(Self::job_key(job_id))
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        match job_json {
            Some(json) => {
                let job: TestJob = serde_json::from_str(&json)
                    .map_err(|e| AppError::Internal(format!("Deserialization error: {}", e)))?;
                Ok(Some(job))
            }
            None => Ok(None),
        }
    }

    async fn update_status(&self, job_id: Uuid, status: JobStatus) -> AppResult<()> {
        let mut job = self
            .get_job(job_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Job".to_string()))?;

        job.status = status;

        if status.is_terminal() {
            job.completed_at = Some(time::OffsetDateTime::now_utc());
        }

        self.save_job(&job).await?;

        tracing::info!(job_id = %job_id, status = ?status, "Job status updated");

        Ok(())
    }

    async fn complete_job(&self, job_id: Uuid, result: JobResult) -> AppResult<()> {
        let mut job = self
            .get_job(job_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Job".to_string()))?;

        job.status = JobStatus::Completed;
        job.completed_at = Some(time::OffsetDateTime::now_utc());
        job.report_id = Some(result.report_id);

        self.save_job(&job).await?;

        tracing::info!(
            job_id = %job_id,
            report_id = %result.report_id,
            passed = result.passed,
            failed = result.failed,
            "Job completed"
        );

        Ok(())
    }

    async fn fail_job(&self, job_id: Uuid, error: String, retryable: bool) -> AppResult<()> {
        let mut job = self
            .get_job(job_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Job".to_string()))?;

        job.error_message = Some(error.clone());

        let new_status = if retryable && job.retry_count < job.max_retries {
            job.retry_count += 1;
            JobStatus::Failed
        } else {
            job.completed_at = Some(time::OffsetDateTime::now_utc());
            JobStatus::Dead
        };

        job.status = new_status;
        self.save_job(&job).await?;

        tracing::warn!(
            job_id = %job_id,
            status = ?new_status,
            retry_count = job.retry_count,
            error = %error,
            "Job failed"
        );

        Ok(())
    }

    async fn queue_length(&self) -> AppResult<u64> {
        let mut conn = self.conn.clone();
        let len: u64 = conn
            .llen(QUEUE_KEY)
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;
        Ok(len)
    }

    async fn list_jobs_by_user(&self, user_id: Uuid, limit: u64) -> AppResult<Vec<TestJob>> {
        let mut conn = self.conn.clone();

        let job_ids: Vec<String> = conn
            .smembers(Self::user_key(user_id))
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        let mut jobs = Vec::new();
        for job_id_str in job_ids.into_iter().take(limit as usize) {
            if let Ok(job_id) = Uuid::parse_str(&job_id_str) {
                if let Some(job) = self.get_job(job_id).await? {
                    jobs.push(job);
                }
            }
        }

        // Sort by created_at descending
        jobs.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(jobs)
    }

    async fn requeue(&self, job_id: Uuid) -> AppResult<()> {
        let mut job = self
            .get_job(job_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Job".to_string()))?;

        if job.status != JobStatus::Failed {
            return Err(AppError::Validation(
                "Only failed jobs can be requeued".to_string(),
            ));
        }

        let mut conn = self.conn.clone();

        job.status = JobStatus::Pending;
        job.started_at = None;
        job.error_message = None;

        self.save_job(&job).await?;

        // Add back to queue
        let _: () = conn
            .rpush(QUEUE_KEY, job_id.to_string())
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        tracing::info!(job_id = %job_id, "Job requeued");

        Ok(())
    }

    async fn delete_job(&self, job_id: Uuid) -> AppResult<()> {
        let job = self
            .get_job(job_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Job".to_string()))?;

        let mut conn = self.conn.clone();

        // Remove from user index
        let _: () = conn
            .srem(Self::user_key(job.user_id), job_id.to_string())
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        // Delete job data
        let _: () = conn
            .del(Self::job_key(job_id))
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        tracing::info!(job_id = %job_id, "Job deleted");

        Ok(())
    }

    async fn cancel_job(&self, job_id: Uuid) -> AppResult<()> {
        let mut job = self
            .get_job(job_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Job".to_string()))?;

        if job.status.is_terminal() {
            return Err(AppError::Validation(
                "Cannot cancel a completed job".to_string(),
            ));
        }

        job.status = JobStatus::Cancelled;
        job.completed_at = Some(time::OffsetDateTime::now_utc());

        self.save_job(&job).await?;

        tracing::info!(job_id = %job_id, "Job cancelled");

        Ok(())
    }
}
