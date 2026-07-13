use std::collections::HashMap;

use axum::{
    Json,
    extract::{FromRequest, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, de::DeserializeOwned};

use crate::{
    application::prompt::{
        CreatePromptCommand, PromptPageQuery, PromptSpecCommand, TestPreviewCommand,
        UpdatePromptCommand,
    },
    domain::prompt::{PromptCategory, PromptViewSourceKind},
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
    pub model_code: Option<String>,
    pub temperature: Option<f64>,
    pub max_output_tokens: Option<u32>,
    pub enabled: bool,
    pub allow_inheritance: bool,
}

impl WorkspaceCreateRequest {
    pub fn into_parts(self) -> (String, CreatePromptCommand) {
        let command = CreatePromptCommand {
            category: self.category,
            spec: PromptSpecCommand {
                display_name: self.display_name,
                prompt_body: self.prompt_body,
                description: self.description,
                model_code: self.model_code,
                temperature: self.temperature,
                max_output_tokens: self.max_output_tokens,
                enabled: self.enabled,
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
    pub model_code: Option<String>,
    pub temperature: Option<f64>,
    pub max_output_tokens: Option<u32>,
    pub enabled: bool,
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
                model_code: self.model_code,
                temperature: self.temperature,
                max_output_tokens: self.max_output_tokens,
                enabled: self.enabled,
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
pub struct TestPreviewRequest {
    pub max_output_tokens: Option<u32>,
    pub workspace_code: Option<String>,
    pub prompt_code: Option<String>,
    pub prompt_body: Option<String>,
    pub temperature: Option<f64>,
    pub model_code: Option<String>,
    pub variables: HashMap<String, String>,
}

impl From<TestPreviewRequest> for TestPreviewCommand {
    fn from(request: TestPreviewRequest) -> Self {
        Self {
            max_output_tokens: request.max_output_tokens,
            workspace_code: request.workspace_code,
            prompt_code: request.prompt_code,
            prompt_body: request.prompt_body,
            temperature: request.temperature,
            model_code: request.model_code,
            variables: request.variables,
        }
    }
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
    pub model_code: Option<String>,
    pub temperature: Option<f64>,
    pub max_output_tokens: Option<u32>,
    pub enabled: bool,
}

impl From<PlatformCreateRequest> for CreatePromptCommand {
    fn from(request: PlatformCreateRequest) -> Self {
        Self {
            category: request.category,
            spec: PromptSpecCommand {
                display_name: request.display_name,
                prompt_body: request.prompt_body,
                description: request.description,
                model_code: request.model_code,
                temperature: request.temperature,
                max_output_tokens: request.max_output_tokens,
                enabled: request.enabled,
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
    pub model_code: Option<String>,
    pub temperature: Option<f64>,
    pub max_output_tokens: Option<u32>,
    pub enabled: bool,
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
                model_code: request.model_code,
                temperature: request.temperature,
                max_output_tokens: request.max_output_tokens,
                enabled: request.enabled,
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
