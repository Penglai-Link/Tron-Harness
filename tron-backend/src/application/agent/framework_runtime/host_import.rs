//! Validate an imported archive in isolation before touching the installed catalog.
use super::*;

impl FrameworkHost {
    pub(super) async fn validate_pack_import(&self, bytes: Vec<u8>) -> Result<Vec<u8>, String> {
        let host = self.clone();
        tokio::task::spawn_blocking(move || {
            use eliteforge_agent_capability_runtime::{CapabilityPackManager, InstallPolicy};
            let staging = tempfile::tempdir().map_err(|e| e.to_string())?;
            let manager = CapabilityPackManager::new(staging.path()).map_err(|e| e.to_string())?;
            let installed = manager
                .install_bytes(&bytes, InstallPolicy::Development)
                .map_err(|e| e.to_string())?;
            let id = &installed.manifest.id;
            let contribution = super::host_navigation::read_navigation(&installed.path, &installed.manifest)?;
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
