use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::error::{AppError, AppResult};

/// Generic paginated list response used by all list endpoints
#[derive(Debug, Serialize, ToSchema)]
pub struct ListResponse<T: Serialize + ToSchema> {
    pub data: Vec<T>,
    pub total: u64,
    pub limit: u64,
    pub offset: u64,
}

pub type ProjectListResponse = ListResponse<super::ProjectResponse>;
pub type CollectionListResponse = ListResponse<super::CollectionResponse>;
pub type EnvironmentListResponse = ListResponse<super::EnvironmentResponse>;
pub type ApiListResponse = ListResponse<super::ApiResponse>;
pub type ScenarioListResponse = ListResponse<super::ScenarioResponse>;
pub type ReportListResponse = ListResponse<super::ReportResponse>;
pub type JobListResponse = ListResponse<super::JobStatusResponse>;

#[derive(Debug, Deserialize, IntoParams)]
pub struct PaginationParams {
    #[param(default = 20, minimum = 1, maximum = 100)]
    pub limit: Option<i64>,
    #[param(default = 0, minimum = 0)]
    pub offset: Option<i64>,
}

/// Validate that a required string field is non-empty and within max length.
pub fn validate_required(value: &str, field: &str, max_len: usize) -> AppResult<()> {
    if value.is_empty() {
        return Err(AppError::Validation(format!("{field} is required")));
    }
    if value.len() > max_len {
        return Err(AppError::Validation(format!(
            "{field} must be at most {max_len} characters"
        )));
    }
    Ok(())
}

/// Validate an optional string field within max length (if present).
pub fn validate_optional(value: &Option<String>, field: &str, max_len: usize) -> AppResult<()> {
    if let Some(v) = value {
        if v.len() > max_len {
            return Err(AppError::Validation(format!(
                "{field} must be at most {max_len} characters"
            )));
        }
    }
    Ok(())
}
