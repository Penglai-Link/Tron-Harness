use serde::Serialize;

#[path = "capability_contents.rs"]
mod contents;
use contents::{CapabilityContentInfo, builtin_contents, installed_contents};

#[path = "capability_readme.rs"]
mod readme;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityPackInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub cua_driver_sdk_version: Option<String>,
    pub description: String,
    pub readme: Option<String>,
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
    query_installed_capability_packs(root, locale, true, None)
}

type InstalledManifest = (
    semver::Version,
    std::path::PathBuf,
    eliteforge_agent_capability_runtime::CapabilityPackManifest,
);

/// Read installed manifests without re-validating every payload file.
///
/// `CapabilityPackManager::installed` verifies checksums and signatures for the
/// whole package tree on each call, which costs seconds for binary packs. Listing
/// only needs the manifest; runtime selection and installation still verify
/// through `CapabilityPackManager`.
fn latest_installed_manifests(root: &std::path::Path) -> Result<Vec<InstalledManifest>, String> {
    let root = root.join("packs");
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut latest = std::collections::BTreeMap::<String, InstalledManifest>::new();
    for package in std::fs::read_dir(&root).map_err(|error| error.to_string())? {
        let package = package.map_err(|error| error.to_string())?;
        if !package
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            continue;
        }
        let versions = package.path().join("versions");
        if !versions.exists() {
            continue;
        }
        for version in std::fs::read_dir(&versions).map_err(|error| error.to_string())? {
            let path = version.map_err(|error| error.to_string())?.path();
            if path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with('.'))
            {
                continue;
            }
            let bytes =
                std::fs::read(path.join("manifest.json")).map_err(|error| error.to_string())?;
            let manifest: eliteforge_agent_capability_runtime::CapabilityPackManifest =
                serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
            manifest.validate().map_err(|error| error.to_string())?;
            let replace = latest
                .get(&manifest.id)
                .is_none_or(|(version, _, _)| manifest.version > *version);
            if replace {
                latest.insert(
                    manifest.id.clone(),
                    (manifest.version.clone(), path, manifest),
                );
            }
        }
    }
    Ok(latest.into_values().collect())
}

fn installed_upstream_sdk_version(
    path: &std::path::Path,
    manifest: &eliteforge_agent_capability_runtime::CapabilityPackManifest,
) -> Result<Option<String>, String> {
    if !crate::domain::computer_use::is_pack(&manifest.id)
        || !manifest
            .resources
            .iter()
            .any(|path| path == "upstream.json")
    {
        return Ok(None);
    }
    let bytes = std::fs::read(path.join("upstream.json")).map_err(|error| error.to_string())?;
    let upstream: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("Invalid computer-use upstream.json: {error}"))?;
    Ok(upstream
        .get("sdkVersion")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|version| !version.is_empty())
        .map(str::to_owned))
}

