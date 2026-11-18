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
        use tron_i18n::t;
        let descriptor = self.0.library_deletion_descriptor();
        let status = match descriptor.message_key {
            "tron.knowledge_base.not_empty" | "tron.knowledge_base.conflict" => {
                StatusCode::CONFLICT
            }
            "tron.knowledge_base.not_found" => StatusCode::NOT_FOUND,
            "tron.knowledge_base.forbidden" => StatusCode::FORBIDDEN,
            "tron.knowledge_base.invalid" => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let message = match descriptor.message_key {
            "tron.knowledge_base.not_empty" => {
                t!("tron.knowledge_base.not_empty", locale = &self.1)
            }
            "tron.knowledge_base.not_found" => {
                t!("tron.knowledge_base.not_found", locale = &self.1)
            }
            "tron.knowledge_base.forbidden" => {
                t!("tron.knowledge_base.forbidden", locale = &self.1)
            }
            "tron.knowledge_base.conflict" => {
                t!("tron.knowledge_base.conflict", locale = &self.1)
            }
            "tron.knowledge_base.invalid" => {
                t!("tron.knowledge_base.invalid", locale = &self.1)
            }
            _ => t!("knowledge.base.delete.failed", locale = &self.1),
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

pub(super) async fn remove(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<EmptyData>>, LibraryDeletionApiError> {
    kb.remove_library(&auth.user().user_code, &id)
        .map_err(|error| LibraryDeletionApiError(error, auth.locale.clone()))?;
    Ok(Json(ApiResponse::protocol_success(EmptyData {})))
}
