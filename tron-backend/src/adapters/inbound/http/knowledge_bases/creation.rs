use super::*;
use crate::{application::rag::RagSettings, response::EmptyData};

// Rust/FE consensus §§2.1,2.2,2.4–2.6: one POST with camelCase settings,
// dedicated request/response DTOs, protocol codes and request-localized failures.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct CreateRequest {
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default = "RagSettings::knowledge_base_default")]
    index_settings: RagSettings,
    #[serde(default)]
    default_parse_preference: crate::application::knowledge_base::DocumentParsePreference,
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CreatedLibraryView {
    id: String,
    name: String,
    description: String,
    document_count: usize,
}
pub(super) struct CreateLibraryApiError(KnowledgeBaseError, String);
impl IntoResponse for CreateLibraryApiError {
    fn into_response(self) -> Response {
        use tron_i18n::t;
        let descriptor = self.0.library_deletion_descriptor();
        let status = match descriptor.message_key {
            "tron.knowledge_base.invalid" => StatusCode::BAD_REQUEST,
            "tron.knowledge_base.forbidden" => StatusCode::FORBIDDEN,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let message = match descriptor.message_key {
            "tron.knowledge_base.invalid" => {
                t!("knowledge.base.create.invalid", locale = &self.1)
            }
            "tron.knowledge_base.forbidden" => {
                t!("tron.knowledge_base.forbidden", locale = &self.1)
            }
            _ => t!("knowledge.base.create.failed", locale = &self.1),
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
pub(super) async fn create(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Json(input): Json<CreateRequest>,
) -> Result<Json<ApiResponse<CreatedLibraryView>>, CreateLibraryApiError> {
    let library = kb
        .create_with_parsing_settings(
            &auth.user().user_code,
            &input.name,
            &input.description,
            input.index_settings,
            input.default_parse_preference,
        )
        .map_err(|error| CreateLibraryApiError(error, auth.locale.clone()))?;
    Ok(Json(ApiResponse::protocol_success(CreatedLibraryView {
        id: library.id,
        name: library.name,
        description: library.description,
        document_count: library.document_count,
    })))
}
