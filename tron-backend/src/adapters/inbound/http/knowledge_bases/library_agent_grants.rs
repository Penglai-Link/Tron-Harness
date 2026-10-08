use super::*;
use crate::response::EmptyData;

pub(super) struct LibraryAgentGrantsApiError(KnowledgeBaseError, String);
impl IntoResponse for LibraryAgentGrantsApiError {
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
                "请选择平台内部智能体",
                "Choose an internal agent on the platform",
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "知识库智能体授权失败",
                "Could not update knowledge base agent authorization",
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct LibraryAgentGrantsRequest {
    agent_codes: Vec<String>,
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct LibraryAgentGrantsVo {
    agent_codes: Vec<String>,
}
pub(super) async fn get_grants(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<LibraryAgentGrantsVo>>, LibraryAgentGrantsApiError> {
    let agent_codes = kb
        .library_agent_grants(&auth.user().user_code, &id)
        .map_err(|error| LibraryAgentGrantsApiError(error, auth.locale))?;
    Ok(Json(ApiResponse::protocol_success(LibraryAgentGrantsVo {
        agent_codes,
    })))
}
pub(super) async fn replace(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path(id): Path<String>,
    Json(body): Json<LibraryAgentGrantsRequest>,
) -> Result<Json<ApiResponse<EmptyData>>, LibraryAgentGrantsApiError> {
    kb.assign_library_agents(&auth.user().user_code, &id, &body.agent_codes)
        .map_err(|error| LibraryAgentGrantsApiError(error, auth.locale))?;
    Ok(Json(ApiResponse::protocol_success(EmptyData {})))
}
