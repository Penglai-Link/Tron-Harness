//! Remote access for the mobile client.
//!
//! The desktop app talks to the backend over the loopback interface and needs
//! no token. Requests from other hosts are rejected unless remote access is
//! enabled and they present the pairing token. The token is generated locally
//! and is only returned by loopback-only endpoints, so a LAN or FRP client
//! cannot read it back.

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    process::Command,
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    Json, Router,
    extract::{ConnectInfo, FromRequestParts, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, Uri, header, request::Parts},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{get, put},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use url::Url;

use crate::{
    app::AppState,
    application::remote_service::{lock_remote_settings, refresh_lan_endpoint},
    config::{AppConfig, RemoteAccessConfig},
    response::{ApiResponse, EmptyData},
    shared::locale::resolve_request_locale,
};

const REMOTE_TOKEN_HEADER: &str = "x-tron-access-token";
const REMOTE_TOKEN_COOKIE: &str = "tron_remote_token";
const REMOTE_PAIRING_HEADER: &str = "x-tron-pairing-token";
const REMOTE_PROTOCOL_VERSION: u32 = 2;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/remote/status", get(status))
        .route("/api/v1/remote/settings", put(update_settings))
        .route(
            "/api/v1/remote/token/rotate",
            axum::routing::post(rotate_token),
        )
        .route("/api/v1/remote/service/status", get(service_status))
        .route("/api/v1/remote/service/pairing", get(service_pairing))
        .route(
            "/api/v1/remote/service/pairing/refresh",
            axum::routing::post(service_refresh_pairing),
        )
        .route("/api/v1/remote/service/devices", get(service_devices))
        .route(
            "/api/v1/remote/service/devices/{device_id}",
            axum::routing::delete(service_revoke_device),
        )
        .route(
            "/api/v1/remote/service/devices/{device_id}/record",
            axum::routing::delete(service_delete_device),
        )
        .route(
            "/api/v1/remote/service/devices/revoke-all",
            axum::routing::post(service_revoke_all),
        )
        .route("/api/v1/remote/service/enabled", put(service_set_enabled))
}

