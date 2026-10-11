use super::error_mapper::rag_error_response;
use crate::{
    adapters::inbound::http::auth::RequestAuthContext, app::AppState, application::rag::*,
    response::ApiResponse,
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::{Value, json};

fn failure(error: impl std::fmt::Display) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiResponse::error(
            "RAG_OPERATION_FAILED",
            error.to_string(),
        )),
    )
        .into_response()
}
fn corpus_ids(
    state: &AppState,
    user: &str,
) -> Result<
    std::collections::BTreeSet<String>,
    crate::application::knowledge_base::KnowledgeBaseError,
> {
    Ok(state
        .knowledge_bases
        .platform_libraries(user)?
        .into_iter()
        .map(|kb| kb.id)
        .collect())
}
fn managed(state: &AppState, user: &str, id: &str) -> bool {
    matches!(id, "memory-card" | "skill")
        || corpus_ids(state, user).is_ok_and(|ids| ids.contains(id))
}
pub async fn corpora(State(state): State<AppState>, auth: RequestAuthContext) -> Response {
    let ids = match corpus_ids(&state, &auth.user().user_code) {
        Ok(ids) => ids,
        Err(error) => return failure(error),
    };
    match state.rag.corpus_summaries(&ids) {
        Ok(rows) => Json(ApiResponse::success(rows)).into_response(),
        Err(error) => rag_error_response(error),
    }
}
pub async fn configure(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(id): Path<String>,
    Json(settings): Json<RagSettings>,
) -> Response {
    if !managed(&state, &auth.user().user_code, &id) {
        return rag_error_response(crate::domain::rag::RagError::SearchScopeInvalid);
    }
    if settings.graph && id != "skill" && settings.graph_model_code.is_none() {
        return failure("开启图谱索引需要选择抽取模型");
    }
    match state.rag.configure_corpus(&id, settings) {
        Ok(settings) => Json(ApiResponse::success(settings)).into_response(),
        Err(error) => rag_error_response(error),
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchInput {
    pub workspace_code: String,
    pub agent_code: String,
    pub query: String,
    #[serde(default)]
    pub corpus_ids: Vec<String>,
    #[serde(default = "filters")]
    pub filters: Value,
    #[serde(default = "limit")]
    pub limit: usize,
}
fn filters() -> Value {
    json!({})
}
fn limit() -> usize {
    8
}
pub async fn search(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Json(input): Json<SearchInput>,
) -> Response {
    if !state
        .workspaces
        .workspace_is_active(&input.workspace_code)
        .unwrap_or(false)
    {
        return rag_error_response(crate::domain::rag::RagError::SearchScopeInvalid);
    }
    let identity = RagIdentity {
        user_code: auth.user().user_code.clone(),
        workspace_code: input.workspace_code,
        agent_code: input.agent_code,
    };
    match state
        .rag
        .search(
            &state.knowledge_bases,
            &identity,
            RagQueryInput {
                query: input.query,
                corpus_ids: input.corpus_ids,
                filters: input.filters,
                limit: input.limit,
            },
            None,
        )
        .await
    {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => rag_error_response(error),
    }
}
/// Management recall derives its entire scope from the authenticated user.
pub async fn recall(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Json(input): Json<RagQueryInput>,
) -> Response {
    match state
        .knowledge_bases
        .recall_for_user(&auth.user().user_code, input)
        .await
    {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => rag_error_response(error),
    }
}
pub async fn tasks(State(state): State<AppState>, auth: RequestAuthContext) -> Response {
    let user = &auth.user().user_code;
    let mut tasks = match state.rag.record_task_views(user) {
        Ok(tasks) => tasks,
        Err(error) => return rag_error_response(error),
    };
    match state.knowledge_bases.rag_task_views(user) {
        Ok(knowledge) => tasks.extend(knowledge),
        Err(error) => return failure(error),
    }
    Json(ApiResponse::success(tasks)).into_response()
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SubmitInput {
    pub corpus_id: String,
    #[serde(default)]
    pub document_id: Option<String>,
}
pub async fn submit(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Json(input): Json<SubmitInput>,
) -> Response {
    let user = &auth.user().user_code;
    if !managed(&state, user, &input.corpus_id) {
        return rag_error_response(crate::domain::rag::RagError::SearchScopeInvalid);
    }
    if input.corpus_id == "skill" {
        let indexer = crate::application::skill::SkillRagIndexService::new(
            state.skills.clone(),
            state.rag.clone(),
        );
        crate::app::telemetry::spawn_traced(async move {
            let _ = indexer.index_all().await;
        });
    } else if input.corpus_id == "memory-card" {
        let records = match state.rag.record_task_views(user) {
            Ok(records) => records,
            Err(error) => return rag_error_response(error),
        };
        let mut sources = std::collections::BTreeSet::new();
        for task in records
            .into_iter()
            .filter(|task| task.corpus_id == input.corpus_id)
        {
            if !sources.insert(task.source_code.clone())
                || matches!(task.state.as_str(), "RUNNING" | "QUEUED")
            {
                continue;
            }
            let queued = match state.rag.prepare_retry_record_task(user, &task.id, true) {
                Ok(task) => task,
                Err(error) => return rag_error_response(error),
            };
            let rag = state.rag.clone();
            crate::app::telemetry::spawn_traced(async move {
                let _ = rag.execute_record_task(queued).await;
            });
        }
    } else {
        let kb = match state.knowledge_bases.detail(user, &input.corpus_id) {
            Ok(kb) => kb,
            Err(error) => return failure(error),
        };
        if input
            .document_id
            .as_ref()
            .is_some_and(|id| !kb.documents.iter().any(|doc| doc.id == *id))
        {
            return failure("文档不存在");
        }
        for doc in kb
            .documents
            .iter()
            .filter(|doc| input.document_id.as_ref().is_none_or(|id| *id == doc.id))
        {
            if let Err(error) = state.knowledge_bases.start_index(user, &kb.id, &doc.id) {
                return failure(error);
            }
        }
    }
    (
        StatusCode::ACCEPTED,
        Json(ApiResponse::success(json!({"accepted":true}))),
    )
        .into_response()
}
pub async fn cancel(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(id): Path<String>,
) -> Response {
    control(state, auth, id, TaskControl::Cancel).await
}
pub async fn retry(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(id): Path<String>,
) -> Response {
    control(state, auth, id, TaskControl::Retry).await
}
pub async fn rebuild(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(id): Path<String>,
) -> Response {
    control(state, auth, id, TaskControl::Rebuild).await
}
#[derive(Clone, Copy)]
enum TaskControl {
    Cancel,
    Retry,
    Rebuild,
}
async fn control(
    state: AppState,
    auth: RequestAuthContext,
    id: String,
    action: TaskControl,
) -> Response {
    let user = &auth.user().user_code;
    let tasks = match state.knowledge_bases.rag_task_views(user) {
        Ok(tasks) => tasks,
        Err(error) => return failure(error),
    };
    if let Some(task) = tasks.into_iter().find(|task| task.id == id) {
        let doc = task.document_id.unwrap();
        // Reject an obsolete task action instead of cancelling a newer attempt.
        let current = match state
            .knowledge_bases
            .ingestion_tasks(user, &task.corpus_id, &doc)
        {
            Ok(tasks) => tasks,
            Err(error) => return failure(error),
        };
        if current.first().is_none_or(|task| task.id != id) {
            return failure("任务状态已改变，请刷新列表");
        }
        let result = match action {
            TaskControl::Retry => state
                .knowledge_bases
                .retry_index(user, &task.corpus_id, &doc),
            TaskControl::Rebuild => state
                .knowledge_bases
                .start_index(user, &task.corpus_id, &doc),
            TaskControl::Cancel => {
                state
                    .knowledge_bases
                    .cancel_index(user, &task.corpus_id, &doc)
                    .await
            }
        };
        return match result {
            Ok(result) => Json(ApiResponse::success(result)).into_response(),
            Err(error) => failure(error),
        };
    }
    let system_task = state
        .rag
        .repository_handle()
        .record_task("SYSTEM", &id)
        .ok()
        .flatten()
        .filter(|task| task.corpus_id == "skill");
    let owner = if system_task.is_some() {
        "SYSTEM"
    } else {
        user
    };
    if matches!(action, TaskControl::Retry | TaskControl::Rebuild) {
        if let Some(task) = system_task {
            if matches!(task.state.as_str(), "RUNNING" | "QUEUED") {
                return failure("任务仍在运行");
            }
            let name = task.payload["domainFields"]["skillName"]
                .as_str()
                .unwrap_or("")
                .to_owned();
            let indexer = crate::application::skill::SkillRagIndexService::new(
                state.skills.clone(),
                state.rag.clone(),
            );
            // Reload the registry and current Agent assignment before recreating evidence.
            return match indexer.index_registry_skill(&name).await {
                Ok(_) => Json(ApiResponse::success(json!({"accepted":true}))).into_response(),
                Err(error) => failure(error),
            };
        }
        let task = match state.rag.prepare_retry_record_task(
            owner,
            &id,
            matches!(action, TaskControl::Rebuild),
        ) {
            Ok(task) => task,
            Err(error) => return rag_error_response(error),
        };
        let task_id = task.id.clone();
        let rag = state.rag.clone();
        crate::app::telemetry::spawn_traced(async move {
            let _ = rag.execute_record_task(task).await;
        });
        (
            StatusCode::ACCEPTED,
            Json(ApiResponse::success(
                json!({"accepted":true,"taskId":task_id}),
            )),
        )
            .into_response()
    } else {
        match state.rag.cancel_record_task(owner, &id) {
            Ok(task) => Json(ApiResponse::success(task)).into_response(),
            Err(error) => rag_error_response(error),
        }
    }
}
