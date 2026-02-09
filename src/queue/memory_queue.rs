use async_trait::async_trait;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::queue::{JobQueue, JobResult, JobStatus, TestJob};

/// In-memory queue for unit testing
#[derive(Clone)]
pub struct InMemoryQueue {
    inner: Arc<Mutex<InMemoryQueueInner>>,
    notify: Arc<Notify>,
}

struct InMemoryQueueInner {
    queue: VecDeque<Uuid>,
    jobs: HashMap<Uuid, TestJob>,
}

impl InMemoryQueue {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(InMemoryQueueInner {
                queue: VecDeque::new(),
                jobs: HashMap::new(),
            })),
            notify: Arc::new(Notify::new()),
        }
    }
}

impl Default for InMemoryQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl JobQueue for InMemoryQueue {
    async fn enqueue(&self, job: TestJob) -> AppResult<Uuid> {
        let job_id = job.id;
        let mut inner = self.inner.lock().await;
        inner.jobs.insert(job_id, job);
        inner.queue.push_back(job_id);
        drop(inner);
        self.notify.notify_one();
        Ok(job_id)
    }

    async fn dequeue(&self, timeout_seconds: u64) -> AppResult<Option<TestJob>> {
        let timeout = std::time::Duration::from_secs(timeout_seconds);

        // Try to get a job immediately
        {
            let mut inner = self.inner.lock().await;
            if let Some(job_id) = inner.queue.pop_front() {
                if let Some(job) = inner.jobs.get_mut(&job_id) {
                    job.status = JobStatus::Running;
                    job.started_at = Some(time::OffsetDateTime::now_utc());
                    return Ok(Some(job.clone()));
                }
            }
        }

        // Wait for notification with timeout
        tokio::select! {
            _ = tokio::time::sleep(timeout) => Ok(None),
            _ = self.notify.notified() => {
                let mut inner = self.inner.lock().await;
                if let Some(job_id) = inner.queue.pop_front() {
                    if let Some(job) = inner.jobs.get_mut(&job_id) {
                        job.status = JobStatus::Running;
                        job.started_at = Some(time::OffsetDateTime::now_utc());
                        return Ok(Some(job.clone()));
                    }
                }
                Ok(None)
            }
        }
    }

    async fn get_job(&self, job_id: Uuid) -> AppResult<Option<TestJob>> {
        let inner = self.inner.lock().await;
        Ok(inner.jobs.get(&job_id).cloned())
    }

    async fn update_status(&self, job_id: Uuid, status: JobStatus) -> AppResult<()> {
        let mut inner = self.inner.lock().await;
        let job = inner
            .jobs
            .get_mut(&job_id)
            .ok_or_else(|| AppError::NotFound("Job".to_string()))?;
        job.status = status;
        if status.is_terminal() {
            job.completed_at = Some(time::OffsetDateTime::now_utc());
        }
        Ok(())
    }

    async fn complete_job(&self, job_id: Uuid, result: JobResult) -> AppResult<()> {
        let mut inner = self.inner.lock().await;
        let job = inner
            .jobs
            .get_mut(&job_id)
            .ok_or_else(|| AppError::NotFound("Job".to_string()))?;
        job.status = JobStatus::Completed;
        job.completed_at = Some(time::OffsetDateTime::now_utc());
        job.report_id = Some(result.report_id);
        Ok(())
    }

    async fn fail_job(&self, job_id: Uuid, error: String, retryable: bool) -> AppResult<()> {
        let mut inner = self.inner.lock().await;
        let job = inner
            .jobs
            .get_mut(&job_id)
            .ok_or_else(|| AppError::NotFound("Job".to_string()))?;

        job.error_message = Some(error);

        if retryable && job.retry_count < job.max_retries {
            job.retry_count += 1;
            job.status = JobStatus::Failed;
        } else {
            job.status = JobStatus::Dead;
            job.completed_at = Some(time::OffsetDateTime::now_utc());
        }
        Ok(())
    }

    async fn queue_length(&self) -> AppResult<u64> {
        let inner = self.inner.lock().await;
        Ok(inner.queue.len() as u64)
    }

    async fn list_jobs_by_user(&self, user_id: Uuid, limit: u64) -> AppResult<Vec<TestJob>> {
        let inner = self.inner.lock().await;
        let mut jobs: Vec<TestJob> = inner
            .jobs
            .values()
            .filter(|j| j.user_id == user_id)
            .cloned()
            .collect();

        // Sort by created_at descending
        jobs.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(jobs.into_iter().take(limit as usize).collect())
    }

    async fn requeue(&self, job_id: Uuid) -> AppResult<()> {
        let mut inner = self.inner.lock().await;
        let job = inner
            .jobs
            .get_mut(&job_id)
            .ok_or_else(|| AppError::NotFound("Job".to_string()))?;

        if job.status != JobStatus::Failed {
            return Err(AppError::Validation(
                "Only failed jobs can be requeued".to_string(),
            ));
        }

        job.status = JobStatus::Pending;
        job.started_at = None;
        job.error_message = None;
        inner.queue.push_back(job_id);
        drop(inner);
        self.notify.notify_one();
        Ok(())
    }

    async fn delete_job(&self, job_id: Uuid) -> AppResult<()> {
        let mut inner = self.inner.lock().await;
        inner
            .jobs
            .remove(&job_id)
            .ok_or_else(|| AppError::NotFound("Job".to_string()))?;
        Ok(())
    }