/// Mobile-facing identity endpoint. Mounted on both listeners; the dedicated
/// remote listener additionally requires the pairing token for every request.
pub fn identity_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/remote/identity", get(identity))
        .route("/api/v1/remote/session", axum::routing::post(session))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteAccessStatusResponse {
    /// The configured value, persisted in application.yaml.
    pub enabled: bool,
    /// Whether the running listener currently enforces remote access.
    pub active: bool,
    pub restart_required: bool,
    pub token: String,
    pub token_configured: bool,
    /// The configured node name; falls back to the computer name.
    pub machine_name: String,
    /// The operating system's human-readable computer name.
    pub computer_name: String,
    pub public_base_url: String,
    pub base_url: String,
    pub pairing_uri: String,
    pub bind_address: String,
    pub port: u16,
    pub lan_address: Option<String>,
    pub product_code: String,
    pub product_name: String,
    pub node_id: String,
    pub allow_insecure_http: bool,
    pub jwt_ttl_seconds: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteIdentityResponse {
    pub protocol_version: u32,
    pub product_code: String,
    pub product_name: String,
    pub node_id: String,
    pub machine_name: String,
    pub version: String,
    pub platform: String,
    pub base_url: String,
    pub profile: Option<RemoteProfileResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSessionResponse {
    pub access_token: String,
    pub expires_in: u64,
    pub identity: RemoteIdentityResponse,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteProfileResponse {
    pub username: String,
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateRemoteAccessRequest {
    pub enabled: bool,
    #[serde(default)]
    pub public_base_url: Option<String>,
    #[serde(default)]
    pub machine_name: Option<String>,
    #[serde(default)]
    pub product_code: Option<String>,
    #[serde(default)]
    pub product_name: Option<String>,
    #[serde(default)]
    pub allow_insecure_http: Option<bool>,
}

/// Handler guard: pairing and token endpoints never leave the Tron computer.
pub struct LocalOnly;

impl<S> FromRequestParts<S> for LocalOnly
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let local = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .is_none_or(|value| value.0.ip().is_loopback());
        if local {
            return Ok(Self);
        }
        let locale = resolve_request_locale(
            parts
                .headers
                .get(header::ACCEPT_LANGUAGE)
                .and_then(|value| value.to_str().ok()),
        );
        Err((
            StatusCode::FORBIDDEN,
            Json(ApiResponse::<EmptyData>::error(
                "REMOTE_ACCESS_LOCAL_ONLY",
                tron_i18n::t!("tron.remote_access.local_only", locale = &locale),
            )),
        )
            .into_response())
    }
}

async fn status(State(state): State<AppState>, _local: LocalOnly) -> Response {
    refresh_lan_endpoint(&state).await;
    no_store(Json(ApiResponse::success(status_snapshot(&state))).into_response())
}

async fn update_settings(
    State(state): State<AppState>,
    _local: LocalOnly,
    Json(request): Json<UpdateRemoteAccessRequest>,
) -> Response {
    let public_base_url = match request.public_base_url.as_deref() {
        Some(value) => match normalize_public_base_url(value) {
            Ok(value) => Some(value),
            Err(()) => {
                return remote_error(
                    StatusCode::BAD_REQUEST,
                    "REMOTE_ACCESS_INVALID_URL",
                    "tron.remote_access.invalid_url",
                    None,
                );
            }
        },
        None => None,
    };
    let configured = configured_config(&state);
    let mut next = configured.clone();
    next.enabled = request.enabled;
    if let Some(value) = public_base_url {
        next.public_base_url = value;
    }
    if let Some(value) = request.machine_name {
        next.machine_name = value.trim().to_owned();
    }
    if let Some(value) = request.product_code {
        let value = value.trim().to_ascii_lowercase();
        if value.is_empty()
            || value.len() > 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b".-_".contains(&byte))
        {
            return remote_error(
                StatusCode::BAD_REQUEST,
                "REMOTE_ACCESS_INVALID_PRODUCT_CODE",
                "tron.remote_access.invalid_product_code",
                None,
            );
        }
        next.product_code = value;
    }
    if let Some(value) = request.product_name {
        next.product_name = value.trim().to_owned();
    }
    if let Some(value) = request.allow_insecure_http {
        next.allow_insecure_http = value;
    }
    if next.token.trim().is_empty() {
        next.token = generate_token();
    }
    if next.node_id.trim().is_empty() {
        next.node_id = generate_node_id();
    }

    let mut changes = vec![
        change("remoteAccess.enabled", serde_json::json!(next.enabled)),
        change(
            "remoteAccess.publicBaseUrl",
            serde_json::json!(next.public_base_url),
        ),
        change(
            "remoteAccess.machineName",
            serde_json::json!(next.machine_name),
        ),
        change(
            "remoteAccess.productCode",
            serde_json::json!(next.product_code),
        ),
        change(
            "remoteAccess.productName",
            serde_json::json!(next.product_name),
        ),
        change("remoteAccess.nodeId", serde_json::json!(next.node_id)),
        change(
            "remoteAccess.allowInsecureHttp",
            serde_json::json!(next.allow_insecure_http),
        ),
    ];
    if configured.token != next.token {
        changes.push(change("remoteAccess.token", serde_json::json!(next.token)));
    }
    if let Err(response) = persist_changes(&state, changes) {
        return response;
    }
    apply_runtime(&state, &next);
    if let Err(error) = sync_remote_service(&state, &next).await {
        tracing::error!("could not synchronize remote service settings: {error}");
        return remote_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "REMOTE_ACCESS_SERVICE_SYNC_FAILED",
            "tron.remote_access.service_sync_failed",
            None,
        );
    }
    refresh_lan_endpoint(&state).await;
    no_store(Json(ApiResponse::success(status_snapshot(&state))).into_response())
}

async fn rotate_token(State(state): State<AppState>, _local: LocalOnly) -> Response {
    let configured = configured_config(&state);
    let mut next = configured.clone();
    next.token = generate_token();
    if let Err(response) = persist_changes(
        &state,
        vec![change("remoteAccess.token", serde_json::json!(next.token))],
    ) {
        return response;
    }
    apply_runtime(&state, &next);
    refresh_lan_endpoint(&state).await;
    no_store(Json(ApiResponse::success(status_snapshot(&state))).into_response())
}

async fn identity(State(state): State<AppState>) -> Response {
    no_store(Json(ApiResponse::success(identity_snapshot(&state))).into_response())
}

async fn session(State(state): State<AppState>, request: Request) -> Response {
    let mut config = configured_config(&state);
    config.node_id = ensure_node_id(&state, &config);
    if !config.enabled {
        return remote_error(
            StatusCode::FORBIDDEN,
            "REMOTE_ACCESS_DISABLED",
            "tron.remote_access.disabled",
            Some(&request),
        );
    }
    if !config.has_token() {
        return remote_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "REMOTE_ACCESS_TOKEN_MISSING",
            "tron.remote_access.token_missing",
            Some(&request),
        );
    }
    let provided = provided_pairing_token(request.headers(), request.uri());
    if !provided
        .as_deref()
        .is_some_and(|value| constant_time_eq(value.as_bytes(), config.token.trim().as_bytes()))
    {
        return remote_error(
            StatusCode::UNAUTHORIZED,
            "REMOTE_ACCESS_UNAUTHORIZED",
            "tron.remote_access.unauthorized",
            Some(&request),
        );
    }
    let access_token = issue_access_token(&config);
    no_store(
        Json(ApiResponse::success(RemoteSessionResponse {
            access_token,
            expires_in: config.jwt_ttl_seconds(),
            identity: identity_snapshot(&state),
        }))
        .into_response(),
    )
}

