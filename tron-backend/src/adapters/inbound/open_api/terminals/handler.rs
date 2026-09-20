use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    response::{IntoResponse, Response},
};

use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    app::AppState,
    application::{
        llm::LlmRuntimeError,
        terminal::{
            IssueTerminalSocketTicketCommand, normalize_terminal_command,
            terminal_command_runtime_values,
        },
    },
    response::ApiResponse,
};

use super::{
    error_mapper::{
        invalid_terminal_request_response, llm_runtime_error_response,
        prompt_render_error_response, terminal_error_response,
    },
    request::{OpenApiGenerateTerminalCommandRequest, OpenApiIssueTerminalSocketTicketRequest},
    response::{OpenApiGenerateTerminalCommandResponse, OpenApiIssueTerminalSocketTicketResponse},
};

pub async fn generate_command(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    request: Result<Json<OpenApiGenerateTerminalCommandRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match request {
        Ok(request) => request,
        Err(rejection) => {
            return invalid_terminal_request_response(
                rejection,
                "TERMINAL_COMMAND_GENERATE_REQUEST_INVALID",
                "terminal command generation request is invalid",
            );
        }
    };
    if let Err(error) = state
        .terminal
        .authorize_workspace_user(&request.workspace_code, &auth.user().user_code)
    {
        return terminal_error_response(error);
    }
    let runtime_values = terminal_command_runtime_values(&request.prompt);
    let prompt = match state.prompts.runtime_prompt_text(
        Some(&request.workspace_code),
        "terminal.command-exact",
        "en-US",
        &runtime_values,
    ) {
        Ok(Some(prompt)) => prompt,
        Ok(None) | Err(_) => return prompt_render_error_response(),
    };
    let config = match state
        .llm_router
        .route_text(&request.workspace_code, request.model_code.as_deref())
    {
        Ok(config) => config,
        Err(error) => return llm_runtime_error_response(LlmRuntimeError::model_resolution(error)),
    };
    match state
        .llm_runtime
        .text_generation_call_with_config(config, prompt)
        .await
    {
        Ok(result) => Json(ApiResponse::success(
            OpenApiGenerateTerminalCommandResponse {
                model_code: result.model_code,
                command: normalize_terminal_command(result.output),
            },
        ))
        .into_response(),
        Err(error) => llm_runtime_error_response(error),
    }
}

pub async fn issue_socket_ticket(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    request: Result<Json<OpenApiIssueTerminalSocketTicketRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match request {
        Ok(request) => request,
        Err(rejection) => {
            return invalid_terminal_request_response(
                rejection,
                "TERMINAL_SOCKET_TICKET_REQUEST_INVALID",
                "terminal socket ticket request is invalid",
            );
        }
    };
    match state
        .terminal
        .issue_socket_ticket(IssueTerminalSocketTicketCommand {
            workspace_code: request.workspace_code,
            app_code: request.app_code,
            user_code: auth.user().user_code.clone(),
        }) {
        Ok(ticket) => Json(ApiResponse::success(
            OpenApiIssueTerminalSocketTicketResponse::from(ticket),
        ))
        .into_response(),
        Err(error) => terminal_error_response(error),
    }
}