    async fn cancel_job(&self, job_id: Uuid) -> AppResult<()> {
        let mut inner = self.inner.lock().await;
        let job = inner
            .jobs
            .get_mut(&job_id)
            .ok_or_else(|| AppError::NotFound("Job".to_string()))?;

        if job.status.is_terminal() {
            return Err(AppError::Validation(
                "Cannot cancel a completed job".to_string(),
            ));
        }

        job.status = JobStatus::Cancelled;
        job.completed_at = Some(time::OffsetDateTime::now_utc());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::{TestJobConfig, TestJobType};

    #[tokio::test]
    async fn test_enqueue_dequeue() {
        let queue = InMemoryQueue::new();

        let job = TestJob::new(
            TestJobType::Scenario,
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            TestJobConfig::default(),
        );
        let job_id = job.id;

        queue.enqueue(job).await.unwrap();

        let dequeued = queue.dequeue(1).await.unwrap().unwrap();
        assert_eq!(dequeued.id, job_id);
        assert_eq!(dequeued.status, JobStatus::Running);
    }

    #[tokio::test]
    async fn test_job_completion() {
        let queue = InMemoryQueue::new();

        let job = TestJob::new(
            TestJobType::Scenario,
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            TestJobConfig::default(),
        );
        let job_id = job.id;

        queue.enqueue(job).await.unwrap();
        let _ = queue.dequeue(1).await.unwrap();

        let result = JobResult {
            report_id: Uuid::new_v4(),
            total_tests: 5,
            passed: 4,
            failed: 1,
            pass_rate: 80.0,
            total_duration_ms: 1000,
        };

        queue.complete_job(job_id, result.clone()).await.unwrap();

        let completed = queue.get_job(job_id).await.unwrap().unwrap();
        assert_eq!(completed.status, JobStatus::Completed);
        assert_eq!(completed.report_id, Some(result.report_id));
    }

    #[tokio::test]
    async fn test_retry_logic() {
        let queue = InMemoryQueue::new();

        let mut job = TestJob::new(
            TestJobType::Scenario,
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            TestJobConfig::default(),
        );
        job.max_retries = 2;
        let job_id = job.id;

        queue.enqueue(job).await.unwrap();
        let _ = queue.dequeue(1).await.unwrap();

        // First failure - should be Failed (retryable)
        queue
            .fail_job(job_id, "Error 1".to_string(), true)
            .await
            .unwrap();
        let job = queue.get_job(job_id).await.unwrap().unwrap();
        assert_eq!(job.status, JobStatus::Failed);
        assert_eq!(job.retry_count, 1);

        // Requeue
        queue.requeue(job_id).await.unwrap();
        let _ = queue.dequeue(1).await.unwrap();

        // Second failure
        queue
            .fail_job(job_id, "Error 2".to_string(), true)
            .await
            .unwrap();
        let job = queue.get_job(job_id).await.unwrap().unwrap();
        assert_eq!(job.retry_count, 2);

        // Requeue again
        queue.requeue(job_id).await.unwrap();
        let _ = queue.dequeue(1).await.unwrap();

        // Third failure - should be Dead (max_retries = 2, exceeded)
        queue
            .fail_job(job_id, "Error 3".to_string(), true)
            .await
            .unwrap();
        let job = queue.get_job(job_id).await.unwrap().unwrap();
        assert_eq!(job.status, JobStatus::Dead);
    }

    #[tokio::test]
    async fn test_cancel_job() {
        let queue = InMemoryQueue::new();

        let job = TestJob::new(
            TestJobType::Api,
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            TestJobConfig::default(),
        );
        let job_id = job.id;

        queue.enqueue(job).await.unwrap();

        // Cancel pending job
        queue.cancel_job(job_id).await.unwrap();
        let job = queue.get_job(job_id).await.unwrap().unwrap();
        assert_eq!(job.status, JobStatus::Cancelled);

        // Cannot cancel again
        let result = queue.cancel_job(job_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_jobs_by_user() {
        let queue = InMemoryQueue::new();
        let user_id = Uuid::new_v4();

        // Create multiple jobs for the same user
        for _ in 0..5 {
            let job = TestJob::new(
                TestJobType::Scenario,
                Uuid::new_v4(),
                Uuid::new_v4(),
                user_id,
                TestJobConfig::default(),
            );
            queue.enqueue(job).await.unwrap();
        }

        // Create a job for another user
        let other_job = TestJob::new(
            TestJobType::Scenario,
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            TestJobConfig::default(),
        );
        queue.enqueue(other_job).await.unwrap();

        let jobs = queue.list_jobs_by_user(user_id, 10).await.unwrap();
        assert_eq!(jobs.len(), 5);
    }

    #[tokio::test]
    async fn test_queue_length() {
        let queue = InMemoryQueue::new();

        assert_eq!(queue.queue_length().await.unwrap(), 0);

        for _ in 0..3 {
            let job = TestJob::new(
                TestJobType::Collection,
                Uuid::new_v4(),
                Uuid::new_v4(),
                Uuid::new_v4(),
                TestJobConfig::default(),
            );
            queue.enqueue(job).await.unwrap();
        }

        assert_eq!(queue.queue_length().await.unwrap(), 3);

        // Dequeue one
        let _ = queue.dequeue(1).await.unwrap();
        assert_eq!(queue.queue_length().await.unwrap(), 2);
    }
}
