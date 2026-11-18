use super::super::auth::RequestAuthContext;
use crate::{
    app::AppState,
    application::short_term_memory::{SavedContextAssembly, context_assembly_error_descriptor},
    domain::short_term_memory::ShortTermMemoryError,
    response::{ApiResponse, EmptyData},
    shared::locale::SupportedLocale,
};
use axum::{
    Json,
    extract::rejection::QueryRejection,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tron_i18n::t;
// Rust/FE consensus §§2.1,2.4–2.6: compact camelCase VO, scoped query and localized errors.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct AssemblyQuery {
    session_code: String,
    snapshot_sequence: Option<String>,
    workspace_code: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AssemblyView {
    snapshot: Option<SnapshotView>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotView {
    snapshot_sequence: String,
    recorded_at: u64,
    model_code: String,
    stage: String,
    step: usize,
    attempt: usize,
    input_capacity: usize,
    protocol_tokens: usize,
    estimated_input_tokens: usize,
    budget_breakdown: Value,
    blocks: Vec<BlockView>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BlockView {
    id: String,
    position: usize,
    sources: Vec<String>,
    original_tokens: Option<usize>,
    configured_tokens: Option<usize>,
    allocated_tokens: usize,
    final_tokens: usize,
    message_count: usize,
    outcome: String,
    reason: Option<String>,
}
impl From<SavedContextAssembly> for SnapshotView {
    fn from(value: SavedContextAssembly) -> Self {
        let report = value.report;
        Self {
            snapshot_sequence: value.sequence.to_string(),
            recorded_at: value.recorded_at,
            model_code: report.model.model,
            stage: report.stage,
            step: report.step,
            attempt: report.attempt,
            input_capacity: report.input_capacity,
            protocol_tokens: report.protocol_tokens,
            estimated_input_tokens: report.estimated_input_tokens,
            budget_breakdown: value.budget_breakdown,
            blocks: report
                .blocks
                .into_iter()
                .map(|block| {
                    let (outcome, reason) = match block.outcome {
                        eliteforge_agent_core::ContextBlockOutcome::Kept => ("Kept", None),
                        eliteforge_agent_core::ContextBlockOutcome::Reduced => ("Reduced", None),
                        eliteforge_agent_core::ContextBlockOutcome::NoGain => ("NoGain", None),
                        eliteforge_agent_core::ContextBlockOutcome::Omitted { reason } => {
                            ("Omitted", Some(reason))
                        }
                        eliteforge_agent_core::ContextBlockOutcome::Failed { reason } => {
                            ("Failed", Some(reason))
                        }
                    };
                    BlockView {
                        id: block.id,
                        position: block.position,
                        sources: block.sources.into_iter().map(|s| s.id).collect(),
                        original_tokens: block.original_tokens,
                        configured_tokens: block.configured_tokens,
                        allocated_tokens: block.allocated_tokens,
                        final_tokens: block.final_tokens,
                        message_count: block.final_message_count,
                        outcome: outcome.into(),
                        reason,
                    }
                })
                .collect(),
        }
    }
}
pub(super) struct AssemblyApiError(ShortTermMemoryError, SupportedLocale);
impl IntoResponse for AssemblyApiError {
    fn into_response(self) -> Response {
        let descriptor = context_assembly_error_descriptor(&self.0);
        let locale = self.1;
        let (status, message) = match descriptor.message_key {
            "tron.chat.context_assembly_not_found" => (
                StatusCode::NOT_FOUND,
                t!("tron.chat.context_assembly_not_found", locale = locale),
            ),
            "tron.chat.context_assembly_request_invalid" => (
                StatusCode::BAD_REQUEST,
                t!(
                    "tron.chat.context_assembly_request_invalid",
                    locale = locale
                ),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                t!("tron.chat.context_assembly_read_failed", locale = locale),
            ),
        };
        (
            status,
            Json(ApiResponse::<EmptyData>::protocol_error(
                descriptor.code,
                message,
            )),
        )
            .into_response()
    }
}
pub(super) async fn read(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    query: Result<Query<AssemblyQuery>, QueryRejection>,
) -> Result<Json<ApiResponse<AssemblyView>>, AssemblyApiError> {
    let locale = SupportedLocale::from_raw(&auth.locale);
    let input = query
        .map_err(|_| AssemblyApiError(ShortTermMemoryError::GoalInvalid, locale))?
        .0;
    let _workspace = input.workspace_code; // Applied by the shared session-scope middleware.
    if input.session_code.trim().is_empty() {
        return Err(AssemblyApiError(ShortTermMemoryError::GoalInvalid, locale));
    }
    let sequence = input
        .snapshot_sequence
        .map(|value| {
            value
                .parse::<u64>()
                .ok()
                .filter(|v| *v > 0)
                .ok_or_else(|| AssemblyApiError(ShortTermMemoryError::GoalInvalid, locale))
        })
        .transpose()?;
    let value = state
        .short_term_memory
        .saved_context_assembly(&input.session_code, sequence)
        .map_err(|e| AssemblyApiError(e, locale))?;
    Ok(Json(ApiResponse::protocol_success(AssemblyView {
        snapshot: value.map(Into::into),
    })))
}
