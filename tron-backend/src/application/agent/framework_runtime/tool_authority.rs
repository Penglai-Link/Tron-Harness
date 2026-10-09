//! Product ACL and registration selection over original SDK tool bindings.
//! Admission changes the catalog, never the executor or resource owner.
use super::super::model::HostExecution;
use crate::application::agent::tool_framework::FrameworkToolRegistryExt;
use crate::domain::agent::{AgentRunContext, EffectiveTool, ToolRegistry, ToolRequest};
use async_trait::async_trait;
use eliteforge_agent_core as fw;

pub(super) fn has_product_implementation(
    registry: &ToolRegistry,
    context: &AgentRunContext,
    name: &str,
) -> bool {
    registry
        .resolve_allowed(
            &ToolRequest {
                tool_name: name.into(),
                input: serde_json::Value::Null,
                invocation_id: None,
            },
            context,
        )
        .is_ok()
}

fn framework_a2ui_binding(extensions: &fw::Extensions, descriptor: &fw::ToolDescriptor) -> bool {
    eliteforge_agent_official_plugin_a2ui::negotiated(extensions)
        && (eliteforge_agent_official_plugin_a2ui::is_a2ui_tool(descriptor)
            || native_skill_descriptor(descriptor))
}

pub(crate) fn map_authorized_descriptor(
    context: &AgentRunContext,
    descriptor: &fw::ToolDescriptor,
    extensions: &fw::Extensions,
) -> Result<EffectiveTool, fw::AgentError> {
    if framework_a2ui_binding(extensions, descriptor) {
        return Ok(EffectiveTool {
            tool_name: descriptor.call_name.clone(),
            domain_code: Some("a2ui".into()),
            description: descriptor.description.clone(),
            input_schema: descriptor.input_schema.clone(),
            skill_name: Some(eliteforge_agent_official_plugin_a2ui::A2UI_SKILL_NAME.into()),
            required_variable_keys: vec![],
        });
    }
    if !context.is_tool_authorized(&descriptor.call_name) {
        return Err(fw::AgentError::ToolSetup(format!(
            "unauthorized tool: {}",
            descriptor.call_name
        )));
    }
    let mut tool = context
        .authorized_tools()
        .iter()
        .chain(context.tool_exposure.hidden_tools.iter())
        .find(|tool| tool.tool_name == descriptor.call_name)
        .cloned()
        .ok_or_else(|| {
            fw::AgentError::ToolSetup(format!("missing tool ACL: {}", descriptor.call_name))
        })?;
    tool.description = descriptor.description.clone();
    tool.input_schema = descriptor.input_schema.clone();
    Ok(tool)
}

fn overridden_native_skill(
    host: &HostExecution,
    state: &super::super::model::HostRunState,
    descriptor: &fw::ToolDescriptor,
) -> bool {
    if !native_skill_descriptor(descriptor) {
        return false;
    }
    let registry = host.harness.framework_registry();
    has_product_implementation(&registry, &state.context, &descriptor.call_name)
        || registry.has_framework_binding(&state.context.run_code, &descriptor.call_name)
        || state
            .tool_catalog
            .get(&descriptor.call_name)
            .is_some_and(|id| id != &descriptor.id)
}

fn native_skill_descriptor(descriptor: &fw::ToolDescriptor) -> bool {
    matches!(
        (descriptor.id.as_str(), descriptor.call_name.as_str()),
        ("skills.load", "skill_load")
            | ("skills.read_resource", "skill_read_resource")
            | ("skills.diagnostics", "skill_diagnostics")
    )
}

