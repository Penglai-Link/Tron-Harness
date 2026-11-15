use async_trait::async_trait;
use eliteforge_agent_capability_remote_access::RemoteAccessHost;
use serde_json::Value;

use crate::{adapters::inbound::http::remote_access, app::AppState};

/// Host adapter for the reusable remote-access capability pack. It prefers the
/// shared remote service; the legacy HTTP-backed functions remain a fallback
/// only for startup paths where the service has not been installed yet.
pub(crate) struct TronRemoteAccessHost;

#[async_trait]
impl RemoteAccessHost for TronRemoteAccessHost {
    async fn status(&self) -> Result<Value, String> {
        let state = AppState::global().ok_or_else(|| "remote access is not ready".to_owned())?;
        match state.remote_service() {
            Some(service) => Ok(service.status_json().await),
            None => remote_access::agent_status(),
        }
    }

    async fn pairing(&self) -> Result<Value, String> {
        let state = AppState::global().ok_or_else(|| "remote access is not ready".to_owned())?;
        match state.remote_service() {
            Some(service) => Ok(service.pairing_json().await),
            None => remote_access::agent_pairing(),
        }
    }

    async fn rotate_token(&self) -> Result<Value, String> {
        let state = AppState::global().ok_or_else(|| "remote access is not ready".to_owned())?;
        match state.remote_service() {
            Some(service) => {
                service.refresh_pairing().await.map_err(|error| error.to_string())?;
                Ok(service.status_json().await)
            }
            None => remote_access::agent_rotate_token(),
        }
    }

    async fn set_enabled(&self, enabled: bool) -> Result<Value, String> {
        let state = AppState::global().ok_or_else(|| "remote access is not ready".to_owned())?;
        match state.remote_service() {
            Some(service) => {
                service.set_enabled(enabled).await.map_err(|error| error.to_string())?;
                Ok(service.status_json().await)
            }
            None => remote_access::agent_set_enabled(enabled),
        }
    }
}
