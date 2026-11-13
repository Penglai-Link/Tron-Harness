//! Tool settings consume inert metadata, independently of execution runtimes.
use super::*;
use crate::domain::agent::ToolCatalogIssue;
use fw::ToolProvider;
use std::collections::BTreeSet;

type CatalogEntries = Vec<(fw::ToolCatalogEntry, fw::ToolSource)>;

#[derive(Default)]
pub(super) struct ToolCatalogMetadata {
    pub imported: Option<ImportedToolCatalog>,
    pub initialized: bool,
    pub generation: u64,
}

#[derive(Clone, Default)]
pub(super) struct ImportedToolCatalog {
    entries: CatalogEntries,
    pack_ids: BTreeSet<String>,
    issues: Vec<ToolCatalogIssue>,
}

impl ImportedToolCatalog {
    pub fn add(&mut self, manifest: &eliteforge_agent_capability_runtime::CapabilityPackManifest) {
        self.pack_ids.insert(manifest.id.clone());
        let providers = manifest
            .implementations
            .iter()
            .filter(|implementation| implementation.extension == "framework.tool-provider")
            .map(|implementation| implementation.id.clone())
            .collect::<Vec<_>>();
        // A single declared provider has an unambiguous origin. Otherwise keep
        // ownership at package level until runtime discovery resolves it.
        let provider_id = (providers.len() == 1)
            .then(|| providers[0].clone())
            .or_else(|| (manifest.plugins.len() == 1).then(|| manifest.plugins[0].clone()));
        if manifest.tools.is_empty() && !providers.is_empty() {
            self.issues.push(ToolCatalogIssue {
                blocks_tool: false,
                tool_code: None,
                tool_name: None,
                provider_id: provider_id.clone(),
                pack_id: Some(manifest.id.clone()),
                message: "Tool provider has no static catalog. Refresh to discover its tools."
                    .into(),
            });
        }
        let source = fw::ToolSource {
            provider_id: provider_id.clone(),
            plugin_id: provider_id,
            pack_id: Some(manifest.id.clone()),
            adapter_id: None,
        };
        self.entries.extend(
            manifest
                .tools
                .iter()
                .cloned()
                .map(|entry| (entry, source.clone())),
        );
    }
}

async fn builtin_metadata() -> Result<&'static CatalogEntries, String> {
    static CATALOG: OnceCell<CatalogEntries> = OnceCell::const_new();
    CATALOG
        .get_or_try_init(|| async {
            // These two plugins expose their canonical catalogs through services.
            // Neither starts processes, opens databases, or discovers skill files.
            let mut plugins = eliteforge_agent_plugin_core::PluginContainer::build(vec![
                Arc::new(eliteforge_agent_official_plugin_tools_agents::AgentToolsPlugin),
                Arc::new(eliteforge_agent_official_plugin_skills::SkillsPlugin::sources_only()),
            ])
            .await
            .map_err(|error| error.to_string())?;
            let services = plugins.services();
            let mut providers = services
                .providers::<fw::ToolProviderService>()
                .into_iter()
                .zip(services.all::<fw::ToolProviderService>())
                .filter_map(|(id, provider)| {
                    provider
                        .0
                        .catalog_metadata()
                        .map(|entries| (id.to_owned(), entries))
                })
                .collect::<Vec<_>>();
            plugins.close();
            providers.extend([
                (
                    "tron.harness.tools".into(),
                    HarnessTools.catalog_metadata().unwrap_or_default(),
                ),
                (
                    "framework.local.scoped.tools".into(),
                    eliteforge_agent_official_plugin_tools_local::local_tool_catalog_metadata(),
                ),
                (
                    "framework.tools.discovery".into(),
                    eliteforge_agent_official_plugin_tools_discovery::ToolDiscoveryPlugin::new()
                        .catalog_metadata()
                        .unwrap_or_default(),
                ),
                (
                    "agent.a2ui".into(),
                    eliteforge_agent_official_plugin_a2ui::A2uiService::new()
                        .map_err(|error| error.to_string())?
                        .catalog_metadata()
                        .unwrap_or_default(),
                ),
                (
                    "tron.connector.tools".into(),
                    connector_tools::HostConnectorTools(std::sync::Weak::new())
                        .catalog_metadata()
                        .unwrap_or_default(),
                ),
            ]);
            let packs = super::super::capability_catalog::capability_pack_catalog();
            let mut entries = Vec::new();
            for (plugin_id, tools) in providers {
                let source = fw::ToolSource {
                    provider_id: Some(if plugin_id == "tron.harness.tools" {
                        crate::application::agent::tool_catalog::HARNESS_TOOL_PROVIDER_ID.into()
                    } else {
                        plugin_id.clone()
                    }),
                    pack_id: packs
                        .iter()
                        .find(|pack| {
                            pack.contents
                                .iter()
                                .any(|item| item.kind == "Plugin" && item.name == plugin_id)
                        })
                        .map(|pack| pack.id.clone()),
                    plugin_id: Some(plugin_id),
                    adapter_id: None,
                };
                entries.extend(tools.into_iter().map(|entry| (entry, source.clone())));
            }
            Ok(entries)
        })
        .await
}

