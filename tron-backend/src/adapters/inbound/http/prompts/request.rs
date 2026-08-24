use axum::{
    Json,
    extract::{FromRequest, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, de::DeserializeOwned};

use crate::{
    application::prompt::{
        CreatePromptCommand, PromptPageQuery, PromptSpecCommand, PromptTranslationSpecCommand,
        PromptVariableSpecCommand, UpdatePromptCommand,
    },
    domain::prompt::{PromptCategory, PromptVariableType, PromptViewSourceKind},
};

use super::response::PromptApiResponse;

pub struct PromptJson<T>(pub T);

impl<S, T> FromRequest<S> for PromptJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = Response;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        Json::<T>::from_request(request, state)
            .await
            .map(|Json(value)| Self(value))
            .map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(PromptApiResponse::error(
                        "PROMPT_REQUEST_INVALID",
                        "prompt request body is invalid",
                    )),
                )
                    .into_response()
            })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspacePageRequest {
    pub workspace_code: String,
    pub current: usize,
    pub size: usize,
    pub search: Option<String>,
    pub category: Option<PromptCategory>,
    pub source_kind: Option<PromptViewSourceKind>,
    pub enabled: Option<bool>,
}

impl WorkspacePageRequest {
    pub fn into_parts(self) -> (String, PromptPageQuery) {
        let query = PromptPageQuery {
            current: self.current,
            size: self.size,
            search: self.search,
            category: self.category,
            source_kind: self.source_kind,
            enabled: self.enabled,
        };
        (self.workspace_code, query)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceDetailRequest {
    pub workspace_code: String,
    pub prompt_code: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceCreateRequest {
    pub workspace_code: String,
    pub display_name: String,
    pub category: PromptCategory,
    pub prompt_body: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub allow_inheritance: bool,
    pub variables: Vec<PromptVariableRequest>,
    #[serde(default)]
    pub translations: Vec<PromptTranslationRequest>,
}

impl WorkspaceCreateRequest {
    pub fn into_parts(self) -> (String, CreatePromptCommand) {
        let command = CreatePromptCommand {
            category: self.category,
            spec: PromptSpecCommand {
                display_name: self.display_name,
                prompt_body: self.prompt_body,
                description: self.description,
                enabled: self.enabled,
                variables: variable_commands(self.variables),
                translations: translation_commands(self.translations),
            },
            allow_inheritance: Some(self.allow_inheritance),
        };
        (self.workspace_code, command)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceUpdateRequest {
    pub allow_inheritance: Option<bool>,
    pub workspace_code: String,
    pub prompt_code: String,
    pub expected_version: u64,
    pub display_name: String,
    pub prompt_body: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub variables: Vec<PromptVariableRequest>,
    #[serde(default)]
    pub translations: Vec<PromptTranslationRequest>,
}

impl WorkspaceUpdateRequest {
    pub fn into_parts(self) -> (String, UpdatePromptCommand) {
        let command = UpdatePromptCommand {
            allow_inheritance: self.allow_inheritance,
            prompt_code: self.prompt_code,
            expected_version: self.expected_version,
            spec: PromptSpecCommand {
                display_name: self.display_name,
                prompt_body: self.prompt_body,
                description: self.description,
                enabled: self.enabled,
                variables: variable_commands(self.variables),
                translations: translation_commands(self.translations),
            },
        };
        (self.workspace_code, command)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceDuplicateRequest {
    pub workspace_code: String,
    pub prompt_code: String,
    pub target_category: PromptCategory,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceDeleteRequest {
    pub workspace_code: String,
    pub prompt_code: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceRevertRequest {
    pub workspace_code: String,
    pub prompt_code: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceSetEnabledRequest {
    pub workspace_code: String,
    pub prompt_code: String,
    pub enabled: bool,
    pub expected_version: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceSetAllowInheritanceRequest {
    pub workspace_code: String,
    pub prompt_code: String,
    pub allow_inheritance: bool,
    pub expected_version: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnalyzeTemplateRequest {
    pub prompt_body: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TranslateDraftRequest {
    pub prompt_code: Option<String>,
    pub source_locale: String,
    pub target_locale: String,
    pub display_name: String,
    pub prompt_body: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PromptVariableRequest {
    pub name: String,
    #[serde(rename = "type", default = "default_prompt_variable_type")]
    pub variable_type: PromptVariableType,
    pub description: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PromptTranslationRequest {
    pub locale: String,
    pub display_name: String,
    pub prompt_body: String,
    pub description: Option<String>,
    #[serde(default = "default_translation_status")]
    pub status: crate::domain::prompt::PromptTranslationStatus,
}

fn default_translation_status() -> crate::domain::prompt::PromptTranslationStatus {
    crate::domain::prompt::PromptTranslationStatus::Published
}

fn default_prompt_variable_type() -> PromptVariableType {
    PromptVariableType::String
}

fn variable_commands(variables: Vec<PromptVariableRequest>) -> Vec<PromptVariableSpecCommand> {
    variables
        .into_iter()
        .map(|variable| PromptVariableSpecCommand {
            name: variable.name,
            variable_type: variable.variable_type,
            description: variable.description,
        })
        .collect()
}

fn translation_commands(
    translations: Vec<PromptTranslationRequest>,
) -> Vec<PromptTranslationSpecCommand> {
    translations
        .into_iter()
        .map(|translation| PromptTranslationSpecCommand {
            locale: translation.locale,
            display_name: translation.display_name,
            prompt_body: translation.prompt_body,
            description: translation.description,
            status: translation.status,
        })
        .collect()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceExportRequest {
    pub workspace_code: String,
    pub current: usize,
    pub size: usize,
    pub search: Option<String>,
    pub category: Option<PromptCategory>,
    pub source_kind: Option<PromptViewSourceKind>,
    pub enabled: Option<bool>,
}

impl WorkspaceExportRequest {
    pub fn into_parts(self) -> (String, PromptPageQuery) {
        let query = PromptPageQuery {
            current: self.current,
            size: self.size,
            search: self.search,
            category: self.category,
            source_kind: self.source_kind,
            enabled: self.enabled,
        };
        (self.workspace_code, query)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlatformPageRequest {
    pub current: usize,
    pub size: usize,
    pub search: Option<String>,
    pub category: Option<PromptCategory>,
    pub source_kind: Option<PromptViewSourceKind>,
    pub enabled: Option<bool>,
}

impl From<PlatformPageRequest> for PromptPageQuery {
    fn from(request: PlatformPageRequest) -> Self {
        Self {
            current: request.current,
            size: request.size,
            search: request.search,
            category: request.category,
            source_kind: request.source_kind,
            enabled: request.enabled,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlatformDetailRequest {
    pub prompt_code: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlatformCreateRequest {
    pub display_name: String,
    pub category: PromptCategory,
    pub prompt_body: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub variables: Vec<PromptVariableRequest>,
    #[serde(default)]
    pub translations: Vec<PromptTranslationRequest>,
}

impl From<PlatformCreateRequest> for CreatePromptCommand {
    fn from(request: PlatformCreateRequest) -> Self {
        Self {
            category: request.category,
            spec: PromptSpecCommand {
                display_name: request.display_name,
                prompt_body: request.prompt_body,
                description: request.description,
                enabled: request.enabled,
                variables: variable_commands(request.variables),
                translations: translation_commands(request.translations),
            },
            allow_inheritance: None,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlatformUpdateRequest {
    pub prompt_code: String,
    pub expected_version: u64,
    pub display_name: String,
    pub prompt_body: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub variables: Vec<PromptVariableRequest>,
    #[serde(default)]
    pub translations: Vec<PromptTranslationRequest>,
}

impl From<PlatformUpdateRequest> for UpdatePromptCommand {
    fn from(request: PlatformUpdateRequest) -> Self {
        Self {
            allow_inheritance: None,
            prompt_code: request.prompt_code,
            expected_version: request.expected_version,
            spec: PromptSpecCommand {
                display_name: request.display_name,
                prompt_body: request.prompt_body,
                description: request.description,
                enabled: request.enabled,
                variables: variable_commands(request.variables),
                translations: translation_commands(request.translations),
            },
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlatformDuplicateRequest {
    pub prompt_code: String,
    pub target_category: PromptCategory,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlatformDeleteRequest {
    pub prompt_code: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlatformSetEnabledRequest {
    pub prompt_code: String,
    pub enabled: bool,
    pub expected_version: u64,
}
