use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::domain::resource::ResourceError;

use super::ResourceHttpApiResponse;

const RESOURCE_MALFORMED_REQUEST_CODE: &str = "100100001";
const RESOURCE_NOT_FOUND_CODE: &str = "100100002";
const RESOURCE_FORBIDDEN_CODE: &str = "100100003";
const RESOURCE_CONFLICT_CODE: &str = "100100004";
const RESOURCE_INVALID_INPUT_CODE: &str = "100100005";
const RESOURCE_VARIABLE_REFERENCE_CODE: &str = "100100006";
const RESOURCE_BINARY_MISSING_CODE: &str = "100100007";
const RESOURCE_PATH_DENIED_CODE: &str = "100100008";
const RESOURCE_STORAGE_PROFILE_CODE: &str = "100100009";
const RESOURCE_STORAGE_LOCATOR_CODE: &str = "100100010";
const RESOURCE_CONNECTION_VERIFICATION_CODE: &str = "100100011";

pub fn resource_error_response(error: ResourceError, locale: &str) -> Response {
    let status = match &error {
        ResourceError::NotFound | ResourceError::MissingBinaryObject => StatusCode::NOT_FOUND,
        ResourceError::Forbidden => StatusCode::FORBIDDEN,
        ResourceError::Conflict | ResourceError::ServerHasAccounts => StatusCode::CONFLICT,
        ResourceError::ConnectionVerificationFailed(_) => StatusCode::BAD_GATEWAY,
        ResourceError::StorageFailure => StatusCode::INTERNAL_SERVER_ERROR,
        ResourceError::WorkspaceRequired
        | ResourceError::InvalidWorkspace
        | ResourceError::InvalidCode
        | ResourceError::InvalidInput
        | ResourceError::InvalidVariableReference
        | ResourceError::PathDenied
        | ResourceError::StorageProfileNotFound
        | ResourceError::StorageLocatorInvalid => StatusCode::BAD_REQUEST,
    };
    (
        status,
        Json(ResourceHttpApiResponse::error(
            resource_error_code(&error),
            localized_resource_error(error, locale),
        )),
    )
        .into_response()
}

pub fn malformed_request_response(status: StatusCode, locale: &str) -> Response {
    (
        status,
        Json(ResourceHttpApiResponse::error(
            RESOURCE_MALFORMED_REQUEST_CODE,
            localized_message(
                locale,
                "请求参数或请求体格式不正确",
                "The request parameters or body are invalid",
            ),
        )),
    )
        .into_response()
}

fn resource_error_code(error: &ResourceError) -> &'static str {
    match error {
        ResourceError::NotFound => RESOURCE_NOT_FOUND_CODE,
        ResourceError::Forbidden => RESOURCE_FORBIDDEN_CODE,
        ResourceError::Conflict | ResourceError::ServerHasAccounts => RESOURCE_CONFLICT_CODE,
        ResourceError::MissingBinaryObject => RESOURCE_BINARY_MISSING_CODE,
        ResourceError::PathDenied => RESOURCE_PATH_DENIED_CODE,
        ResourceError::StorageProfileNotFound => RESOURCE_STORAGE_PROFILE_CODE,
        ResourceError::StorageLocatorInvalid => RESOURCE_STORAGE_LOCATOR_CODE,
        ResourceError::ConnectionVerificationFailed(_) => RESOURCE_CONNECTION_VERIFICATION_CODE,
        ResourceError::WorkspaceRequired
        | ResourceError::InvalidWorkspace
        | ResourceError::InvalidCode
        | ResourceError::InvalidInput => RESOURCE_INVALID_INPUT_CODE,
        ResourceError::InvalidVariableReference => RESOURCE_VARIABLE_REFERENCE_CODE,
        ResourceError::StorageFailure => "-1",
    }
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
        ResourceError::NotFound => ("资源不存在", "The resource was not found"),
        ResourceError::Forbidden => (
            "无权执行资源操作",
            "You are not authorized to manage this resource",
        ),
        ResourceError::WorkspaceRequired => ("必须提供工作空间编码", "workspaceCode is required"),
        ResourceError::InvalidWorkspace => ("工作空间不可用", "The workspace is not active"),
        ResourceError::InvalidCode => (
            "资源业务编码不正确",
            "The resource business code is invalid",
        ),
        ResourceError::InvalidInput => ("资源请求参数不正确", "The resource request is invalid"),
        ResourceError::InvalidVariableReference => (
            "资源变量引用不正确",
            "The resource Variable reference is invalid",
        ),
        ResourceError::Conflict => (
            "资源状态发生冲突",
            "The resource state conflicts with this operation",
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
        ResourceError::ConnectionVerificationFailed(_) => (
            "连接信息未通过连通性检测",
            "The connection information did not pass connectivity verification",
        ),
        ResourceError::ServerHasAccounts => (
            "服务器仍关联账号，不能删除",
            "The server still has accounts and cannot be deleted",
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
