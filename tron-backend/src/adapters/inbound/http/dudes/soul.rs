use super::*;
pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/dudes/{code}/soul", get(read).put(save))
        .route("/api/v1/dudes/main/awakening", get(status).post(begin))
        .route("/api/v1/dudes/main/awakening/defer", post(defer))
}
macro_rules! soul_vo {
    ($name:ident) => {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct $name {
            dude_code: String,
            name: String,
            markdown: String,
            name_confirmed: bool,
            state: SoulState,
        }
        impl From<DudeSoul> for $name {
            fn from(s: DudeSoul) -> Self {
                Self {
                    dude_code: s.dude_code,
                    name: s.name,
                    markdown: s.markdown,
                    name_confirmed: s.name_confirmed,
                    state: s.state,
                }
            }
        }
    };
}
soul_vo!(ReadSoulResponse);
soul_vo!(SaveSoulResponse);
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SaveSoulRequest {
    name: String,
    markdown: String,
}
async fn read(
    State(s): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<ApiResponse<ReadSoulResponse>>, DudeApiError> {
    Ok(Json(ApiResponse::protocol_success(
        s.dudes.soul(&code)?.into(),
    )))
}
async fn save(
    State(s): State<AppState>,
    Path(code): Path<String>,
    Json(r): Json<SaveSoulRequest>,
) -> Result<Json<ApiResponse<SaveSoulResponse>>, DudeApiError> {
    Ok(Json(ApiResponse::protocol_success(
        s.dudes.save_soul(&code, r.name, r.markdown)?.into(),
    )))
}
macro_rules! awakening_vo {
    ($name:ident) => {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct $name {
            dude_code: String,
            name: String,
            state: SoulState,
            started: bool,
            deferred: bool,
            session_code: Option<String>,
            can_resume: bool,
        }
        impl $name {
            fn new(s: DudeSoul, user: &str) -> Self {
                let can_resume = s.user_code.as_deref() == Some(user);
                Self {
                    dude_code: s.dude_code,
                    name: s.name,
                    state: s.state,
                    started: s.awakening_started,
                    deferred: s.awakening_deferred,
                    session_code: if can_resume { s.session_code } else { None },
                    can_resume,
                }
            }
        }
    };
}
awakening_vo!(AwakeningStatusResponse);
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BeginAwakeningResponse {
    dude_code: String,
    name: String,
    state: SoulState,
    started: bool,
    deferred: bool,
    session_code: Option<String>,
    can_resume: bool,
    play_entrance: bool,
}
impl BeginAwakeningResponse {
    fn new(s: DudeSoul, user: &str, play_entrance: bool) -> Self {
        let can_resume = s.user_code.as_deref() == Some(user);
        Self {
            dude_code: s.dude_code,
            name: s.name,
            state: s.state,
            started: s.awakening_started,
            deferred: s.awakening_deferred,
            session_code: if can_resume { s.session_code } else { None },
            can_resume,
            play_entrance,
        }
    }
}
awakening_vo!(DeferAwakeningResponse);
async fn status(
    State(s): State<AppState>,
    auth: RequestAuthContext,
) -> Result<Json<ApiResponse<AwakeningStatusResponse>>, DudeApiError> {
    Ok(Json(ApiResponse::protocol_success(
        AwakeningStatusResponse::new(
            s.dudes.soul(&s.dudes.main()?.dude_code)?,
            &auth.user().user_code,
        ),
    )))
}
async fn begin(
    State(s): State<AppState>,
    auth: RequestAuthContext,
    request: Option<Json<BeginAwakeningRequest>>,
) -> Result<Json<ApiResponse<BeginAwakeningResponse>>, DudeApiError> {
    let (mut soul, play_entrance) = s
        .dudes
        .begin_awakening(&s.dudes.main()?.dude_code, &auth.user().user_code)?;
    if let Some(code) = &soul.session_code {
        let d = s.dudes.session(code)?.ok_or(DudeError::NotFound)?;
        s.chats
            .ensure_dude_session(&d, Some(auth.user().user_code.clone()))
            .map_err(|_| DudeError::Storage)?;
        let requests = s
            .chats
            .list_user_input_requests(code)
            .map_err(|_| DudeError::Storage)?;
        let native = s
            .agent_framework
            .pending_native_interactions(code)
            .await
            .map_err(|_| DudeError::Storage)?;
        if let Some(interaction) = native.iter().find(|i| {
            matches!(
                i.request.kind,
                eliteforge_agent_core::InteractionKind::Input
            )
        }) {
            s.dudes
                .record_soul_question(code, &interaction.request.id.to_string())?;
        }
        if let Some(question) = requests
            .iter()
            .find(|r| r.status == crate::domain::chat::ChatUserInputRequestStatus::Pending)
        {
            s.dudes.record_soul_question(code, &question.request_code)?;
        }
        let events = s.dudes.events(&soul.dude_code)?;
        let failed = events
            .iter()
            .filter(|e| {
                e.dedup_key.starts_with("soul-awakening:") && e.status == DudeEventStatus::Failed
            })
            .max_by_key(|e| e.created_at);
        let no_question = native.is_empty()
            && requests
                .iter()
                .all(|r| r.status != crate::domain::chat::ChatUserInputRequestStatus::Pending);
        let empty = s
            .chats
            .list_messages(code)
            .map_err(|_| DudeError::Storage)?
            .is_empty();
        if soul.state != SoulState::Ready && no_question && (empty || failed.is_some()) {
            s.dudes.publish_event(DudeEventInput {dude_code:soul.dude_code.clone(),source:DudeEventSource::UserMessage,
                dedup_key:failed.map(|e|format!("soul-awakening:{}:retry:{}",soul.dude_code,e.event_id)).unwrap_or_else(||format!("soul-awakening:{}",soul.dude_code)),session_code:Some(code.clone()),task_code:None,workspace_code:None,
                payload:serde_json::json!({"content":crate::application::dude::soul::awakening_message(&auth.locale),"locale":auth.locale,"userCode":auth.user().user_code,"onboarding":true,"a2ui":request.as_ref().and_then(|Json(r)|r.a2ui.as_ref())}),available_at:chrono::Utc::now().timestamp_millis()})?;
        }
    }
    soul = s.dudes.soul(&soul.dude_code)?;
    Ok(Json(ApiResponse::protocol_success(
        BeginAwakeningResponse::new(soul, &auth.user().user_code, play_entrance),
    )))
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BeginAwakeningRequest {
    a2ui: Option<AwakeningA2uiCapabilitiesRequest>,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AwakeningA2uiCapabilitiesRequest {
    version: String,
    catalog_ids: Vec<String>,
}
async fn defer(
    State(s): State<AppState>,
    auth: RequestAuthContext,
) -> Result<Json<ApiResponse<DeferAwakeningResponse>>, DudeApiError> {
    let soul = s.dudes.defer_awakening(&s.dudes.main()?.dude_code)?;
    Ok(Json(ApiResponse::protocol_success(
        DeferAwakeningResponse::new(soul, &auth.user().user_code),
    )))
}
