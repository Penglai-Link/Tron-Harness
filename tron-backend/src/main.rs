use std::error::Error;

use tokio::net::TcpListener;
use tron_backend::{app::build_router, config::AppConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = AppConfig::from_application_yaml()?;
    let socket_addr = config.socket_addr()?;
    let listener = TcpListener::bind(socket_addr).await?;

    println!("tron-backend listening on http://{socket_addr}");
    axum::serve(listener, build_router(config)).await?;

    Ok(())
}
