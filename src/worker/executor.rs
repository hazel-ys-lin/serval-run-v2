use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

use serval_run::error::AppResult;
use serval_run::models::{Api, Environment};
use serval_run::queue::{JobResult, TestJob, TestJobType};
use serval_run::repositories::{ApiRepository, EnvironmentRepository, Repository, ScenarioRepository};
use serval_run::services::{TestConfig, TestResult, TestRunner};
use serval_run::state::AppState;

use super::result_handler::ResultHandler;

/// Job executor that processes test jobs
pub struct JobExecutor {
    state: Arc<AppState>,
}

impl JobExecutor {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    /// Execute a job and return the result
    pub async fn execute(&self, job: TestJob) -> AppResult<JobResult> {
        let start = Instant::now();
        let user_id = job.user_id;

        // Build test config from job config
        let test_config = TestConfig {
            timeout: std::time::Duration::from_secs(job.config.timeout_seconds),
            auth_token: job.config.auth_token.clone(),
            custom_headers: job.config.custom_headers.clone(),
        };

        let test_runner = TestRunner::with_config(test_config);

        // Load environment using Repository trait method
        let environment =
            <EnvironmentRepository as Repository<Environment>>::find_by_id(&self.state.db, job.environment_id)
                .await?;

        // Get project_id from environment
        let project_id = environment.project_id;

        // Execute based on job type
        let results = match job.job_type {
            TestJobType::Scenario => {
                self.execute_scenario(&test_runner, job.target_id, user_id, &environment)
                    .await?
            }
            TestJobType::Api => {
                self.execute_api(&test_runner, job.target_id, user_id, &environment)
                    .await?
            }
            TestJobType::Collection => {
                self.execute_collection(&test_runner, job.target_id, user_id, &environment)
                    .await?
            }
        };

        let total_duration_ms = start.elapsed().as_millis() as i64;

        // Save results and create report (using PostgreSQL)
        let report_id = ResultHandler::save_results(
            &self.state,
            job.user_id,
            project_id,
            &job.job_type,
            job.target_id,
            job.environment_id,
            &results,
        )
        .await?;

        // Calculate summary
        let total_tests = results.len();
        let passed = results.iter().filter(|r| r.pass).count();
        let failed = total_tests - passed;
        let pass_rate = if total_tests > 0 {
            (passed as f64 / total_tests as f64) * 100.0
        } else {
            0.0
        };

        Ok(JobResult {
            report_id,
            total_tests,
            passed,
            failed,
            pass_rate,
            total_duration_ms,
        })
    }

    /// Execute a single scenario test
    async fn execute_scenario(
        &self,
        runner: &TestRunner,
        scenario_id: Uuid,
        user_id: Uuid,
        environment: &Environment,
    ) -> AppResult<Vec<TestResult>> {
        // Use find_by_id_and_user for ownership verification
        let scenario = ScenarioRepository::find_by_id_and_user(&self.state.db, scenario_id, user_id)
            .await?;

        let api = <ApiRepository as Repository<Api>>::find_by_id(&self.state.db, scenario.api_id)
            .await?;

        runner.run_scenario(&scenario, &api, environment).await
    }

    /// Execute all scenarios for an API
    async fn execute_api(
        &self,
        runner: &TestRunner,
        api_id: Uuid,
        user_id: Uuid,
        environment: &Environment,
    ) -> AppResult<Vec<TestResult>> {
        let api = ApiRepository::find_by_id_and_user(&self.state.db, api_id, user_id)
            .await?;

        // Use list_by_api with high limit to get all scenarios
        let scenarios = ScenarioRepository::list_by_api(&self.state.db, api_id, user_id, 1000, 0)
            .await?;

        let mut all_results = Vec::new();
        for scenario in scenarios {
            let results = runner.run_scenario(&scenario, &api, environment).await?;
            all_results.extend(results);
        }

        Ok(all_results)
    }

    /// Execute all scenarios for all APIs in a collection
    async fn execute_collection(
        &self,
        runner: &TestRunner,
        collection_id: Uuid,
        user_id: Uuid,
        environment: &Environment,
    ) -> AppResult<Vec<TestResult>> {
        // Use list_by_collection with high limit to get all APIs
        let apis = ApiRepository::list_by_collection(&self.state.db, collection_id, user_id, 1000, 0)
            .await?;

        let mut all_results = Vec::new();
        for api in &apis {
            let scenarios = ScenarioRepository::list_by_api(&self.state.db, api.id, user_id, 1000, 0)
                .await?;
            for scenario in scenarios {
                let results = runner.run_scenario(&scenario, api, environment).await?;
                all_results.extend(results);
            }
        }

        Ok(all_results)
    }
}
