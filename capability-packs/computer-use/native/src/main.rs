mod backend;
mod desktop;
mod skills;

use cua_driver_sdk::CuaDriver;
use std::sync::Arc;

#[tokio::main(worker_threads = 2)]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.as_slice() == ["--version"] {
        println!(
            "tron-computer-use {} (embedded Cua SDK)",
            env!("CARGO_PKG_VERSION")
        );
        return Ok(());
    }
    if args.first().is_some_and(|arg| arg == "permissions") {
        let status = match args.get(1).map(String::as_str) {
            Some("grant") => cua_driver_sdk::request_mac_os_permissions(),
            Some("status") | None => cua_driver_sdk::current_mac_os_permission_status(),
            _ => return Err("usage: tron-computer-use permissions [status|grant]".into()),
        };
        println!(
            "{}",
            serde_json::json!({"accessibility":status.accessibility,"screenRecording":status.screen_recording})
        );
        return Ok(());
    }
    if cfg!(target_os = "linux")
        && std::fs::read_to_string("/proc/sys/kernel/osrelease")
            .is_ok_and(|release| release.to_ascii_lowercase().contains("microsoft"))
    {
        return Err("this is a Linux desktop runtime; import the WSL2 package containing the Windows runtime to operate the Windows desktop".into());
    }
    // Set physical per-monitor coordinates before SDK/platform initialization.
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::HiDpi::{
            DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
        };
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
    match args.first().map(String::as_str).unwrap_or("backend") {
        "backend" => return backend::serve().await,
        "catalog" => {
            let catalog = backend::tool_catalog().map_err(|error| error.message)?;
            println!("{}", serde_json::to_string(&catalog)?);
            return Ok(());
        }
        "tools" => {
            println!("{}", backend::inventory());
            return Ok(());
        }
        _ => {}
    }
    // The native SDK creates the runtime in this process. No driver executable,
    // standalone daemon or installer is consulted.
    let driver = CuaDriver::create(None)?;
    let result = run(driver.clone(), &args).await;
    let shutdown = driver.shutdown().await;
    result?;
    shutdown?;
    Ok(())
}

async fn run(
    driver: Arc<CuaDriver>,
    args: &[String],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match args.first().map(String::as_str) {
        Some("metadata") => println!("{}", serde_json::to_string_pretty(&driver.metadata().await?)?),
        Some("doctor") => {
            println!("{}", serde_json::to_string_pretty(&driver.metadata().await?)?);
            let result = driver.call_tool("check_permissions".into(), "{}".into()).await?;
            println!("{}", result.raw_json);
        }
        Some("call") if args.len() == 3 => {
            let _owner = desktop::DesktopOwner::claim()?;
            let result = driver.call_tool(args[1].clone(), args[2].clone()).await?;
            println!("{}", result.raw_json);
            if result.is_error { return Err("desktop operation failed; see the structured result".into()); }
        }
        _ => return Err("usage: tron-computer-use [backend|catalog|metadata|tools|doctor|call <tool> <json>|permissions [status|grant]]".into()),
    }
    Ok(())
}
