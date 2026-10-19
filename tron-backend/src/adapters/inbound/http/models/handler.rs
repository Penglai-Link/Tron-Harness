use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    app::AppState,
    application::model::ModelService,
    domain::model::{ModelProtocol, ModelSelectOption, ModelViewSourceKind},
    response::ApiResponse,
};

use super::{
    error_mapper::model_error_response,
    request::{ModelRequest, SetAllowInheritanceRequest},
};

pub async fn list_workspace_models(
    State(models): State<ModelService>,
    Path(workspace_code): Path<String>,
) -> Response {
    match models.list_workspace_models(&workspace_code) {
        Ok(records) => Json(ApiResponse::success(records)).into_response(),
        Err(error) => model_error_response(error),
    }
}

pub async fn list_select_options(
    State(state): State<AppState>,
    Path(workspace_code): Path<String>,
) -> Response {
    match state.models.list_workspace_select_options(&workspace_code) {
        Ok(mut records) => match state
            .model_providers
            .list_text_select_options(&workspace_code)
            .and_then(|mut options| {
                options.extend(
                    state
                        .model_providers
                        .list_auxiliary_route_options(&workspace_code)?,
                );
                Ok(options)
            }) {
            Ok(provider_records) => {
                records.extend(provider_records.into_iter().map(|record| {
                    ModelSelectOption {
                        option_code: record.option_code,
                        model_code: record
                            .selection
                            .model_code
                            .clone()
                            .or_else(|| {
                                record.selection.route_code.map(|route| {
                                    format!("PERSON_AI:ROUTE:{}", route.storage_value())
                                })
                            })
                            .unwrap_or_default(),
                        display_name: record.display_name,
                        model_name: record
                            .selection
                            .model_code
                            .clone()
                            .unwrap_or_else(|| "automatic".into()),
                        source_kind: ModelViewSourceKind::Platform,
                        source_label: record.access_platform.clone(),
                        vendor: record
                            .upstream_vendor_name
                            .clone()
                            .unwrap_or_else(|| record.access_platform.clone()),
                        protocol: match record.model_type {
                            crate::domain::model::ModelType::Embedding => {
                                ModelProtocol::OpenAiCompatibleEmbedding
                            }
                            crate::domain::model::ModelType::Rerank => ModelProtocol::Rerank,
                            _ => ModelProtocol::OpenAiCompatible,
                        },
                        model_type: record.model_type,
                        enabled: record.enabled,
                        model_selection: record.selection,
                        provider_code: Some(record.provider_code),
                        access_platform: Some(record.access_platform),
                        upstream_vendor_name: record.upstream_vendor_name,
                        remote_status: Some(record.remote_status),
                        context_window_tokens: record.context_window_tokens,
                        quality_type: record.quality_type,
                        input_coefficient: record.input_coefficient,
                        cache_hit_coefficient: record.cache_hit_coefficient,
                        output_coefficient: record.output_coefficient,
                        supported_reasoning_efforts: Vec::new(),
                        default_reasoning_effort: None,
                        is_default: record.is_default,
                    }
                }));
                Json(ApiResponse::success(records)).into_response()
            }
            Err(_) => model_error_response(crate::domain::model::ModelError::StorageFailure),
        },
        Err(error) => model_error_response(error),
    }
}

pub async fn restore_workspace_model_from_platform_deprecated() -> Response {
    (
        StatusCode::GONE,
        Json(ApiResponse::error(
            "MODEL_ROUTE_GONE",
            "workspace model restore from platform has been removed",
        )),
    )
        .into_response()
}

pub async fn create_workspace_model(
    State(models): State<ModelService>,
    Path(workspace_code): Path<String>,
    Json(request): Json<ModelRequest>,
) -> Response {
    match models.create_workspace_model(&workspace_code, request.into()) {
        Ok(record) => (StatusCode::CREATED, Json(ApiResponse::success(record))).into_response(),
        Err(error) => model_error_response(error),
    }
}

pub async fn verify_workspace_model_create_draft(
    State(models): State<ModelService>,
    Path(workspace_code): Path<String>,
    Json(request): Json<ModelRequest>,
) -> Response {
    match models
        .verify_workspace_model_create_draft(&workspace_code, request.into())
        .await
    {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => model_error_response(error),
    }
}

pub async fn get_workspace_model(
    State(models): State<ModelService>,
    Path((workspace_code, model_code)): Path<(String, String)>,
) -> Response {
    match models.get_workspace_model(&workspace_code, &model_code) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => model_error_response(error),
    }
}

pub async fn patch_workspace_model(
    State(models): State<ModelService>,
    Path((workspace_code, model_code)): Path<(String, String)>,
    Json(request): Json<ModelRequest>,
) -> Response {
    match models.update_workspace_model(&workspace_code, &model_code, request.into()) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => model_error_response(error),
    }
}

pub async fn verify_workspace_model(
    State(models): State<ModelService>,
    Path((workspace_code, model_code)): Path<(String, String)>,
    request: Option<Json<ModelRequest>>,
) -> Response {
    let result = match request {
        Some(Json(request)) => {
            models
                .verify_workspace_model_draft(&workspace_code, &model_code, request.into())
                .await
        }
        None => {
            models
                .verify_workspace_model(&workspace_code, &model_code)
                .await
        }
    };
    match result {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => model_error_response(error),
    }
}

pub async fn duplicate_workspace_model(
    State(models): State<ModelService>,
    Path((workspace_code, model_code)): Path<(String, String)>,
) -> Response {
    match models.duplicate_workspace_model(&workspace_code, &model_code) {
        Ok(record) => (StatusCode::CREATED, Json(ApiResponse::success(record))).into_response(),
        Err(error) => model_error_response(error),
    }
}

pub async fn enable_workspace_model(
    State(models): State<ModelService>,
    Path((workspace_code, model_code)): Path<(String, String)>,
) -> Response {
    match models.enable_workspace_model(&workspace_code, &model_code) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => model_error_response(error),
    }
}

pub async fn disable_workspace_model(
    State(models): State<ModelService>,
    Path((workspace_code, model_code)): Path<(String, String)>,
) -> Response {
    match models.disable_workspace_model(&workspace_code, &model_code) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => model_error_response(error),
    }
}

pub async fn set_workspace_default(
    State(models): State<ModelService>,
    Path((workspace_code, model_code)): Path<(String, String)>,
) -> Response {
    match models.set_workspace_default(&workspace_code, &model_code) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => model_error_response(error),
    }
}

pub async fn set_allow_inheritance(
    State(models): State<ModelService>,
    Path((workspace_code, model_code)): Path<(String, String)>,
    Json(request): Json<SetAllowInheritanceRequest>,
) -> Response {
    match models.set_workspace_allow_inheritance(&workspace_code, &model_code, request.into()) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => model_error_response(error),
    }
}

pub async fn delete_workspace_model(
    State(models): State<ModelService>,
    Path((workspace_code, model_code)): Path<(String, String)>,
) -> Response {
    match models.delete_workspace_model(&workspace_code, &model_code) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => model_error_response(error),
    }
}
