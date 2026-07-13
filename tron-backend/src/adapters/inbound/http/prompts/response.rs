use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct PromptApiResponse<T> {
    pub code: &'static str,
    pub message: String,
    pub data: T,
}

impl<T> PromptApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            code: "0",
            message: "success".into(),
            data,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PromptEmptyData {}

impl PromptApiResponse<PromptEmptyData> {
    pub fn error(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: PromptEmptyData {},
        }
    }
}
