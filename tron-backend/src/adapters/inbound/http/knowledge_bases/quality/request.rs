use super::{KnowledgeBaseError, QualityApiError};
use axum::{
    Json,
    extract::{FromRequest, Request},
};
use serde::de::DeserializeOwned;

pub(super) struct Body<T>(pub T);
impl<S, T> FromRequest<S> for Body<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = QualityApiError;
    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        let locale = request
            .headers()
            .get(axum::http::header::ACCEPT_LANGUAGE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("zh-CN")
            .to_owned();
        Json::<T>::from_request(request, state)
            .await
            .map(|Json(body)| Body(body))
            .map_err(|_| {
                QualityApiError(
                    KnowledgeBaseError::Invalid("评测请求格式无效或超过256 KiB限制".into()),
                    locale,
                )
            })
    }
}
