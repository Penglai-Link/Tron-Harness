//! Build a self-contained capability pack and validate it with the real installer.
mod desktop;
use eliteforge_agent_capability_runtime::{
    BackendEntrypoint, CapabilityPackManager, InstallPolicy,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    let check_cache = args.len() == 1 && args[0] == "--check-desktop-cache";
    let desktop_directory = match args.as_slice() {
        [] => None,
        [_] if check_cache => None,
        [flag, path] if flag == "--desktop-dir" => Some(PathBuf::from(path)),
        _ => return Err("用法：tron-computer-use-packager [--check-desktop-cache | --desktop-dir <桌面资源目录>]".into()),
    };
    let repository = fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.."))?;
    let source = repository.join("capability-packs/computer-use");
    let mut manifest: eliteforge_agent_capability_runtime::CapabilityPackManifest =
        serde_json::from_slice(&fs::read(source.join("manifest.json"))?)?;
    if let Ok(version) = std::env::var("TRON_COMPUTER_USE_VERSION") {
        if !version.is_empty() && version != manifest.version.to_string() {
            return Err(format!(
                "VERSION 必须与 Computer Use manifest 版本 {} 一致",
                manifest.version
            )
            .into());
        }
    }
    if let Ok(skip) = std::env::var("TRON_COMPUTER_USE_SKIP_TESTS") {
        if !matches!(skip.as_str(), "true" | "false") {
            return Err("SKIP_TESTS 只接受 true 或 false；本目标不运行测试".into());
        }
    }
    let wsl = cfg!(target_os = "linux")
        && fs::read_to_string("/proc/sys/kernel/osrelease")
            .is_ok_and(|value| value.to_ascii_lowercase().contains("microsoft"));
    let windows_package =
        std::env::var_os("TRON_COMPUTER_USE_WINDOWS_PACKAGE").filter(|value| !value.is_empty());
    if wsl && (desktop_directory.is_some() || check_cache) {
        return Err("桌面安装包请在原生 Windows、macOS 或 Linux 环境构建；WSL2 能力包由 Windows 桌面构建自动组装".into());
    }
    if wsl && windows_package.is_none() {
        return Err("WSL2 包必须包含 Windows 原生运行时；请先在 Windows 构建同版本包，再传入 COMPUTER_USE_WINDOWS_PACKAGE=<Windows ZIP>。运行用户无需安装 cua-driver".into());
    }
    if !wsl && windows_package.is_some() {
        return Err("COMPUTER_USE_WINDOWS_PACKAGE 仅用于在 WSL2 中组装 Windows 桌面能力包".into());
    }

    if check_cache {
        if let Err(error) = desktop::check(&repository, &source, &manifest) {
            eprintln!("现成能力包不可用：{error}");
            std::process::exit(2);
        }
        return Ok(());
    }
    if let Some(directory) = desktop_directory {
        return desktop::prepare(&repository, &source, &manifest, &directory);
    }

    let metadata = cargo_metadata(&source)?;
    let upstream: Value = serde_json::from_slice(&fs::read(source.join("upstream.json"))?)?;
    validate_upstream(&source, &metadata, &upstream)?;
    let stage = tempfile::tempdir()?;
    let root = stage.path();
    fs::create_dir_all(root.join("bin"))?;
    copy_tree(&source.join("skills"), &root.join("skills"))?;
    for file in ["README.md", "CUA-LICENSE.md", "upstream.json"] {
        fs::copy(source.join(file), root.join(file))?;
    }
    let executable = format!("bin/tron-computer-use{}", std::env::consts::EXE_SUFFIX);
    manifest.backend = Some(BackendEntrypoint {
        executables: [(
            eliteforge_agent_capability_runtime::platform_key(),
            executable.clone(),
        )]
        .into(),
        args: vec!["backend".into()],
    });
    let native_version = metadata["packages"]
        .as_array()
        .ok_or("invalid Cargo metadata")?
        .iter()
        .find(|package| package["name"] == "tron-computer-use")
        .ok_or("missing native package")?;
    if native_version["version"].as_str() != Some(&manifest.version.to_string()) {
        return Err("能力包 manifest 与 native/Cargo.toml 版本不一致".into());
    }
    if wsl {
        include_windows_runtime(
            root,
            &mut manifest,
            Path::new(windows_package.as_ref().unwrap()),
            &upstream,
        )?;
        fs::copy(source.join("wsl-launcher.sh"), root.join(&executable))?;
    } else {
        let binary = build_native(&source)?;
        let catalog = Command::new(&binary).arg("catalog").output()?;
        if !catalog.status.success() {
            return Err("读取原生能力包工具目录失败".into());
        }
        manifest.tools = serde_json::from_slice(&catalog.stdout)?;
        fs::copy(binary, root.join(&executable))?;
    }
    fs::write(
        root.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    let mode = if wsl { "wsl2" } else { "native" };
    fs::write(
        root.join("build-info.json"),
        serde_json::to_vec_pretty(&json!({
            "mode": mode, "targetOs": std::env::consts::OS, "targetArch": std::env::consts::ARCH,
            "integration": "framework-native",
            "upstreamRevision": upstream["revision"], "sdkVersion": upstream["sdkVersion"],
            "runtimeSha256": hash(&fs::read(root.join(&executable))?),
        }))?,
    )?;
    let archive = eliteforge_agent_capability_runtime::build_capability_archive(root, None)?;
    let validation = tempfile::tempdir()?;
    CapabilityPackManager::new(validation.path())?
        .install_bytes(&archive, InstallPolicy::Development)?;
    let output = repository.join("output/capability-packs/computer-use");
    fs::create_dir_all(&output)?;
    let os = if wsl { "wsl2" } else { std::env::consts::OS };
    let target = output.join(format!(
        "{}-{}-{os}-{}.zip",
        manifest.id,
        manifest.version,
        std::env::consts::ARCH
    ));
    fs::write(&target, &archive)?;
    println!("{}", target.display());
    Ok(())
}

fn cargo(source: &Path) -> Command {
    let mut command = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    command.current_dir(source);
    command
}

fn cargo_metadata(source: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let output = cargo(source)
        .args([
            "metadata",
            "--locked",
            "--format-version",
            "1",
            "--manifest-path",
            "native/Cargo.toml",
        ])
        .stderr(Stdio::inherit())
        .output()?;
    if !output.status.success() {
        return Err("读取已锁定的 Cua SDK 依赖失败".into());
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn build_native(source: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let output = cargo(source)
        .args([
            "build",
            "--locked",
            "--release",
            "--manifest-path",
            "native/Cargo.toml",
            "--message-format=json-render-diagnostics",
        ])
        .stderr(Stdio::inherit())
        .output()?;
    if !output.status.success() {
        return Err("内嵌 Cua SDK 原生程序编译失败".into());
    }
    String::from_utf8(output.stdout)?
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|message| {
            message["reason"] == "compiler-artifact"
                && message["target"]["name"] == "tron-computer-use"
                && message["executable"].is_string()
        })
        .and_then(|message| message["executable"].as_str().map(PathBuf::from))
        .ok_or_else(|| "Cargo 未返回能力包原生程序路径".into())
}

fn validate_upstream(
    source: &Path,
    metadata: &Value,
    upstream: &Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let packages = metadata["packages"]
        .as_array()
        .ok_or("invalid Cargo metadata")?;
    let sdk = packages
        .iter()
        .find(|package| package["name"] == "cua-driver-sdk")
        .ok_or("missing Cua SDK dependency")?;
    let revision = upstream["revision"]
        .as_str()
        .ok_or("missing upstream revision")?;
    for name in ["cua-driver-sdk", "cua-driver-core"] {
        let dependency = packages
            .iter()
            .find(|package| package["name"] == name)
            .ok_or("missing Cua dependency")?;
        if dependency["source"]
            .as_str()
            .and_then(|value| value.rsplit_once('#').map(|(_, hash)| hash))
            != Some(revision)
            || dependency["version"] != upstream["sdkVersion"]
        {
            return Err(format!("{name} 与 upstream.json 的 Git 提交或版本不一致").into());
        }
    }
    let sdk_manifest = Path::new(
        sdk["manifest_path"]
            .as_str()
            .ok_or("missing SDK manifest path")?,
    );
    let rust = sdk_manifest
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .ok_or("invalid upstream workspace")?;
    let files = upstream["files"]
        .as_object()
        .ok_or("missing upstream file hashes")?;
    for (path, digest) in files {
        eliteforge_agent_capability_runtime::safe_relative(path)?;
        let bytes = fs::read(source.join(path))?;
        if Some(hash(&bytes).as_str()) != digest.as_str() {
            return Err(format!("官方资源已被修改：{path}").into());
        }
        let original = if path == "CUA-LICENSE.md" {
            rust.join("../../../LICENSE.md")
        } else {
            rust.join("Skills/cua-driver").join(
                path.strip_prefix("skills/cua-driver/")
                    .ok_or("invalid skill path")?,
            )
        };
        if fs::read(original)? != bytes {
            return Err(format!("官方资源与 SDK 提交不一致：{path}").into());
        }
    }
    Ok(())
}

fn include_windows_runtime(
    root: &Path,
    manifest: &mut eliteforge_agent_capability_runtime::CapabilityPackManifest,
    archive: &Path,
    upstream: &Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let validation = tempfile::tempdir()?;
    let package = CapabilityPackManager::new(validation.path())?
        .install_bytes(&fs::read(archive)?, InstallPolicy::Development)?;
    let info: Value = serde_json::from_slice(&fs::read(package.path.join("build-info.json"))?)?;
    if package.manifest.id != manifest.id
        || package.manifest.version != manifest.version
        || info["mode"] != "native"
        || info["targetOs"] != "windows"
        || info["targetArch"] != std::env::consts::ARCH
        || info["upstreamRevision"] != upstream["revision"]
        || info["sdkVersion"] != upstream["sdkVersion"]
    {
        return Err("Windows 包的系统、架构、能力包版本或上游 SDK 提交不匹配".into());
    }
    let binary = fs::read(package.path.join("bin/tron-computer-use.exe"))?;
    if info["runtimeSha256"].as_str() != Some(hash(&binary).as_str()) {
        return Err("Windows 原生程序校验失败".into());
    }
    if !package.manifest.mcp.is_empty() || package.manifest.backend.is_none() {
        return Err("Windows 包必须使用 Framework 原生能力包入口".into());
    }
    manifest.tools = package.manifest.tools;
    fs::write(root.join("bin/tron-computer-use-windows.exe"), binary)?;
    manifest
        .resources
        .push("bin/tron-computer-use-windows.exe".into());
    Ok(())
}

fn copy_tree(source: &Path, target: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            return Err("资源不允许使用软链接".into());
        }
        if kind.is_dir() {
            copy_tree(&entry.path(), &target.join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), target.join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
