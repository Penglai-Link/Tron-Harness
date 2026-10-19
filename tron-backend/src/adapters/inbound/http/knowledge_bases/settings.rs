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
        let descriptor = self.0.library_deletion_descriptor();
        let (status, zh, en) = match self.0 {
            KnowledgeBaseError::NotFound => (
                StatusCode::NOT_FOUND,
                "知识库不存在",
                "Knowledge base not found",
            ),
            KnowledgeBaseError::Forbidden => (
                StatusCode::FORBIDDEN,
                "无知识库权限",
                "Access to this knowledge base is denied",
            ),
            KnowledgeBaseError::Invalid(_) => (
                StatusCode::BAD_REQUEST,
                "知识库名称、说明或索引配置无效",
                "Invalid knowledge base name, description or index settings",
            ),
            KnowledgeBaseError::Conflict => (
                StatusCode::CONFLICT,
                "知识库状态已改变，请刷新后重试",
                "Knowledge base changed; refresh and try again",
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "知识库配置操作失败，请刷新后重试",
                "Could not configure the knowledge base; refresh and try again",
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
