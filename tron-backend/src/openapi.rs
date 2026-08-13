use axum::{
    Router,
    http::{StatusCode, header::CONTENT_TYPE},
    routing::get,
};
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

use crate::{
    app::AppState,
    health::{__path_health, __path_readiness, HealthData, ReadinessComponents, ReadinessData},
    response::ApiResponse,
};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Tron API",
        version = env!("CARGO_PKG_VERSION"),
        description = "Tron backend HTTP API. Coverage is added progressively by bounded context."
    ),
    paths(health, readiness),
    components(schemas(
        ApiResponse<HealthData>,
        ApiResponse<ReadinessData>,
        HealthData,
        ReadinessComponents,
        ReadinessData
    )),
    tags((name = "System", description = "Service health and readiness"))
)]
struct ApiDoc;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/openapi.yaml", get(openapi_yaml))
        .merge(Scalar::with_url("/api/docs", ApiDoc::openapi()))
}

pub fn document() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}

async fn openapi_yaml() -> Result<([(axum::http::HeaderName, &'static str); 1], String), StatusCode>
{
    document()
        .to_yaml()
        .map(|yaml| ([(CONTENT_TYPE, "application/yaml; charset=utf-8")], yaml))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