pub(crate) struct ToolCatalogPolicy {
    tools_enabled: std::sync::Arc<std::sync::atomic::AtomicBool>,
    management: Option<crate::application::agent::AgentManagementService>,
}
impl ToolCatalogPolicy {
    pub(crate) fn shared(
        tools_enabled: std::sync::Arc<std::sync::atomic::AtomicBool>,
        management: Option<crate::application::agent::AgentManagementService>,
    ) -> Self {
        Self {
            tools_enabled,
            management,
        }
    }
}
#[async_trait]
impl fw::ToolPolicy for ToolCatalogPolicy {
    async fn admit_binding(
        &self,
        binding: &fw::ToolBinding,
        run: &fw::RunContext,
    ) -> Result<fw::ToolBindingAdmission, fw::AgentError> {
        if !self
            .tools_enabled
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Ok(fw::ToolBindingAdmission::Exclude);
        }
        let host = HostExecution::from_extensions(&run.extensions)?;
        let state = host.state.lock().await;
        let name = &binding.descriptor.call_name;
        if (!state.product_tool_authority.contains(name)
            && !framework_a2ui_binding(&run.extensions, &binding.descriptor))
            || overridden_native_skill(&host, &state, &binding.descriptor)
        {
            return Ok(fw::ToolBindingAdmission::Exclude);
        }
        if framework_a2ui_binding(&run.extensions, &binding.descriptor) {
            return Ok(fw::ToolBindingAdmission::Allow);
        }
        Ok(
            if state
                .context
                .tools
                .iter()
                .any(|tool| &tool.tool_name == name)
            {
                fw::ToolBindingAdmission::Allow
            } else {
                fw::ToolBindingAdmission::Hidden
            },
        )
    }

    async fn catalog_ready(
        &self,
        bindings: &[fw::ToolBinding],
        run: &fw::RunContext,
    ) -> Result<(), fw::AgentError> {
        let host = HostExecution::from_extensions(&run.extensions)?;
        let mut state = host.state.lock().await;
        // Legacy contexts use `tools` for both ACL and model exposure until an
        // effective catalog is supplied. Materialize that original ACL before
        // context/model preparation narrows `tools` to actual native bindings.
        // An authorized but unregistered tool remains missing, not unauthorized;
        // copying its DTO does not admit an executor or expose its schema.
        if state.context.effective_tools.is_empty() {
            state.context.effective_tools = state.context.tools.clone();
        }
        // Remove globally denied tools from discovery while preserving the
        // existing distinction between authorized and registered executors.
        if let Some(management) = &self.management {
            let mut denied = std::collections::HashSet::new();
            for tool in state.context.authorized_tools() {
                if !management
                    .platform_tool_allowed(&tool.tool_name)
                    .map_err(|_| {
                        fw::AgentError::ToolSetup("Could not read platform tool permission".into())
                    })?
                {
                    denied.insert(tool.tool_name.clone());
                }
            }
            state
                .context
                .tools
                .retain(|t| !denied.contains(&t.tool_name));
            state
                .context
                .tool_exposure
                .hidden_tools
                .retain(|t| !denied.contains(&t.tool_name));
            let hidden = state.context.tool_exposure.hidden_tools.clone();
            state
                .context
                .tool_exposure
                .visible_domains
                .retain_mut(|domain| {
                    let tools = hidden
                        .iter()
                        .filter(|t| t.domain_code.as_deref() == Some(domain.domain_code.as_str()))
                        .collect::<Vec<_>>();
                    domain.tool_count = tools.len();
                    domain
                        .sample_tools
                        .retain(|name| tools.iter().any(|t| &t.tool_name == name));
                    !tools.is_empty()
                });
        }
        state.tool_catalog = bindings
            .iter()
            .map(|binding| {
                (
                    binding.descriptor.call_name.clone(),
                    binding.descriptor.id.clone(),
                )
            })
            .collect();
        Ok(())
    }

    async fn evaluate(
        &self,
        invocation: &fw::ToolInvocationContext<'_>,
    ) -> Result<fw::ToolPolicyDecision, fw::AgentError> {
        if !self
            .tools_enabled
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Ok(fw::ToolPolicyDecision::Deny(
                "tool extension is disabled".into(),
            ));
        }
        let host = HostExecution::from_extensions(&invocation.run.extensions)?;
        let state = host.state.lock().await;
        if let Some(management) = &self.management {
            if !management
                .platform_tool_allowed(&invocation.descriptor.call_name)
                .map_err(|_| {
                    fw::AgentError::Run("Could not read platform tool permission".into())
                })?
            {
                return Ok(fw::ToolPolicyDecision::Deny(
                    "This tool has no platform permission".into(),
                ));
            }
        }
        if (!state
            .product_tool_authority
            .contains(&invocation.descriptor.call_name)
            && !(framework_a2ui_binding(&invocation.run.extensions, invocation.descriptor)
                && (!native_skill_descriptor(invocation.descriptor)
                    || invocation
                        .call
                        .arguments
                        .get("name")
                        .and_then(serde_json::Value::as_str)
                        == Some(eliteforge_agent_official_plugin_a2ui::A2UI_SKILL_NAME))))
            || overridden_native_skill(&host, &state, invocation.descriptor)
        {
            return Ok(fw::ToolPolicyDecision::Deny(
                "tool authority changed after catalog admission".into(),
            ));
        }
        Ok(fw::ToolPolicyDecision::Allow)
    }
}

/// Hosts without management (isolated tests) preserve the previous run behavior.
pub(crate) struct PlatformPermissionSource(
    pub Option<crate::application::agent::AgentManagementService>,
);
impl tron_agent_harness::ToolPermissionSource for PlatformPermissionSource {
    fn setting(
        &self,
        descriptor: &fw::ToolDescriptor,
    ) -> Result<tron_agent_harness::ToolPermissionSetting, String> {
        match &self.0 {
            Some(source) => tron_agent_harness::ToolPermissionSource::setting(source, descriptor),
            None => Ok(tron_agent_harness::ToolPermissionSetting::default_for(
                &descriptor.id,
            )),
        }
    }
    fn operation(&self, descriptor: &fw::ToolDescriptor) -> Option<bool> {
        self.0
            .as_ref()
            .and_then(|s| tron_agent_harness::ToolPermissionSource::operation(s, descriptor))
    }
}
