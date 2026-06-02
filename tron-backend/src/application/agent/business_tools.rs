mod discovery;
mod platform_tools;

use crate::domain::agent::{EffectiveTool, Tool, ToolError, ToolRegistry};
use crate::{
    application::{
        app::AppService, model::ModelService, skill::SkillRegistryService, task::TaskService,
        variable::VariableService, workspace::WorkspaceService,
    },
    domain::agent::ToolCatalogRecord,
};

pub use discovery::{
    DISCOVERY_EXPAND_TOOL_DOMAIN_TOOL, build_domain_discovery_guidance,
    discovery_expand_tool_definition,
};

pub fn tool_record_to_effective_tool(record: ToolCatalogRecord) -> EffectiveTool {
    EffectiveTool {
        tool_name: record.tool_code,
        domain_code: Some(record.domain_code),
        description: record.description,
        input_schema: record.input_schema,
        skill_name: None,
        required_variable_keys: Vec::new(),
    }
}

pub fn register_default_agent_tools(
    registry: &ToolRegistry,
    workspaces: WorkspaceService,
    apps: AppService,
    tasks: TaskService,
    models: ModelService,
    skills: SkillRegistryService,
    variables: VariableService,
) -> Result<(), ToolError> {
    let mut tools: Vec<std::sync::Arc<dyn Tool>> =
        platform_tools::platform_business_tools(workspaces, apps, tasks, models, skills, variables);
    tools.push(discovery::discovery_tool());

    for tool in tools {
        registry.register(tool)?;
    }
    Ok(())
}
