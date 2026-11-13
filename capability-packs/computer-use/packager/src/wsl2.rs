//! Assemble the WSL2 variant: a POSIX launcher plus the Windows desktop runtime.
use super::*;
use eliteforge_agent_capability_runtime::CapabilityPackManifest;

/// Validate a Windows pack and extract the runtime it embeds.
///
/// The WSL2 pack runs the same Windows SDK binary through WSL interop, so the
/// Windows pack must be the same version, architecture, and upstream commit.
pub(super) fn windows_package(
    archive: &Path,
    version: &str,
    arch: &str,
    upstream: &Value,
) -> Result<(Vec<u8>, Value), Box<dyn std::error::Error>> {
    let stage = tempfile::tempdir()?;
    let installed = CapabilityPackManager::new(stage.path())?
        .install_bytes(&fs::read(archive)?, InstallPolicy::Development)?;
    let info: Value = serde_json::from_slice(&fs::read(installed.path.join("build-info.json"))?)?;
    let platform = format!("{arch}-windows");
    let package = &installed.manifest;
    if !variant::is_windows_package(&package.id)
        || package.version.to_string() != version
        || info["mode"] != "native"
        || info["targetOs"] != "windows"
        || info["targetArch"] != arch
        || info["upstreamRevision"] != upstream["revision"]
        || info["sdkVersion"] != upstream["sdkVersion"]
        || !package.mcp.is_empty()
        || package.tools.is_empty()
    {
        return Err("Windows 包的系统、架构、版本或上游 SDK 提交不匹配".into());
    }
    let entry = package
        .backend
        .as_ref()
        .and_then(|backend| backend.executables.get(&platform))
        .ok_or("Windows 包缺少当前架构的原生入口")?;
    let bytes = fs::read(installed.path.join(entry))?;
    if info["runtimeSha256"].as_str() != Some(hash(&bytes).as_str()) {
        return Err("Windows 原生程序校验失败".into());
    }
    Ok((bytes, serde_json::to_value(&package.tools)?))
}

pub(super) fn stage(
    root: &Path,
    source: &Path,
    manifest: &mut CapabilityPackManifest,
    arch: &str,
    exe: &[u8],
    tools: Value,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::copy(
        source.join("wsl-launcher.sh"),
        root.join("bin/tron-computer-use"),
    )?;
    fs::write(root.join("bin/tron-computer-use-windows.exe"), exe)?;
    manifest.tools = serde_json::from_value(tools)?;
    manifest.backend = Some(BackendEntrypoint {
        executables: [(format!("{arch}-linux"), "bin/tron-computer-use".into())].into(),
        args: vec!["backend".into()],
    });
    manifest
        .resources
        .push("bin/tron-computer-use-windows.exe".into());
    Ok(())
}