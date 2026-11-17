//! Saved-policy tool authority and dynamic MCP bindings for a single invocation.
use super::super::{
    agui_warning::publish_mcp_runtime_warnings,
    helpers::{map_agent_error, map_mcp_error},
};
use super::{AgentInvocationError, AgentRunContextBuilder, McpWarningPublishContext};
use crate::{
    application::agent::tool_record_to_effective_tool,
    domain::agent::{EffectiveTool, InternalAgentToolPolicy, ToolAvailabilityState},
};
use std::collections::{HashMap, HashSet};

pub(super) struct PreparedToolAccess {
    pub(super) policy: Option<InternalAgentToolPolicy>,
    pub(super) tools: Vec<EffectiveTool>,
    pub(super) effective_tools: Vec<EffectiveTool>,
    pub(super) requested_tool_names: HashSet<String>,
}
impl AgentRunContextBuilder {
    pub(super) fn resolve_tool_access(
        &self,
        workspace_code: &str,
        agent_code: &str,
        locale: &str,
        requested_tools: Vec<EffectiveTool>,
    ) -> Result<PreparedToolAccess, AgentInvocationError> {
        let requested_tool_names = requested_tools
            .iter()
            .map(|tool| tool.tool_name.clone())
            .collect::<HashSet<_>>();
        let mut tools = requested_tools;
        let mut effective_tools = tools.clone();
        let internal_agents = if self.platform_scope || workspace_code.is_empty() {
            self.management.all_platform_internal_agents()
        } else {
            self.management
                .list_workspace_internal_agents(workspace_code)
        }
        .map_err(|error| map_agent_error(agent_code, error))?;
        let internal_agent_policy = internal_agents
            .into_iter()
            .find(|policy| policy.agent_code.eq_ignore_ascii_case(agent_code));

        if let Some(policy) = internal_agent_policy.as_ref() {
            // Saved policy owns internal-agent authority. Request DTOs only narrow
            // that authority by name; their descriptions/schemas are never trusted.
            // An empty request keeps the legacy contract: use the saved policy.
            let mut effective_tool_records = self
                .management
                .tools_by_codes(&policy.selected_tool_codes, locale)
                .into_iter()
                .filter(|tool| tool.availability_state == ToolAvailabilityState::Available)
                .filter(|tool| {
                    requested_tool_names.is_empty()
                        || requested_tool_names.contains(&tool.tool_name)
                })
                .collect::<Vec<_>>();
            if self
                .collaboration_owner
                .as_ref()
                .is_some_and(|owner| owner.kind == "ASSISTANT" && owner.code == agent_code)
            {
                // All tools is the platform-wide catalog, including workspace
                // tools. Their own scope and permission checks still apply.
                if policy.tool_assignment_mode
                    != crate::domain::agent::ToolAssignmentMode::AllPlatformTools
                {
                    let aliases = effective_tool_records
                        .iter()
                        .filter_map(|tool| {
                            crate::application::agent::assistant_tool_replacement(&tool.tool_code)
                        })
                        .map(str::to_owned)
                        .collect::<Vec<_>>();
                    effective_tool_records.retain(|tool| {
                        !crate::application::agent::tool_catalog::assistant_workspace_tool(tool)
                    });
                    effective_tool_records.extend(
                        self.management
                            .tools_by_codes(&aliases, locale)
                            .into_iter()
                            .filter(|tool| {
                                tool.availability_state == ToolAvailabilityState::Available
                            }),
                    );
                    let mut seen = HashSet::new();
                    effective_tool_records.retain(|tool| seen.insert(tool.tool_code.clone()));
                }
            } else {
                effective_tool_records.retain(|tool| {
                    !tool
                        .category
                        .as_ref()
                        .is_some_and(|category| category.code == "tron.assistant")
                });
            }
            // The gateway plugin owns on-demand discovery for every run, so
            // saved exposure-mode differences collapse to one behavior: the
            // full authorized list is the run catalog.
            tools = effective_tool_records
                .into_iter()
                .map(tool_record_to_effective_tool)
                .collect();
            effective_tools = tools.clone();
        }

        Ok(PreparedToolAccess {
            policy: internal_agent_policy,
            tools,
            effective_tools,
            requested_tool_names,
        })
    }

    pub(super) async fn materialize_mcp_tools(
        &self,
        access: &mut PreparedToolAccess,
        variable_values: &HashMap<String, String>,
        warning_context: &McpWarningPublishContext<'_>,
    ) -> Result<(), AgentInvocationError> {
        if let Some(policy) = access.policy.as_ref() {
            let mut materialization = self
                .mcp
                .materialize_runtime_tools(
                    warning_context.run_code,
                    warning_context.workspace_code,
                    policy.mcp_access_mode,
                    &policy.selected_server_codes,
                    variable_values,
                    self.tool_registry.as_ref(),
                )
                .await
                .map_err(map_mcp_error)?;
            publish_mcp_runtime_warnings(
                &self.event_bus,
                warning_context,
                &materialization.warnings,
            );
            // Only discovered bindings from policy-selected enabled servers can
            // add MCP authority. A same-name request projection creates no tool.
            let mut permitted = HashSet::new();
            let mut records = HashMap::new();
            let discovered = self
                .management
                .register_labeled_tool_descriptors(
                    &materialization.descriptors,
                    &materialization.tools,
                )
                .map_err(AgentInvocationError::RuntimeFailed)?;
            for (descriptor, record) in materialization.descriptors.iter().zip(discovered) {
                if self
                    .management
                    .platform_tool_allowed(&descriptor.call_name)
                    .map_err(|error| map_agent_error(&policy.agent_code, error))?
                {
                    permitted.insert(descriptor.call_name.clone());
                }
                records.insert(descriptor.call_name.clone(), record);
            }
            // SelectedTools keeps the existing MCP server assignment semantics;
            // the selected enabled servers above already grant these tool codes.
            let server_tool_codes = records
                .values()
                .map(|record| record.tool_code.clone())
                .collect::<Vec<_>>();
            materialization.tools.retain(|tool| {
                records.get(&tool.tool_name).is_some_and(|record| {
                    policy.tool_assignment_mode.selects_policy(
                        record,
                        &server_tool_codes,
                        &policy.selected_category_codes,
                    )
                }) && permitted.contains(&tool.tool_name)
                    && (access.requested_tool_names.is_empty()
                        || access.requested_tool_names.contains(&tool.tool_name))
            });
            access
                .effective_tools
                .extend(materialization.tools.iter().cloned());
            access.tools.extend(materialization.tools);
        }

        Ok(())
    }
}
