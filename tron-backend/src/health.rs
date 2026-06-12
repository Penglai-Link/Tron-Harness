use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::{
    app::{AppReadiness, AppReadinessStatus},
    config::AppConfig,
    response::ApiResponse,
};

#[derive(Debug, Serialize)]
pub(crate) struct HealthData {
    status: &'static str,
    service: &'static str,
    version: &'static str,
}

#[derive(Debug, Serialize)]
pub(crate) struct ReadinessComponents {
    storage: &'static str,
    graph: &'static str,
    rag: &'static str,
}

#[derive(Debug, Serialize)]
pub(crate) struct ReadinessData {
    status: &'static str,
    service: &'static str,
    version: &'static str,
    components: ReadinessComponents,
}

pub(crate) async fn health(State(_config): State<AppConfig>) -> Json<ApiResponse<HealthData>> {
    Json(ApiResponse::success(HealthData {
        status: "ok",
        service: "tron-backend",
        version: env!("CARGO_PKG_VERSION"),
    }))
}

pub(crate) async fn readiness(State(readiness): State<AppReadiness>) -> Response {
    let readiness = readiness_payload(readiness.check());
    if readiness.status == "ready" {
        Json(ApiResponse::success(readiness)).into_response()
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse {
                code: "DEPENDENCY_NOT_READY",
                message: readiness_failure_message(&readiness.components).into(),
                data: readiness,
            }),
        )
            .into_response()
    }
}

fn readiness_payload(status: AppReadinessStatus) -> ReadinessData {
    ReadinessData {
        status: if status.storage_ready && status.graph_ready && status.rag_ready {
            "ready"
        } else {
            "not_ready"
        },
        service: "tron-backend",
        version: env!("CARGO_PKG_VERSION"),
        components: ReadinessComponents {
            storage: component_status(status.storage_ready),
            graph: component_status(status.graph_ready),
            rag: component_status(status.rag_ready),
        },
    }
}

fn component_status(ready: bool) -> &'static str {
    if ready { "up" } else { "down" }
}

fn readiness_failure_message(components: &ReadinessComponents) -> &'static str {
    match (components.storage, components.graph, components.rag) {
        ("down", "down", _) => "storage and graph dependencies not ready",
        ("down", _, _) => "storage dependency not ready",
        (_, "down", _) => "graph dependency not ready",
        (_, _, "down") => "RAG dependency not ready",
        _ => "dependency not ready",
    }
}

#[cfg(test)]
mod tests {
    use axum::{body::to_bytes, extract::State};
    use serde_json::Value;

    use super::*;

    #[tokio::test]
    async fn readiness_returns_service_unavailable_when_graph_dependency_is_not_ready() {
        let response = readiness(State(AppReadiness::new(
            ":memory:",
            "/dev/null/tron.graph.grafeo",
        )))
        .await;

        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body should be readable");
        let json: Value = serde_json::from_slice(&body).expect("response should be valid JSON");

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(json["code"], "DEPENDENCY_NOT_READY");
        assert_eq!(json["data"]["status"], "not_ready");
        assert_eq!(json["data"]["components"]["storage"], "up");
        assert_eq!(json["data"]["components"]["graph"], "down");
        assert_eq!(json["data"]["components"]["rag"], "down");
    }
}