pub fn query_installed_capability_packs(
    root: &std::path::Path,
    locale: &str,
    include_contents: bool,
    id: Option<&str>,
) -> Result<Vec<CapabilityPackInfo>, String> {
    let mut packs = capability_pack_entries(include_contents, id, locale);
    // Built-in pack labels are stable resource keys resolved with the request
    // locale; installed pack metadata belongs to its provider.
    for pack in packs.iter_mut() {
        if let Some((name_key, description_key)) = builtin_pack_text_keys(&pack.id) {
            if !pack.name.is_empty() {
                pack.name = tron_i18n::translate(locale, name_key).into_owned();
            }
            if !pack.description.is_empty() {
                pack.description = tron_i18n::translate(locale, description_key).into_owned();
            }
        }
    }
    for pack in packs.iter_mut().filter(|_| include_contents) {
        for item in pack
            .contents
            .iter_mut()
            .filter(|item| item.kind == "Plugin")
        {
            if let Some(name) = contents::plugin_display_name(&item.name, locale) {
                item.display_name = name.into();
            }
        }
        let descriptors = contents::builtin_tool_descriptors(&pack.id);
        for item in pack.contents.iter_mut().filter(|item| item.kind == "Tool") {
            if let Some(descriptor) = descriptors.iter().find(|d| d.call_name == item.name) {
                let display = descriptor.resolve_display(locale);
                item.description = display.description;
                item.display_name = display.name;
            }
        }
    }
    // Imported metadata belongs to its provider; never translate by a colliding
    // built-in call name or activate a plugin merely to display its manifest.
    for (_, path, manifest) in latest_installed_manifests(root)?
        .into_iter()
        .filter(|(_, _, manifest)| id.is_none_or(|id| manifest.id == id))
    {
        let mut contents = if include_contents {
            installed_contents(&path, &manifest, locale)?
        } else {
            vec![]
        };
        for item in contents.iter_mut().filter(|item| item.kind == "Tool") {
            if let Some(entry) = manifest
                .tools
                .iter()
                .find(|entry| entry.descriptor.call_name == item.name)
            {
                let display = entry.descriptor.resolve_display(locale);
                item.display_name = display.name;
                item.description = display.description;
            }
        }
        packs.push(CapabilityPackInfo {
            cua_driver_sdk_version: installed_upstream_sdk_version(&path, &manifest)?,
            readme: if include_contents && id.is_some() {
                readme::installed_readme(&path)?
            } else {
                None
            },
            contents,
            id: manifest.id,
            name: manifest.name,
            version: manifest.version.to_string(),
            description: tron_i18n::translate(
                locale,
                "agent_runtime.capability.imported_package_description",
            )
            .into_owned(),
            provider: tron_i18n::translate(locale, "agent_runtime.capability.external_provider")
                .into_owned(),
            updated_at: None,
            effective: false,
            kind: "capability".into(),
        });
    }
    Ok(packs)
}

/// Stable resource keys for a built-in capability pack label.
fn builtin_pack_text_keys(id: &str) -> Option<(&'static str, &'static str)> {
    Some(match id {
        "framework.external-agent" => (
            "agent_runtime.capability.pack.external_agent.name",
            "agent_runtime.capability.pack.external_agent.description",
        ),
        "framework.local.scoped" => (
            "agent_runtime.capability.pack.scoped_local.name",
            "agent_runtime.capability.pack.scoped_local.description",
        ),
        eliteforge_agent_capability_tools::AGENT_COLLABORATION_PACK => (
            "agent_runtime.capability.pack.agent_collaboration.name",
            "agent_runtime.capability.pack.agent_collaboration.description",
        ),
        eliteforge_agent_official_plugin_a2ui::A2UI_PACK_ID => (
            "agent_runtime.capability.pack.a2ui.name",
            "agent_runtime.capability.pack.a2ui.description",
        ),
        "framework.personai" => (
            "agent_runtime.capability.pack.personai.name",
            "agent_runtime.capability.pack.personai.description",
        ),
        "framework.web" => (
            "agent_runtime.capability.pack.web.name",
            "agent_runtime.capability.pack.web.description",
        ),
        "framework.media" => (
            "agent_runtime.capability.pack.media.name",
            "agent_runtime.capability.pack.media.description",
        ),
        eliteforge_agent_capability_development::PACK_ID => (
            "agent_runtime.capability.pack.development.name",
            "agent_runtime.capability.pack.development.description",
        ),
        eliteforge_agent_capability_remote_access::PACK_ID => (
            "agent_runtime.capability.pack.remote_access.name",
            "agent_runtime.capability.pack.remote_access.description",
        ),
        "tron.harness" => (
            "agent_runtime.capability.pack.tron_harness.name",
            "agent_runtime.capability.pack.tron_harness.description",
        ),
        _ => return None,
    })
}

