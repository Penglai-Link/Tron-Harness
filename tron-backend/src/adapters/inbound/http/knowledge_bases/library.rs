use super::*;
use crate::{application::knowledge_base::KnowledgeBase, response::EmptyData};
use serde::Serialize;

// Rust/FE consensus: camelCase counts (§2.1), REST DELETE (§2.2), dedicated DTOs
// and ApiResponse (§2.4–2.5), Accept-Language errors (§2.6).

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct KnowledgeLibraryListVo {
    id: String,
    name: String,
    description: String,
    document_count: usize,
    authorized_agent_count: usize,
}

impl From<KnowledgeBase> for KnowledgeLibraryListVo {
    fn from(kb: KnowledgeBase) -> Self {
        Self {
            id: kb.id,
            name: kb.name,
            description: kb.description,
            document_count: kb.document_count,
            authorized_agent_count: kb.agent_codes.len(),
        }
    }
}

pub(super) struct LibraryDeletionApiError(KnowledgeBaseError, String);
impl IntoResponse for LibraryDeletionApiError {
    fn into_response(self) -> Response {
        let descriptor = self.0.library_deletion_descriptor();
        let (status, zh, en) = match descriptor.message_key {
            "tron.knowledge_base.not_empty" => (
                StatusCode::CONFLICT,
                "请先删除知识库内的所有资源",
                "Delete all resources in the knowledge base first",
            ),
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
            "tron.knowledge_base.conflict" => (
                StatusCode::CONFLICT,
                "知识库状态已改变，请刷新后重试",
                "Knowledge base changed; refresh and try again",
            ),
            "tron.knowledge_base.invalid" => (
                StatusCode::BAD_REQUEST,
                "知识库请求无效",
                "Invalid knowledge base request",
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "知识库删除失败",
                "Could not delete the knowledge base",
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

pub(super) async fn remove(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<EmptyData>>, LibraryDeletionApiError> {
    kb.remove_library(&auth.user().user_code, &id)
        .map_err(|error| LibraryDeletionApiError(error, auth.locale.clone()))?;
    Ok(Json(ApiResponse::protocol_success(EmptyData {})))
}
