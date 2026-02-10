use rust_decimal::Decimal;
use uuid::Uuid;

use serval_run::error::AppResult;
use serval_run::models::CreateReport;
use serval_run::queue::TestJobType;
use serval_run::repositories::mongo::{ExecutionLog, MongoRepository};
use serval_run::repositories::ReportRepository;
use serval_run::services::TestResult;
use serval_run::state::AppState;

/// Handles saving test results and creating reports
pub struct ResultHandler;

impl ResultHandler {
    /// Save test results and create a report in PostgreSQL
    pub async fn save_results(
        state: &AppState,
        user_id: Uuid,
        project_id: Uuid,
        job_type: &TestJobType,
        target_id: Uuid,
        environment_id: Uuid,
        results: &[TestResult],
    ) -> AppResult<Uuid> {
        // Calculate summary
        let total_tests = results.len();
        let passed = results.iter().filter(|r| r.pass).count();
        let failed = total_tests - passed;

        // Determine report level and collection_id based on job type
        let (report_level, collection_id) = match job_type {
            TestJobType::Scenario => (0_i16, None), // scenario level
            TestJobType::Api => (1_i16, None),      // api level
            TestJobType::Collection => (1_i16, Some(target_id)), // collection level
        };

        // Create report using PostgreSQL repository
        let create_report = CreateReport {
            environment_id,
            collection_id,
            report_level,
            report_type: Some(job_type.as_str().to_string()),
        };

        let report =
            ReportRepository::create(&state.db, project_id, user_id, &create_report).await?;

        // Save individual test results to PostgreSQL
        Self::save_responses(state, report.id, results).await?;

        // Calculate pass rate
        let pass_rate = if total_tests > 0 {
            Decimal::from(passed as i64) * Decimal::from(100) / Decimal::from(total_tests as i64)
        } else {
            Decimal::ZERO
        };

        // Finish the report with calculated stats
        ReportRepository::finish_report(
            &state.db,
            report.id,
            user_id,
            pass_rate,
            total_tests as i32,
        )
        .await?;

        tracing::info!(
            report_id = %report.id,
            total = total_tests,
            passed = passed,
            failed = failed,
            "Report saved to PostgreSQL"
        );

        Ok(report.id)
    }

    /// Save individual test responses to PostgreSQL
    async fn save_responses(
        state: &AppState,
        report_id: Uuid,
        results: &[TestResult],
    ) -> AppResult<()> {
        use sea_orm::{ActiveModelTrait, Set};
        use serval_run::entity::response::ActiveModel;

        for result in results {
            let response_model = ActiveModel {
                id: Set(Uuid::new_v4()),
                report_id: Set(report_id),
                api_id: Set(result.api_id),
                scenario_id: Set(result.scenario_id),
                example_index: Set(result.example_index),
                response_data: Set(result.response_data.clone()),
                response_status: Set(result.response_status),
                pass: Set(result.pass),
                error_message: Set(result.error_message.clone()),
                request_time: Set(result.request_time),
                request_duration_ms: Set(Some(result.request_duration_ms as i32)),
            };

            response_model.insert(&state.db).await.map_err(|e| {
                serval_run::error::AppError::Database(format!("Failed to save response: {}", e))
            })?;
        }

        // Save execution logs to MongoDB (non-fatal)
        let logs: Vec<ExecutionLog> = results
            .iter()
            .map(|r| ExecutionLog {
                report_id: report_id.to_string(),
                api_id: r.api_id.to_string(),
                scenario_id: r.scenario_id.to_string(),
                example_index: r.example_index,
                response_status: r.response_status,
                response_data: r.response_data.clone(),
                pass: r.pass,
                error_message: r.error_message.clone(),
                duration_ms: r.request_duration_ms,
                created_at: bson::DateTime::now(),
            })
            .collect();

        if let Err(e) =
            MongoRepository::save_execution_logs(&state.mongo_db(), report_id, logs).await
        {
            tracing::warn!(error = %e, "Failed to save execution logs to MongoDB (non-fatal)");
        }

        Ok(())
    }
}