fn identity_snapshot(state: &AppState) -> RemoteIdentityResponse {
    let config = configured_config(state);
    let node_id = ensure_node_id(state, &config);
    let lan = detect_lan_ip();
    let base_url = resolve_base_url(&config, lan.as_ref(), advertised_port(state, &config));
    let machine_name = resolve_machine_name(&config);
    let profile = state.profiles.current_context().ok().map(|context| {
        let username = context.person_ai_username.clone();
        RemoteProfileResponse {
            display_name: username.clone(),
            username,
        }
    });
    RemoteIdentityResponse {
        protocol_version: REMOTE_PROTOCOL_VERSION,
        product_code: config.product_code().to_owned(),
        product_name: config.product_name().to_owned(),
        node_id,
        machine_name,
        version: env!("CARGO_PKG_VERSION").to_owned(),
        platform: std::env::consts::OS.to_owned(),
        base_url,
        profile,
    }
}

fn ensure_node_id(state: &AppState, config: &RemoteAccessConfig) -> String {
    if !config.node_id.trim().is_empty() {
        return config.node_id.clone();
    }
    if let Ok(current) = state.remote_access.read() {
        if !current.node_id.trim().is_empty() {
            return current.node_id.clone();
        }
    }
    let node_id = generate_node_id();
    let persisted = persist_changes(
        state,
        vec![change(
            "remoteAccess.nodeId",
            serde_json::json!(node_id.clone()),
        )],
    )
    .is_ok();
    if persisted {
        if let Ok(mut current) = state.remote_access.write() {
            current.node_id = node_id.clone();
        }
    }
    node_id
}

fn generate_node_id() -> String {
    let mut bytes = [0_u8; 12];
    rand::rng().fill_bytes(&mut bytes);
    format!("node-{}", URL_SAFE_NO_PAD.encode(bytes))
}

fn no_store(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

/// Reject non-loopback requesters unless remote access is enabled and the
/// pairing token matches.
/// Guard the desktop listener: non-loopback requesters need remote access and
/// the pairing token. Loopback callers (the desktop app) stay open.
pub async fn require_remote_access(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    gate_remote_request(state, request, next, false).await
}

/// Guard the dedicated remote listener: every caller, including one that
/// arrives from localhost through an FRP tunnel, must present the token.
pub async fn require_remote_token(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    gate_remote_request(state, request, next, true).await
}

async fn gate_remote_request(
    state: AppState,
    request: Request,
    next: Next,
    require_for_loopback: bool,
) -> Response {
    if matches!(request.uri().path(), "/api/v1/health" | "/api/v1/readiness") {
        return next.run(request).await;
    }
    let remote = if require_for_loopback {
        true
    } else {
        match request
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|value| value.0)
        {
            Some(address) => !address.ip().is_loopback(),
            // Without connect info the listener socket is the only signal we have.
            None => !listener_is_loopback(&state),
        }
    };
    if !remote {
        return next.run(request).await;
    }
    let config = state
        .remote_access
        .read()
        .map(|value| value.clone())
        .unwrap_or_default();
    if !config.enabled {
        return remote_error(
            StatusCode::FORBIDDEN,
            "REMOTE_ACCESS_DISABLED",
            "tron.remote_access.disabled",
            Some(&request),
        );
    }
    if config.token.trim().is_empty() {
        return remote_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "REMOTE_ACCESS_TOKEN_MISSING",
            "tron.remote_access.token_missing",
            Some(&request),
        );
    }
    if !config.allow_insecure_http && !is_secure_transport(request.headers()) {
        return remote_error(
            StatusCode::UPGRADE_REQUIRED,
            "INSECURE_TRANSPORT",
            "tron.remote_access.insecure_transport",
            Some(&request),
        );
    }
    let path = request.uri().path();
    if path == "/api/v1/remote/session" {
        let provided = provided_pairing_token(request.headers(), request.uri());
        if !provided
            .as_deref()
            .is_some_and(|value| constant_time_eq(value.as_bytes(), config.token.trim().as_bytes()))
        {
            return remote_error(
                StatusCode::UNAUTHORIZED,
                "REMOTE_ACCESS_UNAUTHORIZED",
                "tron.remote_access.unauthorized",
                Some(&request),
            );
        }
        return next.run(request).await;
    }
    let Some(access_token) = bearer_token(request.headers()) else {
        return remote_error(
            StatusCode::UNAUTHORIZED,
            "REMOTE_ACCESS_JWT_REQUIRED",
            "tron.remote_access.jwt_required",
            Some(&request),
        );
    };
    if !verify_access_token(&config, &access_token) {
        return remote_error(
            StatusCode::UNAUTHORIZED,
            "REMOTE_ACCESS_JWT_INVALID",
            "tron.remote_access.jwt_invalid",
            Some(&request),
        );
    }
    next.run(request).await
}

