//! Tron-owned context and business tools assembled as a native capability pack.
//!
//! Authentication, database queries, attachments, RAG, goals and project/worktree
//! selection stay with Tron. The host supplies authorized framework providers;
//! this package adds run-scope validation and prevents shadowing shared tools.
//! Selecting this pack alone never installs filesystem/command/model plugins.

use std::{collections::HashSet, sync::Arc};

use async_trait::async_trait;
use eliteforge_agent_core::{
    AgentError, CapabilityPackBuilder, ContextContributor, ContextContributorPlugin, RunContext,
    RunRequest, Tool, ToolDescriptor, ToolError, ToolLease, ToolOutput, ToolProvider,
    ToolProviderPlugin,
};
use eliteforge_agent_official_plugin_tools_local::local_tool_catalog;

pub const PACK_ID: &str = "tron.harness";

/// Skills bundled with the native package, independent of the source checkout.
pub fn skill_source() -> eliteforge_agent_core::CapabilitySkillSource {
    eliteforge_agent_core::capability_skills!()
}

/// Derive catalog names from the same embedded bundle used by the runtime.
pub fn skill_names() -> Vec<&'static str> {
    let eliteforge_agent_core::CapabilitySkillSource::Embedded(bundle) = skill_source() else {
        unreachable!("Tron skills are embedded");
    };
    bundle
        .files
        .iter()
        .filter_map(|file| file.path.strip_suffix("/SKILL.md"))
        .collect()
}

/// Context extension description published by the Tron harness capability package.
pub const CONTEXT_IMPLEMENTATION_DESCRIPTION: &str =
    "Prepares Tron conversation context within a bounded token budget.";
/// Model extension description published by the Tron harness capability package.
pub const MODEL_IMPLEMENTATION_DESCRIPTION: &str =
    "Connects Tron model routing and token estimation to the Framework model extension.";
/// Observation extension description published by the Tron harness capability package.
pub const OBSERVATION_IMPLEMENTATION_DESCRIPTION: &str =
    "Records tool-call audit events for runtime observation.";
/// Product-specific tools description published by the Tron harness capability package.
pub const TOOL_IMPLEMENTATION_DESCRIPTION: &str = "Adds Tron-specific business tools with run-scope authorization and platform risk approval policy.";

/// Trusted host routing information, deliberately not deserializable from a
/// model argument or HTTP body. Providers must still check current permissions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TronRunScope {
    pub user_code: String,
    pub workspace_code: String,
    pub session_code: String,
    pub run_code: String,
    pub app_code: Option<String>,
    pub task_code: Option<String>,
    pub agent_code: Option<String>,
}

impl TronRunScope {
    pub fn attach(self, request: &mut RunRequest) -> Result<(), AgentError> {
        self.validate(&request.session_id)?;
        request.extensions.insert(self);
        Ok(())
    }

    fn validate(&self, session: &str) -> Result<(), AgentError> {
        if [
            &self.user_code,
            &self.workspace_code,
            &self.session_code,
            &self.run_code,
        ]
        .iter()
        .any(|value| value.trim().is_empty())
        {
            return Err(AgentError::InvalidConfiguration(
                "Tron user, workspace, session and run identifiers are required".into(),
            ));
        }
        if self.session_code != session {
            return Err(AgentError::InvalidConfiguration(
                "Tron harness scope does not belong to this session".into(),
            ));
        }
        Ok(())
    }

    pub fn from_run(run: &RunContext) -> Result<Arc<Self>, AgentError> {
        run.check_cancelled()?;
        let scope = run.extensions.get::<Self>().ok_or_else(|| {
            AgentError::InvalidConfiguration("Tron harness run scope is missing".into())
        })?;
        scope.validate(&run.session_id)?;
        Ok(scope)
    }
}

/// The same contracts that the shared runtime consumes. No product-specific
/// loop, filesystem implementation, command manager or model client is copied.
pub struct TronHarness {
    context_sources: Vec<Arc<dyn ContextContributor>>,
    preparation: Option<Arc<dyn eliteforge_agent_core::Plugin>>,
    tools: Arc<dyn ToolProvider>,
    tool_policy: Option<Arc<dyn eliteforge_agent_core::ToolPolicy>>,
    shared_ids: HashSet<String>,
    shared_names: HashSet<String>,
}

