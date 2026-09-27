use std::error::Error;

use tokio::net::TcpListener;
use tron_backend::{
    app::{AppState, build_router_with_state},
    config::AppConfig,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = AppConfig::from_application_yaml()?;
    let socket_addr = config.socket_addr()?;
    let listener = TcpListener::bind(socket_addr).await?;
    let state = AppState::new(config)?;
    let framework = state.agent_framework.clone();
    let router = build_router_with_state(state)?;

    println!("tron-backend listening on http://{socket_addr}");
    let result = axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            shutdown_signal().await;
            framework.close().await;
        })
        .await;
    result?;

    Ok(())
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
