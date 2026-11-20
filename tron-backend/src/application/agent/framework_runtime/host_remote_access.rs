use async_trait::async_trait;
use eliteforge_agent_capability_remote_access::RemoteAccessHost;
use eliteforge_agent_core as fw;
use serde_json::Value;
use std::sync::Arc;

use crate::{adapters::inbound::http::remote_access, app::AppState};

/// Capability pack declaration for the shared remote-access pack.
///
/// Tron owns the category shown in tool settings, so the upstream provider is
/// wrapped to declare `tron.platform` on both its catalog metadata and leased
/// bindings. The pack ID, plugin ID and skill source stay upstream values.
pub(crate) fn capability_pack() -> fw::CapabilityPackBuilder<()> {
    use eliteforge_agent_capability_remote_access as remote;
    let capability = remote::RemoteAccessCapability::new(Arc::new(TronRemoteAccessHost));
    fw::CapabilityPackBuilder::new(remote::PACK_ID, ())
        .skills(remote::skill_source())
        .plugin(fw::ToolProviderPlugin::from_arc(
            remote::TOOL_PROVIDER_ID,
            Arc::new(PlatformCategorizedTools {
                inner: capability.tool_provider(),
            }),
        ))
}

/// Declares Tron's platform category on every tool leased by the wrapped pack.
struct PlatformCategorizedTools {
    inner: Arc<dyn fw::ToolProvider>,
}

fn platform_category() -> fw::ToolCategory {
    crate::application::agent::tool_catalog::builtin_category("tron.platform")
}

#[async_trait]
impl fw::ToolProvider for PlatformCategorizedTools {
    fn descriptors(&self) -> Option<Vec<fw::ToolDescriptor>> {
        self.inner.descriptors()
    }

    fn catalog_metadata(&self) -> Option<Vec<fw::ToolCatalogEntry>> {
        self.inner.catalog_metadata().map(|entries| {
            entries
                .into_iter()
                .map(|mut entry| {
                    entry.category = Some(platform_category());
                    entry
                })
                .collect()
        })
    }

    async fn lease(&self, run: &fw::RunContext) -> Result<fw::ToolLease, fw::AgentError> {
        let mut lease = self.inner.lease(run).await?;
        for tool in &mut lease.tools {
            tool.category = Some(platform_category());
        }
        Ok(lease)
    }
}

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
                service
                    .refresh_pairing()
                    .await
                    .map_err(|error| error.to_string())?;
                Ok(service.status_json().await)
            }
            None => remote_access::agent_rotate_token(),
        }
    }

    async fn set_enabled(&self, enabled: bool) -> Result<Value, String> {
        let state = AppState::global().ok_or_else(|| "remote access is not ready".to_owned())?;
        match state.remote_service() {
            Some(service) => {
                service
                    .set_enabled(enabled)
                    .await
                    .map_err(|error| error.to_string())?;
                Ok(service.status_json().await)
            }
            None => remote_access::agent_set_enabled(enabled),
        }
    }
}
