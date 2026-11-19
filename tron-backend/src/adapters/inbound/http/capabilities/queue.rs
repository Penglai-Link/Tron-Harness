use super::super::auth::RequestAuthContext;
use super::{ensure_pack_available, failure};
use crate::{
    app::AppState,
    application::agent::{CapabilityQueueEnqueueError, CapabilityQueueSnapshot},
    application::capability::{
        CapabilityQueueOperation, CapabilityQueueOutcome, CapabilityQueueStatus,
    },
    response::ApiResponse,
};
use axum::{
    Json, Router,
    extract::{State, rejection::JsonRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::{Deserialize, Serialize};

pub(super) fn routes() -> Router<AppState> {
    Router::new().route(
        "/api/v1/capabilities/queue",
        get(get_queue).post(enqueue_capability_operation),
    )
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CapabilityQueueItemVo {
    id: String,
    pack_id: String,
    pack_name: String,
    /// `enable`, `disable` or `uninstall`.
    operation: &'static str,
    force: bool,
    /// `queued` or `running`. Running is a transient in-memory state.
    status: &'static str,
    created_at: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CapabilityOperationLogVo {
    id: String,
    pack_id: String,
    pack_name: String,
    operation: &'static str,
    force: bool,
    succeeded: bool,
    error: Option<String>,
    error_code: Option<String>,
    finished_at: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CapabilityQueueVo {
    agent_busy: bool,
    items: Vec<CapabilityQueueItemVo>,
    log: Vec<CapabilityOperationLogVo>,
}

fn queue_vo(snapshot: CapabilityQueueSnapshot) -> CapabilityQueueVo {
    CapabilityQueueVo {
        agent_busy: snapshot.agent_busy,
        items: snapshot
            .items
            .into_iter()
            .map(|item| CapabilityQueueItemVo {
                id: item.id,
                pack_id: item.pack_id,
                pack_name: item.pack_name,
                operation: item.operation.storage_value(),
                force: item.force,
                status: match item.status {
                    CapabilityQueueStatus::Queued => "queued",
                    CapabilityQueueStatus::Running => "running",
                },
                created_at: item.created_at,
            })
            .collect(),
        log: snapshot
            .log
            .into_iter()
            .map(|record| CapabilityOperationLogVo {
                id: record.item_id,
                pack_id: record.pack_id,
                pack_name: record.pack_name,
                operation: record.operation.storage_value(),
                force: record.force,
                succeeded: record.succeeded,
                error: record.error,
                error_code: record.error_code,
                finished_at: record.finished_at,
            })
            .collect(),
    }
}

async fn get_queue(auth: RequestAuthContext, State(state): State<AppState>) -> Response {
    match state.agent_framework.capability_queue_snapshot() {
        Ok(snapshot) => Json(ApiResponse::success(queue_vo(snapshot))).into_response(),
        Err(error) => {
            tracing::warn!(%error, "capability queue snapshot failed");
            failure(
                StatusCode::INTERNAL_SERVER_ERROR,
                "CAPABILITY_QUEUE_UNAVAILABLE",
                tron_i18n::t!("capabilities.queue.unavailable", locale = &auth.locale).into_owned(),
            )
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum CapabilityOperationRequest {
    Enable,
    Disable,
    Uninstall,
}

impl From<CapabilityOperationRequest> for CapabilityQueueOperation {
    fn from(request: CapabilityOperationRequest) -> Self {
        match request {
            CapabilityOperationRequest::Enable => Self::Enable,
            CapabilityOperationRequest::Disable => Self::Disable,
            CapabilityOperationRequest::Uninstall => Self::Uninstall,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EnqueueCapabilityOperationRequest {
    pack_id: String,
    #[serde(default)]
    pack_name: Option<String>,
    operation: CapabilityOperationRequest,
    #[serde(default)]
    force: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EnqueueCapabilityOperationVo {
    item_id: String,
    /// `pending`, `succeeded` or `failed` for the item created by this request.
    status: &'static str,
    error: Option<String>,
    /// Stable code for a failed immediate uninstall (for example the durable-run
    /// reference code that opens the force confirmation).
    code: Option<String>,
}

async fn enqueue_capability_operation(
    auth: RequestAuthContext,
    State(state): State<AppState>,
    request: Result<Json<EnqueueCapabilityOperationRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match request {
        Ok(request) => request,
        Err(_) => {
            return failure(
                StatusCode::BAD_REQUEST,
                "CAPABILITY_REQUEST_INVALID",
                tron_i18n::t!("capabilities.center.invalid_request", locale = &auth.locale)
                    .into_owned(),
            );
        }
    };
    let pack_id = request.pack_id.trim();
    if pack_id.is_empty() || pack_id.len() > 128 {
        return failure(
            StatusCode::BAD_REQUEST,
            "CAPABILITY_REQUEST_INVALID",
            tron_i18n::t!("capabilities.center.invalid_request", locale = &auth.locale)
                .into_owned(),
        );
    }
    let operation = CapabilityQueueOperation::from(request.operation);
    let is_builtin = crate::application::agent::capability_pack_catalog()
        .iter()
        .any(|pack| pack.id == pack_id);
    if operation == CapabilityQueueOperation::Uninstall && is_builtin {
        return failure(
            StatusCode::CONFLICT,
            "CAPABILITY_PACK_BUNDLED",
            tron_i18n::t!("capabilities.uninstall_bundled", locale = &auth.locale).into_owned(),
        );
    }
    if let Err(response) = ensure_pack_available(&state, pack_id, &auth.locale).await {
        return response;
    }
    let force = request.force && operation == CapabilityQueueOperation::Uninstall;
    let (item, was_empty) = match state.agent_framework.enqueue_capability_operation(
        pack_id,
        request.pack_name.as_deref(),
        operation,
        force,
        &auth.locale,
    ) {
        Ok(result) => result,
        Err(CapabilityQueueEnqueueError::Full) => {
            return failure(
                StatusCode::CONFLICT,
                "CAPABILITY_QUEUE_FULL",
                tron_i18n::t!("capabilities.queue.full", locale = &auth.locale).into_owned(),
            );
        }
        Err(CapabilityQueueEnqueueError::Unavailable(error)) => {
            tracing::warn!(%error, "capability queue enqueue failed");
            return failure(
                StatusCode::INTERNAL_SERVER_ERROR,
                "CAPABILITY_QUEUE_UNAVAILABLE",
                tron_i18n::t!("capabilities.queue.unavailable", locale = &auth.locale).into_owned(),
            );
        }
    };
    // When the queue was empty and every Agent is idle this single pass keeps
    // the existing click-to-apply behaviour. A busy queue waits for the runner.
    if was_empty && state.agent_framework.external_idle() {
        if let Err(error) = state.agent_framework.process_capability_queue(1).await {
            tracing::warn!(%error, "capability queue immediate pass failed");
        }
    }
    let (status, error, code) = match state.agent_framework.capability_queue_outcome(&item.id) {
        Ok(CapabilityQueueOutcome::Succeeded) => ("succeeded", None, None),
        Ok(CapabilityQueueOutcome::Failed { error, code }) => ("failed", Some(error), code),
        Ok(CapabilityQueueOutcome::Pending) => ("pending", None, None),
        Err(error) => {
            tracing::warn!(%error, "capability queue outcome lookup failed");
            ("pending", None, None)
        }
    };
    Json(ApiResponse::success(EnqueueCapabilityOperationVo {
        item_id: item.id,
        status,
        error,
        code,
    }))
    .into_response()
}