fn status_snapshot(state: &AppState) -> RemoteAccessStatusResponse {
    let configured = configured_config(state);
    let node_id = ensure_node_id(state, &configured);
    let runtime = state
        .remote_access
        .read()
        .map(|value| value.clone())
        .unwrap_or_default();
    let lan = detect_lan_ip();
    let port = advertised_port(state, &configured);
    let base_url = resolve_base_url(&configured, lan.as_ref(), port);
    let machine_name = resolve_machine_name(&configured);
    let listener_active = state.remote_access_active();
    let restart_required = configured.enabled && !listener_active;
    RemoteAccessStatusResponse {
        enabled: configured.enabled,
        active: listener_active && runtime.enabled,
        restart_required,
        token: configured.token.clone(),
        token_configured: configured.has_token(),
        machine_name: machine_name.clone(),
        computer_name: resolve_computer_name(),
        public_base_url: configured.public_base_url.clone(),
        base_url: base_url.clone(),
        pairing_uri: if configured.enabled && configured.has_token() {
            pairing_uri(&configured, &node_id, &base_url, &machine_name)
        } else {
            String::new()
        },
        bind_address: configured.listen_address().to_owned(),
        port,
        lan_address: lan.map(|value| value.to_string()),
        product_code: configured.product_code().to_owned(),
        product_name: configured.product_name().to_owned(),
        node_id,
        allow_insecure_http: configured.allow_insecure_http,
        jwt_ttl_seconds: configured.jwt_ttl_seconds(),
    }
}

async fn sync_remote_service(state: &AppState, next: &RemoteAccessConfig) -> Result<(), String> {
    let _guard = lock_remote_settings().await;
    if let Some(service) = state.remote_service() {
        let mut settings = service.current_settings().await;
        settings.enabled = next.enabled;
        settings.machine_name = if next.machine_name.trim().is_empty() {
            resolve_computer_name()
        } else {
            next.machine_name.clone()
        };
        settings.product_code = next.product_code().to_owned();
        settings.product_name = next.product_name().to_owned();
        let public = next.public_base_url.trim();
        settings.public_base_url = if public.is_empty() || public.starts_with("https://") {
            public.to_owned()
        } else {
            String::new()
        };
        service
            .update_settings(settings)
            .await
            .map_err(|error| error.to_string())?;
    }
    crate::application::remote_service::ensure_remote_listener(state.clone()).await;
    Ok(())
}

fn configured_config(state: &AppState) -> RemoteAccessConfig {
    AppConfig::from_yaml_file(&state.config.application_config_path)
        .map(|config| config.remote_access)
        .unwrap_or_else(|_| state.config.remote_access.clone())
}

fn apply_runtime(state: &AppState, next: &RemoteAccessConfig) {
    if let Ok(mut current) = state.remote_access.write() {
        *current = next.clone();
    }
}

fn change(
    path: &str,
    value: serde_json::Value,
) -> crate::application::system_config::SystemConfigChange {
    crate::application::system_config::SystemConfigChange {
        path: path.to_owned(),
        value,
    }
}

fn persist_changes(
    state: &AppState,
    changes: Vec<crate::application::system_config::SystemConfigChange>,
) -> Result<(), Response> {
    let document = state.system_config.get().map_err(|_| {
        remote_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "REMOTE_ACCESS_CONFIG_READ_FAILED",
            "tron.remote_access.config_read_failed",
            None,
        )
    })?;
    state
        .system_config
        .update(&document.revision, changes, false)
        .map(|_| ())
        .map_err(|_| {
            remote_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "REMOTE_ACCESS_CONFIG_WRITE_FAILED",
                "tron.remote_access.config_write_failed",
                None,
            )
        })
}

fn listener_is_loopback(state: &AppState) -> bool {
    matches!(
        state.config.bind_address.trim(),
        "127.0.0.1" | "localhost" | "::1" | "[::1]"
    )
}

fn resolve_base_url(config: &RemoteAccessConfig, lan: Option<&IpAddr>, port: u16) -> String {
    if !config.public_base_url.trim().is_empty() {
        return config
            .public_base_url
            .trim()
            .trim_end_matches('/')
            .to_owned();
    }
    match lan {
        Some(address) => format!("http://{address}:{port}"),
        None => format!("http://127.0.0.1:{port}"),
    }
}

/// Port the remote endpoint advertises: the one the dedicated listener actually
/// bound, or the port it would pick if remote access is enabled later.
fn advertised_port(state: &AppState, config: &RemoteAccessConfig) -> u16 {
    state
        .remote_listen_port()
        .unwrap_or_else(|| config.listen_port(state.config.port))
}

fn pairing_uri(
    config: &RemoteAccessConfig,
    node_id: &str,
    base_url: &str,
    machine_name: &str,
) -> String {
    let mut url = Url::parse("tron://pair").expect("static pairing scheme");
    let secure = base_url.to_ascii_lowercase().starts_with("https://");
    url.query_pairs_mut()
        .append_pair("v", "2")
        .append_pair("url", base_url)
        .append_pair("token", &config.token)
        .append_pair("name", machine_name)
        .append_pair("product", config.product_code())
        .append_pair("productName", config.product_name())
        .append_pair("node", node_id)
        .append_pair("secure", if secure { "1" } else { "0" });
    url.to_string()
}

