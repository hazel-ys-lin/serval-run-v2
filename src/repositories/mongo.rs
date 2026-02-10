use bson::doc;
use mongodb::Database;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

/// MongoDB repository for document storage
pub struct MongoRepository;

/// Gherkin document stored in MongoDB
#[derive(Debug, Serialize, Deserialize)]
pub struct GherkinDocument {
    pub scenario_id: String,
    pub api_id: String,
    pub raw_gherkin: String,
    pub parsed_feature: serde_json::Value,
    pub created_at: bson::DateTime,
}

/// Execution log stored in MongoDB
#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionLog {
    pub report_id: String,
    pub api_id: String,
    pub scenario_id: String,
    pub example_index: i32,
    pub response_status: i16,
    pub response_data: Option<serde_json::Value>,
    pub pass: bool,
    pub error_message: Option<String>,
    pub duration_ms: i64,
    pub created_at: bson::DateTime,
}

impl MongoRepository {
    /// Save a raw Gherkin document when creating scenarios from Gherkin
    pub async fn save_gherkin_document(
        db: &Database,
        api_id: Uuid,
        raw_gherkin: &str,
        parsed_feature: &serde_json::Value,
    ) -> AppResult<()> {
        let collection = db.collection::<GherkinDocument>("gherkin_documents");

        let doc = GherkinDocument {
            scenario_id: String::new(), // Will be set per-scenario if needed
            api_id: api_id.to_string(),
            raw_gherkin: raw_gherkin.to_string(),
            parsed_feature: parsed_feature.clone(),
            created_at: bson::DateTime::now(),
        };

        collection
            .insert_one(doc)
            .await
            .map_err(|e| AppError::Database(format!("MongoDB insert error: {}", e)))?;

        Ok(())
    }

    /// Save execution logs for test results
    pub async fn save_execution_logs(
        db: &Database,
        _report_id: Uuid,
        logs: Vec<ExecutionLog>,
    ) -> AppResult<()> {
        if logs.is_empty() {
            return Ok(());
        }

        let collection = db.collection::<ExecutionLog>("execution_logs");

        collection
            .insert_many(logs)
            .await
            .map_err(|e| AppError::Database(format!("MongoDB insert error: {}", e)))?;

        Ok(())
    }
}
