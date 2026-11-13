//! Build one platform variant of the Computer Use capability pack and validate
//! it with the real installer.
mod variant;
mod wsl2;
use eliteforge_agent_capability_runtime::{
    BackendEntrypoint, CapabilityPackManager, CapabilityPackManifest, InstallPolicy,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args_os().len() > 1 {
        return Err(
            "用法：tron-computer-use-packager（无参数；平台变体由 TRON_COMPUTER_USE_TARGET 指定）"
                .into(),
        );
    }
    let repository = fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.."))?;
    let source = repository.join("capability-packs/computer-use");
    let mut manifest: CapabilityPackManifest =
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
    let variant = variant::current()?;
    let metadata = cargo_metadata(&source)?;
    let native_version = metadata["packages"]
        .as_array()
        .ok_or("invalid Cargo metadata")?
        .iter()
        .find(|package| package["name"] == "tron-computer-use")
        .ok_or("missing native package")?;
    if native_version["version"].as_str() != Some(&manifest.version.to_string()) {
        return Err("能力包 manifest 与 native/Cargo.toml 版本不一致".into());
    }
    let upstream: Value = serde_json::from_slice(&fs::read(source.join("upstream.json"))?)?;
    validate_upstream(&source, &metadata, &upstream)?;

    let stage = tempfile::tempdir()?;
    let root = stage.path();
    fs::create_dir_all(root.join("bin"))?;
    copy_tree(&source.join("skills"), &root.join("skills"))?;
    for file in ["README.md", "CUA-LICENSE.md", "upstream.json"] {
        fs::copy(source.join(file), root.join(file))?;
    }
    manifest.id = variant.id.clone();
    manifest.name = variant.name.clone();

    let (mode, target_os, entrypoint) = if variant.family == variant::Family::Wsl {
        let (exe, tools) =
            wsl_runtime(&source, &manifest.version.to_string(), &variant, &upstream)?;
        wsl2::stage(root, &source, &mut manifest, &variant.arch, &exe, tools)?;
        ("wsl2", "linux", "bin/tron-computer-use".to_owned())
    } else {
        let binary = build_native(&source, variant.rust_target.as_deref())?;
        let catalog = Command::new(&binary).arg("catalog").output()?;
        if !catalog.status.success() {
            return Err("读取原生能力包工具目录失败".into());
        }
        manifest.tools = serde_json::from_slice(&catalog.stdout)?;
        let name = format!("bin/tron-computer-use{}", std::env::consts::EXE_SUFFIX);
        fs::copy(&binary, root.join(&name))?;
        manifest.backend = Some(BackendEntrypoint {
            executables: [(variant.platform_key.clone(), name.clone())].into(),
            args: vec!["backend".into()],
        });
        ("native", variant.family.os(), name)
    };
    fs::write(
        root.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    fs::write(
        root.join("build-info.json"),
        serde_json::to_vec_pretty(&json!({
            "mode": mode, "targetOs": target_os, "targetArch": variant.arch,
            "integration": "framework-native",
            "upstreamRevision": upstream["revision"], "sdkVersion": upstream["sdkVersion"],
            "runtimeSha256": hash(&fs::read(root.join(&entrypoint))?),
        }))?,
    )?;
    let archive = eliteforge_agent_capability_runtime::build_capability_archive(root, None)?;
    let validation = tempfile::tempdir()?;
    let installed = CapabilityPackManager::new(validation.path())?
        .install_bytes(&archive, InstallPolicy::Development)?;
    if installed.manifest.id != variant.id || installed.manifest.version != manifest.version {
        return Err("归档校验后的能力包身份与目标变体不一致".into());
    }
    let output = variant::output_directory(&repository);
    fs::create_dir_all(&output)?;
    let target = output.join(format!("{}-{}.zip", variant.id, manifest.version));
    fs::write(&target, &archive)?;
    println!("{}", target.display());
    Ok(())
}

fn wsl_runtime(
    source: &Path,
    version: &str,
    variant: &variant::Variant,
    upstream: &Value,
) -> Result<(Vec<u8>, Value), Box<dyn std::error::Error>> {
    if let Some(package) =
        std::env::var_os("TRON_COMPUTER_USE_WINDOWS_PACKAGE").filter(|value| !value.is_empty())
    {
        return wsl2::windows_package(Path::new(&package), version, &variant.arch, upstream);
    }
    if !cfg!(windows) {
        return Err("WSL2 包需要 Windows 原生运行时；请在 Windows 上构建，或用 COMPUTER_USE_WINDOWS_PACKAGE=<同版本同架构 Windows 包 ZIP>".into());
    }
    let binary = build_native(source, variant.rust_target.as_deref())?;
    let catalog = Command::new(&binary).arg("catalog").output()?;
    if !catalog.status.success() {
        return Err("读取原生能力包工具目录失败".into());
    }
    Ok((fs::read(binary)?, serde_json::from_slice(&catalog.stdout)?))
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

fn build_native(
    source: &Path,
    rust_target: Option<&str>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut command = cargo(source);
    command.args([
        "build",
        "--locked",
        "--release",
        "--manifest-path",
        "native/Cargo.toml",
        "--message-format=json-render-diagnostics",
    ]);
    if let Some(target) = rust_target {
        command.args(["--target", target]);
    }
    let output = command.stderr(Stdio::inherit()).output()?;
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
            return Err("Cua 依赖与 upstream.json 记录的提交或版本不一致".into());
        }
    }
    let sdk_manifest = PathBuf::from(sdk["manifest_path"].as_str().ok_or("missing SDK manifest")?);
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