fn normalize_public_base_url(value: &str) -> Result<String, ()> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    let mut url = Url::parse(trimmed).map_err(|_| ())?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(());
    }
    url.set_query(None);
    url.set_fragment(None);
    Ok(url.to_string().trim_end_matches('/').to_owned())
}

/// Find the LAN address a phone can reach.
///
/// The default route is not always the LAN interface (VPN/proxy tunnel
/// interfaces can win), so probe common gateway addresses and then fall back to
/// enumerating interfaces. Public-route probing is the last resort.
pub(crate) fn detect_lan_ip() -> Option<IpAddr> {
    for target in [
        "192.168.1.1:80",
        "192.168.0.1:80",
        "192.168.2.1:80",
        "10.0.0.1:80",
        "10.0.1.1:80",
        "172.16.0.1:80",
    ] {
        if let Some(address) = probe_local_address(target) {
            if is_private_ipv4(address) {
                return Some(IpAddr::V4(address));
            }
        }
    }
    if let Some(address) = interface_ipv4() {
        return Some(IpAddr::V4(address));
    }
    for target in ["1.1.1.1:80", "8.8.8.8:80"] {
        if let Some(address) = probe_local_address(target) {
            if !address.is_loopback() && !is_benchmark_ipv4(address) {
                return Some(IpAddr::V4(address));
            }
        }
    }
    None
}

fn probe_local_address(target: &str) -> Option<Ipv4Addr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect(target).ok()?;
    match socket.local_addr().ok()?.ip() {
        IpAddr::V4(address) => Some(address),
        IpAddr::V6(_) => None,
    }
}

fn is_private_ipv4(address: Ipv4Addr) -> bool {
    let octets = address.octets();
    address.is_private()
        || address.is_link_local()
        || (octets[0] == 100 && (64..=127).contains(&octets[1]))
}

fn is_benchmark_ipv4(address: Ipv4Addr) -> bool {
    let octets = address.octets();
    octets[0] == 198 && (octets[1] == 18 || octets[1] == 19)
}

#[cfg(unix)]
fn interface_ipv4() -> Option<Ipv4Addr> {
    let mut candidates = Vec::new();
    if let Ok(output) = Command::new("ifconfig").output() {
        let text = String::from_utf8_lossy(&output.stdout);
        collect_ifconfig_addresses(&text, &mut candidates);
    }
    if candidates.is_empty() {
        if let Ok(output) = Command::new("ip")
            .args(["-4", "-o", "addr", "show"])
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            collect_ifconfig_addresses(&text, &mut candidates);
        }
    }
    if let Some(address) = candidates
        .iter()
        .copied()
        .find(|address| is_private_ipv4(*address) && !address.is_link_local())
    {
        return Some(address);
    }
    candidates
        .into_iter()
        .find(|address| !address.is_link_local())
}

#[cfg(unix)]
fn collect_ifconfig_addresses(text: &str, candidates: &mut Vec<Ipv4Addr>) {
    for line in text.lines() {
        let Some(index) = line.find("inet ") else {
            continue;
        };
        let value = line[index + 5..].split_whitespace().next().unwrap_or("");
        let value = value.split('/').next().unwrap_or(value);
        if let Ok(address) = value.parse::<Ipv4Addr>() {
            if address.is_loopback() || candidates.contains(&address) {
                continue;
            }
            candidates.push(address);
        }
    }
}

#[cfg(windows)]
fn interface_ipv4() -> Option<Ipv4Addr> {
    let output = Command::new("ipconfig").output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let mut candidates = Vec::new();
    for token in text.split_whitespace() {
        let token = token.trim_end_matches([':', ',', ';']);
        if let Ok(address) = token.parse::<Ipv4Addr>() {
            if address.is_loopback() || address.is_link_local() || candidates.contains(&address) {
                continue;
            }
            candidates.push(address);
        }
    }
    if let Some(address) = candidates
        .iter()
        .copied()
        .find(|address| is_private_ipv4(*address))
    {
        return Some(address);
    }
    candidates.into_iter().next()
}

fn resolve_machine_name(config: &RemoteAccessConfig) -> String {
    if !config.machine_name.trim().is_empty() {
        return config.machine_name.trim().to_owned();
    }
    resolve_computer_name()
}

/// The human-readable name of this computer. macOS exposes it through
/// `scutil --get ComputerName`, which is friendlier than the shell hostname.
pub(crate) fn resolve_computer_name() -> String {
    static COMPUTER_NAME: OnceLock<String> = OnceLock::new();
    COMPUTER_NAME.get_or_init(compute_computer_name).clone()
}

