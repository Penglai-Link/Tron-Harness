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
        use tron_i18n::t;
        let descriptor = self.0.library_deletion_descriptor();
        let status = match &self.0 {
            KnowledgeBaseError::Forbidden => StatusCode::FORBIDDEN,
            KnowledgeBaseError::NotFound => StatusCode::NOT_FOUND,
            KnowledgeBaseError::Conflict => StatusCode::CONFLICT,
            KnowledgeBaseError::Invalid(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let message: String = match &self.0 {
            KnowledgeBaseError::Forbidden => {
                t!("tron.knowledge_base.forbidden", locale = &self.1).into_owned()
            }
            KnowledgeBaseError::NotFound => {
                t!("tron.knowledge_base.not_found", locale = &self.1).into_owned()
            }
            KnowledgeBaseError::Conflict => {
                t!("knowledge.preflight.conflict", locale = &self.1).into_owned()
            }
            // The inspection error already carries a stable locale-independent
            // message; render it with the request locale instead of forwarding text.
            KnowledgeBaseError::Invalid(_) | KnowledgeBaseError::Storage(_) => {
                self.0.localized(&self.1)
            }
            KnowledgeBaseError::NotEmpty => {
                t!("tron.knowledge_base.not_empty", locale = &self.1).into_owned()
            }
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
    use tron_i18n::t;
    let locale = auth.locale.clone();
    let error = |message: String| {
        PreflightApiError(KnowledgeBaseError::from_detail(message), locale.clone())
    };
    let mut file = None;
    let mut preference = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| error(t!("knowledge.preflight.form_invalid", locale = &locale).into_owned()))?
    {
        match field.name().unwrap_or("") {
            "file" => {
                let name = field.file_name().unwrap_or_default().to_owned();
                if file.is_some() {
                    return Err(error(
                        t!("knowledge.preflight.single_file", locale = &locale).into_owned(),
                    ));
                }
                file = Some((
                    name,
                    field.bytes().await.map_err(|_| {
                        error(
                            t!("knowledge.preflight.file_unreadable", locale = &locale)
                                .into_owned(),
                        )
                    })?,
                ));
            }
            "parsePreference" => {
                preference = match field.text().await.ok().and_then(|v| {
                    serde_json::from_value::<DocumentParsePreference>(serde_json::Value::String(v))
                        .ok()
                }) {
                    Some(value) => Some(value),
                    None => {
                        return Err(error(
                            t!("knowledge.preflight.preference_invalid", locale = &locale)
                                .into_owned(),
                        ));
                    }
                };
            }
            _ => {}
        }
    }
    let (name, bytes) = file.ok_or_else(|| {
        error(t!("knowledge.preflight.file_required", locale = &locale).into_owned())
    })?;
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
        _ => Err(error(
            t!("knowledge.preflight.timeout", locale = &locale).into_owned(),
        )),
    }
}
