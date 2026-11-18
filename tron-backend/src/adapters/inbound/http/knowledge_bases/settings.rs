use super::*;
use crate::application::{knowledge_base::KnowledgeLibrarySettings, rag::RagSettings};
use crate::response::EmptyData;

// Rust/FE consensus §§2.1, 2.2, 2.4–2.6: dedicated DTO/VO and localized errors.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct UpdateSettingsRequest {
    name: String,
    description: String,
    default_parse_preference: crate::application::knowledge_base::DocumentParsePreference,
    index_settings: RagSettings,
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ReadSettingsView {
    name: String,
    description: String,
    default_parse_preference: crate::application::knowledge_base::DocumentParsePreference,
    index_settings: RagSettings,
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct UpdatedSettingsView {
    name: String,
    description: String,
    default_parse_preference: crate::application::knowledge_base::DocumentParsePreference,
    index_settings: RagSettings,
}
pub(super) struct SettingsApiError(KnowledgeBaseError, String);
impl IntoResponse for SettingsApiError {
    fn into_response(self) -> Response {
        use tron_i18n::t;
        let descriptor = self.0.library_deletion_descriptor();
        let status = match &self.0 {
            KnowledgeBaseError::NotFound => StatusCode::NOT_FOUND,
            KnowledgeBaseError::Forbidden => StatusCode::FORBIDDEN,
            KnowledgeBaseError::Invalid(_) => StatusCode::BAD_REQUEST,
            KnowledgeBaseError::Conflict => StatusCode::CONFLICT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let message = match &self.0 {
            KnowledgeBaseError::NotFound => {
                t!("tron.knowledge_base.not_found", locale = &self.1)
            }
            KnowledgeBaseError::Forbidden => {
                t!("tron.knowledge_base.forbidden", locale = &self.1)
            }
            KnowledgeBaseError::Invalid(_) => {
                // Structured validation detail is rendered at the display edge.
                use std::borrow::Cow;
                Cow::Owned(self.0.localized(&self.1))
            }
            KnowledgeBaseError::Conflict => {
                t!("tron.knowledge_base.conflict", locale = &self.1)
            }
            _ => t!("knowledge.base.settings.failed", locale = &self.1),
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
pub(super) async fn get(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<ReadSettingsView>>, SettingsApiError> {
    let value = kb
        .library_settings(&auth.user().user_code, &id)
        .map_err(|e| SettingsApiError(e, auth.locale.clone()))?;
    Ok(Json(ApiResponse::protocol_success(ReadSettingsView {
        name: value.name,
        description: value.description,
        default_parse_preference: value.default_parse_preference,
        index_settings: value.index_settings,
    })))
}
pub(super) async fn update(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path(id): Path<String>,
    Json(input): Json<UpdateSettingsRequest>,
) -> Result<Json<ApiResponse<UpdatedSettingsView>>, SettingsApiError> {
    let value = kb
        .update_library_settings(
            &auth.user().user_code,
            &id,
            KnowledgeLibrarySettings {
                name: input.name,
                description: input.description,
                default_parse_preference: input.default_parse_preference,
                index_settings: input.index_settings,
            },
        )
        .map_err(|e| SettingsApiError(e, auth.locale.clone()))?;
    Ok(Json(ApiResponse::protocol_success(UpdatedSettingsView {
        name: value.name,
        description: value.description,
        default_parse_preference: value.default_parse_preference,
        index_settings: value.index_settings,
    })))
}
