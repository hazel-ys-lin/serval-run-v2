use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Response {
    pub id: Uuid,
    pub report_id: Uuid,
    pub api_id: Uuid,
    pub scenario_id: Uuid,
    pub example_index: i32,

    // Actual response data
    pub response_data: Option<serde_json::Value>,
    pub response_status: i16,

    // Test result
    pub pass: bool,
    pub error_message: Option<String>,

    // Timing
    pub request_time: OffsetDateTime,
    pub request_duration_ms: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct CreateResponse {
    pub api_id: Uuid,
    pub scenario_id: Uuid,
    pub example_index: i32,
    pub response_data: Option<serde_json::Value>,
    pub response_status: i16,
    pub pass: bool,
    pub error_message: Option<String>,
    pub request_duration_ms: Option<i32>,
}
