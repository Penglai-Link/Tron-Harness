use super::*;
use crate::response::EmptyData;
use serde::Serialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ReplaceRequest {
    library_ids: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AgentLibraryVo {
    id: String,
    name: String,
    description: String,
    document_count: usize,
    granted: bool,
}

pub(super) struct AgentGrantsApiError(KnowledgeBaseError, String);
impl IntoResponse for AgentGrantsApiError {
    fn into_response(self) -> Response {
        let descriptor = self.0.library_deletion_descriptor();
        let (status, zh, en) = match descriptor.message_key {
            "tron.knowledge_base.not_found" => (
                StatusCode::NOT_FOUND,
                "知识库不存在",
                "Knowledge base not found",
            ),
            "tron.knowledge_base.forbidden" => (
                StatusCode::FORBIDDEN,
                "无知识库权限",
                "Access to this knowledge base is denied",
            ),
            "tron.knowledge_base.invalid" => (
                StatusCode::BAD_REQUEST,
                "请选择已定义的内部智能体",
                "Choose a defined internal agent",
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "知识库关联操作失败",
                "Could not update knowledge base assignments",
            ),
        };
        (
            status,
            Json(ApiResponse::<EmptyData>::protocol_error(
                descriptor.code,
                if self.1.starts_with("en") { en } else { zh },
            )),
        )
            .into_response()
    }
}

pub(super) async fn list(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path(agent): Path<String>,
) -> Result<Json<ApiResponse<Vec<AgentLibraryVo>>>, AgentGrantsApiError> {
    let libraries = kb
        .list_agent_knowledge_libraries(&auth.user().user_code, &agent)
        .map_err(|error| AgentGrantsApiError(error, auth.locale.clone()))?;
    Ok(Json(ApiResponse::protocol_success(
        libraries
            .into_iter()
            .map(|library| AgentLibraryVo {
                granted: library.agent_codes.contains(&agent),
                id: library.id,
                name: library.name,
                description: library.description,
                document_count: library.document_count,
            })
            .collect(),
    )))
}

pub(super) async fn replace(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path(agent): Path<String>,
    Json(body): Json<ReplaceRequest>,
) -> Result<Json<ApiResponse<EmptyData>>, AgentGrantsApiError> {
    kb.set_agent_knowledge_libraries(
        &auth.user().user_code,
        &agent,
        &body.library_ids,
    )
    .map_err(|error| AgentGrantsApiError(error, auth.locale.clone()))?;
    Ok(Json(ApiResponse::protocol_success(EmptyData {})))
}
