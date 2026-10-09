use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityPackInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub provider: String,
    pub updated_at: Option<String>,
    pub effective: bool,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionImplementationInfo {
    pub extension_point: String,
    pub extension_name: String,
    pub implementation: String,
    pub description: String,
    pub provider_pack_id: String,
    pub provider: String,
    pub effective: bool,
}

pub fn installed_capability_packs(
    root: &std::path::Path,
) -> Result<Vec<CapabilityPackInfo>, String> {
    use eliteforge_agent_capability_runtime::CapabilityPackManager;
    let manager = CapabilityPackManager::new(root).map_err(|error| error.to_string())?;
    let mut packs = capability_pack_catalog();
    for installed in manager.installed().map_err(|error| error.to_string())? {
        packs.push(CapabilityPackInfo {
            id: installed.manifest.id,
            name: installed.manifest.name,
            version: installed.manifest.version.to_string(),
            description: "Imported external capability package".into(),
            provider: "External".into(),
            updated_at: None,
            effective: false,
            kind: "capability".into(),
        });
    }
    Ok(packs)
}

pub fn install_capability_pack(
    root: &std::path::Path,
    bytes: &[u8],
) -> Result<CapabilityPackInfo, String> {
    use eliteforge_agent_capability_runtime::{CapabilityPackManager, InstallPolicy};
    let manager = CapabilityPackManager::new(root).map_err(|error| error.to_string())?;
    let installed = manager
        .install_bytes(bytes, InstallPolicy::Development)
        .map_err(|error| error.to_string())?;
    Ok(CapabilityPackInfo {
        id: installed.manifest.id,
        name: installed.manifest.name,
        version: installed.manifest.version.to_string(),
        description: "Imported external capability package".into(),
        provider: "External".into(),
        updated_at: None,
        effective: false,
        kind: "capability".into(),
    })
}

pub fn capability_pack_catalog() -> Vec<CapabilityPackInfo> {
    [
        (
            "framework.external-agent",
            "External Agent",
            "External Agent Invocation and Session Bridge",
            "EliteForge",
            "0.1.0",
            Some("2026-09-27"),
        ),
        (
            "framework.local.scoped",
            "Scoped Local Tools",
            "Scoped filesystem, command and shared PTY capabilities",
            "EliteForge",
            "0.1.0",
            Some("2026-09-27"),
        ),
        (
            eliteforge_agent_capability_tools::AGENT_COLLABORATION_PACK,
            "Agent Collaboration",
            "Named child-agent delegation and independent threads",
            "EliteForge",
            "0.1.0",
            Some("2026-10-09"),
        ),
        (
            "tron.harness",
            "Tron Harness",
            "Tron context preparation and product-specific agent tools",
            "Tron",
            "0.1.0",
            Some("2026-09-27"),
        ),
    ]
    .into_iter()
    .map(
        |(id, name, description, provider, version, updated_at)| CapabilityPackInfo {
            id: id.into(),
            name: name.into(),
            version: version.into(),
            description: description.into(),
            provider: provider.into(),
            updated_at: updated_at.map(str::to_owned),
            effective: true,
            kind: "capability".into(),
        },
    )
    .collect()
}

pub fn capability_pack_catalog_with_state(
    enabled: &std::collections::BTreeMap<String, bool>,
) -> Vec<CapabilityPackInfo> {
    capability_pack_catalog()
        .into_iter()
        .map(|mut pack| {
            pack.effective = enabled.get(&pack.id).copied().unwrap_or(true);
            pack
        })
        .collect()
}

pub fn extension_implementation_catalog() -> Vec<ExtensionImplementationInfo> {
    [
        (
            "model",
            "Model",
            "TronChatModel",
            tron_agent_harness::MODEL_IMPLEMENTATION_DESCRIPTION,
            "framework.model.connection.tron",
            "Tron",
        ),
        (
            "tool",
            "Tool",
            "ScopedLocalToolsPlugin + Tron HarnessTools",
            "",
            "framework.local.scoped",
            "EliteForge / Tron",
        ),
        (
            "observe",
            "Observe",
            "ToolAudit middleware",
            tron_agent_harness::OBSERVATION_IMPLEMENTATION_DESCRIPTION,
            "tron.tool.audit",
            "Tron",
        ),
        (
            "context",
            "Context",
            "TronContext + bounded context preparation",
            tron_agent_harness::CONTEXT_IMPLEMENTATION_DESCRIPTION,
            "tron.harness",
            "Tron",
        ),
        (
            "runtime",
            "Runtime",
            "Independent ExternalAgentLoop",
            "Independent external execution with native session ownership and observation events",
            "framework.external-agent",
            "EliteForge",
        ),
        (
            "storage",
            "Storage",
            "LocalStoragePlugin",
            eliteforge_agent_official_plugin_storage_local::STORAGE_IMPLEMENTATION_DESCRIPTION,
            "framework.storage.local",
            "EliteForge",
        ),
    ]
    .into_iter()
    .map(
        |(
            extension_point,
            extension_name,
            implementation,
            description,
            provider_pack_id,
            provider,
        )| {
            let description = match extension_point {
                "tool" => format!(
                    "{} {}",
                    eliteforge_agent_official_plugin_tools_local::TOOL_IMPLEMENTATION_DESCRIPTION,
                    tron_agent_harness::TOOL_IMPLEMENTATION_DESCRIPTION,
                ),
                _ => description.to_owned(),
            };
            ExtensionImplementationInfo {
                extension_point: extension_point.into(),
                extension_name: extension_name.into(),
                implementation: implementation.into(),
                description,
                provider_pack_id: provider_pack_id.into(),
                provider: provider.into(),
                effective: true,
            }
        },
    )
    .collect()
}

pub fn extension_implementation_catalog_with_state(
    enabled: &std::collections::BTreeMap<String, bool>,
) -> Vec<ExtensionImplementationInfo> {
    extension_implementation_catalog()
        .into_iter()
        .map(|mut implementation| {
            implementation.effective = enabled
                .get(&implementation.extension_point)
                .copied()
                .unwrap_or(true);
            implementation
        })
        .collect()
}
