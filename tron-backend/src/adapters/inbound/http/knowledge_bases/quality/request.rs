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
        let locale = crate::shared::locale::resolve_request_locale(
            request
                .headers()
                .get(axum::http::header::ACCEPT_LANGUAGE)
                .and_then(|v| v.to_str().ok()),
        );
        let invalid = KnowledgeBaseError::invalid("knowledge.quality.request_invalid");
        Json::<T>::from_request(request, state)
            .await
            .map(|Json(body)| Body(body))
            .map_err(|_| QualityApiError(invalid, locale))
    }
}
