use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    response::{IntoResponse, Response},
};

use crate::{
    adapters::inbound::http::auth::RequestAuthContext, application::resource::ResourceService,
};

use super::{
    DataSourceCreateRequest, ResourceActivationResponse, ResourceHttpApiResponse,
    ServerAccountCreateRequest, ServerCreateRequest, ServerCreateResponse,
    error_mapper::resource_error_response, handler::json_body,
};

pub async fn create_data_source(
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
    request: Result<Json<DataSourceCreateRequest>, JsonRejection>,
) -> Response {
    let locale = auth.locale.clone();
    let request = match json_body(request, &locale) {
        Ok(request) => request,
        Err(response) => return response,
    };
    match resources
        .create_data_source_for_http(&auth.user().user_code, request.into())
        .await
    {
        Ok(result) => Json(ResourceHttpApiResponse::success(
            ResourceActivationResponse::from(result),
        ))
        .into_response(),
        Err(error) => resource_error_response(error, &locale),
    }
}

pub async fn create_server(
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
    request: Result<Json<ServerCreateRequest>, JsonRejection>,
) -> Response {
    let locale = auth.locale.clone();
    let request = match json_body(request, &locale) {
        Ok(request) => request,
        Err(response) => return response,
    };
    match resources
        .create_server_for_http(&auth.user().user_code, request.into())
        .await
    {
        Ok(result) => Json(ResourceHttpApiResponse::success(
            ServerCreateResponse::from(result),
        ))
        .into_response(),
        Err(error) => resource_error_response(error, &locale),
    }
}

pub async fn create_server_account(
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
    request: Result<Json<ServerAccountCreateRequest>, JsonRejection>,
) -> Response {
    let locale = auth.locale.clone();
    let request = match json_body(request, &locale) {
        Ok(request) => request,
        Err(response) => return response,
    };
    match resources.create_server_account_for_http(&auth.user().user_code, request.into()) {
        Ok(result) => Json(ResourceHttpApiResponse::success(
            ResourceActivationResponse::from(result),
        ))
        .into_response(),
        Err(error) => resource_error_response(error, &locale),
    }
}
