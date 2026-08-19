use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    response::{IntoResponse, Response},
};

use crate::{
    adapters::inbound::http::auth::RequestAuthContext, application::resource::ResourceService,
};

use super::{
    ConnectionDeleteRequest, ConnectionDeleteResponse, ConnectionPageRequest,
    ConnectionPageResponse, DataSourceCreateRequest, DataSourceUpdateRequest,
    ResourceCommitOutcomeResponse, ResourceHttpApiResponse, ServerAccountCreateRequest,
    ServerCreateRequest, ServerCreateResponse, error_mapper::resource_error_response,
    handler::json_body,
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
    let command = match request.try_into() {
        Ok(command) => command,
        Err(error) => return resource_error_response(error, &locale),
    };
    match resources
        .create_data_source_for_http(&auth.user().user_code, command)
        .await
    {
        Ok(result) => Json(ResourceHttpApiResponse::success(
            ResourceCommitOutcomeResponse::from(result),
        ))
        .into_response(),
        Err(error) => resource_error_response(error, &locale),
    }
}

pub async fn update_data_source(
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
    request: Result<Json<DataSourceUpdateRequest>, JsonRejection>,
) -> Response {
    let locale = auth.locale.clone();
    let request = match json_body(request, &locale) {
        Ok(request) => request,
        Err(response) => return response,
    };
    match resources
        .update_data_source_for_http(&auth.user().user_code, request.into())
        .await
    {
        Ok(result) => Json(ResourceHttpApiResponse::success(
            ResourceCommitOutcomeResponse::from(result),
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
            ResourceCommitOutcomeResponse::from(result),
        ))
        .into_response(),
        Err(error) => resource_error_response(error, &locale),
    }
}

pub async fn delete_connection(
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
    request: Result<Json<ConnectionDeleteRequest>, JsonRejection>,
) -> Response {
    let locale = auth.locale.clone();
    let request = match json_body(request, &locale) {
        Ok(request) => request,
        Err(response) => return response,
    };
    let command: crate::application::resource::ResourceDeleteCommand = request.into();
    match resources.delete_connection(&auth.user().user_code, command) {
        Ok(result) => Json(ResourceHttpApiResponse::success(
            ConnectionDeleteResponse::from(result),
        ))
        .into_response(),
        Err(error) => resource_error_response(error, &locale),
    }
}

pub async fn page_connections(
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
    request: Result<Json<ConnectionPageRequest>, JsonRejection>,
) -> Response {
    let locale = auth.locale.clone();
    let request = match json_body(request, &locale) {
        Ok(request) => request,
        Err(response) => return response,
    };
    let query = match request.try_into() {
        Ok(query) => query,
        Err(error) => return resource_error_response(error, &locale),
    };
    match resources.query_connections(&auth.user().user_code, query) {
        Ok(page) => Json(ResourceHttpApiResponse::success(
            ConnectionPageResponse::from(page),
        ))
        .into_response(),
        Err(error) => resource_error_response(error, &locale),
    }
}