pub fn install_capability_pack(
    root: &std::path::Path,
    bytes: &[u8],
    locale: &str,
) -> Result<CapabilityPackInfo, String> {
    use eliteforge_agent_capability_runtime::{CapabilityPackManager, InstallPolicy};
    let manager = CapabilityPackManager::new(root).map_err(|error| error.to_string())?;
    let installed = manager
        .install_bytes(bytes, InstallPolicy::Development)
        .map_err(|error| error.to_string())?;
    let contents = installed_contents(&installed.path, &installed.manifest, locale)?;
    Ok(CapabilityPackInfo {
        cua_driver_sdk_version: installed_upstream_sdk_version(
            &installed.path,
            &installed.manifest,
        )?,
        readme: None,
        contents,
        id: installed.manifest.id,
        name: installed.manifest.name,
        version: installed.manifest.version.to_string(),
        description: tron_i18n::translate(
            locale,
            "agent_runtime.capability.imported_package_description",
        )
        .into_owned(),
        provider: tron_i18n::translate(locale, "agent_runtime.capability.external_provider")
            .into_owned(),
        updated_at: None,
        effective: false,
        kind: "capability".into(),
    })
}

pub fn capability_pack_catalog() -> Vec<CapabilityPackInfo> {
    capability_pack_entries(true, None, tron_i18n::DEFAULT_LOCALE.as_str())
}
fn capability_pack_entries(
    include_contents: bool,
    id: Option<&str>,
    locale: &str,
) -> Vec<CapabilityPackInfo> {
    [
        (
            "framework.external-agent",
            "External Agent",
            "External Agent Invocation and Session Bridge",
            "EliteForge",
            env!("TRON_FRAMEWORK_VERSION"),
            Some("2026-09-27"),
        ),
        (
            "framework.local.scoped",
            "Scoped Local Tools",
            "Scoped filesystem, command and shared PTY capabilities",
            "EliteForge",
            env!("TRON_FRAMEWORK_VERSION"),
            Some("2026-09-27"),
        ),
        (
            eliteforge_agent_capability_tools::AGENT_COLLABORATION_PACK,
            "Agent Collaboration",
            "Named child-agent delegation and independent threads",
            "EliteForge",
            env!("TRON_FRAMEWORK_VERSION"),
            Some("2026-10-09"),
        ),
        (
            eliteforge_agent_official_plugin_a2ui::A2UI_PACK_ID,
            "A2UI Interfaces",
            "Optional interactive cards, forms, guidance and skills for A2UI v0.9.1 clients",
            "EliteForge",
            env!("TRON_FRAMEWORK_VERSION"),
            Some("2026-10-10"),
        ),
        (
            "framework.personai",
            "PersonAI",
            "Web search, image generation, task querying and AIGC detection with host credentials",
            "EliteForge",
            env!("TRON_FRAMEWORK_VERSION"),
            Some("2026-10-10"),
        ),
        (
            "framework.web",
            "Web Fetch",
            "Read public webpage text",
            "EliteForge",
            env!("TRON_FRAMEWORK_VERSION"),
            Some("2026-10-10"),
        ),
        (
            "framework.media",
            "Media",
            "Read scoped images and PDF files or attachments",
            "EliteForge",
            env!("TRON_FRAMEWORK_VERSION"),
            Some("2026-10-10"),
        ),
        (
            eliteforge_agent_capability_development::PACK_ID,
            "EliteForge Agent Development",
            "Framework and Agent UI development skills, including the Tron capability scaffold generator",
            "EliteForge",
            eliteforge_agent_capability_development::PACK_VERSION,
            Some("2026-11-12"),
        ),
        (
            eliteforge_agent_capability_remote_access::PACK_ID,
            "Remote Access",
            "Mobile pairing, short-lived access tokens and remote listener management for PersonAI products",
            "EliteForge",
            env!("TRON_FRAMEWORK_VERSION"),
            Some("2026-11-14"),
        ),
        (
            "tron.harness",
            "Tron Harness",
            "Tron context, business tool policy and bundled theme / memory skills",
            "Tron",
            env!("CARGO_PKG_VERSION"),
            Some("2026-10-10"),
        ),
    ]
    .into_iter()
    .filter(|entry| id.is_none_or(|id| entry.0 == id))
    .map(
        |(id, name, description, provider, version, updated_at)| CapabilityPackInfo {
            contents: if include_contents {
                builtin_contents(id, locale)
            } else {
                vec![]
            },
            id: id.into(),
            name: name.into(),
            version: version.into(),
            cua_driver_sdk_version: None,
            description: description.into(),
            readme: None,
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

pub(super) fn pack_extension_implementations(locale: &str) -> Vec<ExtensionImplementationInfo> {
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
            "EliteForge Agent Development Skills",
            "Embedded Framework integration, Agent UI integration and UI theme guidance",
            eliteforge_agent_capability_development::PACK_ID,
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
                description: extension_description_key(extension_point, implementation)
                    .map_or_else(
                        || description.to_owned(),
                        |key| tron_i18n::translate(locale, key).into_owned(),
                    ),
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

/// Stable resource key for a built-in extension implementation description.
fn extension_description_key(extension_point: &str, implementation: &str) -> Option<&'static str> {
    Some(match (extension_point, implementation) {
        ("model", "TronChatModel") => "capabilities.extension.model.connection.description",
        ("tool", "ScopedLocalToolsPlugin") => {
            "capabilities.extension.tools.local_scoped.description"
        }
        ("tool", "Tron HarnessTools") => "capabilities.extension.tools.harness.description",
        ("tool-policy", "TronToolPolicy") => "capabilities.extension.tool_policy.description",
        ("tool", "AgentToolsPlugin") => "capabilities.extension.tools.collaboration.description",
        ("tool", "A2uiService") => "capabilities.extension.tools.a2ui.description",
        ("skill", "EliteForge Agent Development Skills") => {
            "capabilities.extension.skill.development.description"
        }
        ("skill", "A2UI Guidance") => "capabilities.extension.skill.a2ui.description",
        ("context", "A2UI Guidance") => "capabilities.extension.context.a2ui.description",
        ("tool", "PersonAiToolProvider") => "capabilities.extension.tools.personai.description",
        ("tool", "WebToolsPlugin") => "capabilities.extension.tools.web.description",
        ("tool", "MediaToolsPlugin") => "capabilities.extension.tools.media.description",
        ("observe", "ToolAudit middleware") => "capabilities.extension.observe.audit.description",
        ("context", "TronContext + bounded context preparation") => {
            "capabilities.extension.context.harness.description"
        }
        ("external-agent", "HostExternalAgent") => {
            "capabilities.extension.external_agent.host.description"
        }
        ("runtime", "Independent ExternalAgentLoop") => {
            "capabilities.extension.external_agent.runtime.description"
        }
        ("storage", "LocalStoragePlugin") => "capabilities.extension.storage.local.description",
        _ => return None,
    })
}

pub fn extension_implementation_catalog(locale: &str) -> Vec<ExtensionImplementationInfo> {
    pack_extension_implementations(locale)
}

pub fn extension_implementation_catalog_with_state(
    packs: &std::collections::BTreeMap<String, bool>,
    enabled: &std::collections::BTreeMap<String, bool>,
    locale: &str,
) -> Vec<ExtensionImplementationInfo> {
    let extension_enabled = |point: &str| enabled.get(point).copied().unwrap_or(true);
    extension_implementation_catalog(locale)
        .into_iter()
        .map(|mut implementation| {
            let pack_enabled = packs
                .get(&implementation.provider_pack_id)
                .copied()
                .unwrap_or(true);
            implementation.enabled = match implementation.configuration {
                ExtensionConfiguration::Required => true,
                ExtensionConfiguration::Extension => {
                    extension_enabled(&implementation.extension_point)
                }
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
    locale: &str,
) -> Result<Vec<ExtensionImplementationInfo>, String> {
    let mut items = extension_implementation_catalog_with_state(packs, enabled, locale);
    for (_, _, manifest) in latest_installed_manifests(root)? {
        for implementation in manifest.implementations {
            let enabled = packs.get(&manifest.id).copied().unwrap_or(false);
            items.push(ExtensionImplementationInfo {
                extension_name: implementation.extension.clone(),
                extension_point: implementation.extension,
                implementation: implementation.id,
                description: String::new(),
                provider_pack_id: manifest.id.clone(),
                provider: tron_i18n::translate(
                    locale,
                    "agent_runtime.capability.external_provider",
                )
                .into_owned(),
                effective: enabled,
                enabled,
                configuration: ExtensionConfiguration::CapabilityPack,
            });
        }
    }
    Ok(items)
}