impl TronHarness {
    /// Supply descriptors of every other selected tool pack (MCP, human, agents,
    /// discovery, etc.). Local tools are always reserved, even if not selected.
    /// Descriptors are metadata only: adding one does not grant tool authority.
    pub fn new(
        tools: Arc<dyn ToolProvider>,
        shared_tools: impl IntoIterator<Item = ToolDescriptor>,
    ) -> Self {
        let catalog: Vec<_> = local_tool_catalog()
            .into_iter()
            .chain(shared_tools)
            .collect();
        Self {
            context_sources: Vec::new(),
            preparation: None,
            tool_policy: None,
            tools,
            shared_ids: catalog.iter().map(|tool| tool.id.clone()).collect(),
            shared_names: catalog.into_iter().map(|tool| tool.call_name).collect(),
        }
    }

    pub fn with_context_source(mut self, source: impl ContextContributor + 'static) -> Self {
        self.context_sources.push(Arc::new(source));
        self
    }

    /// Select the host's context projection and summary persistence policy.
    /// Framework supplies the preparation/reduction execution mechanism.
    pub fn with_context_preparation(
        mut self,
        policy: impl eliteforge_agent_core::ContextPreparationPolicy + 'static,
        max_reductions: usize,
    ) -> Self {
        self.preparation = Some(Arc::new(
            eliteforge_agent_core::ContextAssemblerPlugin::new(
                eliteforge_agent_core::PreparingContextAssembler::new(policy, max_reductions),
            ),
        ));
        self
    }

    /// Global product policy is additional to the host's per-Agent ACL.
    pub fn with_tool_policy(
        mut self,
        policy: impl eliteforge_agent_core::ToolPolicy + 'static,
    ) -> Self {
        self.tool_policy = Some(Arc::new(policy));
        self
    }

    pub fn capability_pack(self) -> CapabilityPackBuilder<()> {
        let shared = Arc::new(self);
        let mut pack = CapabilityPackBuilder::new(PACK_ID, ())
            .skills(skill_source())
            .plugin(ToolProviderPlugin::from_arc(
                "tron.harness.tools",
                shared.clone(),
            ));
        if let Some(policy) = &shared.tool_policy {
            pack = pack.plugin(eliteforge_agent_core::ToolPolicyPlugin::from_arc(
                "tron.harness.tool-permissions",
                policy.clone(),
            ));
        }
        for source in &shared.context_sources {
            pack = pack.plugin(ContextContributorPlugin::from_arc(
                source.block_id(),
                source.clone(),
            ));
        }
        match &shared.preparation {
            Some(plugin) => pack.plugin_arc(plugin.clone()),
            None => pack,
        }
    }
}

#[async_trait]
impl ToolProvider for TronHarness {
    async fn lease(&self, run: &RunContext) -> Result<ToolLease, AgentError> {
        let scope = TronRunScope::from_run(run)?;
        let mut lease = self.tools.lease(run).await?;
        // Let Agent shutdown own cleanup even when catalog validation fails.
        run.own_resources(std::mem::take(&mut lease.resources));
        let mut ids = HashSet::new();
        let mut names = HashSet::new();
        for binding in &mut lease.tools {
            let descriptor = &binding.descriptor;
            if self.shared_ids.contains(&descriptor.id)
                || self.shared_names.contains(&descriptor.call_name)
            {
                return Err(AgentError::ToolSetup(format!(
                    "Tron harness must use the shared tool instead of redefining {} / {}",
                    descriptor.id, descriptor.call_name,
                )));
            }
            if !ids.insert(descriptor.id.clone()) || !names.insert(descriptor.call_name.clone()) {
                return Err(AgentError::ToolSetup(format!(
                    "duplicate Tron harness tool: {} / {}",
                    descriptor.id, descriptor.call_name,
                )));
            }
            binding.executor = Arc::new(ScopedTool {
                scope: scope.clone(),
                inner: binding.executor.clone(),
            });
        }
        Ok(lease)
    }
}

struct ScopedTool {
    scope: Arc<TronRunScope>,
    inner: Arc<dyn Tool>,
}

#[async_trait]
impl Tool for ScopedTool {
    async fn call(
        &self,
        arguments: serde_json::Value,
        run: &RunContext,
    ) -> Result<ToolOutput, ToolError> {
        let scope =
            TronRunScope::from_run(run).map_err(|error| ToolError::Failed(error.to_string()))?;
        if scope != self.scope {
            return Err(ToolError::Failed(
                "Tron tool lease belongs to another run scope".into(),
            ));
        }
        self.inner.call(arguments, run).await
    }
}

pub mod tool_policy;
pub mod tool_risk;
pub use tool_policy::{
    ToolPermission, ToolPermissionSetting, ToolPermissionSource, TronToolPolicy,
    TronToolPolicyLocale,
};
pub use tool_risk::{ToolRisk, identify_tool_risk};
