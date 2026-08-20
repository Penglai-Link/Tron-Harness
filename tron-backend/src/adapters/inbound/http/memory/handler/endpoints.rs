use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::{Map, Value, json};

use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    app::AppMemoryService,
    application::{
        memory::{
            MemoryDeleteCommand, MemoryError, MemoryItem, MemoryRunContext, MemorySaveCommand,
            MemorySearchCommand, MemoryWorkDomain, MemoryWorkDomainService,
            decode_memory_code_parts,
        },
        workspace::WorkspaceService,
    },
    domain::workspace::WorkspaceDetail,
    response::ApiResponse,
    shared::time::timestamp_from_millis,
};

use super::{
    error_mapper::memory_error_response,
    request::{
        CreateMemoryWorkDomainRequest, MemoryDeleteRequest, MemoryRecordListQuery,
        MemoryRecordMutationRequest, MemoryWorkDomainHttpQuery, UpdateMemoryWorkDomainRequest,
    },
    response::{MemoryMetadataItemResponse, MemoryRecordResponse, MemoryWorkDomainResponse},
};

pub async fn list_memory_records(
    State(workspaces): State<WorkspaceService>,
    State(memory): State<AppMemoryService>,
    State(domains): State<MemoryWorkDomainService>,
    auth: RequestAuthContext,
    Path(workspace_code): Path<String>,
    Query(query): Query<MemoryRecordListQuery>,
) -> Response {
    let workspace = match workspace_detail(&workspaces, &workspace_code, &auth) {
        Ok(workspace) => workspace,
        Err(error) => return memory_error_response(error),
    };
    let target = match target_from_query(&domains, &auth, &workspace, &query) {
        Ok(target) => target,
        Err(error) => return memory_error_response(error),
    };
    let context = memory_context(&auth, &workspace.workspace_code, target.work_domain_code());
    let command = search_command_from_query(&query, &target);
    match memory.search(context, command).await {
        Ok(result) => match response_catalog(&domains, &auth, &workspace)
            .and_then(|catalog| records_from_items(result.items, &catalog))
        {
            Ok(records) => Json(ApiResponse::success(records)).into_response(),
            Err(error) => memory_error_response(error),
        },
        Err(error) => memory_error_response(error),
    }
}

pub async fn get_memory_record(
    State(workspaces): State<WorkspaceService>,
    State(memory): State<AppMemoryService>,
    State(domains): State<MemoryWorkDomainService>,
    auth: RequestAuthContext,
    Path((workspace_code, memory_code)): Path<(String, String)>,
) -> Response {
    match record_by_code(
        &workspaces,
        &memory,
        &domains,
        &auth,
        &workspace_code,
        &memory_code,
    )
    .await
    {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => memory_error_response(error),
    }
}

pub async fn create_memory_record(
    State(workspaces): State<WorkspaceService>,
    State(memory): State<AppMemoryService>,
    State(domains): State<MemoryWorkDomainService>,
    auth: RequestAuthContext,
    Path(workspace_code): Path<String>,
    Json(request): Json<MemoryRecordMutationRequest>,
) -> Response {
    let workspace = match workspace_detail(&workspaces, &workspace_code, &auth) {
        Ok(workspace) => workspace,
        Err(error) => return memory_error_response(error),
    };
    let target = match target_from_mutation(&domains, &auth, &workspace, &request) {
        Ok(target) => target,
        Err(error) => return memory_error_response(error),
    };
    let context = memory_context(&auth, &workspace.workspace_code, target.work_domain_code());
    let command = save_command_from_request(None, request, &target);
    match memory.save(context, command).await {
        Ok(result) => {
            match record_by_code(
                &workspaces,
                &memory,
                &domains,
                &auth,
                &workspace.workspace_code,
                &result.memory_code,
            )
            .await
            {
                Ok(record) => {
                    (StatusCode::CREATED, Json(ApiResponse::success(record))).into_response()
                }
                Err(error) => memory_error_response(error),
            }
        }
        Err(error) => memory_error_response(error),
    }
}

