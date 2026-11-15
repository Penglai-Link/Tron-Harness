//! Wiring between Tron's existing services and the shared remote server.

use std::sync::Arc;

use async_trait::async_trait;
use eliteforge_agent_remote_protocol::{ProductIdentity, RemoteSettings};
use eliteforge_agent_remote_rpc::ProfileSummary;
use eliteforge_agent_remote_server::{
    FileRemoteStore, RemoteAccessService, RemoteProduct, RemoteStore,
};
use tokio::sync::broadcast;

use crate::{
    app::AppState,
    application::agent::AgentAguiStreamService,
    application::profile::ProfileRegistryService,
    config::AppConfig,
};
use tron_remote_pack::{AguiEventSource, AguiLive, TronRemotePack};

pub(crate) struct TronRemoteProduct {
    pub profiles: ProfileRegistryService,
    pub identity: ProductIdentity,
}

#[async_trait]
impl RemoteProduct for TronRemoteProduct {
    fn product(&self) -> ProductIdentity {
        self.identity.clone()
    }

    fn profile(&self) -> Option<ProfileSummary> {
        self.profiles.current_context().ok().map(|context| ProfileSummary {
            username: context.person_ai_username.clone(),
            display_name: context.person_ai_username,
        })
    }
}

pub(crate) struct TronAguiSource {
    pub presentation: AgentAguiStreamService,
}

#[async_trait]
impl AguiEventSource for TronAguiSource {
    async fn replay(&self, session_id: &str, cursor: Option<u64>) -> Result<Vec<serde_json::Value>, String> {
        let (events, _) = self
            .presentation
            .protocol_events_since(session_id, cursor)
            .map_err(|error| format!("{error:?}"))?;
        Ok(events.into_iter().map(|event| event.data).collect())
    }

    fn subscribe(&self, session_id: &str) -> Result<AguiLive, String> {
        let mut source = self.presentation.subscribe();
        let session_id = session_id.to_owned();
        let (tx, receiver) = broadcast::channel(1024);
        tokio::spawn(async move {
            loop {
                match source.recv().await {
                    Ok(event) if event.stream_key == session_id => {
                        let _ = tx.send(event.event.data);
                    }
                    Ok(_) => {}
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
        Ok(AguiLive { receiver })
    }
}

pub(crate) async fn start(config: &AppConfig, profiles: ProfileRegistryService, presentation: AgentAguiStreamService) -> Result<Arc<RemoteAccessService>, String> {
    let root = remote_data_root(config);
    let store = Arc::new(FileRemoteStore::new(root));
    let existing = store.load_settings().await?;
    let public_base_url = config.remote_access.public_base_url.trim().to_owned();
    let public_base_url = if public_base_url.starts_with("https://") {
        public_base_url
    } else {
        String::new()
    };
    let settings = RemoteSettings {
        enabled: config.remote_access.enabled,
        public_base_url,
        bind_address: config.remote_access.bind_address.clone(),
        lan_base_url: format!(
            "http://{}:{}",
            crate::adapters::inbound::http::remote_access::detect_lan_ip()
                .map(|ip| ip.to_string())
                .unwrap_or_else(|| "127.0.0.1".into()),
            config.remote_access.listen_port(config.port)
        ),
        port: config.remote_access.listen_port(config.port),
        trusted_proxies: existing.trusted_proxies,
        allowed_origins: existing.allowed_origins,
        ticket_ttl_seconds: 300,
        machine_name: if config.remote_access.machine_name.trim().is_empty() {
            format!("Tron {}", std::env::consts::OS)
        } else {
            config.remote_access.machine_name.clone()
        },
        product_code: config.remote_access.product_code().to_owned(),
        product_name: config.remote_access.product_name().to_owned(),
    };
    store.save_settings(&settings).await?;
    let identity = ProductIdentity {
        product_code: settings.product_code.clone(),
        product_name: settings.product_name.clone(),
        node_id: config.remote_access.node_id.clone(),
        version: env!("CARGO_PKG_VERSION").into(),
        platform: std::env::consts::OS.into(),
        machine_name: settings.machine_name.clone(),
    };
    let product = Arc::new(TronRemoteProduct { profiles, identity });
    let pack = Arc::new(TronRemotePack::new(
        format!("http://127.0.0.1:{}", config.port),
        Some(Arc::new(TronAguiSource { presentation })),
    ));
    RemoteAccessService::start(product, store, pack, config.remote_access.listen_port(config.port))
        .await
        .map_err(|error| error.to_string())
}

impl AppState {
    pub async fn init_remote_access(&self) -> Result<(), String> {
        if self.remote_service.get().is_some() {
            return Ok(());
        }
        let service = start(&self.config, self.profiles.clone(), self.agent_presentation.clone()).await?;
        let _ = self.remote_service.set(service);
        Ok(())
    }

    pub fn remote_service(&self) -> Option<Arc<RemoteAccessService>> {
        self.remote_service.get().cloned()
    }
}

fn remote_data_root(config: &AppConfig) -> std::path::PathBuf {
    if let Some(parent) = std::path::Path::new(&config.application_config_path).parent() {
        return parent.join("remote-access");
    }
    std::env::var("HOME")
        .map(|home| std::path::PathBuf::from(home).join(".tron").join("remote-access"))
        .unwrap_or_else(|_| std::path::PathBuf::from(".tron/remote-access"))
}