fn compute_computer_name() -> String {
    #[cfg(target_os = "macos")]
    if let Ok(output) = Command::new("scutil")
        .args(["--get", "ComputerName"])
        .output()
    {
        if output.status.success() {
            let value = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            if !value.is_empty() {
                return value;
            }
        }
    }
    #[cfg(windows)]
    if let Ok(value) = std::env::var("COMPUTERNAME") {
        if !value.trim().is_empty() {
            return value.trim().to_owned();
        }
    }
    if let Ok(value) = std::env::var("HOSTNAME") {
        if !value.trim().is_empty() {
            return value.trim().to_owned();
        }
    }
    if let Ok(value) = std::fs::read_to_string("/etc/hostname") {
        if !value.trim().is_empty() {
            return value.trim().to_owned();
        }
    }
    "Tron".to_owned()
}

fn generate_token() -> String {
    let mut bytes = [0_u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            let (scheme, token) = value.trim().split_once(char::is_whitespace)?;
            scheme
                .eq_ignore_ascii_case("bearer")
                .then(|| token.trim().to_owned())
        })
        .filter(|value| !value.is_empty())
}

fn provided_pairing_token(headers: &HeaderMap, uri: &Uri) -> Option<String> {
    if let Some(value) = bearer_token(headers) {
        return Some(value);
    }
    if let Some(value) = headers
        .get(REMOTE_PAIRING_HEADER)
        .or_else(|| headers.get(REMOTE_TOKEN_HEADER))
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Some(value.to_owned());
    }
    if let Some(value) = uri.query().and_then(|query| {
        url::form_urlencoded::parse(query.as_bytes())
            .find(|(key, _)| key == "_tron_token" || key == "access_token")
            .map(|(_, value)| value.into_owned())
    }) {
        if !value.is_empty() {
            return Some(value);
        }
    }
    for value in headers.get_all(header::COOKIE) {
        let Ok(value) = value.to_str() else {
            continue;
        };
        for part in value.split(';') {
            let Some((name, content)) = part.trim().split_once('=') else {
                continue;
            };
            if name == REMOTE_TOKEN_COOKIE && !content.is_empty() {
                return Some(content.to_owned());
            }
        }
    }
    None
}

#[derive(Serialize, Deserialize)]
struct AccessTokenClaims {
    sub: String,
    prod: String,
    iat: u64,
    exp: u64,
    v: u32,
}

fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0)
}

fn jwt_signing_key(pairing_token: &str) -> Vec<u8> {
    Sha256::digest(format!("tron-remote-jwt:{pairing_token}").as_bytes()).to_vec()
}

fn issue_access_token(config: &RemoteAccessConfig) -> String {
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256","typ":"JWT"}"#);
    let now = now_seconds();
    let claims = AccessTokenClaims {
        sub: config.node_id.clone(),
        prod: config.product_code().to_owned(),
        iat: now,
        exp: now.saturating_add(config.jwt_ttl_seconds()),
        v: 1,
    };
    let payload =
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).unwrap_or_else(|_| b"{}".to_vec()));
    let signing_input = format!("{header}.{payload}");
    let mut mac = Hmac::<Sha256>::new_from_slice(&jwt_signing_key(&config.token))
        .expect("HMAC accepts the derived key length");
    mac.update(signing_input.as_bytes());
    let signature = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
    format!("{signing_input}.{signature}")
}

fn verify_access_token(config: &RemoteAccessConfig, token: &str) -> bool {
    let mut parts = token.split('.');
    let (Some(header), Some(payload), Some(signature), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    let Ok(claims) = URL_SAFE_NO_PAD
        .decode(payload)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<AccessTokenClaims>(&bytes).ok())
        .ok_or(())
    else {
        return false;
    };
    let now = now_seconds();
    if claims.exp <= now || claims.iat > now.saturating_add(60) || claims.v != 1 {
        return false;
    }
    if claims.prod.as_str() != config.product_code() {
        return false;
    }
    if !config.node_id.trim().is_empty() && claims.sub != config.node_id {
        return false;
    }
    let signing_input = format!("{header}.{payload}");
    let mut mac = Hmac::<Sha256>::new_from_slice(&jwt_signing_key(&config.token))
        .expect("HMAC accepts the derived key length");
    mac.update(signing_input.as_bytes());
    let Ok(provided) = URL_SAFE_NO_PAD.decode(signature) else {
        return false;
    };
    constant_time_eq(&provided, mac.finalize().into_bytes().as_slice())
}

fn is_secure_transport(headers: &HeaderMap) -> bool {
    if headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(',')
                .any(|part| part.trim().eq_ignore_ascii_case("https"))
        })
    {
        return true;
    }
    headers
        .get("forwarded")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value.split(',').any(|entry| {
                entry.split(';').any(|pair| {
                    pair.trim().split_once('=').is_some_and(|(key, value)| {
                        key.trim().eq_ignore_ascii_case("proto")
                            && value.trim().trim_matches('"').eq_ignore_ascii_case("https")
                    })
                })
            })
        })
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0_u8;
    for (left, right) in left.iter().zip(right) {
        difference |= left ^ right;
    }
    difference == 0
}

