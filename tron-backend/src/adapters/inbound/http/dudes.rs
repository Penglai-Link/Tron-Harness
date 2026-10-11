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
        .route("/api/v1/notifications", get(notifications))
        .route("/api/v1/notifications/{id}", patch(update_notification))
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
        let (status, code) = match self.0 {
            DudeError::NotFound => (StatusCode::NOT_FOUND, "101018001"),
            DudeError::Invalid => (StatusCode::BAD_REQUEST, "101018002"),
            DudeError::Conflict => (StatusCode::CONFLICT, "101018003"),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "101018004"),
        };
        (
            status,
            Json(ApiResponse::<EmptyData>::protocol_error(
                code,
                self.0.to_string(),
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
            group_name: Option<String>,
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
                    group_name: p.group_name,
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
    pub group_name: Option<String>,
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
    pub group_name: Option<String>,
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
            group_name: r.group_name,
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
                group_name: r.group_name,
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
        .ensure_dude_session(&d, Some(auth.user_code))
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
        .ensure_dude_session(&d, Some(auth.user_code))
        .map_err(|_| DudeError::Storage)?;
    Ok(Json(ApiResponse::protocol_success(
        ResolveDudeSessionResponse::from_record(chat, d),
    )))
}
async fn sessions(
    State(s): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<ApiResponse<Vec<ListDudeSessionResponse>>>, DudeApiError> {
    let mut result = vec![];
    for d in s.dudes.sessions(&code)? {
        let chat = s
            .chats
            .ensure_dude_session(&d, None)
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
