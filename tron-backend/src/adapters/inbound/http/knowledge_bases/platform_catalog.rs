use super::*;
use crate::response::EmptyData;
use serde::Serialize;
pub(super) struct PlatformCatalogApiError(KnowledgeBaseError, String);
impl IntoResponse for PlatformCatalogApiError {
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
                t!("tron.knowledge_base.invalid", locale = &self.1)
            }
            _ => t!("knowledge.base.catalog.failed", locale = &self.1),
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PlatformLibraryVo {
    id: String,
    name: String,
    description: String,
    document_count: usize,
    authorized_agent_count: usize,
}
pub(super) async fn list(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
) -> Result<Json<ApiResponse<Vec<PlatformLibraryVo>>>, PlatformCatalogApiError> {
    let user = &auth.user().user_code;
    let libraries = kb
        .platform_libraries(user)
        .map_err(|error| PlatformCatalogApiError(error, auth.locale.clone()))?;
    let items = libraries
        .into_iter()
        .map(|library| PlatformLibraryVo {
            id: library.id,
            name: library.name,
            description: library.description,
            document_count: library.document_count,
            authorized_agent_count: library.agent_codes.len(),
        })
        .collect();
    Ok(Json(ApiResponse::protocol_success(items)))
}