pub(crate) fn agent_status() -> Result<serde_json::Value, String> {
    let state =
        crate::app::AppState::global().ok_or_else(|| "remote access is not ready".to_owned())?;
    serde_json::to_value(status_snapshot(state))
        .map_err(|error| format!("could not serialize remote access status: {error}"))
}

pub(crate) fn agent_pairing() -> Result<serde_json::Value, String> {
    let mut value = agent_status()?;
    if let Some(object) = value.as_object_mut() {
        object.remove("token");
        object.remove("pairingUri");
    }
    Ok(value)
}

pub(crate) fn agent_rotate_token() -> Result<serde_json::Value, String> {
    let state =
        crate::app::AppState::global().ok_or_else(|| "remote access is not ready".to_owned())?;
    let configured = configured_config(state);
    let mut next = configured.clone();
    next.token = generate_token();
    persist_agent_changes(
        state,
        vec![change("remoteAccess.token", serde_json::json!(next.token))],
    )?;
    apply_runtime(state, &next);
    serde_json::to_value(status_snapshot(state))
        .map_err(|error| format!("could not serialize remote access status: {error}"))
}

pub(crate) fn agent_set_enabled(enabled: bool) -> Result<serde_json::Value, String> {
    let state =
        crate::app::AppState::global().ok_or_else(|| "remote access is not ready".to_owned())?;
    let configured = configured_config(state);
    let mut next = configured.clone();
    next.enabled = enabled;
    if next.token.trim().is_empty() {
        next.token = generate_token();
    }
    if next.node_id.trim().is_empty() {
        next.node_id = generate_node_id();
    }
    let mut changes = vec![
        change("remoteAccess.enabled", serde_json::json!(next.enabled)),
        change(
            "remoteAccess.nodeId",
            serde_json::json!(next.node_id.clone()),
        ),
    ];
    if configured.token != next.token {
        changes.push(change("remoteAccess.token", serde_json::json!(next.token)));
    }
    persist_agent_changes(state, changes)?;
    apply_runtime(state, &next);
    serde_json::to_value(status_snapshot(state))
        .map_err(|error| format!("could not serialize remote access status: {error}"))
}

fn persist_agent_changes(
    state: &AppState,
    changes: Vec<crate::application::system_config::SystemConfigChange>,
) -> Result<(), String> {
    let document = state
        .system_config
        .get()
        .map_err(|error| format!("could not read remote access settings: {error:?}"))?;
    state
        .system_config
        .update(&document.revision, changes, false)
        .map(|_| ())
        .map_err(|error| format!("could not save remote access settings: {error:?}"))
}

async fn service_status(State(state): State<AppState>, _local: LocalOnly) -> Response {
    refresh_lan_endpoint(&state).await;
    match state.remote_service() {
        Some(service) => {
            no_store(Json(ApiResponse::success(service.status_json().await)).into_response())
        }
        None => remote_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "REMOTE_ACCESS_SERVICE_UNAVAILABLE",
            "tron.remote_access.service_unavailable",
            None,
        ),
    }
}

async fn service_pairing(State(state): State<AppState>, _local: LocalOnly) -> Response {
    refresh_lan_endpoint(&state).await;
    match state.remote_service() {
        Some(service) => {
            let status = service.status().await;
            if !status.enabled || !status.active {
                return remote_error(
                    StatusCode::CONFLICT,
                    "REMOTE_ACCESS_NOT_LISTENING",
                    "tron.remote_access.not_listening",
                    None,
                );
            }
            no_store(
                Json(ApiResponse::success(serde_json::json!({
                    "pairingUri": status.pairing_uri,
                    "pairingTicketExpiresAt": status.pairing_ticket_expires_at,
                    "transport": status.transport,
                    "nodeKeyFingerprint": status.node_key_fingerprint,
                    "baseUrl": status.base_url,
                    "lanBaseUrl": status.lan_base_url,
                    "publicBaseUrl": status.public_base_url,
                })))
                .into_response(),
            )
        }
        None => remote_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "REMOTE_ACCESS_SERVICE_UNAVAILABLE",
            "tron.remote_access.service_unavailable",
            None,
        ),
    }
}

