//! Resolve which platform variant this build produces.
//!
//! Each platform ships as its own capability pack so a download only carries
//! the binaries that platform can run. The pack id encodes the platform variant.
use std::{env, fs, path::Path};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Family {
    Windows,
    Macos,
    Linux,
    Wsl,
}

impl Family {
    fn parse(value: &str) -> Result<Self, Box<dyn std::error::Error>> {
        match value.trim().to_ascii_lowercase().as_str() {
            "windows" => Ok(Self::Windows),
            "macos" | "mac" | "darwin" => Ok(Self::Macos),
            "linux" => Ok(Self::Linux),
            "wsl" | "wsl2" => Ok(Self::Wsl),
            other => Err(format!(
                "未知的 TRON_COMPUTER_USE_TARGET：{other}；可选 windows、macos、linux、wsl"
            )
            .into()),
        }
    }

    fn slug(self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::Macos => "macos",
            Self::Linux => "linux",
            Self::Wsl => "wsl",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Windows => "Windows",
            Self::Macos => "macOS",
            Self::Linux => "Linux",
            Self::Wsl => "Windows WSL2",
        }
    }

    pub(super) fn os(self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::Macos => "macos",
            Self::Linux => "linux",
            Self::Wsl => "linux",
        }
    }
}

pub(super) struct Variant {
    pub family: Family,
    pub arch: String,
    pub rust_target: Option<String>,
    pub id: String,
    pub name: String,
    /// Platform key the framework uses to pick the backend executable.
    pub platform_key: String,
}

pub(super) fn is_wsl() -> bool {
    cfg!(target_os = "linux")
        && fs::read_to_string("/proc/sys/kernel/osrelease")
            .is_ok_and(|value| value.to_ascii_lowercase().contains("microsoft"))
}

pub(super) fn current() -> Result<Variant, Box<dyn std::error::Error>> {
    let wsl = is_wsl();
    let family = match env::var("TRON_COMPUTER_USE_TARGET")
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        Some(value) => Family::parse(&value)?,
        None if wsl => Family::Wsl,
        None if cfg!(windows) => Family::Windows,
        None if cfg!(target_os = "macos") => Family::Macos,
        None if cfg!(target_os = "linux") => Family::Linux,
        None => {
            return Err(
                "无法识别当前系统，请显式传入 TRON_COMPUTER_USE_TARGET=windows|macos|linux|wsl"
                    .into(),
            );
        }
    };
    match family {
        Family::Windows if !cfg!(windows) => {
            return Err("Windows 能力包只能在 Windows 上构建".into());
        }
        Family::Macos if !cfg!(target_os = "macos") => {
            return Err("macOS 能力包只能在 macOS 上构建".into());
        }
        Family::Linux if !cfg!(target_os = "linux") || wsl => {
            return Err("Linux 能力包只能在原生 Linux 上构建；WSL2 请用 TARGET=wsl".into());
        }
        Family::Wsl
            if !wsl
                && !cfg!(windows)
                && env::var_os("TRON_COMPUTER_USE_WINDOWS_PACKAGE")
                    .filter(|value| !value.is_empty())
                    .is_none() =>
        {
            // Assembly only embeds a Windows runtime, so a provided package
            // makes the host platform irrelevant.
            return Err(
                "WSL2 能力包需要 Windows 运行时；请传入 COMPUTER_USE_WINDOWS_PACKAGE，或在 Windows/WSL2 中构建"
                    .into(),
            );
        }
        _ => {}
    }

    let rust_target = env::var("TRON_COMPUTER_USE_RUST_TARGET")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    if let Some(target) = rust_target.as_deref() {
        let expected = family.os();
        let matches = match family {
            Family::Windows => target.contains("windows"),
            Family::Macos => target.contains("apple-darwin"),
            Family::Linux | Family::Wsl => target.contains("linux"),
        };
        if !matches {
            return Err(format!(
                "Rust 目标 {target} 与能力包平台 {expected} 不一致"
            )
            .into());
        }
    }
    let arch = rust_target
        .as_deref()
        .and_then(|target| target.split('-').next())
        .unwrap_or(std::env::consts::ARCH)
        .to_owned();
    if !matches!(arch.as_str(), "x86_64" | "aarch64") {
        return Err(format!("不支持的架构：{arch}；仅支持 x86_64、aarch64").into());
    }

    let id = format!("tron.computer-use.{}-{arch}", family.slug());
    let name = format!(
        "Computer Use · {} ({})",
        family.label(),
        arch_label(family, &arch)
    );
    let platform_key = format!("{arch}-{}", family.os());
    Ok(Variant {
        family,
        arch,
        rust_target,
        id,
        name,
        platform_key,
    })
}

fn arch_label(family: Family, arch: &str) -> &'static str {
    match (family, arch) {
        (Family::Macos, "aarch64") => "Apple Silicon",
        (Family::Macos, "x86_64") => "Intel",
        (_, "aarch64") => "arm64",
        _ => "x86_64",
    }
}

/// The Windows package that a WSL2 pack may embed as its desktop runtime.
pub(super) fn is_windows_package(id: &str) -> bool {
    id == "tron.computer-use" || id.starts_with("tron.computer-use.windows-")
}

pub(super) fn output_directory(repository: &Path) -> std::path::PathBuf {
    repository.join("output/capability-packs/computer-use")
}