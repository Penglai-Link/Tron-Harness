use axum::{
    Json,
    extract::rejection::JsonRejection,
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
        | ResourceError::InvalidField { .. }
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

pub fn resource_error_with_field(error: ResourceError, field: &str) -> ResourceError {
    match error {
        ResourceError::InvalidInput => ResourceError::InvalidField {
            field: field.to_string(),
            reason: "does not satisfy the resource validation rules".to_string(),
        },
        ResourceError::InvalidCode => ResourceError::InvalidField {
            field: field.to_string(),
            reason: "must match the resource code format".to_string(),
        },
        ResourceError::InvalidField {
            field: nested,
            reason,
        } if !nested.is_empty() => {
            let field = if nested == field || nested.starts_with(&format!("{field}.")) {
                nested
            } else {
                format!("{field}.{nested}")
            };
            ResourceError::InvalidField { field, reason }
        }
        error => error,
    }
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

pub fn malformed_json_request_response(error: &JsonRejection, locale: &str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ResourceHttpApiResponse::error(
            RESOURCE_MALFORMED_REQUEST_CODE,
            localized_json_request_error(error.body_text(), locale),
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
        | ResourceError::InvalidInput
        | ResourceError::InvalidField { .. } => RESOURCE_INVALID_INPUT_CODE,
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
        ResourceError::InvalidField { field, reason } => {
            return localized_invalid_field_error(field, reason, locale);
        }
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

fn localized_invalid_field_error(field: String, reason: String, locale: &str) -> String {
    let field = field.trim();
    let reason = reason.trim();
    if locale.starts_with("en") {
        format!("Invalid resource field '{field}': {reason}")
    } else {
        format!("资源字段不正确：{field}（{reason}）")
    }
}

fn localized_json_request_error(detail: String, locale: &str) -> String {
    let detail = detail.trim();
    let field = [
        "missing field `",
        "missing field '",
        "unknown field `",
        "unknown field '",
        " for key `",
        " for key '",
    ]
    .iter()
    .find_map(|marker| {
        detail.find(marker).and_then(|start| {
            let value = &detail[start + marker.len()..];
            let end = value.find(['`', '\''])?;
            Some(value[..end].to_string())
        })
    });
    if let Some(field) = field {
        if locale.to_ascii_lowercase().starts_with("zh") {
            return format!("资源字段不正确：{field}（{detail}）");
        }
        return format!("Invalid resource field '{field}': {detail}");
    }
    if locale.to_ascii_lowercase().starts_with("zh") {
        format!("资源请求字段格式不正确：{detail}")
    } else {
        format!("Invalid resource request field: {detail}")
    }
}

fn localized_message(locale: &str, zh: &str, en: &str) -> String {
    if locale.to_ascii_lowercase().starts_with("zh") {
        zh.to_string()
    } else {
        en.to_string()
    }
}