async fn service_refresh_pairing(State(state): State<AppState>, _local: LocalOnly) -> Response {
    refresh_lan_endpoint(&state).await;
    let Some(service) = state.remote_service() else {
        return remote_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "REMOTE_ACCESS_SERVICE_UNAVAILABLE",
            "tron.remote_access.service_unavailable",
            None,
        );
    };
    let current = service.status().await;
    if !current.enabled || !current.active {
        return remote_error(
            StatusCode::CONFLICT,
            "REMOTE_ACCESS_NOT_LISTENING",
            "tron.remote_access.not_listening",
            None,
        );
    }
    match service.refresh_pairing().await {
        Ok(pairing) => {
            let status = service.status().await;
            no_store(
                Json(ApiResponse::success(serde_json::json!({
                    "pairingUri": pairing.uri,
                    "pairingTicketExpiresAt": pairing.expires_at_ms,
                    "transport": pairing.transport,
                    "nodeKeyFingerprint": status.node_key_fingerprint,
                    "baseUrl": status.base_url,
                    "lanBaseUrl": status.lan_base_url,
                    "publicBaseUrl": status.public_base_url,
                })))
                .into_response(),
            )
        }
        Err(_) => remote_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "REMOTE_ACCESS_PAIRING_FAILED",
            "tron.remote_access.pairing_failed",
            None,
        ),
    }
}

async fn service_devices(State(state): State<AppState>, _local: LocalOnly) -> Response {
    let Some(service) = state.remote_service() else {
        return remote_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "REMOTE_ACCESS_SERVICE_UNAVAILABLE",
            "tron.remote_access.service_unavailable",
            None,
        );
    };
    no_store(Json(ApiResponse::success(service.devices().await)).into_response())
}

async fn service_revoke_device(
    State(state): State<AppState>,
    _local: LocalOnly,
    axum::extract::Path(device_id): axum::extract::Path<String>,
) -> Response {
    let Some(service) = state.remote_service() else {
        return remote_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "REMOTE_ACCESS_SERVICE_UNAVAILABLE",
            "tron.remote_access.service_unavailable",
            None,
        );
    };
    match service.revoke_device(&device_id).await {
        Ok(()) => no_store(Json(ApiResponse::success(service.status_json().await)).into_response()),
        Err(_) => remote_error(
            StatusCode::NOT_FOUND,
            "REMOTE_ACCESS_DEVICE_NOT_FOUND",
            "tron.remote_access.device_not_found",
            None,
        ),
    }
}

async fn service_delete_device(
    State(state): State<AppState>,
    _local: LocalOnly,
    axum::extract::Path(device_id): axum::extract::Path<String>,
) -> Response {
    let Some(service) = state.remote_service() else {
        return remote_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "REMOTE_ACCESS_SERVICE_UNAVAILABLE",
            "tron.remote_access.service_unavailable",
            None,
        );
    };
    match service.remove_device(&device_id).await {
        Ok(()) => no_store(Json(ApiResponse::success(service.status_json().await)).into_response()),
        Err(_) => remote_error(
            StatusCode::NOT_FOUND,
            "REMOTE_ACCESS_DEVICE_NOT_FOUND",
            "tron.remote_access.device_not_found",
            None,
        ),
    }
}

async fn service_revoke_all(State(state): State<AppState>, _local: LocalOnly) -> Response {
    let Some(service) = state.remote_service() else {
        return remote_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "REMOTE_ACCESS_SERVICE_UNAVAILABLE",
            "tron.remote_access.service_unavailable",
            None,
        );
    };
    match service.revoke_all().await {
        Ok(()) => no_store(Json(ApiResponse::success(service.status_json().await)).into_response()),
        Err(_) => remote_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "REMOTE_ACCESS_REVOKE_FAILED",
            "tron.remote_access.revoke_failed",
            None,
        ),
    }
}

#[derive(Debug, serde::Deserialize)]
struct ServiceEnabledRequest {
    enabled: bool,
}

async fn service_set_enabled(
    State(state): State<AppState>,
    _local: LocalOnly,
    Json(request): Json<ServiceEnabledRequest>,
) -> Response {
    let Some(service) = state.remote_service() else {
        return remote_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "REMOTE_ACCESS_SERVICE_UNAVAILABLE",
            "tron.remote_access.service_unavailable",
            None,
        );
    };
    let _guard = lock_remote_settings().await;
    match service.set_enabled(request.enabled).await {
        Ok(status) => {
            crate::application::remote_service::ensure_remote_listener(state).await;
            no_store(Json(ApiResponse::success(status)).into_response())
        }
        Err(_) => remote_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "REMOTE_ACCESS_SET_ENABLED_FAILED",
            "tron.remote_access.set_enabled_failed",
            None,
        ),
    }
}

fn remote_error(
    status: StatusCode,
    code: &'static str,
    message_key: &'static str,
    request: Option<&Request>,
) -> Response {
    let locale = match request {
        Some(request) => resolve_request_locale(
            request
                .headers()
                .get(header::ACCEPT_LANGUAGE)
                .and_then(|value| value.to_str().ok()),
        ),
        // Settings endpoints without a request keep the documented zh-CN default.
        None => tron_i18n::DEFAULT_LOCALE.as_str().to_owned(),
    };
    (
        status,
        Json(ApiResponse::<EmptyData>::error(
            code,
            tron_i18n::translate(&locale, message_key),
        )),
    )
        .into_response()
}
