use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Response},
};

use crate::application::prompt::PromptService;

use super::{
    error_mapper::prompt_error_response,
    request::{AnalyzeTemplateRequest, PromptJson, TranslateDraftRequest},
    response::PromptApiResponse,
};

pub async fn analyze_template(
    State(prompts): State<PromptService>,
    PromptJson(request): PromptJson<AnalyzeTemplateRequest>,
) -> Response {
    prompt_response(prompts.analyze_prompt_template(&request.prompt_body))
}

pub async fn translate_draft(
    State(prompts): State<PromptService>,
    PromptJson(request): PromptJson<TranslateDraftRequest>,
) -> Response {
    prompt_response(
        prompts
            .translate_prompt_draft(
                request.prompt_code.as_deref(),
                &request.source_locale,
                &request.target_locale,
                &request.display_name,
                &request.prompt_body,
                request.description.as_deref(),
            )
            .await,
    )
}

fn prompt_response<T: serde::Serialize>(
    result: Result<T, crate::domain::prompt::PromptError>,
) -> Response {
    match result {
        Ok(data) => Json(PromptApiResponse::success(data)).into_response(),
        Err(error) => prompt_error_response(error),
    }
}
