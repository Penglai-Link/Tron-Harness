use serde::Serialize;

#[path = "capability_contents.rs"]
mod contents;
use contents::{CapabilityContentInfo, builtin_contents, manifest_contents};

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
    pub contents: Vec<CapabilityContentInfo>,
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
    pub enabled: bool,
    pub configuration: ExtensionConfiguration,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum ExtensionConfiguration {
    Required,
    Extension,
    CapabilityPack,
}

pub fn installed_capability_packs(
    root: &std::path::Path,
    locale: &str,
) -> Result<Vec<CapabilityPackInfo>, String> {
    use eliteforge_agent_capability_runtime::CapabilityPackManager;
    let manager = CapabilityPackManager::new(root).map_err(|error| error.to_string())?;
    let mut packs = capability_pack_catalog();
    for pack in &mut packs {
        for item in pack.contents.iter_mut().filter(|item| item.kind == "Plugin") {
            if let Some(name) = contents::plugin_display_name(&item.name, locale) {
                item.display_name = name.into();
            }
        }
        let descriptors=contents::builtin_tool_descriptors(&pack.id);
        for item in pack.contents.iter_mut().filter(|item|item.kind=="Tool") {
            if let Some(descriptor)=descriptors.iter().find(|d|d.call_name==item.name) {
                let display=descriptor.resolve_display(locale);
                item.description=display.description;
                item.display_name=display.name;
            }
        }
    }
    // Imported metadata belongs to its provider; never translate by a colliding
    // built-in call name or activate a plugin merely to display its manifest.
    for installed in manager.installed().map_err(|error| error.to_string())? {
        let mut contents=manifest_contents(&installed.manifest);
        for item in contents.iter_mut().filter(|item|item.kind=="Tool") {
            if let Some(entry)=installed.manifest.tools.iter().find(|entry|entry.descriptor.call_name==item.name) {
                let display=entry.descriptor.resolve_display(locale);
                item.display_name=display.name;item.description=display.description;
            }
        }
        packs.push(CapabilityPackInfo {
            contents,
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
        contents: manifest_contents(&installed.manifest),
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
            eliteforge_agent_official_plugin_a2ui::A2UI_PACK_ID,
            "A2UI Interfaces",
            "Optional interactive cards, forms, guidance and skills for A2UI v0.9.1 clients",
            "EliteForge",
            "0.1.0",
            Some("2026-10-10"),
        ),
        (
            "framework.personai",
            "PersonAI",
            "Web search, image generation, task querying and AIGC detection with host credentials",
            "EliteForge",
            "0.1.0",
            Some("2026-10-10"),
        ),
        (
            "framework.web",
            "Web Fetch",
            "Read public webpage text",
            "EliteForge",
            "0.1.0",
            Some("2026-10-10"),
        ),
        (
            "framework.media",
            "Media",
            "Read scoped images and PDF files or attachments",
            "EliteForge",
            "0.1.0",
            Some("2026-10-10"),
        ),
        (
            "tron.harness",
            "Tron Harness",
            "Tron context, business tool policy and bundled theme / memory skills",
            "Tron",
            "0.1.0",
            Some("2026-10-10"),
        ),
    ]
    .into_iter()
    .map(
        |(id, name, description, provider, version, updated_at)| CapabilityPackInfo {
            contents: builtin_contents(id),
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

pub(super) fn pack_extension_implementations() -> Vec<ExtensionImplementationInfo> {
    [
        (
            "model",
            "Model",
            "TronChatModel",
            tron_harness_capability::MODEL_IMPLEMENTATION_DESCRIPTION,
            "framework.model.connection.tron",
            "Tron",
        ),
        (
            "tool",
            "Tool",
            "ScopedLocalToolsPlugin",
            eliteforge_agent_official_plugin_tools_local::TOOL_IMPLEMENTATION_DESCRIPTION,
            "framework.local.scoped",
            "EliteForge",
        ),
        (
            "tool",
            "Tool",
            "Tron HarnessTools",
            "Tron business tool provider",
            "tron.harness",
            "Tron",
        ),
        (
            "tool-policy",
            "Tool Policy",
            "TronToolPolicy",
            "Platform tool permissions and risk approval",
            "tron.harness",
            "Tron",
        ),
        (
            "tool",
            "Tool",
            "AgentToolsPlugin",
            "Child-agent and independent thread tools",
            "framework.tool-pack.agent-collaboration",
            "EliteForge",
        ),
        (
            "tool",
            "Tool",
            "A2uiService",
            "Publish, expand and manage interactive surfaces",
            "framework.a2ui",
            "EliteForge",
        ),
        (
            "skill",
            "Skill",
            "A2UI Guidance",
            "A2UI interface instructions and reference resources",
            "framework.a2ui",
            "EliteForge",
        ),
        (
            "context",
            "Context",
            "A2UI Guidance",
            "Negotiated A2UI catalog and usage guidance",
            "framework.a2ui",
            "EliteForge",
        ),
        (
            "tool",
            "Tool",
            "PersonAiToolProvider",
            "Account-bound remote tools",
            "framework.personai",
            "EliteForge",
        ),
        (
            "tool",
            "Tool",
            "WebToolsPlugin",
            "Public webpage fetching",
            "framework.web",
            "EliteForge",
        ),
        (
            "tool",
            "Tool",
            "MediaToolsPlugin",
            "Scoped image and PDF readers",
            "framework.media",
            "EliteForge",
        ),
        (
            "observe",
            "Observe",
            "ToolAudit middleware",
            tron_harness_capability::OBSERVATION_IMPLEMENTATION_DESCRIPTION,
            "tron.tool.audit",
            "Tron",
        ),
        (
            "context",
            "Context",
            "TronContext + bounded context preparation",
            tron_harness_capability::CONTEXT_IMPLEMENTATION_DESCRIPTION,
            "tron.harness",
            "Tron",
        ),
        (
            "external-agent",
            "External Agent",
            "HostExternalAgent",
            "Host adapter for external Agent execution and session bridging",
            "framework.external-agent",
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
            ExtensionImplementationInfo {
                extension_point: extension_point.into(),
                extension_name: extension_name.into(),
                implementation: implementation.into(),
                description: description.to_owned(),
                provider_pack_id: provider_pack_id.into(),
                provider: provider.into(),
                effective: true,
                enabled: true,
                configuration: match (extension_point, provider_pack_id) {
                    ("model" | "storage" | "tool-policy", _) => ExtensionConfiguration::Required,
                    ("tool", "framework.local.scoped")
                    | ("observe", "tron.tool.audit")
                    | ("context", "tron.harness")
                    | ("runtime", "framework.external-agent") => ExtensionConfiguration::Extension,
                    _ => ExtensionConfiguration::CapabilityPack,
                },
            }
        },
    )
    .collect()
}

pub fn extension_implementation_catalog() -> Vec<ExtensionImplementationInfo> {
    pack_extension_implementations()
}

pub fn extension_implementation_catalog_with_state(
    packs: &std::collections::BTreeMap<String, bool>,
    enabled: &std::collections::BTreeMap<String, bool>,
) -> Vec<ExtensionImplementationInfo> {
    let extension_enabled = |point: &str| enabled.get(point).copied().unwrap_or(true);
    extension_implementation_catalog()
        .into_iter()
        .map(|mut implementation| {
            let pack_enabled = packs.get(&implementation.provider_pack_id).copied().unwrap_or(true);
            implementation.enabled = match implementation.configuration {
                ExtensionConfiguration::Required => true,
                ExtensionConfiguration::Extension => extension_enabled(&implementation.extension_point),
                ExtensionConfiguration::CapabilityPack => pack_enabled,
            };
            implementation.effective = match implementation.extension_point.as_str() {
                // These services remain installed even when optional pack contributions are omitted.
                "model" | "storage" | "tool-policy" => true,
                "tool" => pack_enabled && extension_enabled("tool"),
                "context" if implementation.provider_pack_id == "tron.harness" => {
                    pack_enabled && extension_enabled("context") && extension_enabled("tool")
                }
                "context" => pack_enabled && extension_enabled("tool"),
                "runtime" | "external-agent" => pack_enabled && extension_enabled("runtime"),
                "observe" => extension_enabled("observe"),
                _ => pack_enabled,
            };
            implementation
        })
        .collect()
}

pub fn installed_extension_implementations(
    root: &std::path::Path,
    packs: &std::collections::BTreeMap<String, bool>,
    enabled: &std::collections::BTreeMap<String, bool>,
) -> Result<Vec<ExtensionImplementationInfo>, String> {
    use eliteforge_agent_capability_runtime::CapabilityPackManager;
    let mut items = extension_implementation_catalog_with_state(packs, enabled);
    let manager = CapabilityPackManager::new(root).map_err(|error| error.to_string())?;
    for installed in manager.installed().map_err(|error| error.to_string())? {
        let manifest = installed.manifest;
        for implementation in manifest.implementations {
            let enabled = packs.get(&manifest.id).copied().unwrap_or(false);
            items.push(ExtensionImplementationInfo {
                extension_name: implementation.extension.clone(),
                extension_point: implementation.extension,
                implementation: implementation.id,
                description: String::new(),
                provider_pack_id: manifest.id.clone(),
                provider: "External".into(),
                effective: enabled,
                enabled,
                configuration: ExtensionConfiguration::CapabilityPack,
            });
        }
    }
    Ok(items)
}

#[cfg(test)]
mod extension_catalog_tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn pack_and_extension_switches_compose_without_disabling_other_providers() {
        let packs = BTreeMap::from([("framework.local.scoped".into(), false)]);
        let items = extension_implementation_catalog_with_state(&packs, &BTreeMap::new());
        let tools: Vec<_> = items.iter().filter(|item| item.extension_point == "tool").collect();
        assert_eq!(tools.iter().filter(|item| item.effective).count(), 6);
        let local = tools.iter().find(|item| item.provider_pack_id == "framework.local.scoped").unwrap();
        assert!(local.enabled);
        assert!(!local.effective);

        let items = extension_implementation_catalog_with_state(&packs, &BTreeMap::from([("tool".into(), false)]));
        assert!(items.iter().filter(|item| item.extension_point == "tool").all(|item| !item.effective));
        assert!(items.iter().filter(|item| item.configuration == ExtensionConfiguration::Required).all(|item| item.effective));
        assert!(!items.iter().find(|item| item.implementation == "TronContext + bounded context preparation").unwrap().effective);
    }
}
