use std::{error::Error, future::IntoFuture, time::Duration};

use tokio::net::TcpListener;
use tron_backend::{
    app::{AppState, build_router_with_state},
    config::AppConfig,
};

fn main() -> Result<(), Box<dyn Error>> {
    // Electron reads the SDK's platform codes before any backend is running.
    // This command must not load configuration or initialize storage.
    if std::env::args().nth(1).as_deref() == Some("--wsl-platforms") {
        use crab_serverapi::version::os_type;

        println!(
            "{}",
            serde_json::json!({
                "ubuntu": {
                    "x86_64": os_type::WSL_UBUNTU_X64,
                    "aarch64": os_type::WSL_UBUNTU_ARM64,
                },
                "debian": {
                    "x86_64": os_type::WSL_DEBIAN_X64,
                    "aarch64": os_type::WSL_DEBIAN_ARM64,
                },
            })
        );
        return Ok(());
    }
    // Native dialogs must run on the process main thread on macOS. A short-lived
    // helper process keeps the HTTP server responsive without a GUI event loop.
    if std::env::args().nth(1).as_deref() == Some("--pick-directory") {
        let title = if std::env::args().nth(2).as_deref() == Some("en-US") {
            "Choose a working directory"
        } else {
            "选择工作目录"
        };
        let selected = rfd::FileDialog::new().set_title(title).pick_folder();
        let path = selected.as_ref().map(|path| path.to_str().ok_or_else(||
            std::io::Error::other("directory path is not valid UTF-8"))).transpose()?;
        println!("{}", serde_json::to_string(&path)?);
        return Ok(());
    }
    run_server()
}

#[tokio::main]
async fn run_server() -> Result<(), Box<dyn Error>> {
    let config = AppConfig::from_application_yaml()?;
    tron_backend::adapters::outbound::observability::logs::init_application_logging(&config)
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    let socket_addr = config.socket_addr()?;
    let listener = TcpListener::bind(socket_addr).await?;
    let state = AppState::new(config)?;
    let framework = state.agent_framework.clone();
    framework
        .observability()
        .await
        .map_err(|_| std::io::Error::other("initialize OTel collector"))?;
    let ingestion = state.knowledge_bases.clone();
    let router = build_router_with_state(state)?;

    tracing::info!("tron-backend listening on http://{socket_addr}");
    tokio::select! {
        // Finalize routes once; serving Router directly runs with_state(())
        // and clones every route's middleware state for each new connection.
        result = axum::serve(listener, router.into_make_service_with_connect_info::<std::net::SocketAddr>()).into_future() => {
            result?;
            Ok(())
        }
        _ = shutdown_signal() => {
            // A process exit must not depend on HTTP draining, a stuck callback,
            // or Tokio's blocking-pool teardown. The OS releases file locks on exit.
            std::thread::spawn(|| {
                std::thread::sleep(Duration::from_secs(2));
                tracing::warn!("shutdown cleanup deadline reached; exiting process");
                std::process::exit(0);
            });
            tracing::warn!("shutdown requested: stopping ingestion, HTTP and Agent work");
            ingestion.stop_ingestion_workers();
            framework.cancel_all_runs();
            let pty = framework.pty();
            let (report, terminal) = tokio::join!(
                framework.close(),
                tokio::task::spawn_blocking(move || pty.close()),
            );
            tracing::warn!("shutdown cleanup finished: {report:?}; terminal={terminal:?}");
            std::process::exit(0);
        }
    }
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("install SIGTERM handler");
        tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = terminate.recv() => {} }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
