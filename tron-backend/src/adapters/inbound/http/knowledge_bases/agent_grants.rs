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
        use tron_i18n::t;
        let descriptor = self.0.library_deletion_descriptor();
        let status = match descriptor.message_key {
            "tron.knowledge_base.not_found" => StatusCode::NOT_FOUND,
            "tron.knowledge_base.forbidden" => StatusCode::FORBIDDEN,
            "tron.knowledge_base.invalid" => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let message = match descriptor.message_key {
            "tron.knowledge_base.not_found" => {
                t!("tron.knowledge_base.not_found", locale = &self.1)
            }
            "tron.knowledge_base.forbidden" => {
                t!("tron.knowledge_base.forbidden", locale = &self.1)
            }
            "tron.knowledge_base.invalid" => {
                t!("knowledge.base.agent_grants.invalid", locale = &self.1)
            }
            _ => t!("knowledge.base.agent_grants.failed", locale = &self.1),
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
    kb.set_agent_knowledge_libraries(&auth.user().user_code, &agent, &body.library_ids)
        .map_err(|error| AgentGrantsApiError(error, auth.locale.clone()))?;
    Ok(Json(ApiResponse::protocol_success(EmptyData {})))
}
