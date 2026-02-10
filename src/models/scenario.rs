use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Scenario {
    pub id: Uuid,
    pub api_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub steps: serde_json::Value, // JSONB: [{keyword, keywordType, text}]
    pub examples: serde_json::Value, // JSONB: [{example, expected_response_body, expected_status_code}]
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Gherkin step structure with optional doc string and data table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GherkinStep {
    pub keyword: String, // Given, When, Then, And, But
    pub keyword_type: String,
    pub text: String,
    /// Multi-line doc string (for JSON bodies, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_string: Option<String>,
    /// Data table embedded in the step
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_table: Option<Vec<serde_json::Value>>,
}

/// Test example with expected results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestExample {
    pub example: serde_json::Value, // dynamic test data
    pub expected_response_body: serde_json::Value,
    pub expected_status_code: i16,
}

#[derive(Debug, Deserialize)]
pub struct CreateScenario {
    pub title: String,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub steps: Vec<GherkinStep>,
    pub examples: Vec<TestExample>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateScenario {
    pub title: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub steps: Option<Vec<GherkinStep>>,
    pub examples: Option<Vec<TestExample>>,
}
