use crate::adapters::inbound::http::auth::RequestAuthContext;
use crate::{
    app::AppState,
    application::{
        agent::{
            CreateWorkspaceInternalAgentCommand, UpdateWorkspaceInternalAgentDefinitionCommand,
        },
        dude::*,
    },
    domain::dude::*,
    response::{ApiResponse, EmptyData},
};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, patch, post, put},
};
use serde::{Deserialize, Serialize};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/dudes", get(list).post(create))
        .route("/api/v1/dudes/main/session", post(main_session))
        .route(
            "/api/v1/dudes/{code}",
            get(detail).patch(update).delete(delete),
        )
        .route("/api/v1/dudes/{code}/main", put(designate_main))
        .route(
            "/api/v1/dudes/{code}/sessions",
            get(sessions).post(resolve_session),
        )
        .route("/api/v1/dudes/{code}/events", get(events))
        .route("/api/v1/dudes/{code}/threads", get(threads))
        .route(
            "/api/v1/dudes/{code}/threads/{session}/messages",
            post(thread_message),
        )
        .route(
            "/api/v1/dudes/{code}/threads/{session}/answer",
            post(thread_answer),
        )
        .route(
            "/api/v1/dudes/{code}/threads/{session}/history/query",
            post(thread_history),
        )
        .route(
            "/api/v1/notifications",
            get(notifications).delete(delete_archived_notifications),
        )
        .route(
            "/api/v1/notifications/{id}",
            patch(update_notification).delete(delete_notification),
        )
}
#[derive(Debug)]
pub struct DudeApiError(pub DudeError);
impl From<DudeError> for DudeApiError {
    fn from(e: DudeError) -> Self {
        Self(e)
    }
}
impl IntoResponse for DudeApiError {
    fn into_response(self) -> Response {
        let descriptor = error_descriptor(&self.0);
        let (status, zh, en) = match descriptor.message_key {
            "tron.dude.not_found" => (
                StatusCode::NOT_FOUND,
                "Dude 或关联记录不存在",
                "Dude or related record was not found",
            ),
            "tron.dude.request_invalid" => (
                StatusCode::BAD_REQUEST,
                "Dude 配置或请求无效",
                "Dude configuration or request is invalid",
            ),
            "tron.dude.state_changed" => (
                StatusCode::CONFLICT,
                "Dude 状态已变化，请刷新后重试",
                "Dude state changed; refresh and retry",
            ),
            "tron.dude.storage_failed" => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Dude 存储操作失败",
                "Dude storage operation failed",
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Dude 运行失败，请查看工作记录",
                "Dude execution failed; inspect its work history",
            ),
        };
        let english = crate::shared::auth_holder::AuthHolder::get()
            .is_some_and(|c| c.locale.to_ascii_lowercase().starts_with("en"));
        (
            status,
            Json(ApiResponse::<EmptyData>::protocol_error(
                descriptor.code,
                if english { en } else { zh },
            )),
        )
            .into_response()
    }
}
macro_rules! dude_response {
    ($name:ident) => {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        pub struct $name {
            dude_code: String,
            agent_code: String,
            name: String,
            role_prompt: String,
            loop_code: Option<String>,
            avatar_id: String,
            candidate_model_codes: Vec<String>,
            kind: DudeKind,
            enabled: bool,
            heartbeat_minutes: u32,
            memory_auto_approve: bool,
            is_main: bool,
            created_at: i64,
            updated_at: i64,
            tool_assignment_mode: crate::domain::agent::ToolAssignmentMode,
            mcp_access_mode: crate::domain::agent::McpAccessMode,
            tool_exposure_mode: crate::domain::agent::ToolExposureMode,
            expanded_domain_persistence_mode: crate::domain::agent::ExpandedDomainPersistenceMode,
            max_steps: u32,
            max_tool_calls: u32,
            max_parallel_tool_calls: u32,
            selected_tool_codes: Vec<String>,
            selected_server_codes: Vec<String>,
        }
        impl $name {
            fn from_record(service: &DudeService, d: Dude) -> Result<Self, DudeError> {
                let p = service.definition(&d.dude_code)?;
                Ok(Self {
                    dude_code: d.dude_code,
                    agent_code: p.agent_code,
                    name: p.name,
                    role_prompt: p.role_prompt,
                    loop_code: p.loop_code,
                    avatar_id: p.avatar_id,
                    candidate_model_codes: p.candidate_model_codes,
                    kind: d.kind,
                    enabled: d.enabled,
                    heartbeat_minutes: d.heartbeat_minutes,
                    memory_auto_approve: d.memory_auto_approve,
                    is_main: d.is_main,
                    created_at: d.created_at,
                    updated_at: d.updated_at,
                    tool_assignment_mode: p.tool_assignment_mode,
                    mcp_access_mode: p.mcp_access_mode,
                    tool_exposure_mode: p.tool_exposure_mode,
                    expanded_domain_persistence_mode: p.expanded_domain_persistence_mode,
                    max_steps: p.max_steps,
                    max_tool_calls: p.max_tool_calls,
                    max_parallel_tool_calls: p.max_parallel_tool_calls,
                    selected_tool_codes: p.selected_tool_codes,
                    selected_server_codes: p.selected_server_codes,
                })
            }
        }
    };
}
dude_response!(ListDudeResponse);
dude_response!(GetDudeResponse);
dude_response!(CreateDudeResponse);
dude_response!(UpdateDudeResponse);
dude_response!(DesignateMainResponse);
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateDudeRequest {
    pub dude_code: String,
    pub name: String,
    pub role_prompt: String,
    pub loop_code: Option<String>,
    pub avatar_id: Option<String>,
    #[serde(default)]
    pub candidate_model_codes: Vec<String>,
    #[serde(default = "custom_kind")]
    pub kind: DudeKind,
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default = "cadence")]
    pub heartbeat_minutes: u32,
    #[serde(default)]
    pub memory_auto_approve: bool,
}
fn custom_kind() -> DudeKind {
    DudeKind::Custom
}
fn yes() -> bool {
    true
}
fn cadence() -> u32 {
    30
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateDudeRequest {
    pub name: Option<String>,
    pub role_prompt: Option<String>,
    pub loop_code: Option<String>,
    pub avatar_id: Option<String>,
    pub candidate_model_codes: Option<Vec<String>>,
    pub enabled: Option<bool>,
    pub heartbeat_minutes: Option<u32>,
    pub memory_auto_approve: Option<bool>,
}
async fn list(
    State(s): State<AppState>,
) -> Result<Json<ApiResponse<Vec<ListDudeResponse>>>, DudeApiError> {
    let records = s
        .dudes
        .list()?
        .into_iter()
        .map(|d| ListDudeResponse::from_record(&s.dudes, d))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Json(ApiResponse::protocol_success(records)))
}
async fn detail(
    State(s): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<ApiResponse<GetDudeResponse>>, DudeApiError> {
    Ok(Json(ApiResponse::protocol_success(
        GetDudeResponse::from_record(&s.dudes, s.dudes.get(&code)?)?,
    )))
}
async fn create(
    State(s): State<AppState>,
    Json(r): Json<CreateDudeRequest>,
) -> Result<Json<ApiResponse<CreateDudeResponse>>, DudeApiError> {
    let d = s.dudes.create(CreateDudeCommand {
        definition: CreateWorkspaceInternalAgentCommand {
            agent_code: r.dude_code,
            name: r.name,
            role_prompt: r.role_prompt,
            loop_code: r.loop_code,
            avatar_id: r.avatar_id,
            group_name: None,
            candidate_model_codes: r.candidate_model_codes,
        },
        kind: r.kind,
        enabled: r.enabled,
        heartbeat_minutes: r.heartbeat_minutes,
        memory_auto_approve: r.memory_auto_approve,
    })?;
    Ok(Json(ApiResponse::protocol_success(
        CreateDudeResponse::from_record(&s.dudes, d)?,
    )))
}
async fn update(
    State(s): State<AppState>,
    Path(code): Path<String>,
    Json(r): Json<UpdateDudeRequest>,
) -> Result<Json<ApiResponse<UpdateDudeResponse>>, DudeApiError> {
    let d = s.dudes.update(
        &code,
        UpdateDudeCommand {
            definition: UpdateWorkspaceInternalAgentDefinitionCommand {
                name: r.name,
                role_prompt: r.role_prompt,
                loop_code: r.loop_code,
                avatar_id: r.avatar_id,
                group_name: None,
                candidate_model_codes: r.candidate_model_codes,
            },
            enabled: r.enabled,
            heartbeat_minutes: r.heartbeat_minutes,
            memory_auto_approve: r.memory_auto_approve,
        },
    )?;
    Ok(Json(ApiResponse::protocol_success(
        UpdateDudeResponse::from_record(&s.dudes, d)?,
    )))
}
async fn delete(
    State(s): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<ApiResponse<EmptyData>>, DudeApiError> {
    s.dudes.delete(&code)?;
    Ok(Json(ApiResponse::protocol_success(EmptyData {})))
}
async fn designate_main(
    State(s): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<ApiResponse<DesignateMainResponse>>, DudeApiError> {
    s.dudes.designate_main(&code)?;
    Ok(Json(ApiResponse::protocol_success(
        DesignateMainResponse::from_record(&s.dudes, s.dudes.get(&code)?)?,
    )))
}
macro_rules! session_response {
    ($name:ident) => {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        pub struct $name {
            id: String,
            session_code: String,
            workspace_code: Option<String>,
            project_code: Option<String>,
            user_code: Option<String>,
            app_code: Option<String>,
            task_code: Option<String>,
            source_type: String,
            source_code: Option<String>,
            agent_code: Option<String>,
            system_agent_session_id: Option<String>,
            title: String,
            status: String,
            last_run_code: Option<String>,
            last_run_status: Option<String>,
            created_at: i64,
            updated_at: i64,
            owner_dude_code: String,
            purpose: DudeSessionPurpose,
        }
        impl $name {
            fn from_record(chat: crate::domain::chat::ChatSession, d: DudeSession) -> Self {
                Self {
                    id: chat.id.to_string(),
                    session_code: chat.session_code,
                    workspace_code: d.workspace_code,
                    project_code: chat.project_code,
                    user_code: chat.user_code,
                    app_code: chat.app_code,
                    task_code: chat.task_code,
                    source_type: "Dude".into(),
                    source_code: Some(d.dude_code.clone()),
                    agent_code: chat.agent_code,
                    system_agent_session_id: chat.system_agent_session_id,
                    title: chat.title,
                    status: format!("{:?}", chat.status),
                    last_run_code: chat.last_run_code,
                    last_run_status: chat.last_run_status,
                    created_at: d.created_at,
                    updated_at: d.updated_at,
                    owner_dude_code: d.dude_code,
                    purpose: d.purpose,
                }
            }
        }
    };
}
session_response!(ResolveDudeSessionResponse);
session_response!(MainDudeSessionResponse);
session_response!(ListDudeSessionResponse);
async fn main_session(
    State(s): State<AppState>,
    auth: RequestAuthContext,
) -> Result<Json<ApiResponse<MainDudeSessionResponse>>, DudeApiError> {
    let code = s.dudes.main()?.dude_code;
    let d = s
        .dudes
        .resolve_session(&code, DudeSessionPurpose::Interactive, "interactive", None)?;
    let chat = s
        .chats
        .ensure_dude_session(&d, Some(auth.user().user_code.clone()))
        .map_err(|_| DudeError::Storage)?;
    Ok(Json(ApiResponse::protocol_success(
        MainDudeSessionResponse::from_record(chat, d),
    )))
}
async fn resolve_session(
    State(s): State<AppState>,
    auth: RequestAuthContext,
    Path(code): Path<String>,
) -> Result<Json<ApiResponse<ResolveDudeSessionResponse>>, DudeApiError> {
    let d = s
        .dudes
        .resolve_session(&code, DudeSessionPurpose::Interactive, "interactive", None)?;
    let chat = s
        .chats
        .ensure_dude_session(&d, Some(auth.user().user_code.clone()))
        .map_err(|_| DudeError::Storage)?;
    Ok(Json(ApiResponse::protocol_success(
        ResolveDudeSessionResponse::from_record(chat, d),
    )))
}
async fn sessions(
    State(s): State<AppState>,
    auth: RequestAuthContext,
    Path(code): Path<String>,
) -> Result<Json<ApiResponse<Vec<ListDudeSessionResponse>>>, DudeApiError> {
    let mut result = vec![];
    for d in s.dudes.sessions(&code)? {
        let chat = s
            .chats
            .ensure_dude_session(&d, Some(auth.user().user_code.clone()))
            .map_err(|_| DudeError::Storage)?;
        result.push(ListDudeSessionResponse::from_record(chat, d));
    }
    Ok(Json(ApiResponse::protocol_success(result)))
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListDudeEventResponse {
    event_id: String,
    source: DudeEventSource,
    status: DudeEventStatus,
    session_code: Option<String>,
    task_code: Option<String>,
    attempts: u32,
    result: Option<String>,
    created_at: i64,
}
async fn events(
    State(s): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<ApiResponse<Vec<ListDudeEventResponse>>>, DudeApiError> {
    Ok(Json(ApiResponse::protocol_success(
        s.dudes
            .events(&code)?
            .into_iter()
            .map(|e| ListDudeEventResponse {
                event_id: e.event_id,
                source: e.source,
                status: e.status,
                session_code: e.session_code,
                task_code: e.task_code,
                attempts: e.attempts,
                result: e.result,
                created_at: e.created_at,
            })
            .collect(),
    )))
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListNotificationResponse {
    id: String,
    dude_code: Option<String>,
    title: String,
    body: String,
    category: String,
    target: serde_json::Value,
    read_at: Option<i64>,
    resolved_at: Option<i64>,
    archived_at: Option<i64>,
    created_at: i64,
}
async fn notifications(
    State(s): State<AppState>,
) -> Result<Json<ApiResponse<Vec<ListNotificationResponse>>>, DudeApiError> {
    Ok(Json(ApiResponse::protocol_success(
        s.dudes
            .notifications()?
            .into_iter()
            .map(|n| ListNotificationResponse {
                id: n.id,
                dude_code: n.dude_code,
                title: n.title,
                body: n.body,
                category: n.category,
                target: n.target,
                read_at: n.read_at,
                resolved_at: n.resolved_at,
                archived_at: n.archived_at,
                created_at: n.created_at,
            })
            .collect(),
    )))
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateNotificationRequest {
    pub read: bool,
    #[serde(default)]
    pub archived: bool,
}
async fn update_notification(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(r): Json<UpdateNotificationRequest>,
) -> Result<Json<ApiResponse<EmptyData>>, DudeApiError> {
    s.dudes.update_notification(&id, r.read, r.archived)?;
    Ok(Json(ApiResponse::protocol_success(EmptyData {})))
}
async fn delete_notification(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<EmptyData>>, DudeApiError> {
    s.dudes.delete_archived_notification(&id)?;
    Ok(Json(ApiResponse::protocol_success(EmptyData {})))
}
async fn delete_archived_notifications(
    State(s): State<AppState>,
) -> Result<Json<ApiResponse<usize>>, DudeApiError> {
    Ok(Json(ApiResponse::protocol_success(
        s.dudes.delete_archived_notifications()?,
    )))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListDudeThreadResponse {
    session_code: String,
    title: Option<String>,
    status: String,
    latest_run_code: Option<String>,
    closed: bool,
    agent_code: String,
    origin_session_code: String,
    task_code: Option<String>,
}
async fn threads(
    State(s): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<ApiResponse<Vec<ListDudeThreadResponse>>>, DudeApiError> {
    s.dudes.get(&code)?;
    let records = s
        .agent_framework
        .dude_thread_catalog(&code)
        .await
        .map_err(|e| DudeError::Runtime(e.to_string()))?;
    Ok(Json(ApiResponse::protocol_success(
        records
            .into_iter()
            .map(|t| ListDudeThreadResponse {
                session_code: t.summary.session_id,
                title: t.summary.title,
                status: format!("{:?}", t.summary.status),
                latest_run_code: t.summary.latest_run_id.map(|r| r.to_string()),
                closed: t.closed,
                agent_code: t.agent_code,
                origin_session_code: t.origin_session_code,
                task_code: t.task_code,
            })
            .collect(),
    )))
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadDudeThreadHistoryRequest {
    pub size: usize,
    pub cursor: Option<eliteforge_agent_core::HistoryCursor>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadDudeThreadHistoryResponse {
    session_id: String,
    version: String,
    next_cursor: Option<eliteforge_agent_core::HistoryCursor>,
    runs: Vec<eliteforge_agent_core::HistoryRun>,
    records: eliteforge_agent_core::HistoryRecords,
    pending_interactions: Vec<eliteforge_agent_core::HistoryInteraction>,
    active_runs: Vec<eliteforge_agent_core::HistoryRun>,
}
async fn thread_history(
    State(s): State<AppState>,
    Path((code, thread)): Path<(String, String)>,
    Json(r): Json<ReadDudeThreadHistoryRequest>,
) -> Result<Json<ApiResponse<ReadDudeThreadHistoryResponse>>, DudeApiError> {
    verify_thread(&s, &code, &thread).await?;
    if r.size == 0 || r.size > 100 {
        return Err(DudeError::Invalid.into());
    }
    let mut h = s
        .agent_framework
        .dude_thread_history(
            &code,
            &thread,
            eliteforge_agent_core::HistoryQuery {
                limit: r.size,
                cursor: r.cursor,
            },
        )
        .await
        .map_err(|e| DudeError::Runtime(e.to_string()))?;
    for message in &mut h.records.messages {
        message.content = message
            .content
            .drain(..)
            .filter_map(crate::adapters::inbound::http::chats::history::public_history_block)
            .collect();
    }
    Ok(Json(ApiResponse::protocol_success(
        ReadDudeThreadHistoryResponse {
            session_id: h.session_id,
            version: h.version,
            next_cursor: h.next_cursor,
            runs: h.runs,
            records: h.records,
            pending_interactions: h.pending_interactions,
            active_runs: h.active_runs,
        },
    )))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SendDudeThreadMessageRequest {
    pub content: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendDudeThreadMessageResponse {
    session_code: String,
    status: String,
    latest_run_code: Option<String>,
}
async fn thread_message(
    State(s): State<AppState>,
    auth: RequestAuthContext,
    Path((code, thread)): Path<(String, String)>,
    Json(r): Json<SendDudeThreadMessageRequest>,
) -> Result<Json<ApiResponse<SendDudeThreadMessageResponse>>, DudeApiError> {
    verify_thread(&s, &code, &thread).await?;
    if r.content.trim().is_empty() || r.content.len() > 1024 * 1024 {
        return Err(DudeError::Invalid.into());
    }
    let summary = s
        .agent_framework
        .dude_thread_send(
            &code,
            &thread,
            &r.content,
            &auth.user().user_code,
            &auth.locale,
            &s.dude_invocation_service(),
        )
        .await
        .map_err(|e| DudeError::Runtime(e.to_string()))?;
    Ok(Json(ApiResponse::protocol_success(
        SendDudeThreadMessageResponse {
            session_code: summary.session_id,
            status: format!("{:?}", summary.status),
            latest_run_code: summary.latest_run_id.map(|r| r.to_string()),
        },
    )))
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnswerDudeThreadRequest {
    pub interaction_id: String,
    pub value: serde_json::Value,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnswerDudeThreadResponse {
    session_code: String,
    status: String,
    latest_run_code: Option<String>,
}
async fn thread_answer(
    State(s): State<AppState>,
    auth: RequestAuthContext,
    Path((code, thread)): Path<(String, String)>,
    Json(r): Json<AnswerDudeThreadRequest>,
) -> Result<Json<ApiResponse<AnswerDudeThreadResponse>>, DudeApiError> {
    verify_thread(&s, &code, &thread).await?;
    let interaction = eliteforge_agent_core::RunId::parse_str(&r.interaction_id)
        .map_err(|_| DudeError::Invalid)?;
    let summary = s
        .agent_framework
        .dude_thread_answer(
            &code,
            &thread,
            interaction,
            r.value,
            &auth.user().user_code,
            &auth.locale,
            &s.dude_invocation_service(),
        )
        .await
        .map_err(|e| DudeError::Runtime(e.to_string()))?;
    Ok(Json(ApiResponse::protocol_success(
        AnswerDudeThreadResponse {
            session_code: summary.session_id,
            status: format!("{:?}", summary.status),
            latest_run_code: summary.latest_run_id.map(|r| r.to_string()),
        },
    )))
}

async fn verify_thread(state: &AppState, dude: &str, thread: &str) -> Result<(), DudeError> {
    state.dudes.get(dude)?;
    if !state
        .agent_framework
        .dude_thread_catalog(dude)
        .await
        .map_err(|e| DudeError::Runtime(e.to_string()))?
        .iter()
        .any(|t| t.summary.session_id == thread)
    {
        return Err(DudeError::NotFound);
    }
    Ok(())
}
