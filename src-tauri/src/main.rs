use std::io;
use std::net::TcpListener;
use std::path::Path;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, RunEvent, State, WebviewUrl, WebviewWindowBuilder};
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use url::Url;

const SIDE_CAR_BASELINE_PORT: u16 = 8080;
const HOST: &str = "127.0.0.1";
const MAIN_WINDOW_LABEL: &str = "tron-main";
const TARGET_WINDOW_LABEL_PREFIX: &str = "tron-target-";
const BACKEND_HEALTH_PATH: &str = "/api/v1/health";
const MAX_PORT_PROBE: u16 = 80;

const INJECTION_TEMPLATE: &str = include_str!("../injection/tron-desktop-injection.js");

#[derive(Clone)]
struct DesktopRuntimeState {
    database_path: PathBuf,
    graph_path: PathBuf,
    backend_port: u16,
    backend_url: String,
    backend_process: Arc<Mutex<Option<Child>>>,
    frontend_dist_dir: PathBuf,
    injection_enabled: Arc<Mutex<bool>>,
    backend_ready: Arc<Mutex<bool>>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DesktopBackendStatus {
    backend_url: String,
    backend_port: u16,
    ready: bool,
    injection_enabled: bool,
    database_path: String,
    graph_path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenTargetPageInput {
    url: String,
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    page_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OpenTargetPageOutput {
    page_id: String,
    opened: bool,
    mode: String,
    reason: Option<String>,
    backend_url: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CloseTargetPageOutput {
    closed: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetInjectionEnabledInput {
    enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CloseTargetPageInput {
    page_id: String,
}

#[derive(Debug, Deserialize)]
struct DesktopInjectionEventInput {
    event: serde_json::Value,
}

const ALLOWED_INJECTION_EVENTS: &[&str] = &["tron:frame-ready", "tron:dom-event"];

fn now_millis() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => (duration.as_millis() % i64::MAX as u128) as i64,
        Err(_) => 0,
    }
}

fn next_page_id() -> String {
    format!("target-{}", now_millis())
}

fn is_http_url(input: &str) -> bool {
    matches!(Url::parse(input), Ok(url) if (url.scheme() == "http" || url.scheme() == "https"))
}

fn normalize_target_url(input: &str) -> String {
    if is_http_url(input) {
        return input.to_string();
    }

    if input.is_empty() {
        return String::new();
    }

    format!("https://{input}")
}

fn sanitize_url(raw: &str) -> String {
    let trimmed = raw.trim();
    normalize_target_url(trimmed)
}

fn load_port() -> u16 {
    std::env::var("TRON_BACKEND_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(SIDE_CAR_BASELINE_PORT)
}

fn build_frontend_dist_dir(handle: &AppHandle) -> Result<PathBuf, String> {
    let resource_dir = handle
        .path()
        .resource_dir()
        .map_err(|error| format!("resource dir resolution failed: {error}"))?;
    let candidate = resource_dir.join("frontend");

    if candidate.join("index.html").exists() {
        return Ok(candidate);
    }

    let nested_candidate = candidate.join("frontend");
    if nested_candidate.join("index.html").exists() {
        return Ok(nested_candidate);
    }

    if candidate.exists() {
        return Ok(candidate);
    }

    Err("frontend dist resource was not found in app bundle".to_string())
}

fn resolve_sidecar_path(handle: &AppHandle) -> Result<PathBuf, String> {
    let resource_dir = handle
        .path()
        .resource_dir()
        .map_err(|error| format!("resource dir resolution failed: {error}"))?;
    let candidate_dir = resource_dir.join("binaries");

    if cfg!(windows) {
        let windows_path = candidate_dir.join("tron-backend.exe");
        if windows_path.exists() {
            return Ok(windows_path);
        }
    }

    let unix_path = candidate_dir.join("tron-backend");
    if unix_path.exists() {
        return Ok(unix_path);
    }

    Err("tron-backend sidecar binary was not found in app resources".to_string())
}

fn find_free_port(start: u16) -> io::Result<u16> {
    if start == 0 {
        return Ok(SIDE_CAR_BASELINE_PORT);
    }

    for offset in 0..MAX_PORT_PROBE {
        let candidate = start.saturating_add(offset);
        if candidate == 0 {
            continue;
        }

        if TcpListener::bind((HOST, candidate)).is_ok() {
            return Ok(candidate);
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AddrInUse,
        format!("no free port found in {start}..{}", start.saturating_add(MAX_PORT_PROBE)),
    ))
}

fn resolve_desktop_data_dir() -> Result<PathBuf, String> {
    let home_dir: Option<PathBuf> = if cfg!(windows) {
        std::env::var_os("USERPROFILE").map(PathBuf::from).or_else(|| {
            std::env::var_os("HOMEDRIVE").and_then(|drive| {
                std::env::var_os("HOMEPATH").map(|path| {
                    let mut home = PathBuf::from(drive);
                    home.push(path);
                    home
                })
            })
        })
    } else {
        std::env::var_os("HOME").map(PathBuf::from)
    };

    let home = home_dir.ok_or_else(|| {
        "home directory not found (HOME or USERPROFILE is required)".to_string()
    })?;
    Ok(Path::new(&home).join(".tron"))
}

fn resolve_tron_database_path(app_data_dir: &Path) -> PathBuf {
    let legacy_path = app_data_dir.join("tron.sqlite");
    if legacy_path.exists() {
        return legacy_path;
    }

    app_data_dir.join("tron.sqlite3")
}

fn start_sidecar_backend(state: &DesktopRuntimeState, handle: &AppHandle) -> Result<Child, String> {
    let sidecar_binary = resolve_sidecar_path(handle)?;

    let db_path = state.database_path.to_string_lossy().into_owned();
    let graph_storage_path = state.graph_path.to_string_lossy().into_owned();
    let frontend_dist_dir = state.frontend_dist_dir.to_string_lossy().into_owned();

    let child = Command::new(sidecar_binary)
        .env("TRON_BACKEND_PORT", state.backend_port.to_string())
        .env("TRON_BACKEND_DATABASE_URL", db_path)
        .env("TRON_BACKEND_GRAPH_STORAGE_PATH", graph_storage_path)
        .env("TRON_BACKEND_FRONTEND_DIST_DIR", frontend_dist_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| format!("failed to start tron-backend: {error}"))?;

    Ok(child)
}

async fn wait_backend_ready(url: &str) -> bool {
    let client = Client::new();
    for _ in 0..120 {
        match client.get(format!("{url}{BACKEND_HEALTH_PATH}")).send().await {
            Ok(response) if response.status().is_success() => return true,
            Ok(_) | Err(_) => {}
        }
        sleep(Duration::from_millis(150)).await;
    }

    false
}

async fn is_backend_ready_once(url: &str) -> bool {
    let client = Client::new();
    match client.get(format!("{url}{BACKEND_HEALTH_PATH}")).send().await {
        Ok(response) if response.status().is_success() => true,
        Ok(_) | Err(_) => false,
    }
}

fn probe_existing_backend_port(start_port: u16) -> Option<u16> {
    for offset in 0..MAX_PORT_PROBE {
        let candidate = start_port.saturating_add(offset);
        if candidate == 0 {
            continue;
        }

        let url = format!("http://{}:{candidate}", HOST);
        if tauri::async_runtime::block_on(is_backend_ready_once(&url)) {
            return Some(candidate);
        }
    }

    None
}

fn resolve_backend_port(base_port: u16) -> Result<u16, String> {
    if let Some(existing_port) = probe_existing_backend_port(base_port) {
        let killed = kill_running_backend_processes()?;
        if killed {
            if !wait_until_backend_stopped(existing_port) {
                return Err("existing backend did not exit after termination".into());
            }
        }

        return Ok(existing_port);
    }

    find_free_port(base_port).map_err(|error| error.to_string())
}

fn kill_running_backend_processes() -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    let status = Command::new("taskkill")
        .args(["/F", "/T", "/IM", "tron-backend.exe"])
        .status()
        .map_err(|error| format!("failed to execute taskkill: {error}"))?;

    #[cfg(not(target_os = "windows"))]
    let status = Command::new("pkill")
        .arg("-x")
        .arg("tron-backend")
        .status()
        .map_err(|error| format!("failed to execute pkill: {error}"))?;

    match status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        Some(code) => Err(format!("terminator exited with code {code}")),
        None => Err("terminator exited without status".to_string()),
    }
}

fn wait_until_backend_stopped(port: u16) -> bool {
    for _ in 0..30 {
        let url = format!("http://{}:{port}", HOST);
        let ready = tauri::async_runtime::block_on(is_backend_ready_once(&url));
        if !ready {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    false
}

fn build_injection_script(backend_host: &str, page_id: &str, injection_enabled: bool) -> String {
    let injection_flag = if injection_enabled { "true" } else { "false" };
    INJECTION_TEMPLATE
        .replace("__TRON_APP_ORIGIN__", &serde_json::to_string(backend_host).unwrap_or_else(|_| "\"\"".to_string()))
        .replace("__TRON_PAGE_ID__", &serde_json::to_string(page_id).unwrap_or_else(|_| "\"unknown\"".to_string()))
        .replace("__TRON_INJECTION_ENABLED__", injection_flag)
}

async fn open_target_window(
    app: &AppHandle,
    state: &DesktopRuntimeState,
    request: OpenTargetPageInput,
) -> Result<OpenTargetPageOutput, String> {
    let raw_url = sanitize_url(&request.url);
    if !is_http_url(&raw_url) {
        return Err("Only http/https URLs are allowed".to_string());
    }

    let page_id = request.page_id.unwrap_or_else(next_page_id);
    let target_mode = request.mode.as_deref().unwrap_or("iframe");

    if target_mode != "window" {
        return Ok(OpenTargetPageOutput {
            page_id,
            opened: false,
            mode: target_mode.to_string(),
            reason: Some("requested-iframe-mode".to_string()),
            backend_url: Some(state.backend_url.clone()),
        });
    }

    let label = format!("{TARGET_WINDOW_LABEL_PREFIX}{page_id}");
    if let Some(existing) = app.get_webview_window(&label) {
        existing
            .set_focus()
            .map_err(|error| format!("failed to focus existing target window: {error}"))?;
        return Ok(OpenTargetPageOutput {
            page_id,
            opened: true,
            mode: "window".to_string(),
            reason: None,
            backend_url: Some(state.backend_url.clone()),
        });
    }

    let injection_enabled = *state
        .injection_enabled
        .lock()
        .await;
    let script = build_injection_script(&state.backend_url, &page_id, injection_enabled);

    let target_url = Url::parse(&raw_url).map_err(|error| format!("invalid target URL: {error}"))?;

    let _window = WebviewWindowBuilder::new(
        app,
        &label,
        WebviewUrl::External(target_url),
    )
    .title(&format!("Tron Target {page_id}"))
    .initialization_script_for_all_frames(&script)
    .inner_size(1280.0, 860.0)
    .resizable(true)
    .visible(true)
    .build()
    .map_err(|error| format!("failed to open target window: {error}"))?;

    Ok(OpenTargetPageOutput {
        page_id,
        opened: true,
        mode: "window".to_string(),
        reason: None,
        backend_url: Some(state.backend_url.clone()),
    })
}

#[tauri::command(rename = "desktopBackendStatus")]
async fn desktop_backend_status(state: State<'_, DesktopRuntimeState>) -> Result<DesktopBackendStatus, String> {
    Ok(DesktopBackendStatus {
        backend_url: state.backend_url.clone(),
        backend_port: state.backend_port,
        ready: *state.backend_ready.lock().await,
        injection_enabled: *state.injection_enabled.lock().await,
        database_path: state.database_path.to_string_lossy().into_owned(),
        graph_path: state.graph_path.to_string_lossy().into_owned(),
    })
}

#[tauri::command(rename = "openTargetPage")]
async fn open_target_page(app: AppHandle, state: State<'_, DesktopRuntimeState>, request: OpenTargetPageInput) -> Result<OpenTargetPageOutput, String> {
    open_target_window(&app, &state, request).await
}

#[tauri::command(rename = "closeTargetPage")]
async fn close_target_page(app: AppHandle, request: CloseTargetPageInput) -> Result<CloseTargetPageOutput, String> {
    let label = format!("{TARGET_WINDOW_LABEL_PREFIX}{}", request.page_id);
    if let Some(window) = app.get_webview_window(&label) {
        window
            .close()
            .map_err(|error| format!("failed to close target page: {error}"))?;
        return Ok(CloseTargetPageOutput { closed: true });
    }

    Ok(CloseTargetPageOutput { closed: false })
}

#[tauri::command(rename = "setInjectionEnabled")]
async fn set_injection_enabled(state: State<'_, DesktopRuntimeState>, request: SetInjectionEnabledInput) -> Result<bool, String> {
    let mut enabled = state.injection_enabled.lock().await;
    *enabled = request.enabled;
    Ok(*enabled)
}

#[tauri::command(rename = "desktopInjectionEvent")]
async fn desktop_injection_event(_state: State<'_, DesktopRuntimeState>, request: DesktopInjectionEventInput) -> Result<(), String> {
    let value = request
        .event
        .as_object()
        .ok_or_else(|| "invalid event payload format".to_string())?;

    let event_type = value
        .get("eventType")
        .and_then(|value| value.as_str())
        .unwrap_or("");

    if !ALLOWED_INJECTION_EVENTS.contains(&event_type) {
        return Err("event type is not allowed".to_string());
    }

    if !value.get("pageId").and_then(|value| value.as_str()).is_some() {
        return Err("event payload missing pageId".to_string());
    }

    if !value.get("frameId").and_then(|value| value.as_str()).is_some() {
        return Err("event payload missing frameId".to_string());
    }

    Ok(())
}

fn build_runtime_state(app_handle: &AppHandle, backend_port: u16) -> Result<DesktopRuntimeState, String> {
    let app_data_dir = resolve_desktop_data_dir()?;
    std::fs::create_dir_all(&app_data_dir)
        .map_err(|error| format!("app data dir creation failed: {error}"))?;
    let database_path = resolve_tron_database_path(&app_data_dir);
    let graph_path = app_data_dir.join("tron.graph.grafeo");

    let frontend_dist_dir = build_frontend_dist_dir(app_handle)?;

    let backend_url = format!("http://{}:{backend_port}", HOST);
    Ok(DesktopRuntimeState {
        database_path,
        graph_path,
        backend_port,
        backend_url,
        backend_process: Arc::new(Mutex::new(None)),
        frontend_dist_dir,
        injection_enabled: Arc::new(Mutex::new(true)),
        backend_ready: Arc::new(Mutex::new(false)),
    })
}

fn stop_backend_process(state: &DesktopRuntimeState) {
    let mut process_guard = tauri::async_runtime::block_on(state.backend_process.lock());
    if let Some(process) = process_guard.as_mut() {
        let _ = process.kill();
        let _ = process.wait();
    }
    *process_guard = None;
}

fn ensure_main_window_ready(app_handle: &AppHandle) -> Result<(), String> {
    let window = app_handle
        .get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or_else(|| "main window was not found".to_string())?;

    let _ = window.set_focus();
    let _ = window.set_title("Tron");
    Ok(())
}

fn show_startup_message(app_handle: &AppHandle, message: &str) {
    if let Some(window) = app_handle.get_webview_window(MAIN_WINDOW_LABEL) {
        let html = serde_json::to_string(&format!(
            "<pre style=\"padding:24px;font-family:monospace;white-space:pre-wrap;\">{message}</pre>"
        ))
        .unwrap_or_else(|_| "\"Backend starting...\"".to_string());
        let _ = window.eval(&format!(
            "document.documentElement.innerHTML = {html}; document.title = 'Tron';"
        ));
    }
}

fn start_window_navigator(
    app_handle: AppHandle,
    state: DesktopRuntimeState,
) -> Result<(), String> {
    ensure_main_window_ready(&app_handle)?;

    let backend_url = state.backend_url.clone();
    let app_handle_for_monitor = app_handle.clone();
    let state_for_monitor = state.clone();

    tauri::async_runtime::spawn(async move {
        let ready = wait_backend_ready(&backend_url).await;
        {
            let mut ready_guard = state_for_monitor.backend_ready.lock().await;
            *ready_guard = ready;
        }

        if ready {
            if let Some(main_window) = app_handle_for_monitor.get_webview_window(MAIN_WINDOW_LABEL) {
                let target_url = match Url::parse(&backend_url) {
                    Ok(url) => url,
                    Err(error) => {
                        let _ = main_window.eval(&format!(
                            "document.documentElement.innerHTML = {error_html};",
                            error_html = serde_json::to_string(&format!(
                                "Invalid backend URL: {error}"
                            ))
                            .unwrap_or_else(|_| "\"Invalid backend URL\"".to_string())
                        ));
                        let _ = main_window.set_focus();
                        return;
                    }
                };

                if let Err(error) = main_window.navigate(target_url) {
                    let _ = main_window.eval(&format!(
                        "document.documentElement.innerHTML = {error_html};",
                        error_html = serde_json::to_string(&format!(
                            "Failed to open backend UI: {error}"
                        ))
                        .unwrap_or_else(|_| "\"Failed to open backend UI\"".to_string())
                    ));
                }
                let _ = main_window.set_focus();
            }
            return;
        }

        {
            let mut process_guard = state_for_monitor.backend_process.lock().await;
            if let Some(process) = process_guard.as_mut() {
                let _ = process.kill();
                let _ = process.wait();
            }
            *process_guard = None;
        }

        show_startup_message(
            &app_handle_for_monitor,
            "Backend failed to start in time. Your data directory is ~/.tron. Check logs and retry.",
        );
    });

    Ok(())
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle().clone();
            let backend_port = resolve_backend_port(load_port())?;
            let state = build_runtime_state(&app_handle, backend_port)?;
            app.manage(state.clone());

            let child = start_sidecar_backend(&state, &app_handle)?;
            {
                let mut process_guard = tauri::async_runtime::block_on(state.backend_process.lock());
                *process_guard = Some(child);
            }

            start_window_navigator(app_handle.clone(), state.clone())?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            desktop_backend_status,
            open_target_page,
            close_target_page,
            set_injection_enabled,
            desktop_injection_event,
        ])
        .build(tauri::generate_context!("tauri.conf.json"))
        .expect("error while running tron desktop")
        .run(|app_handle, event| {
            if matches!(event, RunEvent::ExitRequested { .. } | RunEvent::Exit) {
                let state = app_handle.state::<DesktopRuntimeState>();
                stop_backend_process(&state.inner());
            }
        });
}
