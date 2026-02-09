use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use time::OffsetDateTime;
use uuid::Uuid;

/// Job status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    /// Job is waiting in queue
    Pending,
    /// Job is currently being processed
    Running,
    /// Job completed successfully
    Completed,
    /// Job failed (may be retried)
    Failed,
    /// Job failed permanently (max retries exceeded)
    Dead,
    /// Job was cancelled by user
    Cancelled,
}

impl JobStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Dead | Self::Cancelled)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Dead => "dead",
            Self::Cancelled => "cancelled",
        }
    }
}

/// Test job type (what level of tests to run)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestJobType {
    Scenario,
    Api,
    Collection,
}

impl TestJobType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Scenario => "scenario",
            Self::Api => "api",
            Self::Collection => "collection",
        }
    }
}

/// Test configuration for a job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestJobConfig {
    /// Timeout in seconds for each request
    pub timeout_seconds: u64,
    /// Auth token for authenticated APIs
    pub auth_token: Option<String>,
    /// Custom headers
    pub custom_headers: HashMap<String, String>,
}

impl Default for TestJobConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 30,
            auth_token: None,
            custom_headers: HashMap::new(),
        }
    }
}

/// Test job submitted to the queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestJob {
    /// Unique job identifier
    pub id: Uuid,

    /// Job type/level (scenario, api, collection)
    pub job_type: TestJobType,

    /// Target entity ID (scenario_id, api_id, or collection_id)
    pub target_id: Uuid,

    /// Environment to run tests against
    pub environment_id: Uuid,

    /// User who created the job
    pub user_id: Uuid,

    /// Current status
    pub status: JobStatus,

    /// Test configuration
    pub config: TestJobConfig,

    /// Retry information
    pub retry_count: u32,
    pub max_retries: u32,

    /// Timestamps
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub started_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub completed_at: Option<OffsetDateTime>,

    /// Error message if failed
    pub error_message: Option<String>,

    /// Result reference (report_id when completed)
    pub report_id: Option<Uuid>,
}

impl TestJob {
    pub fn new(
        job_type: TestJobType,
        target_id: Uuid,
        environment_id: Uuid,
        user_id: Uuid,
        config: TestJobConfig,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            job_type,
            target_id,
            environment_id,
            user_id,
            status: JobStatus::Pending,
            config,
            retry_count: 0,
            max_retries: 3,
            created_at: OffsetDateTime::now_utc(),
            started_at: None,
            completed_at: None,
            error_message: None,
            report_id: None,
        }
    }

    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }
}

/// Result of job execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    /// Associated report ID (stored in PostgreSQL)
    pub report_id: Uuid,
    /// Summary statistics
    pub total_tests: usize,
    pub passed: usize,
    pub failed: usize,
    pub pass_rate: f64,
    pub total_duration_ms: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_status_is_terminal() {
        assert!(!JobStatus::Pending.is_terminal());
        assert!(!JobStatus::Running.is_terminal());
        assert!(JobStatus::Completed.is_terminal());
        assert!(!JobStatus::Failed.is_terminal());
        assert!(JobStatus::Dead.is_terminal());
        assert!(JobStatus::Cancelled.is_terminal());
    }

    #[test]
    fn test_create_job() {
        let job = TestJob::new(
            TestJobType::Scenario,
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            TestJobConfig::default(),
        );

        assert_eq!(job.status, JobStatus::Pending);
        assert_eq!(job.retry_count, 0);
        assert_eq!(job.max_retries, 3);
        assert!(job.started_at.is_none());
        assert!(job.completed_at.is_none());
    }

    #[test]
    fn test_job_serialization() {
        let job = TestJob::new(
            TestJobType::Api,
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            TestJobConfig {
                timeout_seconds: 60,
                auth_token: Some("token123".to_string()),
                custom_headers: HashMap::new(),
            },
        );

        let json = serde_json::to_string(&job).unwrap();
        let deserialized: TestJob = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, job.id);
        assert_eq!(deserialized.job_type, TestJobType::Api);
        assert_eq!(deserialized.config.timeout_seconds, 60);
    }
}
