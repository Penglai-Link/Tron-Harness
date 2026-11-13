use super::*;
use eliteforge_agent_capability_runtime::CapabilityPackManifest;

pub(super) fn prepare(
    repository: &Path,
    source: &Path,
    manifest: &CapabilityPackManifest,
    directory: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let upstream: Value = serde_json::from_slice(&fs::read(source.join("upstream.json"))?)?;
    let archive = cache_path(repository, manifest);
    let validation = tempfile::tempdir()?;
    let (installed, bytes) = cached_package(&archive, validation.path(), manifest, &upstream)?;
    println!("复用现成能力包：{}", archive.display());
    publish(
        source,
        &installed.path,
        manifest,
        &upstream,
        &bytes,
        directory,
    )
}

pub(super) fn check(
    repository: &Path,
    source: &Path,
    manifest: &CapabilityPackManifest,
) -> Result<(), Box<dyn std::error::Error>> {
    let upstream: Value = serde_json::from_slice(&fs::read(source.join("upstream.json"))?)?;
    let validation = tempfile::tempdir()?;
    cached_package(
        &cache_path(repository, manifest),
        validation.path(),
        manifest,
        &upstream,
    )?;
    Ok(())
}

fn cache_path(repository: &Path, manifest: &CapabilityPackManifest) -> PathBuf {
    repository
        .join("output/capability-packs/computer-use")
        .join(format!(
            "{}-{}-{}-{}.zip",
            manifest.id,
            manifest.version,
            std::env::consts::OS,
            std::env::consts::ARCH
        ))
}

fn cached_package(
    archive: &Path,
    validation: &Path,
    expected: &CapabilityPackManifest,
    upstream: &Value,
) -> Result<
    (
        eliteforge_agent_capability_runtime::InstalledCapabilityPack,
        Vec<u8>,
    ),
    Box<dyn std::error::Error>,
> {
    let bytes = fs::read(archive)?;
    let installed = CapabilityPackManager::new(validation)?
        .install_bytes(&bytes, InstallPolicy::Development)?;
    let info: Value = serde_json::from_slice(&fs::read(installed.path.join("build-info.json"))?)?;
    if installed.manifest.id != expected.id
        || installed.manifest.version != expected.version
        || info["mode"] != "native"
        || info["targetOs"] != std::env::consts::OS
        || info["targetArch"] != std::env::consts::ARCH
        || info["upstreamRevision"] != upstream["revision"]
        || info["sdkVersion"] != upstream["sdkVersion"]
        || info["integration"] != "framework-native"
        || !installed.manifest.mcp.is_empty()
        || installed.manifest.tools.is_empty()
    {
        return Err("现成包的平台、架构、版本或 SDK 提交不匹配".into());
    }
    let entry = installed
        .manifest
        .backend
        .as_ref()
        .and_then(|backend| {
            backend
                .executables
                .get(&eliteforge_agent_capability_runtime::platform_key())
        })
        .ok_or("missing native capability backend entrypoint")?;
    if info["runtimeSha256"].as_str() != Some(hash(&fs::read(installed.path.join(entry))?).as_str())
    {
        return Err("现成包的原生程序哈希不匹配".into());
    }
    for (file, digest) in upstream["files"]
        .as_object()
        .ok_or("missing official file hashes")?
    {
        eliteforge_agent_capability_runtime::safe_relative(file)?;
        if digest.as_str() != Some(hash(&fs::read(installed.path.join(file))?).as_str()) {
            return Err(format!("现成包中的官方资源不匹配：{file}").into());
        }
    }
    Ok((installed, bytes))
}

fn publish(
    source: &Path,
    native: &Path,
    manifest: &CapabilityPackManifest,
    upstream: &Value,
    archive: &[u8],
    directory: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    publish_catalog(
        &directory.join("native"),
        std::env::consts::OS,
        "native",
        manifest,
        archive,
    )?;
    if cfg!(windows) {
        // A POSIX entrypoint lets a Windows build ship the WSL2 package without
        // installing a Linux Rust toolchain or compiling a Linux desktop SDK.
        let stage = tempfile::tempdir()?;
        let root = stage.path();
        copy_tree(native, root)?;
        // The installer adds package metadata that must be regenerated for
        // the modified WSL archive, not copied from the native package.
        for file in ["checksums.json", "signature.json", "receipt.json"] {
            match fs::remove_file(root.join(file)) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        let binary = "bin/tron-computer-use-windows.exe";
        fs::rename(root.join("bin/tron-computer-use.exe"), root.join(binary))?;
        fs::copy(
            source.join("wsl-launcher.sh"),
            root.join("bin/tron-computer-use"),
        )?;
        let mut manifest = manifest.clone();
        // Use the validated native manifest: it contains the SDK tool catalog.
        manifest.tools = serde_json::from_slice::<CapabilityPackManifest>(&fs::read(
            native.join("manifest.json"),
        )?)?
        .tools;
        manifest.backend = Some(BackendEntrypoint {
            executables: [(
                format!("{}-linux", std::env::consts::ARCH),
                "bin/tron-computer-use".into(),
            )]
            .into(),
            args: vec!["backend".into()],
        });
        manifest.resources.push(binary.into());
        fs::write(
            root.join("manifest.json"),
            serde_json::to_vec_pretty(&manifest)?,
        )?;
        fs::write(
            root.join("build-info.json"),
            serde_json::to_vec_pretty(&json!({
                "mode":"wsl2", "targetOs":"linux", "targetArch":std::env::consts::ARCH,
                "integration":"framework-native",
                "upstreamRevision":upstream["revision"], "sdkVersion":upstream["sdkVersion"],
                "runtimeSha256":hash(&fs::read(root.join("bin/tron-computer-use"))?),
            }))?,
        )?;
        let archive = eliteforge_agent_capability_runtime::build_capability_archive(root, None)?;
        let validation = tempfile::tempdir()?;
        CapabilityPackManager::new(validation.path())?
            .install_bytes(&archive, InstallPolicy::Development)?;
        publish_catalog(
            &directory.join("wsl2"),
            "linux",
            "wsl2",
            &manifest,
            &archive,
        )?;
    }
    println!("桌面内置能力包：{}", directory.display());
    Ok(())
}

fn publish_catalog(
    directory: &Path,
    os: &str,
    mode: &str,
    manifest: &CapabilityPackManifest,
    archive: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(directory)?;
    let file = "tron.computer-use.zip";
    fs::write(directory.join(file), archive)?;
    fs::write(
        directory.join("catalog.json"),
        serde_json::to_vec_pretty(&json!({
            "schemaVersion":1, "targetOs":os, "targetArch":std::env::consts::ARCH, "mode":mode,
            "packs":[{"id":manifest.id,"version":manifest.version.to_string(),"file":file,"sha256":hash(archive)}]
        }))?,
    )?;
    Ok(())
}
