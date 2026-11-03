//! One plugin environment per profile; Agent bindings only select capabilities.
use super::*;
use eliteforge_agent_framework::FrameworkBuilder;
use eliteforge_agent_official_plugin_storage_local::LocalStoragePlugin;
impl FrameworkHost {
    pub fn sync_capability_resources(&self) -> Result<(), String> {
        use crate::application::capability::CapabilityResource;
        let Some(skills) = self.0.skill_registry.get() else {
            return Ok(());
        };
        let (state, _) = self.capability_state();
        let mut sources: Vec<(String, Vec<fw::CapabilitySkillSource>)> = Vec::new();
        let harness =
            tron_harness_capability::TronHarness::new(Arc::new(HarnessTools), []).capability_pack();
        sources.push((
            "tron.harness".into(),
            fw::CapabilityPack::registry_skill_sources(&harness),
        ));
        let a2ui =
            eliteforge_agent_official_plugin_a2ui::A2uiPack::new().map_err(|e| e.to_string())?;
        sources.push((
            "framework.a2ui".into(),
            fw::CapabilityPack::registry_skill_sources(&a2ui),
        ));
        let manager =
            eliteforge_agent_capability_runtime::CapabilityPackManager::new(&self.0.pack_root)
                .map_err(|e| e.to_string())?;
        let installed = manager.installed().map_err(|e| e.to_string())?;
        let mut existing_mcp = self
            .0
            .mcp
            .get()
            .map(|mcp| {
                mcp.list_platform_servers(crate::domain::mcp::PlatformMcpServerListQuery {
                    search: None,
                    status: None,
                })
            })
            .transpose()
            .map_err(|e| e.to_string())?
            .unwrap_or_default()
            .into_iter()
            .map(|server| server.server_code)
            .collect::<std::collections::BTreeSet<_>>();
        let mut mcp_ownership = self
            .0
            .capability_resources
            .get()
            .map(|resources| resources.list())
            .transpose()?
            .unwrap_or_default()
            .into_iter()
            .filter(|resource| resource.kind == "mcp")
            .map(|resource| (resource.code.clone(), resource))
            .collect::<BTreeMap<_, _>>();
        let mut mcp_resources = Vec::new();
        let mut mcp_declarations = Vec::new();
        let mut selected = std::collections::BTreeSet::new();
        for pack in installed.iter().rev() {
            if !selected.insert(pack.manifest.id.clone()) {
                continue;
            }
            let managed = manager
                .select(
                    &pack.manifest.id,
                    &semver::VersionReq::STAR,
                    "tron",
                    "default",
                    serde_json::Value::Null,
                )
                .map_err(|e| e.to_string())?;
            sources.push((
                pack.manifest.id.clone(),
                fw::CapabilityPack::registry_skill_sources(&managed),
            ));
            if let (Some(_mcp), Some(_resources)) =
                (self.0.mcp.get(), self.0.capability_resources.get())
            {
                let active = state.get(&pack.manifest.id).copied().unwrap_or(false);
                for declaration in &pack.manifest.mcp {
                    if existing_mcp.contains(&declaration.id)
                        && !mcp_ownership.contains_key(&declaration.id)
                    {
                        return Err(format!(
                            "MCP {} conflicts with an independent server",
                            declaration.id
                        ));
                    }
                    let mut declaration = declaration.clone();
                    if let eliteforge_agent_capability_runtime::McpDeclarationTransport::Stdio {
                        program,
                        ..
                    } = &mut declaration.transport
                    {
                        *program = pack.path.join(&*program).display().to_string();
                    }
                    mcp_declarations.push((declaration.clone(), active, pack.path.clone()));
                    existing_mcp.insert(declaration.id.clone());
                    let resource = CapabilityResource {
                        pack_id: pack.manifest.id.clone(),
                        kind: "mcp".into(),
                        code: declaration.id.clone(),
                        available: active,
                    };
                    if mcp_ownership
                        .get(&declaration.id)
                        .is_some_and(|owner| owner.pack_id != resource.pack_id)
                    {
                        return Err(format!(
                            "MCP {} conflicts with another capability pack",
                            declaration.id
                        ));
                    }
                    mcp_ownership.insert(declaration.id.clone(), resource.clone());
                    mcp_resources.push(resource);
                }
            }
        }
        if let Some(mcp) = self.0.mcp.get() {
            mcp.register_pack_contributions(&mcp_declarations)
                .map_err(|e| e.to_string())?;
        }
        if let Some(resources) = self.0.capability_resources.get() {
            resources.register(&mcp_resources)?;
        }
        for (id, items) in sources {
            let active = state.get(&id).copied().unwrap_or(Self::builtin_pack(&id));
            let mut directories = Vec::new();
            for source in items {
                let directory = match source {
                    fw::CapabilitySkillSource::Directory(path) => path,
                    fw::CapabilitySkillSource::Embedded(embedded) => {
                        let safe = id
                            .as_bytes()
                            .iter()
                            .map(|b| format!("{b:02x}"))
                            .collect::<String>();
                        let root = self
                            .0
                            .path
                            .with_extension("skills-catalog")
                            .join(safe)
                            .join(embedded.content_hash);
                        for file in embedded.files {
                            let relative = Path::new(file.path);
                            if relative.is_absolute()
                                || relative
                                    .components()
                                    .any(|c| !matches!(c, std::path::Component::Normal(_)))
                            {
                                return Err("invalid embedded skill path".into());
                            }
                            let path = root.join(relative);
                            fs::create_dir_all(path.parent().unwrap())
                                .map_err(|e| e.to_string())?;
                            fs::write(&path, file.contents).map_err(|e| e.to_string())?;
                        }
                        root
                    }
                };
                directories.push(directory);
            }
            skills
                .register_pack_directories(&id, &directories, active)
                .map_err(|e| e.to_string())?;
        }
        skills.rescan_registry().map_err(|e| e.to_string())?;
        if let Some(resources) = self.0.capability_resources.get() {
            let tools = self
                .0
                .agent_management
                .get()
                .map(|management| management.list_tools(None, None, "en-US"))
                .unwrap_or_else(|| {
                    crate::application::agent::BusinessToolCatalog::list_tools(None, None, "en-US")
                });
            let mut owned = Vec::new();
            for pack in super::super::capability_catalog::capability_pack_catalog() {
                let available = state.get(&pack.id).copied().unwrap_or(true);
                for item in pack.contents.iter().filter(|c| c.kind == "Tool") {
                    if let Some(tool) = tools.iter().find(|t| t.tool_name == item.name) {
                        owned.push(CapabilityResource {
                            pack_id: pack.id.clone(),
                            kind: "tool".into(),
                            code: tool.tool_code.clone(),
                            available,
                        });
                    }
                }
            }
            for skill in skills
                .list_registry(crate::domain::skill::RegistrySkillListQuery { search: None })
                .map_err(|e| e.to_string())?
            {
                if let Some(pack_id) = skill.provider_pack_id {
                    owned.push(CapabilityResource {
                        pack_id,
                        kind: "skill".into(),
                        code: skill.skill_name,
                        available: skill.available,
                    });
                }
            }
            resources.register(&owned)?;
        }
        Ok(())
    }
    fn builtin_tool_catalog(
        &self,
        framework: &Framework,
    ) -> Result<Vec<(fw::ToolCatalogEntry, fw::ToolSource)>, String> {
        let services = framework.services().map_err(|e| e.to_string())?;
        let installed = self.0.installed_packs.read().unwrap().clone();
        let packs = super::super::capability_catalog::capability_pack_catalog();
        let mut catalog_entries = Vec::new();
        for (plugin_id, provider) in services
            .providers::<fw::ToolProviderService>()
            .into_iter()
            .zip(services.all::<fw::ToolProviderService>())
        {
            if installed
                .values()
                .any(|ids| ids.iter().any(|id| id == plugin_id))
            {
                continue;
            }
            let Some(entries) = provider.0.catalog_metadata() else {
                continue;
            };
            let pack = packs.iter().find(|pack| {
                pack.contents
                    .iter()
                    .any(|item| item.kind == "Plugin" && item.name == plugin_id)
            });
            let source = fw::ToolSource {
                provider_id: Some(if plugin_id == "tron.harness.tools" {
                    crate::application::agent::tool_catalog::HARNESS_TOOL_PROVIDER_ID.to_owned()
                } else {
                    plugin_id.to_owned()
                }),
                plugin_id: Some(plugin_id.to_owned()),
                pack_id: pack.map(|pack| pack.id.clone()),
                adapter_id: None,
            };
            catalog_entries.extend(entries.into_iter().map(|entry| (entry, source.clone())));
        }
        Ok(catalog_entries)
    }

