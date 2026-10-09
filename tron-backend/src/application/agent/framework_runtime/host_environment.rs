//! One plugin environment per profile; Agent bindings only select capabilities.
use super::*;
use eliteforge_agent_framework::FrameworkBuilder;
use eliteforge_agent_official_plugin_storage_local::LocalStoragePlugin;
impl FrameworkHost {
    pub(super) async fn framework_builder(
        &self,
        packs: &BTreeMap<String, bool>,
        _extensions: &BTreeMap<String, bool>,
    ) -> Result<FrameworkBuilder, String> {
        let mut builder = Framework::builder()
            .default_models(false)
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
            .plugin(
                LocalStoragePlugin::new(&self.0.path, self.0.path.with_extension("artifacts"))
                    .with_database(self.shared_database().await.map_err(|e| e.to_string())?),
            )
            .plugin(eliteforge_agent_framework::ReactPlugin)
            .plugin(fw::AgentLoopPlugin::named(
                "tron.configured-loop",
                super::super::configured_loop::ConfiguredLoop(
                    self.0.agent_management.get().cloned(),
                ),
            ))
            .plugin(fw::WorkflowNodePlugin::new(
                "tron.branch",
                super::super::configured_loop::BranchNode,
            ))
            .plugin(fw::WorkflowNodePlugin::new(
                "tron.model-stage",
                super::super::configured_loop::ModelStageNode,
            ))
            .plugin(fw::AgentLoopPlugin::named(
                "tron.external-loop",
                eliteforge_agent_framework::ExternalAgentLoop,
            ))
            .plugin(fw::AgentLoopPlugin::named(
                "tron.continuation-loop",
                eliteforge_agent_framework::ContinuationLoop,
            ))
            .plugin(fw::ExternalAgentPlugin::new(
                super::super::external::HostExternalAgent,
            ))
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
                "tron.remote.tools",
                remote_tools::RemoteTools(Arc::downgrade(&self.0)),
            ))
            .plugin(fw::ToolPolicyPlugin::new(
                "tron.harness.tool-authority",
                super::super::tools::ToolCatalogPolicy::shared(self.0.tools_enabled.clone()),
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
                tron_agent_harness::TronHarness::new(Arc::new(HarnessTools), [])
                    .with_context_preparation(TronContext, 32)
                    .capability_pack(),
            );
        if packs
            .get(eliteforge_agent_official_plugin_a2ui::A2UI_PACK_ID)
            .copied()
            .unwrap_or(true)
        {
            builder = builder.capability_pack(
                eliteforge_agent_official_plugin_a2ui::A2uiPack::new()
                    .map_err(|e| e.to_string())?,
            );
        }
        let manager =
            eliteforge_agent_capability_runtime::CapabilityPackManager::new(&self.0.pack_root)
                .map_err(|error| error.to_string())?;
        let mut installed = BTreeMap::new();
        for (package, active) in packs {
            if !active || Self::builtin_pack(package) {
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
    fn builtin_pack(id: &str) -> bool {
        matches!(
            id,
            "framework.external-agent"
                | "framework.a2ui"
                | "framework.local.scoped"
                | "framework.tool-pack.agent-collaboration"
                | "framework.personai"
                | "framework.web"
                | "framework.media"
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
        let pack_enabled = |id: &str| packs.get(id).copied().unwrap_or(true);
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
            agent_loop: Some("tron.configured-loop".into()),
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
                        selected(id)
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
            skills: Some(
                self.0
                    .skills
                    .read()
                    .unwrap()
                    .iter()
                    .filter(|(pack, _)| pack_enabled(pack))
                    .flat_map(|(_, names)| names.clone())
                    .chain(
                        pack_enabled(eliteforge_agent_official_plugin_a2ui::A2UI_PACK_ID).then(
                            || eliteforge_agent_official_plugin_a2ui::A2UI_SKILL_NAME.to_owned(),
                        ),
                    )
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .collect(),
            ),
            context_sources: Some(
                registry
                    .providers::<fw::ContextContributorService>()
                    .into_iter()
                    .filter(|id| {
                        selected(id)
                            && (*id != "agent.a2ui"
                                || pack_enabled(
                                    eliteforge_agent_official_plugin_a2ui::A2UI_PACK_ID,
                                ))
                    })
                    .map(str::to_owned)
                    .collect(),
            ),
            workflow_nodes: Some(
                registry
                    .providers::<fw::WorkflowNodeService>()
                    .into_iter()
                    .filter(|id| selected(id))
                    .map(str::to_owned)
                    .collect(),
            ),
            ..AgentSelection::default()
        })
    }
    pub(super) async fn framework(&self) -> Result<&Framework, fw::AgentError> {
        self.0
            .framework
            .get_or_try_init(|| self.initialize_framework())
            .await
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
        let mut installed_skills = BTreeMap::new();
        for (id, active) in &packs {
            if !active || Self::builtin_pack(id) {
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
