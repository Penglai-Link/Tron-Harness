//! Wiring between Tron's existing services and the shared remote server.

use std::{
    net::SocketAddr,
    sync::{Arc, OnceLock},
};
use tokio::{net::TcpListener, task::JoinHandle};

use async_trait::async_trait;
use eliteforge_agent_remote_protocol::{ProductIdentity, RemoteSettings};
use eliteforge_agent_remote_rpc::ProfileSummary;
use eliteforge_agent_remote_server::{
    FileRemoteStore, RemoteAccessService, RemoteProduct, RemoteStore,
};
use tokio::sync::broadcast;

use crate::{
    app::AppState, application::agent::AgentAguiStreamService,
    application::profile::ProfileRegistryService, config::AppConfig,
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
        self.profiles
            .current_context()
            .ok()
            .map(|context| ProfileSummary {
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
    async fn replay(
        &self,
        session_id: &str,
        cursor: Option<u64>,
    ) -> Result<Vec<serde_json::Value>, String> {
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

pub(crate) async fn start(
    config: &AppConfig,
    profiles: ProfileRegistryService,
    presentation: AgentAguiStreamService,
) -> Result<Arc<RemoteAccessService>, String> {
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
            crate::adapters::inbound::http::remote_access::resolve_computer_name()
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
    RemoteAccessService::start(
        product,
        store,
        pack,
        config.remote_access.listen_port(config.port),
    )
    .await
    .map_err(|error| error.to_string())
}

impl AppState {
    pub async fn init_remote_access(&self) -> Result<(), String> {
        if self.remote_service.get().is_some() {
            return Ok(());
        }
        let service = start(
            &self.config,
            self.profiles.clone(),
            self.agent_presentation.clone(),
        )
        .await?;
        let _ = self.remote_service.set(service);
        Ok(())
    }

    pub fn remote_service(&self) -> Option<Arc<RemoteAccessService>> {
        self.remote_service.get().cloned()
    }
}

fn listener_port_file(config: &AppConfig) -> std::path::PathBuf {
    remote_data_root(config).join("listen-port")
}

fn read_preferred_port(config: &AppConfig) -> Option<u16> {
    std::fs::read_to_string(listener_port_file(config))
        .ok()
        .and_then(|value| value.trim().parse::<u16>().ok())
}

fn write_preferred_port(config: &AppConfig, port: u16) {
    let path = listener_port_file(config);
    if let Some(parent) = path.parent()
        && std::fs::create_dir_all(parent).is_err()
    {
        return;
    }
    let _ = std::fs::write(path, port.to_string());
}

fn remote_data_root(config: &AppConfig) -> std::path::PathBuf {
    if let Some(parent) = std::path::Path::new(&config.application_config_path).parent() {
        return parent.join("remote-access");
    }
    std::env::var("HOME")
        .map(|home| {
            std::path::PathBuf::from(home)
                .join(".tron")
                .join("remote-access")
        })
        .unwrap_or_else(|_| std::path::PathBuf::from(".tron/remote-access"))
}

struct RemoteListenerSlot {
    task: Option<JoinHandle<()>>,
    addr: Option<SocketAddr>,
}

fn listener_slot() -> &'static tokio::sync::Mutex<RemoteListenerSlot> {
    static SLOT: OnceLock<tokio::sync::Mutex<RemoteListenerSlot>> = OnceLock::new();
    SLOT.get_or_init(|| {
        tokio::sync::Mutex::new(RemoteListenerSlot {
            task: None,
            addr: None,
        })
    })
}

/// Starts, stops, or repairs the dedicated remote listener to match the
/// current service settings. Called at startup and after settings changes so
/// enabling remote access from the UI takes effect without restarting Tron.
pub async fn ensure_remote_listener(state: AppState) {
    let Some(service) = state.remote_service() else {
        return;
    };
    let mut slot = listener_slot().lock().await;
    let status = service.status().await;
    if !status.enabled {
        if let Some(task) = slot.task.take() {
            task.abort();
        }
        slot.addr = None;
        service.set_active(false);
        state.set_remote_access_active(false);
        state.set_remote_listen_port(None);
        service.shutdown().await;
        return;
    }

    let config = state
        .remote_access
        .read()
        .map(|value| (*value).clone())
        .unwrap_or_else(|_| state.config.remote_access.clone());
    // A configured port is a preference, not a hard requirement. When the
    // public URL is empty (the common LAN case), scan forward for the first
    // free port and remember it so restarts keep the same QR address.
    let auto_port = config.port == 0;
    let preferred = if auto_port {
        read_preferred_port(&state.config).unwrap_or(18_081)
    } else {
        config.listen_port(state.config.port)
    };
    let bind_host = config.listen_address();
    let allow_fallback = auto_port && config.public_base_url.trim().is_empty();
    let mut candidates = vec![preferred];
    if allow_fallback {
        for offset in 1..=32u16 {
            if let Some(port) = preferred.checked_add(offset)
                && port != state.config.port
            {
                candidates.push(port);
            }
        }
    }

    for port in candidates {
        if port == state.config.port {
            continue;
        }
        let addr = format!("{bind_host}:{port}");
        let Ok(addr) = addr.parse::<SocketAddr>() else {
            continue;
        };
        if let Some(task) = slot.task.as_ref()
            && !task.is_finished()
            && slot.addr == Some(addr)
        {
            service.set_active(true);
            state.set_remote_access_active(true);
            state.set_remote_listen_port(Some(port));
            return;
        }
        if let Some(task) = slot.task.take() {
            task.abort();
        }
        slot.addr = None;
        let Ok(listener) = TcpListener::bind(addr).await else {
            continue;
        };
        let lan_base_url = format!(
            "http://{}:{port}",
            crate::adapters::inbound::http::remote_access::detect_lan_ip()
                .map(|ip| ip.to_string())
                .unwrap_or_else(|| "127.0.0.1".into())
        );
        let mut settings = service.current_settings().await;
        settings.port = port;
        settings.lan_base_url = lan_base_url;
        if let Err(error) = service.update_settings(settings).await {
            tracing::error!("could not update remote service port: {error}");
            continue;
        }
        let router = service.clone().router();
        let task = tokio::spawn(async move {
            if let Err(error) = axum::serve(
                listener,
                router.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            {
                tracing::error!("tron-backend remote access listener stopped: {error}");
            }
        });
        slot.task = Some(task);
        slot.addr = Some(addr);
        service.set_active(true);
        state.set_remote_access_active(true);
        state.set_remote_listen_port(Some(port));
        if auto_port {
            write_preferred_port(&state.config, port);
        }
        tracing::info!("tron-backend remote access listening on http://{addr}");
        return;
    }

    service.set_active(false);
    state.set_remote_access_active(false);
    state.set_remote_listen_port(None);
    tracing::error!("tron-backend remote access could not find a free listener port");
}

/// Keep the endpoint the remote service advertises in step with the network.
///
/// The service snapshots the LAN base URL when its listener binds, so switching
/// Wi-Fi (or waking up on another network) otherwise leaves the settings dialog
/// and the pairing QR pointing at an address that no longer exists. Every
/// status/pairing read re-detects the address and rewrites the snapshot; the
/// service also re-issues its pairing ticket from the new address.
pub(crate) async fn refresh_lan_endpoint(state: &AppState) {
    let _guard = lock_remote_settings().await;
    let Some(service) = state.remote_service() else {
        return;
    };
    let Some(address) = crate::adapters::inbound::http::remote_access::detect_lan_ip() else {
        return;
    };
    let current = service.current_settings().await;
    let port = match state.remote_listen_port() {
        Some(port) => port,
        None if current.port != 0 => current.port,
        None => return,
    };
    let wanted = format!("http://{address}:{port}");
    if current.lan_base_url.trim().trim_end_matches('/') == wanted {
        return;
    }
    tracing::info!(
        "remote LAN endpoint moved from {} to {wanted}",
        current.lan_base_url
    );
    let mut next = current;
    next.lan_base_url = wanted;
    if let Err(error) = service.update_settings(next).await {
        tracing::warn!("could not refresh the remote LAN endpoint: {error}");
    }
}

/// Serialize read-modify-write cycles over the remote service settings.
///
/// The remote service stores its settings as one document, so two writers that
/// each read, edit and write whole structs lose each other's fields. Writing
/// them also re-issues the pairing ticket, so two concurrent reads must not
/// invalidate the ticket the other one just returned.
pub(crate) async fn lock_remote_settings() -> tokio::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}
