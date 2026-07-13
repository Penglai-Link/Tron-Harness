use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Response},
};

use crate::application::prompt::PromptService;

use super::{
    error_mapper::prompt_error_response,
    request::{
        AnalyzeTemplateRequest, PromptJson, WorkspaceCreateRequest, WorkspaceDeleteRequest,
        WorkspaceDetailRequest, WorkspaceDuplicateRequest, WorkspaceExportRequest,
        WorkspacePageRequest, WorkspaceRevertRequest, WorkspaceSetAllowInheritanceRequest,
        WorkspaceSetEnabledRequest, WorkspaceUpdateRequest,
    },
    response::PromptApiResponse,
};

pub async fn workspace_page(
    State(prompts): State<PromptService>,
    PromptJson(request): PromptJson<WorkspacePageRequest>,
) -> Response {
    let (workspace_code, query) = request.into_parts();
    prompt_response(prompts.workspace_page(&workspace_code, query))
}

pub async fn workspace_detail(
    State(prompts): State<PromptService>,
    PromptJson(request): PromptJson<WorkspaceDetailRequest>,
) -> Response {
    prompt_response(prompts.workspace_detail(&request.workspace_code, &request.prompt_code))
}

pub async fn workspace_create(
    State(prompts): State<PromptService>,
    PromptJson(request): PromptJson<WorkspaceCreateRequest>,
) -> Response {
    let (workspace_code, command) = request.into_parts();
    prompt_response(prompts.create_workspace_prompt(&workspace_code, command))
}

pub async fn workspace_update(
    State(prompts): State<PromptService>,
    PromptJson(request): PromptJson<WorkspaceUpdateRequest>,
) -> Response {
    let (workspace_code, command) = request.into_parts();
    prompt_response(prompts.update_workspace_prompt(&workspace_code, command))
}

pub async fn workspace_duplicate(
    State(prompts): State<PromptService>,
    PromptJson(request): PromptJson<WorkspaceDuplicateRequest>,
) -> Response {
    prompt_response(prompts.duplicate_workspace_prompt(
        &request.workspace_code,
        &request.prompt_code,
        request.target_category,
    ))
}

pub async fn workspace_delete(
    State(prompts): State<PromptService>,
    PromptJson(request): PromptJson<WorkspaceDeleteRequest>,
) -> Response {
    prompt_response(prompts.delete_workspace_prompt(&request.workspace_code, &request.prompt_code))
}

pub async fn workspace_revert(
    State(prompts): State<PromptService>,
    PromptJson(request): PromptJson<WorkspaceRevertRequest>,
) -> Response {
    prompt_response(prompts.revert_workspace_prompt(&request.workspace_code, &request.prompt_code))
}

pub async fn workspace_set_enabled(
    State(prompts): State<PromptService>,
    PromptJson(request): PromptJson<WorkspaceSetEnabledRequest>,
) -> Response {
    prompt_response(prompts.set_workspace_enabled(
        &request.workspace_code,
        &request.prompt_code,
        request.enabled,
        request.expected_version,
    ))
}

pub async fn workspace_set_allow_inheritance(
    State(prompts): State<PromptService>,
    PromptJson(request): PromptJson<WorkspaceSetAllowInheritanceRequest>,
) -> Response {
    prompt_response(prompts.set_workspace_allow_inheritance(
        &request.workspace_code,
        &request.prompt_code,
        request.allow_inheritance,
        request.expected_version,
    ))
}

pub async fn analyze_template(
    State(prompts): State<PromptService>,
    PromptJson(request): PromptJson<AnalyzeTemplateRequest>,
) -> Response {
    prompt_response(prompts.analyze_prompt_template(&request.prompt_body))
}

pub async fn workspace_export(
    State(prompts): State<PromptService>,
    PromptJson(request): PromptJson<WorkspaceExportRequest>,
) -> Response {
    let (workspace_code, query) = request.into_parts();
    prompt_response(prompts.workspace_export(&workspace_code, query))
}

fn prompt_response<T: serde::Serialize>(
    result: Result<T, crate::domain::prompt::PromptError>,
) -> Response {
    match result {
        Ok(data) => Json(PromptApiResponse::success(data)).into_response(),
        Err(error) => prompt_error_response(error),
    }
}
