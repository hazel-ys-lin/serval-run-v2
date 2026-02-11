use serde::Deserialize;
use utoipa::IntoParams;

use crate::error::{AppError, AppResult};

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
