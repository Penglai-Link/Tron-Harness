use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::domain::resource::ResourceError;

use super::ResourceHttpApiResponse;

/// The Resource business-code range is registered outside this repository.  No
/// provisional resource code is minted here: before that registration lands,
/// all Resource failures deliberately use the platform's reserved internal
/// error code.  The status and localized message remain precise, and this one
/// seam is the only place that needs the assigned range later.
const UNREGISTERED_RESOURCE_ERROR_CODE: &str = "-1";

pub fn resource_error_response(error: ResourceError, locale: &str) -> Response {
    let status = match &error {
        ResourceError::NotFound | ResourceError::MissingBinaryObject => StatusCode::NOT_FOUND,
        ResourceError::Forbidden => StatusCode::FORBIDDEN,
        ResourceError::Conflict
        | ResourceError::DraftRevisionConflict
        | ResourceError::ActiveVersionConflict
        | ResourceError::DefinitionDisabled
        | ResourceError::DefinitionInUse
        | ResourceError::BuiltinDefinition => StatusCode::CONFLICT,
        ResourceError::StorageFailure => StatusCode::INTERNAL_SERVER_ERROR,
        ResourceError::WorkspaceRequired
        | ResourceError::InvalidWorkspace
        | ResourceError::InvalidScope
        | ResourceError::InvalidCode
        | ResourceError::InvalidVersion
        | ResourceError::InvalidInput
        | ResourceError::InvalidSchema
        | ResourceError::InvalidBinarySchema
        | ResourceError::InvalidVariableReference
        | ResourceError::ValidationFailed
        | ResourceError::LifecycleDenied
        | ResourceError::PathDenied
        | ResourceError::StorageProfileNotFound
        | ResourceError::StorageLocatorInvalid => StatusCode::BAD_REQUEST,
    };
    (
        status,
        Json(ResourceHttpApiResponse::error(
            UNREGISTERED_RESOURCE_ERROR_CODE,
            localized_resource_error(error, locale),
        )),
    )
        .into_response()
}

pub fn malformed_request_response(status: StatusCode, locale: &str) -> Response {
    (
        status,
        Json(ResourceHttpApiResponse::error(
            UNREGISTERED_RESOURCE_ERROR_CODE,
            localized_message(
                locale,
                "请求参数或请求体格式不正确",
                "The request parameters or body are invalid",
            ),
        )),
    )
        .into_response()
}

pub fn internal_error_response(locale: &str) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ResourceHttpApiResponse::error(
            "-1",
            localized_message(
                locale,
                "资源服务内部错误",
                "Resource service failed internally",
            ),
        )),
    )
        .into_response()
}

fn localized_resource_error(error: ResourceError, locale: &str) -> String {
    let (zh, en) = match error {
        ResourceError::NotFound => (
            "资源或资源定义不存在",
            "The resource or definition was not found",
        ),
        ResourceError::Forbidden => (
            "无权执行资源操作",
            "You are not authorized to manage this resource",
        ),
        ResourceError::WorkspaceRequired => ("必须提供工作空间编码", "workspaceCode is required"),
        ResourceError::InvalidWorkspace => ("工作空间不可用", "The workspace is not active"),
        ResourceError::InvalidScope => ("资源作用域不正确", "The resource scope is invalid"),
        ResourceError::InvalidCode => (
            "资源业务编码不正确",
            "The resource business code is invalid",
        ),
        ResourceError::InvalidVersion => ("资源版本不正确", "The resource version is invalid"),
        ResourceError::InvalidInput => ("资源请求参数不正确", "The resource request is invalid"),
        ResourceError::InvalidSchema => (
            "资源结构化属性定义不正确",
            "The resource attribute schema is invalid",
        ),
        ResourceError::InvalidBinarySchema => (
            "资源二进制约束不正确",
            "The resource binary constraints are invalid",
        ),
        ResourceError::InvalidVariableReference => (
            "资源变量引用不正确",
            "The resource Variable reference is invalid",
        ),
        ResourceError::ValidationFailed => {
            ("资源未通过校验", "The resource did not pass validation")
        }
        ResourceError::Conflict => (
            "资源状态发生冲突",
            "The resource state conflicts with this operation",
        ),
        ResourceError::DraftRevisionConflict => (
            "草稿已被其他修改覆盖",
            "The draft revision no longer matches",
        ),
        ResourceError::ActiveVersionConflict => {
            ("当前生效版本已变化", "The active version no longer matches")
        }
        ResourceError::LifecycleDenied => (
            "不允许此资源生命周期操作",
            "This resource lifecycle operation is not allowed",
        ),
        ResourceError::DefinitionDisabled => {
            ("资源定义已禁用", "The resource definition is disabled")
        }
        ResourceError::DefinitionInUse => (
            "资源定义仍被资源版本引用",
            "The resource definition is still referenced by resource versions",
        ),
        ResourceError::BuiltinDefinition => (
            "内置资源定义不能删除",
            "Built-in resource definitions cannot be deleted",
        ),
        ResourceError::StorageFailure => {
            ("资源存储操作失败", "The resource storage operation failed")
        }
        ResourceError::MissingBinaryObject => (
            "资源二进制内容不存在",
            "The resource binary object is unavailable",
        ),
        ResourceError::PathDenied => (
            "资源临时文件路径不合法",
            "The resource temporary-file path is not allowed",
        ),
        ResourceError::StorageProfileNotFound => (
            "资源存储配置不存在",
            "The resource storage profile was not found",
        ),
        ResourceError::StorageLocatorInvalid => (
            "资源存储位置不正确",
            "The resource storage locator is invalid",
        ),
    };
    localized_message(locale, zh, en)
}

fn localized_message(locale: &str, zh: &str, en: &str) -> String {
    if locale.to_ascii_lowercase().starts_with("zh") {
        zh.to_string()
    } else {
        en.to_string()
    }
}