impl FrameworkHost {
    pub(super) fn invalidate_tool_metadata(&self) {
        let mut metadata = self.0.tool_catalog_metadata.write().unwrap();
        metadata.imported = None;
        metadata.initialized = false;
        metadata.generation = metadata.generation.wrapping_add(1);
    }

    pub(super) async fn ensure_tool_metadata(&self) -> Result<(), String> {
        let (imported, generation) = {
            let metadata = self.0.tool_catalog_metadata.read().unwrap();
            if metadata.initialized {
                return Ok(());
            }
            (
                metadata
                    .imported
                    .clone()
                    .ok_or("Capability metadata is unavailable. Refresh the tool catalog.")?,
                metadata.generation,
            )
        };
        let mut entries = builtin_metadata().await?.clone();
        let mut issues = imported.issues;
        let bundled =
            crate::application::agent::BusinessToolCatalog::list_tools(None, None, "en-US");
        let mut ids = entries
            .iter()
            .map(|(entry, _)| entry.descriptor.id.clone())
            .collect::<BTreeSet<_>>();
        let mut names = entries
            .iter()
            .map(|(entry, _)| entry.descriptor.call_name.clone())
            .collect::<BTreeSet<_>>();
        for (entry, source) in imported.entries {
            if bundled.iter().any(|tool| {
                tool.tool_code == entry.descriptor.id
                    || tool.tool_name == entry.descriptor.call_name
            }) || ids.contains(&entry.descriptor.id)
                || names.contains(&entry.descriptor.call_name)
            {
                issues.push(ToolCatalogIssue {
                    blocks_tool: false,
                    tool_code: Some(entry.descriptor.id.clone()),
                    tool_name: Some(entry.descriptor.call_name.clone()),
                    provider_id: source.provider_id,
                    pack_id: source.pack_id,
                    message: format!("Tool {} conflicts with another tool", entry.descriptor.id),
                });
                continue;
            }
            ids.insert(entry.descriptor.id.clone());
            names.insert(entry.descriptor.call_name.clone());
            entries.push((entry, source));
        }
        self.register_tool_catalog_metadata(&entries, issues, &imported.pack_ids, Some(generation))
    }

    pub(super) fn register_tool_catalog_metadata(
        &self,
        entries: &CatalogEntries,
        mut issues: Vec<ToolCatalogIssue>,
        installed: &BTreeSet<String>,
        expected_generation: Option<u64>,
    ) -> Result<(), String> {
        self.ensure_open()?;
        let mut metadata = self.0.tool_catalog_metadata.write().unwrap();
        if let Some(generation) = expected_generation {
            if metadata.generation != generation {
                return Err(
                    "Capability catalog changed while loading metadata. Refresh to retry.".into(),
                );
            }
            if metadata.initialized {
                return Ok(());
            }
        }
        let (Some(resources), Some(management)) = (
            self.0.capability_resources.get(),
            self.0.agent_management.get(),
        ) else {
            return Ok(());
        };
        let (records, registration_issues) = management.register_tool_catalog_entries(entries)?;
        issues.extend(registration_issues);
        for issue in &issues {
            tracing::warn!(tool_code = ?issue.tool_code, provider_id = ?issue.provider_id,
                pack_id = ?issue.pack_id, error = %issue.message, "Tool catalog entry could not be loaded");
        }
        let (state, _) = self.capability_state();
        let owned = records
            .into_iter()
            .filter_map(|record| {
                record.source.pack_id.map(|pack_id| {
                    let available = state
                        .get(&pack_id)
                        .copied()
                        .unwrap_or(!installed.contains(&pack_id));
                    crate::application::capability::CapabilityResource {
                        pack_id,
                        kind: "tool".into(),
                        code: record.tool_code,
                        available,
                    }
                })
            })
            .collect::<Vec<_>>();
        resources.register(&owned)?;
        *self.0.tool_catalog_issues.write().unwrap() = issues;
        metadata.initialized = true;
        Ok(())
    }
}
