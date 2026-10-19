use super::*;
use crate::application::document::preflight::{DocumentExtractionMethod, DocumentFeatures};
use crate::application::knowledge_base::{DocumentParsePlan, DocumentParsePreference};
use crate::response::EmptyData;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PreflightView {
    plan: PreflightPlanView,
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PreflightPlanView {
    preference: DocumentParsePreference,
    method: DocumentExtractionMethod,
    reason: String,
    rules_version: String,
    features: PreflightFeaturesView,
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PreflightFeaturesView {
    format: String,
    source_sha256: String,
    units: usize,
    inspected_units: usize,
    text_characters: usize,
    image_only_units: usize,
    images: usize,
    tables: usize,
    formulas: usize,
    charts: usize,
    text_boxes: usize,
    columns: usize,
    uncertain: bool,
    local_text: bool,
}
impl From<DocumentFeatures> for PreflightFeaturesView {
    fn from(f: DocumentFeatures) -> Self {
        Self {
            format: f.format,
            source_sha256: f.source_sha256,
            units: f.units,
            inspected_units: f.inspected_units,
            text_characters: f.text_characters,
            image_only_units: f.image_only_units,
            images: f.images,
            tables: f.tables,
            formulas: f.formulas,
            charts: f.charts,
            text_boxes: f.text_boxes,
            columns: f.columns,
            uncertain: f.uncertain,
            local_text: f.local_text,
        }
    }
}
impl From<DocumentParsePlan> for PreflightPlanView {
    fn from(p: DocumentParsePlan) -> Self {
        Self {
            preference: p.preference,
            method: p.method,
            reason: p.reason,
            rules_version: p.rules_version,
            features: p.features.into(),
        }
    }
}
pub(super) struct PreflightApiError(KnowledgeBaseError, String);
impl IntoResponse for PreflightApiError {
    fn into_response(self) -> Response {
        let descriptor = self.0.library_deletion_descriptor();
        let (status, english) = match self.0 {
            KnowledgeBaseError::Forbidden => (
                StatusCode::FORBIDDEN,
                "Access to this knowledge base is denied",
            ),
            KnowledgeBaseError::NotFound => (StatusCode::NOT_FOUND, "Knowledge base not found"),
            KnowledgeBaseError::Conflict => {
                (StatusCode::CONFLICT, "File checking is busy. Please retry.")
            }
            KnowledgeBaseError::Invalid(_) => (
                StatusCode::BAD_REQUEST,
                "The file or parsing choice is invalid. No parsing task was submitted.",
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "File checking failed. No parsing task was submitted.",
            ),
        };
        let message = if self.1.starts_with("en") {
            english.to_owned()
        } else {
            self.0.to_string()
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

pub(super) async fn inspect(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path(id): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<PreflightView>>, PreflightApiError> {
    let locale = auth.locale.clone();
    let error = |message: &str| {
        PreflightApiError(KnowledgeBaseError::Invalid(message.into()), locale.clone())
    };
    let mut file = None;
    let mut preference = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| error("上传表单无效"))?
    {
        match field.name().unwrap_or("") {
            "file" => {
                let name = field.file_name().unwrap_or_default().to_owned();
                if file.is_some() {
                    return Err(error("每次检查只允许一个文件"));
                }
                file = Some((
                    name,
                    field
                        .bytes()
                        .await
                        .map_err(|_| error("文件超过上传限制或读取失败"))?,
                ));
            }
            "parsePreference" => {
                preference = match field.text().await.ok().and_then(|v| {
                    serde_json::from_value::<DocumentParsePreference>(serde_json::Value::String(v))
                        .ok()
                }) {
                    Some(value) => Some(value),
                    None => return Err(error("解析方式无效")),
                };
            }
            _ => {}
        }
    }
    let (name, bytes) = file.ok_or_else(|| error("需要file"))?;
    // Bound CPU-heavy inspections across requests; a timed-out inspection keeps
    // its permit until the worker exits and can never create a supplier task.
    static INSPECTIONS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);
    let permit = INSPECTIONS
        .try_acquire()
        .map_err(|_| PreflightApiError(KnowledgeBaseError::Conflict, locale.clone()))?;
    let user = auth.user().user_code.clone();
    let result = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        kb.preflight_document(&user, &id, &name, &bytes, preference)
    });
    match tokio::time::timeout(std::time::Duration::from_secs(10), result).await {
        Ok(Ok(plan)) => Ok(Json(ApiResponse::protocol_success(PreflightView {
            plan: plan
                .map_err(|e| PreflightApiError(e, locale.clone()))?
                .into(),
        }))),
        _ => Err(error("文件检查超时或失败；尚未提交解析")),
    }
}
