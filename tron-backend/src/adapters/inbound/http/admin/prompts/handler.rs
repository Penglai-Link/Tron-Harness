use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Response},
};

use crate::{
    adapters::inbound::http::prompts::{
        error_mapper::prompt_error_response,
        request::{
            PlatformCreateRequest, PlatformDeleteRequest, PlatformDetailRequest,
            PlatformDuplicateRequest, PlatformPageRequest, PlatformSetEnabledRequest,
            PlatformUpdateRequest, PromptJson,
        },
        response::PromptApiResponse,
    },
    application::prompt::PromptService,
};

pub async fn platform_page(
    State(prompts): State<PromptService>,
    PromptJson(request): PromptJson<PlatformPageRequest>,
) -> Response {
    prompt_response(prompts.platform_page(request.into()))
}

pub async fn platform_detail(
    State(prompts): State<PromptService>,
    PromptJson(request): PromptJson<PlatformDetailRequest>,
) -> Response {
    prompt_response(prompts.platform_detail(&request.prompt_code))
}

pub async fn platform_create(
    State(prompts): State<PromptService>,
    PromptJson(request): PromptJson<PlatformCreateRequest>,
) -> Response {
    prompt_response(prompts.create_platform_prompt(request.into()))
}

pub async fn platform_update(
    State(prompts): State<PromptService>,
    PromptJson(request): PromptJson<PlatformUpdateRequest>,
) -> Response {
    prompt_response(prompts.update_platform_prompt(request.into()))
}

pub async fn platform_duplicate(
    State(prompts): State<PromptService>,
    PromptJson(request): PromptJson<PlatformDuplicateRequest>,
) -> Response {
    prompt_response(
        prompts.duplicate_platform_prompt(&request.prompt_code, request.target_category),
    )
}

pub async fn platform_delete(
    State(prompts): State<PromptService>,
    PromptJson(request): PromptJson<PlatformDeleteRequest>,
) -> Response {
    prompt_response(prompts.delete_platform_prompt(&request.prompt_code))
}

pub async fn platform_set_enabled(
    State(prompts): State<PromptService>,
    PromptJson(request): PromptJson<PlatformSetEnabledRequest>,
) -> Response {
    prompt_response(prompts.set_platform_enabled(
        &request.prompt_code,
        request.enabled,
        request.expected_version,
    ))
}

fn prompt_response<T: serde::Serialize>(
    result: Result<T, crate::domain::prompt::PromptError>,
) -> Response {
    match result {
        Ok(data) => Json(PromptApiResponse::success(data)).into_response(),
        Err(error) => prompt_error_response(error),
    }
}
