//! Install desktop-shipped packages before the profile's skill/MCP catalog is bound.
use eliteforge_agent_capability_runtime::{CapabilityPackManager, InstallPolicy, safe_relative};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Catalog {
    schema_version: u32,
    target_os: String,
    target_arch: String,
    mode: String,
    packs: Vec<Bundle>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Bundle {
    id: String,
    version: String,
    file: String,
    sha256: String,
}

pub(super) fn install(root: &Path) -> Result<Vec<String>, String> {
    let Some(directory) =
        std::env::var_os("TRON_BUNDLED_CAPABILITY_PACKS_DIR").filter(|value| !value.is_empty())
    else {
        return Ok(Vec::new());
    };
    install_from(root, &PathBuf::from(directory))
        .map_err(|error| format!("desktop capability bundle initialization failed: {error}"))
}

fn install_from(root: &Path, directory: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let directory = directory.canonicalize()?;
    let catalog_path = directory.join("catalog.json");
    if fs::metadata(&catalog_path)?.len() > 1024 * 1024 {
        return Err("desktop capability catalog exceeds 1 MiB".into());
    }
    let catalog: Catalog = serde_json::from_slice(&fs::read(catalog_path)?)?;
    let wsl = cfg!(target_os = "linux")
        && fs::read_to_string("/proc/sys/kernel/osrelease")
            .is_ok_and(|release| release.to_ascii_lowercase().contains("microsoft"));
    let expected_mode = if wsl { "wsl2" } else { "native" };
    if catalog.schema_version != 1
        || catalog.target_os != std::env::consts::OS
        || catalog.target_arch != std::env::consts::ARCH
        || catalog.mode != expected_mode
    {
        return Err(format!(
            "desktop capability bundle does not match {}/{}/{}",
            std::env::consts::OS,
            std::env::consts::ARCH,
            expected_mode
        )
        .into());
    }
    let manager = CapabilityPackManager::new(root)?;
    let receipts_path = root.join("desktop-bundles.json");
    let mut receipts: BTreeMap<String, String> = match fs::read(&receipts_path) {
        Ok(bytes) => serde_json::from_slice(&bytes)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => BTreeMap::new(),
        Err(error) => return Err(error.into()),
    };
    let previous = receipts.clone();
    let installed = manager.installed()?;
    let mut defaults = Vec::new();
    for bundle in catalog.packs {
        let version = semver::Version::parse(&bundle.version)?;
        let current = installed
            .iter()
            .filter(|pack| pack.manifest.id == bundle.id)
            .max_by(|left, right| left.manifest.version.cmp(&right.manifest.version));
        // An absent package after a prior delivery is an explicit uninstall.
        // Preserve that decision, just as capability enablement is preserved.
        if current.is_none() && receipts.contains_key(&bundle.id) {
            continue;
        }
        if current.is_some_and(|pack| pack.manifest.version > version) {
            defaults.push(bundle.id.clone());
            receipts.insert(bundle.id, bundle.version);
            continue;
        }
        let archive_path = directory
            .join(safe_relative(&bundle.file)?)
            .canonicalize()?;
        if !archive_path.starts_with(&directory)
            || fs::metadata(&archive_path)?.len() > 128 * 1024 * 1024
        {
            return Err("invalid bundled capability archive path or size".into());
        }
        let archive = fs::read(archive_path)?;
        if format!("{:x}", Sha256::digest(&archive)) != bundle.sha256 {
            return Err(format!("bundled archive checksum mismatch: {}", bundle.id).into());
        }
        let validation = tempfile::tempdir()?;
        let validated = CapabilityPackManager::new(validation.path())?
            .install_bytes(&archive, InstallPolicy::Development)?;
        if validated.manifest.id != bundle.id || validated.manifest.version != version {
            return Err(format!("bundled archive identity mismatch: {}", bundle.id).into());
        }
        if current
            .is_some_and(|pack| pack.manifest.version == version && pack.digest != validated.digest)
        {
            return Err(format!("bundled capability {} version {} changed; increment its version before distribution", bundle.id, version).into());
        }
        if current.is_none_or(|pack| pack.manifest.version < version) {
            manager.install_bytes(&archive, InstallPolicy::Development)?;
            tracing::info!(pack_id = %bundle.id, %version, "Installed desktop bundled capability package");
        }
        defaults.push(bundle.id.clone());
        receipts.insert(bundle.id, bundle.version);
    }
    if receipts != previous {
        let temporary = receipts_path.with_extension("tmp");
        fs::write(&temporary, serde_json::to_vec_pretty(&receipts)?)?;
        fs::rename(temporary, receipts_path)?;
    }
    Ok(defaults)
}
