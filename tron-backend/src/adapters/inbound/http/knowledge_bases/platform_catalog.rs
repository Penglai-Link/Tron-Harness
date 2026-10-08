use super::*;
use crate::response::EmptyData;
use serde::Serialize;
pub(super) struct PlatformCatalogApiError(KnowledgeBaseError, String);
impl IntoResponse for PlatformCatalogApiError {
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
                "知识库请求无效",
                "Invalid knowledge base request",
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "知识库列表加载失败",
                "Could not load knowledge bases",
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
