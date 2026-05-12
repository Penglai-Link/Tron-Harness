use axum::{Json, extract::State};
use serde::Serialize;

use crate::{config::AppConfig, response::ApiResponse};

#[derive(Debug, Serialize)]
pub(crate) struct HealthData {
    status: &'static str,
    service: &'static str,
    version: &'static str,
}

pub(crate) async fn health(State(_config): State<AppConfig>) -> Json<ApiResponse<HealthData>> {
    Json(ApiResponse::success(HealthData {
        status: "ok",
        service: "tron-backend",
        version: env!("CARGO_PKG_VERSION"),
    }))
}
