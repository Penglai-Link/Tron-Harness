use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub code: &'static str,
    pub message: &'static str,
    pub data: T,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            code: "SUCCESS",
            message: "ok",
            data,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct EmptyData {}

impl ApiResponse<EmptyData> {
    pub fn error(code: &'static str, message: &'static str) -> Self {
        Self {
            code,
            message,
            data: EmptyData {},
        }
    }
}