pub async fn patch_memory_record(
    State(workspaces): State<WorkspaceService>,
    State(memory): State<AppMemoryService>,
    State(domains): State<MemoryWorkDomainService>,
    auth: RequestAuthContext,
    Path((workspace_code, memory_code)): Path<(String, String)>,
    Json(request): Json<MemoryRecordMutationRequest>,
) -> Response {
    let context = match context_from_memory_code(&domains, &auth, &workspace_code, &memory_code) {
        Ok(context) => context,
        Err(error) => return memory_error_response(error),
    };
    let command =
        save_command_from_request(Some(memory_code.clone()), request, &TargetScope::default());
    match memory.save(context, command).await {
        Ok(_) => {
            match record_by_code(
                &workspaces,
                &memory,
                &domains,
                &auth,
                &workspace_code,
                &memory_code,
            )
            .await
            {
                Ok(record) => Json(ApiResponse::success(record)).into_response(),
                Err(error) => memory_error_response(error),
            }
        }
        Err(error) => memory_error_response(error),
    }
}

pub async fn archive_memory_record(
    State(workspaces): State<WorkspaceService>,
    State(memory): State<AppMemoryService>,
    State(domains): State<MemoryWorkDomainService>,
    auth: RequestAuthContext,
    Path((workspace_code, memory_code)): Path<(String, String)>,
    request: Option<Json<MemoryDeleteRequest>>,
) -> Response {
    mutate_memory_status(
        &workspaces,
        &memory,
        &domains,
        &auth,
        &workspace_code,
        &memory_code,
        "ARCHIVE",
        request.map(|Json(request)| request),
    )
    .await
}

pub async fn disable_memory_record(
    State(workspaces): State<WorkspaceService>,
    State(memory): State<AppMemoryService>,
    State(domains): State<MemoryWorkDomainService>,
    auth: RequestAuthContext,
    Path((workspace_code, memory_code)): Path<(String, String)>,
    request: Option<Json<MemoryDeleteRequest>>,
) -> Response {
    mutate_memory_status(
        &workspaces,
        &memory,
        &domains,
        &auth,
        &workspace_code,
        &memory_code,
        "DISABLE",
        request.map(|Json(request)| request),
    )
    .await
}

pub async fn list_work_domains(
    State(domains): State<MemoryWorkDomainService>,
    Query(query): Query<MemoryWorkDomainHttpQuery>,
) -> Response {
    match domains.list_domains(query) {
        Ok(domains) => Json(ApiResponse::success(
            domains
                .into_iter()
                .map(MemoryWorkDomainResponse::from)
                .collect::<Vec<_>>(),
        ))
        .into_response(),
        Err(error) => memory_error_response(error),
    }
}

pub async fn create_work_domain(
    State(domains): State<MemoryWorkDomainService>,
    Json(request): Json<CreateMemoryWorkDomainRequest>,
) -> Response {
    match domains.create_domain(request.into()) {
        Ok(domain) => (
            StatusCode::CREATED,
            Json(ApiResponse::success(MemoryWorkDomainResponse::from(domain))),
        )
            .into_response(),
        Err(error) => memory_error_response(error),
    }
}

pub async fn patch_work_domain(
    State(domains): State<MemoryWorkDomainService>,
    Path(domain_code): Path<String>,
    Json(request): Json<UpdateMemoryWorkDomainRequest>,
) -> Response {
    match domains.update_domain(&domain_code, request.into()) {
        Ok(domain) => {
            Json(ApiResponse::success(MemoryWorkDomainResponse::from(domain))).into_response()
        }
        Err(error) => memory_error_response(error),
    }
}

pub async fn enable_work_domain(
    State(domains): State<MemoryWorkDomainService>,
    Path(domain_code): Path<String>,
) -> Response {
    match domains.set_domain_status(&domain_code, "ACTIVE") {
        Ok(domain) => {
            Json(ApiResponse::success(MemoryWorkDomainResponse::from(domain))).into_response()
        }
        Err(error) => memory_error_response(error),
    }
}

pub async fn disable_work_domain(
    State(domains): State<MemoryWorkDomainService>,
    Path(domain_code): Path<String>,
) -> Response {
    match domains.set_domain_status(&domain_code, "DISABLED") {
        Ok(domain) => {
            Json(ApiResponse::success(MemoryWorkDomainResponse::from(domain))).into_response()
        }
        Err(error) => memory_error_response(error),
    }
}
