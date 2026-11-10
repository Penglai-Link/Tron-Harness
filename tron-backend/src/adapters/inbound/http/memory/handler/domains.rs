use super::super::{error_mapper::ApiError, request::*, response::MemoryWorkDomainResponse};
use crate::{application::memory::MemoryWorkDomainService, response::ApiResponse};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
pub async fn list_work_domains(
    State(domains): State<MemoryWorkDomainService>,
    headers: HeaderMap,
    Query(query): Query<MemoryWorkDomainHttpQuery>,
) -> Response {
    match domains.list_domains(query) {
        Ok(domains) => Json(ApiResponse::protocol_success(
            domains
                .into_iter()
                .map(MemoryWorkDomainResponse::from)
                .collect::<Vec<_>>(),
        ))
        .into_response(),
        Err(error) => ApiError(
            error,
            crate::shared::locale::SupportedLocale::from_request(
                headers.get("accept-language").and_then(|v| v.to_str().ok()),
            ),
        )
        .into_response(),
    }
}

pub async fn create_work_domain(
    State(domains): State<MemoryWorkDomainService>,
    headers: HeaderMap,
    Json(request): Json<CreateMemoryWorkDomainRequest>,
) -> Response {
    match domains.create_domain(request.into()) {
        Ok(domain) => (
            StatusCode::OK,
            Json(ApiResponse::protocol_success(
                MemoryWorkDomainResponse::from(domain),
            )),
        )
            .into_response(),
        Err(error) => ApiError(
            error,
            crate::shared::locale::SupportedLocale::from_request(
                headers.get("accept-language").and_then(|v| v.to_str().ok()),
            ),
        )
        .into_response(),
    }
}

pub async fn patch_work_domain(
    State(domains): State<MemoryWorkDomainService>,
    headers: HeaderMap,
    Path(domain_code): Path<String>,
    Json(request): Json<UpdateMemoryWorkDomainRequest>,
) -> Response {
    match domains.update_domain(&domain_code, request.into()) {
        Ok(domain) => Json(ApiResponse::protocol_success(
            MemoryWorkDomainResponse::from(domain),
        ))
        .into_response(),
        Err(error) => ApiError(
            error,
            crate::shared::locale::SupportedLocale::from_request(
                headers.get("accept-language").and_then(|v| v.to_str().ok()),
            ),
        )
        .into_response(),
    }
}

pub async fn enable_work_domain(
    State(domains): State<MemoryWorkDomainService>,
    headers: HeaderMap,
    Path(domain_code): Path<String>,
) -> Response {
    match domains.set_domain_status(&domain_code, "ACTIVE") {
        Ok(domain) => Json(ApiResponse::protocol_success(
            MemoryWorkDomainResponse::from(domain),
        ))
        .into_response(),
        Err(error) => ApiError(
            error,
            crate::shared::locale::SupportedLocale::from_request(
                headers.get("accept-language").and_then(|v| v.to_str().ok()),
            ),
        )
        .into_response(),
    }
}

pub async fn disable_work_domain(
    State(domains): State<MemoryWorkDomainService>,
    headers: HeaderMap,
    Path(domain_code): Path<String>,
) -> Response {
    match domains.set_domain_status(&domain_code, "DISABLED") {
        Ok(domain) => Json(ApiResponse::protocol_success(
            MemoryWorkDomainResponse::from(domain),
        ))
        .into_response(),
        Err(error) => ApiError(
            error,
            crate::shared::locale::SupportedLocale::from_request(
                headers.get("accept-language").and_then(|v| v.to_str().ok()),
            ),
        )
        .into_response(),
    }
}