    async fn discover_imported_tools(
        &self,
        framework: &Framework,
        mut catalog_entries: Vec<(fw::ToolCatalogEntry, fw::ToolSource)>,
    ) -> Result<(), String> {
        let (Some(resources), Some(management)) = (
            self.0.capability_resources.get(),
            self.0.agent_management.get(),
        ) else {
            return Ok(());
        };
        let services = framework.services().map_err(|e| e.to_string())?;
        let installed = self.0.installed_packs.read().unwrap().clone();
        let (state, _) = self.capability_state();
        let options = fw::ModelInvocationOptions::for_call(&fw::RunLimits::default())
            .map_err(|e| e.to_string())?;
        let run = options.run.as_ref().ok_or("catalog context unavailable")?;
        let bundled =
            crate::application::agent::BusinessToolCatalog::list_tools(None, None, "en-US");
        let mut ids = catalog_entries
            .iter()
            .map(|(entry, _)| entry.descriptor.id.clone())
            .collect::<std::collections::HashSet<_>>();
        let mut names = catalog_entries
            .iter()
            .map(|(entry, _)| entry.descriptor.call_name.clone())
            .collect::<std::collections::HashSet<_>>();
        for (provider_id, provider) in services
            .providers::<fw::ToolProviderService>()
            .into_iter()
            .zip(services.all::<fw::ToolProviderService>())
        {
            let Some((pack, _)) = installed
                .iter()
                .find(|(_, ids)| ids.iter().any(|id| id == provider_id))
            else {
                continue;
            };
            // Prefer inert metadata: listing settings must not require the
            // credentials, filesystem grants or resources of an Agent run.
            let entries = if let Some(entries) = provider.0.catalog_metadata() {
                entries.into_iter().map(|entry| (entry, fw::ToolSource::default())).collect()
            } else {
                let mut lease = provider.0.lease(run).await.map_err(|e| e.to_string())?;
                let entries = lease.tools.iter()
                    .map(|tool| (tool.catalog_entry(), tool.source.clone()))
                    .collect::<Vec<_>>();
                let report = lease.resources.close_async(Duration::from_secs(5)).await;
                if !report.is_success() {
                    return Err(format!("catalog lease cleanup failed: {report:?}"));
                }
                entries
            };
            for (entry, mut source) in entries {
                if bundled.iter().any(|t| {
                    t.tool_code == entry.descriptor.id || t.tool_name == entry.descriptor.call_name
                }) || !ids.insert(entry.descriptor.id.clone())
                    || !names.insert(entry.descriptor.call_name.clone())
                {
                    return Err(format!("tool {} conflicts with a bundled tool", entry.descriptor.id));
                }
                source.plugin_id.get_or_insert_with(|| provider_id.to_owned());
                source.pack_id.get_or_insert_with(|| pack.clone());
                source.provider_id.get_or_insert_with(|| provider_id.to_owned());
                catalog_entries.push((entry, source));
            }
        }
        let records = management.register_tool_catalog_entries(&catalog_entries)?;
        let owned = records
            .into_iter()
            .filter_map(|record| {
                record.source.pack_id.map(|pack_id| {
                    let available = state
                        .get(&pack_id)
                        .copied()
                        .unwrap_or(!installed.contains_key(&pack_id));
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
        Ok(())
    }
    pub async fn ensure_capability_catalog(&self) -> Result<(), String> {
        let _guard = self.0.config_lock.lock().await;
        self.ensure_open()?;
        self.framework()
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    pub(super) async fn framework_builder(
        &self,
        _packs: &BTreeMap<String, bool>,
        _extensions: &BTreeMap<String, bool>,
    ) -> Result<FrameworkBuilder, String> {
        let mut builder = Framework::builder()
            .plugin(
                self.telemetry()
                    .await
                    .map_err(|_| "telemetry initialization failed")?
                    .plugin()?,
            )
            .capability_skills_root(self.0.path.with_extension("skills"))
            .plugin(
                eliteforge_agent_official_plugin_agui::AgUiPlugin::new(
                    eliteforge_agent_official_plugin_agui::AgUiOptions {
                        include_error_details: true,
                        ..Default::default()
                    },
                )
                .map_err(|error| error.to_string())?,
            )
            .plugin({
                let mut storage =
                    LocalStoragePlugin::new(&self.0.path, self.0.path.with_extension("artifacts"))
                        .with_database(self.shared_database().await.map_err(|e| e.to_string())?);
                // Match the assembler's source-size safety guard. The owning
                // memory block's token quota controls summary size, not the
                // storage plugin's smaller general-purpose default record cap.
                storage.config.summaries.max_record_bytes = 16 * 1024 * 1024;
                // Persistent product conversations are not an in-memory cache:
                // normal saved history must not block every new chat/child at 1024.
                storage.config.conversations.max_sessions = usize::MAX;
                storage
            })
            // Stable two-tool gateway: every other provider's bindings become
            // wrapped inventory so `request.tools` stays the same pair for the
            // whole run and provider prompt caches survive tool growth.
            .plugin(eliteforge_agent_official_plugin_tools_discovery::ToolDiscoveryPlugin::new())
            .plugin(fw::AgentLoopPlugin::named(
                "framework.react",
                eliteforge_agent_framework::ReactAgentLoop,
            ))
            .plugin(fw::AgentLoopPlugin::named(
                "tron.continuation-loop",
                eliteforge_agent_framework::ContinuationLoop,
            ))
            .capability_pack(
                eliteforge_agent_capability_external::external_agent_pack(
                    super::super::external::HostExternalAgent,
                ),
            )
            .plugin(eliteforge_agent_framework::GoalPlugin)
            .plugin(fw::AgentLoopPlugin::named(
                "tron.summary-loop",
                super::super::summary_loop::SummaryLoop,
            ))
            .plugin(fw::ContextAssemblerPlugin::named(
                "tron.driver.context",
                fw::DefaultContextAssembler::default(),
            ))
            .plugin(eliteforge_agent_framework::ModelRuntimePlugin::new(
                eliteforge_agent_framework::ModelRuntime::new(
                    eliteforge_agent_framework::ModelRuntimeUpdate::new("", 0, vec![]),
                )
                .map_err(|error| error.to_string())?,
            ))
            .plugin(fw::ToolProviderPlugin::new(
                "tron.onboarding-ui",
                super::super::onboarding::FormTools,
            ))
            .plugin(fw::ToolProviderPlugin::new(
                "tron.remote.tools",
                remote_tools::RemoteTools(Arc::downgrade(&self.0)),
            ))
            .plugin(fw::ToolPolicyPlugin::new(
                "tron.harness.tool-authority",
                super::super::tools::ToolCatalogPolicy::shared(
                    self.0.tools_enabled.clone(),
                    self.0.agent_management.get().cloned(),
                    self.0.skill_registry.get().cloned(),
                ),
            ))
            .plugin(fw::ToolMiddlewarePlugin::new(
                "tron.harness.audit",
                OptionalToolAudit(self.0.observe_enabled.clone()),
            ))
            .capability_pack(eliteforge_agent_capability_tools::scoped_local_tools_pack(
                Arc::new(LocalScope),
                self.pty(),
            ))
            .capability_pack(eliteforge_agent_capability_tools::agent_collaboration_pack())
            .capability_pack(
                tron_harness_capability::TronHarness::new(Arc::new(HarnessTools), [])
                    .with_tool_policy(tron_harness_capability::TronToolPolicy(Arc::new(
                        super::super::tools::PlatformPermissionSource(
                            self.0.agent_management.get().cloned(),
                        ),
                    )))
                    .with_context_preparation(TronContext, 32)
                    .capability_pack(),
            );
        {
            builder = builder.capability_pack(
                eliteforge_agent_official_plugin_a2ui::A2uiPack::new()
                    .map_err(|e| e.to_string())?,
            );
        }
        if let Some(skills) = self.0.skill_registry.get() {
            builder = builder.plugin(eliteforge_agent_official_plugin_skills::SkillsPlugin::new(
                skills.provider(),
            ));
        }
        let manager =
            eliteforge_agent_capability_runtime::CapabilityPackManager::new(&self.0.pack_root)
                .map_err(|error| error.to_string())?;
        let mut installed = BTreeMap::new();
        let packages = manager
            .installed()
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|p| p.manifest.id)
            .collect::<std::collections::BTreeSet<_>>();
        for package in &packages {
            if Self::builtin_pack(package) {
                continue;
            }
            let managed = manager
                .select(
                    package,
                    &semver::VersionReq::STAR,
                    "tron",
                    "default",
                    serde_json::Value::Null,
                )
                .map_err(|error| error.to_string())?;
            let managed = managed
                .contributions(
                    eliteforge_agent_capability_runtime::CapabilityContributionSelection {
                        mcp: Some(vec![]),
                        ..Default::default()
                    },
                )
                .map_err(|e| e.to_string())?;
            installed.insert(
                package.clone(),
                fw::CapabilityPack::plugins(&managed)
                    .iter()
                    .map(|p| p.id().to_owned())
                    .collect(),
            );
            builder = builder.capability_pack(managed);
        }
        *self.0.installed_packs.write().unwrap() = installed;
        Ok(builder)
    }
    pub(super) fn builtin_pack(id: &str) -> bool {
        matches!(
            id,
            "framework.external-agent"
                | "framework.a2ui"
                | "framework.local.scoped"
                | "framework.tool-pack.agent-collaboration"
                | "framework.personai"
                | "framework.web"
                | "framework.media"
                | "framework.tools.discovery"
                | "tron.harness"
        )
    }
    pub(super) fn main_selection(
        &self,
        framework: &Framework,
        packs: &BTreeMap<String, bool>,
        extensions: &BTreeMap<String, bool>,
    ) -> Result<AgentSelection, String> {
        let registry = framework.services().map_err(|e| e.to_string())?;
        let enabled = |point: &str| extensions.get(point).copied().unwrap_or(true);
        let pack_enabled = |id: &str| {
            packs
                .get(id)
                .copied()
                .unwrap_or_else(|| Self::builtin_pack(id))
        };
        if pack_enabled(eliteforge_agent_official_plugin_a2ui::A2UI_PACK_ID)
            && registry
                .get::<eliteforge_agent_official_plugin_a2ui::A2uiService>()
                .is_none()
        {
            return Err(
                "A2UI is not installed in this Framework; restart the profile to enable it".into(),
            );
        }
        let installed = self.0.installed_packs.read().unwrap();
        for (id, active) in packs {
            if *active && !Self::builtin_pack(id) && !installed.contains_key(id) {
                return Err(format!(
                    "capability package {id} is not installed in this Framework; restart the profile to install it"
                ));
            }
        }
        let selected = |id: &str| {
            !installed
                .iter()
                .any(|(pack, providers)| !pack_enabled(pack) && providers.iter().any(|p| p == id))
        };
        let one = |ids: Vec<&str>| -> Result<Option<String>, String> {
            let ids: Vec<_> = ids.into_iter().filter(|id| selected(id)).collect();
            match ids.as_slice() {
                [] => Ok(None),
                [id] => Ok(Some((*id).into())),
                _ => Err(format!(
                    "main Agent requires an explicit capability choice: {ids:?}"
                )),
            }
        };
        Ok(AgentSelection {
            agent_loop: Some("framework.react".into()),
            external_agent: Some("framework.external-agent".into()),
            model_strategy: one(registry.providers::<fw::ModelStrategyService>())?,
            context_assembler: Some(
                if pack_enabled("tron.harness") && enabled("context") && enabled("tool") {
                    "framework.context-assembler"
                } else {
                    "tron.driver.context"
                }
                .into(),
            ),
            tool_providers: Some(
                registry
                    .providers::<fw::ToolProviderService>()
                    .into_iter()
                    .filter(|id| {
                        // Gateway infrastructure follows the tool extension,
                        // independently of capability-pack switches.
                        if *id == "framework.tools.discovery" {
                            return Self::gateway_enabled_for(extensions);
                        }
                        selected(id)
                            && *id != "tron.onboarding-ui"
                            && enabled("tool")
                            && (*id != "agent.a2ui"
                                || pack_enabled(
                                    eliteforge_agent_official_plugin_a2ui::A2UI_PACK_ID,
                                ))
                            && (*id != "tron.harness.tools" || pack_enabled("tron.harness"))
                            && (*id != "tools.agents"
                                || pack_enabled(
                                    eliteforge_agent_capability_tools::AGENT_COLLABORATION_PACK,
                                ))
                            && (*id != "framework.local.scoped.tools"
                                || pack_enabled("framework.local.scoped"))
                    })
                    .map(str::to_owned)
                    .collect(),
            ),
            skills_from_host: self.0.skill_registry.get().is_some(),
            skills: if self.0.skill_registry.get().is_some() {
                None
            } else {
                Some(
                    self.0
                        .skills
                        .read()
                        .unwrap()
                        .iter()
                        .filter(|(pack, _)| pack_enabled(pack))
                        .flat_map(|(_, names)| names.clone())
                        .chain(
                            pack_enabled(eliteforge_agent_official_plugin_a2ui::A2UI_PACK_ID).then(
                                || {
                                    eliteforge_agent_official_plugin_a2ui::A2UI_SKILL_NAME
                                        .to_owned()
                                },
                            ),
                        )
                        .collect::<std::collections::BTreeSet<_>>()
                        .into_iter()
                        .collect(),
                )
            },
            context_sources: Some(
                registry
                    .providers::<fw::ContextContributorService>()
                    .into_iter()
                    .filter({
                        let mut seen = std::collections::HashSet::new();
                        move |id| seen.insert(*id)
                    })
                    .filter(|id| {
                        selected(id)
                            && (!matches!(*id, "agent.a2ui" | "skills.default")
                                || Self::gateway_enabled_for(extensions))
                            && (*id != "agent.a2ui"
                                || pack_enabled(
                                    eliteforge_agent_official_plugin_a2ui::A2UI_PACK_ID,
                                ))
                    })
                    .map(str::to_owned)
                    .collect(),
            ),
            ..AgentSelection::default()
        })
    }
    pub(super) async fn framework(&self) -> Result<Framework, fw::AgentError> {
        self.0
            .framework
            .get_or_try_init(|| self.initialize_framework())
            .await
    }

    pub(crate) async fn context_layout_catalog(
        &self,
    ) -> Result<Vec<super::super::ContextLayoutContributorInfo>, fw::AgentError> {
        Ok(self
            .context_block_catalog()
            .await?
            .into_iter()
            .filter(|source| source.kind == fw::ContextSourceKind::Data)
            .collect())
    }

    pub(crate) async fn context_block_catalog(
        &self,
    ) -> Result<Vec<super::super::ContextLayoutContributorInfo>, fw::AgentError> {
        let framework = self.framework().await?;
        let services = framework.services()?;
        Ok(services
            .providers::<fw::ContextContributorService>()
            .into_iter()
            .zip(services.all::<fw::ContextContributorService>())
            .map(|(provider_id, service)| {
                let source = &service.0;
                super::super::ContextLayoutContributorInfo {
                    provider_id: provider_id.to_owned(),
                    block_id: source.block_id().to_owned(),
                    metadata: source.layout_metadata(),
                    kind: source.kind(),
                }
            })
            .collect())
    }

    async fn initialize_framework(&self) -> Result<Framework, fw::AgentError> {
        if let Some(parent) = self.0.path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent)
                .map_err(|e| fw::AgentError::InvalidConfiguration(e.to_string()))?;
        }
        self.prepare_artifacts()
            .map_err(fw::AgentError::InvalidConfiguration)?;
        let (packs, extensions) = self.capability_state();
        self.0.tools_enabled.store(
            extensions.get("tool").copied().unwrap_or(true),
            std::sync::atomic::Ordering::Release,
        );
        self.0.observe_enabled.store(
            extensions.get("observe").copied().unwrap_or(true),
            std::sync::atomic::Ordering::Release,
        );
        // Discover before opening the Framework so a failed package lookup cannot
        // leave profile storage or plugin shutdown callbacks without an owner.
        let manager =
            eliteforge_agent_capability_runtime::CapabilityPackManager::new(&self.0.pack_root)
                .map_err(|e| fw::AgentError::InvalidConfiguration(e.to_string()))?;
        let mut installed_skills = BTreeMap::from([(
            tron_harness_capability::PACK_ID.to_owned(),
            tron_harness_capability::skill_names()
                .into_iter()
                .map(str::to_owned)
                .collect::<Vec<_>>(),
        )]);
        let packages = manager
            .installed()
            .map_err(|e| fw::AgentError::InvalidConfiguration(e.to_string()))?
            .into_iter()
            .map(|p| p.manifest.id)
            .collect::<std::collections::BTreeSet<_>>();
        for id in &packages {
            if Self::builtin_pack(id) {
                continue;
            }
            let pack = manager
                .select(
                    id,
                    &semver::VersionReq::STAR,
                    "tron",
                    "default",
                    serde_json::Value::Null,
                )
                .map_err(|e| fw::AgentError::InvalidConfiguration(e.to_string()))?;
            let selected: Arc<dyn fw::CapabilityPack> = Arc::new(pack);
            let sources = fw::selected_capability_skill_sources(&[selected])
                .map_err(|e| fw::AgentError::InvalidConfiguration(e.to_string()))?;
            let mut names = std::collections::BTreeSet::new();
            for source in sources {
                if let fw::CapabilitySkillSource::Directory(directory) = source.source {
                    let provider =
                        eliteforge_agent_official_plugin_skills::FileSystemSkillProvider::new(
                            eliteforge_agent_official_plugin_skills::FileSystemSkillsConfig::new()
                                .without_system_directory()
                                .without_local_directory()
                                .with_system_directory(directory),
                        );
                    let catalog = provider.discover().await?;
                    names.extend(catalog.skills.into_iter().map(|s| s.name));
                }
            }
            installed_skills.insert(id.clone(), names.into_iter().collect());
        }
        let framework = self
            .framework_builder(&packs, &extensions)
            .await
            .map_err(fw::AgentError::InvalidConfiguration)?
            .build()
            .await?;
        let builtin_catalog = match self.builtin_tool_catalog(&framework) {
            Ok(catalog) => catalog,
            Err(error) => {
                let _ = framework.close().await;
                return Err(fw::AgentError::InvalidConfiguration(error));
            }
        };
        if let Err(error) = self
            .discover_imported_tools(&framework, builtin_catalog)
            .await
        {
            let _ = framework.close().await;
            return Err(fw::AgentError::InvalidConfiguration(error));
        }
        *self.0.skills.write().unwrap() = installed_skills;
        Ok(framework)
    }
}

struct OptionalToolAudit(Arc<std::sync::atomic::AtomicBool>);
#[async_trait::async_trait]
impl fw::ToolMiddleware for OptionalToolAudit {
    async fn intercept(
        &self,
        invocation: &fw::ToolInvocationContext<'_>,
        next: fw::ToolNext<'_>,
    ) -> Result<fw::ToolOutput, fw::ToolError> {
        if self.0.load(std::sync::atomic::Ordering::Acquire) {
            fw::ToolMiddleware::intercept(&ToolAudit, invocation, next).await
        } else {
            next.run().await
        }
    }
}
