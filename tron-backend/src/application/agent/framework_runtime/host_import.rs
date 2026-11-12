//! Validate an imported archive in isolation before touching the installed catalog.
use super::*;

impl FrameworkHost {
    // The caller holds config_lock across this check and the import commit.
    pub(super) fn validate_center_import(
        &self,
        manifest: &serde_json::Value,
        reviewed: &serde_json::Value,
        expected_version: Option<&str>,
    ) -> Result<(), String> {
        if manifest != reviewed {
            return Err("Downloaded manifest differs from the reviewed release".into());
        }
        let parsed: eliteforge_agent_capability_runtime::CapabilityPackManifest =
            serde_json::from_value(manifest.clone()).map_err(|e| e.to_string())?;
        let all = crate::application::agent::query_installed_capability_packs(
            &self.0.pack_root,
            "en-US",
            false,
            None,
        )?;
        let current = all.iter().find(|p| p.id == parsed.id);
        if current.map(|p| p.version.as_str()) != expected_version {
            return Err("Installed version changed; refresh before installing".into());
        }
        if current.is_some_and(|p| {
            !crate::application::capability::center::newer(&parsed.version.to_string(), &p.version)
        }) {
            return Err("Only a newer release can replace an installed package".into());
        }
        for dependency in &parsed.dependencies {
            if !all.iter().any(|p| {
                p.id == dependency.id
                    && semver::Version::parse(&p.version)
                        .is_ok_and(|v| dependency.version.matches(&v))
            }) {
                return Err(format!(
                    "Missing compatible dependency: {} {}",
                    dependency.id, dependency.version
                ));
            }
        }
        Ok(())
    }

    pub(super) async fn validate_pack_import(&self, bytes: Vec<u8>) -> Result<Vec<u8>, String> {
        let host = self.clone();
        tokio::task::spawn_blocking(move || {
            use eliteforge_agent_capability_runtime::{CapabilityPackManager, InstallPolicy};
            let staging = tempfile::tempdir().map_err(|e| e.to_string())?;
            // Validate signed archives against the same host trust store as installation.
            let keys = host.0.pack_root.join("trusted-keys.json");
            if keys.exists() {
                std::fs::copy(keys, staging.path().join("trusted-keys.json"))
                    .map_err(|e| e.to_string())?;
            }
            let manager = CapabilityPackManager::new(staging.path()).map_err(|e| e.to_string())?;
            let installed = manager
                .install_bytes(&bytes, InstallPolicy::Development)
                .map_err(|e| e.to_string())?;
            let id = &installed.manifest.id;
            let contribution =
                super::host_navigation::read_navigation(&installed.path, &installed.manifest)?;
            if let Some(navigation) = host.0.navigation.get() {
                navigation.validate_pack(id, &contribution)?;
            } else if !contribution.domains.is_empty() || !contribution.menus.is_empty() {
                return Err("navigation service is unavailable".into());
            }
            if let Some(skills) = host.0.skill_registry.get() {
                let roots = installed
                    .manifest
                    .skills
                    .iter()
                    .map(|path| installed.path.join(path))
                    .collect::<Vec<_>>();
                skills
                    .validate_pack_directories(id, &roots)
                    .map_err(|e| e.to_string())?;
            }
            if let (Some(mcp), Some(resources)) =
                (host.0.mcp.get(), host.0.capability_resources.get())
            {
                let existing = mcp
                    .list_platform_servers(crate::domain::mcp::PlatformMcpServerListQuery {
                        search: None,
                        status: None,
                    })
                    .map_err(|e| e.to_string())?
                    .into_iter()
                    .map(|server| server.server_code)
                    .collect::<std::collections::BTreeSet<_>>();
                for declaration in &installed.manifest.mcp {
                    let owner = resources.resource("mcp", &declaration.id)?;
                    if owner.as_ref().is_some_and(|owner| &owner.pack_id != id)
                        || (owner.is_none() && existing.contains(&declaration.id))
                    {
                        return Err(format!(
                            "MCP {} conflicts with an existing server",
                            declaration.id
                        ));
                    }
                }
            }
            Ok(bytes)
        })
        .await
        .map_err(|e| e.to_string())?
    }
}
