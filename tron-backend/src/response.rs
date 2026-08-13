use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiResponse<T> {
    pub code: &'static str,
    pub message: String,
    pub data: T,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            code: "SUCCESS",
            message: "ok".into(),
            data,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct EmptyData {}

impl ApiResponse<EmptyData> {
    pub fn error(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: EmptyData {},
        }
    }
}
