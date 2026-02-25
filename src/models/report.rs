use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Report {
    pub id: Uuid,
    pub project_id: Uuid,
    pub environment_id: Uuid,
    pub collection_id: Option<Uuid>, // nullable for project-level reports

    // Report metadata
    /// Report scope level: 0 = scenario, 1 = api, 2 = collection.
    /// Note: migration 007 comment is outdated — actual values are defined here.
    pub report_level: i16,
    pub report_type: Option<String>,

    // Status and results
    pub finished: bool,
    pub calculated: bool,
    pub pass_rate: Option<Decimal>, // percentage (0.00 - 100.00)
    pub response_count: i32,

    pub created_at: OffsetDateTime,
    pub finished_at: Option<OffsetDateTime>,
}

#[derive(Debug, Deserialize)]
pub struct CreateReport {
    pub environment_id: Uuid,
    pub collection_id: Option<Uuid>,
    pub report_level: i16,
    pub report_type: Option<String>,
}

/// Report summary for list view
#[derive(Debug, Serialize)]
pub struct ReportSummary {
    pub id: Uuid,
    pub project_id: Uuid,
    pub environment_id: Uuid,
    pub collection_id: Option<Uuid>,
    pub report_level: i16,
    pub finished: bool,
    pub pass_rate: Option<Decimal>,
    pub response_count: i32,
    pub created_at: OffsetDateTime,
    pub finished_at: Option<OffsetDateTime>,
}

impl From<Report> for ReportSummary {
    fn from(report: Report) -> Self {
        Self {
            id: report.id,
            project_id: report.project_id,
            environment_id: report.environment_id,
            collection_id: report.collection_id,
            report_level: report.report_level,
            finished: report.finished,
            pass_rate: report.pass_rate,
            response_count: report.response_count,
            created_at: report.created_at,
            finished_at: report.finished_at,
        }
    }
}
