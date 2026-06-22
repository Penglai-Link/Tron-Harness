use axum::{
    Json, Router,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Serialize;

use crate::{
    adapters::inbound::http::auth::RequestAuthContext, app::AppState, response::ApiResponse,
};

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/v1/dashboard/overview", get(overview))
}

async fn overview(_auth: RequestAuthContext) -> Response {
    Json(ApiResponse::success(DashboardOverviewDto::default())).into_response()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DashboardOverviewDto {
    metrics: Vec<DashboardMetricDto>,
    activities: Vec<DashboardActivityDto>,
}

impl Default for DashboardOverviewDto {
    fn default() -> Self {
        Self {
            metrics: vec![
                metric("System Health", "Ready", "Backend readiness is healthy"),
                metric("Active Agents", "0", "No active agent runs reported"),
                metric("Running Tasks", "0", "No running tasks reported"),
                metric("Models", "0", "Configure workspace models as needed"),
                metric("MCP Servers", "0", "No active MCP servers reported"),
                metric("Skills", "0", "Workspace skill bindings are available"),
                metric("Apps", "0", "No app activity reported"),
                metric("Memory Items", "0", "No memory activity reported"),
            ],
            activities: vec![DashboardActivityDto {
                title: "API mode connected".into(),
                meta: "Dashboard overview served by backend".into(),
                tone: "success".into(),
            }],
        }
    }
}

#[derive(Debug, Serialize)]
struct DashboardMetricDto {
    label: String,
    value: String,
    trend: String,
}

#[derive(Debug, Serialize)]
struct DashboardActivityDto {
    title: String,
    meta: String,
    tone: String,
}

fn metric(label: &str, value: &str, trend: &str) -> DashboardMetricDto {
    DashboardMetricDto {
        label: label.into(),
        value: value.into(),
        trend: trend.into(),
    }
}
