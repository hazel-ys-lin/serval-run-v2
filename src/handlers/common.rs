use serde::Deserialize;
use utoipa::IntoParams;

#[derive(Debug, Deserialize, IntoParams)]
pub struct PaginationParams {
    #[param(default = 20, minimum = 1, maximum = 100)]
    pub limit: Option<i64>,
    #[param(default = 0, minimum = 0)]
    pub offset: Option<i64>,
}
