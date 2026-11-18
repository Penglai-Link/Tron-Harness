use super::*;
use crate::response::EmptyData;

pub(super) struct LibraryAgentGrantsApiError(KnowledgeBaseError, String);
impl IntoResponse for LibraryAgentGrantsApiError {
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
                t!(
                    "knowledge.base.library_agent_grants.invalid",
                    locale = &self.1
                )
            }
            _ => t!(
                "knowledge.base.library_agent_grants.failed",
                locale = &self.1
            ),
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
