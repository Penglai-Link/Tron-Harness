use super::super::{error_mapper::ApiError, request::*, response::*};
use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    app::AppMemoryService,
    application::{memory::*, workspace::WorkspaceService},
    response::ApiResponse,
    shared::locale::SupportedLocale,
};
use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
fn locale(headers: &HeaderMap) -> SupportedLocale {
    SupportedLocale::from_request(headers.get("accept-language").and_then(|v| v.to_str().ok()))
}
fn context(
    auth: &RequestAuthContext,
    w: &WorkspaceService,
    scope: MemoryScope,
    code: &str,
) -> Result<MemoryRunContext, MemoryError> {
    if scope == MemoryScope::Space && !w.workspace_is_active(code).unwrap_or(false) {
        return Err(MemoryError::Forbidden("memory.scope_invalid".into()));
    }
    Ok(MemoryRunContext::new(
        &auth.user().user_code,
        if scope == MemoryScope::Space {
            code
        } else {
            ""
        },
    ))
}
fn owned_context(
    memory: &AppMemoryService,
    auth: &RequestAuthContext,
    w: &WorkspaceService,
    code: &str,
) -> Result<MemoryRunContext, MemoryError> {
    let record = memory.owned_record(&auth.user().user_code, code)?;
    context(auth, w, record.scope, &record.scope_code)
}
pub async fn query_memory_records(
    State(memory): State<AppMemoryService>,
    State(w): State<WorkspaceService>,
    auth: RequestAuthContext,
    headers: HeaderMap,
    body: Result<Json<QueryMemoryRecordsDto>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<ApiResponse<PageData<QueryMemoryRecordVo>>>, ApiError> {
    let l = locale(&headers);
    let Json(dto) =
        body.map_err(|_| ApiError(MemoryError::Validation("memory.invalid_json".into()), l))?;
    let map = |e| ApiError(e, l);
    let c = context(&auth, &w, dto.scope, &dto.scope_code).map_err(map)?;
    let page = memory
        .query(
            &c,
            MemoryRecordQuery {
                scope: dto.scope,
                scope_code: dto.scope_code,
                keyword: dto.keyword,
                enabled: dto.enabled,
                current: dto.current.unwrap_or(1),
                size: dto.size.unwrap_or(20),
            },
        )
        .map_err(map)?;
    Ok(Json(ApiResponse::protocol_success(PageData {
        current: page.current,
        size: page.size,
        total: page.total,
        records: page.records.into_iter().map(Into::into).collect(),
    })))
}
pub async fn create_memory_record(
    State(memory): State<AppMemoryService>,
    State(w): State<WorkspaceService>,
    auth: RequestAuthContext,
    headers: HeaderMap,
    body: Result<Json<CreateMemoryRecordDto>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<ApiResponse<CreateMemoryRecordVo>>, ApiError> {
    let l = locale(&headers);
    let Json(dto) =
        body.map_err(|_| ApiError(MemoryError::Validation("memory.invalid_json".into()), l))?;
    let map = |e| ApiError(e, l);
    let c = context(&auth, &w, dto.scope, &dto.scope_code).map_err(map)?;
    let result=memory.save(c,MemorySaveCommand{scope:Some(dto.scope),scope_code:Some(dto.scope_code),title:Some(dto.title),text:Some(dto.text),source_label:"Manual".into(),source_refs:vec![serde_json::json!({"kind":"Manual","userCode":auth.user().user_code,"savedAt":chrono::Utc::now().timestamp_millis()})],..Default::default()}).await.map_err(map)?;
    Ok(Json(ApiResponse::protocol_success(result.record.into())))
}
pub async fn get_memory_record(
    State(memory): State<AppMemoryService>,
    State(w): State<WorkspaceService>,
    auth: RequestAuthContext,
    headers: HeaderMap,
    Path(code): Path<String>,
) -> Result<Json<ApiResponse<GetMemoryRecordVo>>, ApiError> {
    let l = locale(&headers);
    let map = |e| ApiError(e, l);
    let c = owned_context(&memory, &auth, &w, &code).map_err(map)?;
    Ok(Json(ApiResponse::protocol_success(
        memory.get(&c, &code).map_err(map)?.into(),
    )))
}
pub async fn patch_memory_record(
    State(memory): State<AppMemoryService>,
    State(w): State<WorkspaceService>,
    auth: RequestAuthContext,
    headers: HeaderMap,
    Path(code): Path<String>,
    body: Result<Json<UpdateMemoryRecordDto>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<ApiResponse<UpdateMemoryRecordVo>>, ApiError> {
    let l = locale(&headers);
    let Json(dto) =
        body.map_err(|_| ApiError(MemoryError::Validation("memory.invalid_json".into()), l))?;
    let map = |e| ApiError(e, l);
    let c = owned_context(&memory, &auth, &w, &code).map_err(map)?;
    let result = memory
        .save(
            c,
            MemorySaveCommand {
                memory_code: Some(code),
                version: Some(dto.version),
                title: dto.title,
                text: dto.text,
                enabled: dto.enabled,
                ..Default::default()
            },
        )
        .await
        .map_err(map)?;
    Ok(Json(ApiResponse::protocol_success(result.record.into())))
}
pub async fn delete_memory_record(
    State(memory): State<AppMemoryService>,
    State(w): State<WorkspaceService>,
    auth: RequestAuthContext,
    headers: HeaderMap,
    Path(code): Path<String>,
    body: Result<Json<DeleteMemoryRecordDto>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<ApiResponse<DeleteMemoryRecordVo>>, ApiError> {
    let l = locale(&headers);
    let Json(dto) =
        body.map_err(|_| ApiError(MemoryError::Validation("memory.invalid_json".into()), l))?;
    let map = |e| ApiError(e, l);
    let c = owned_context(&memory, &auth, &w, &code).map_err(map)?;
    let r = memory
        .delete(
            c,
            MemoryDeleteCommand {
                memory_code: code,
                version: dto.version,
            },
        )
        .await
        .map_err(map)?;
    Ok(Json(ApiResponse::protocol_success(DeleteMemoryRecordVo {
        memory_code: r.memory_code,
        deleted: r.deleted,
    })))
}
