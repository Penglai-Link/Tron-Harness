mod app;
mod automation;
mod common;
mod model;
mod platform_mcp;
mod platform_model;
mod platform_prompt;
mod platform_resource;
mod platform_variable;
mod resource;
mod skill;
mod task;
mod variable;
mod workspace;

use std::sync::Arc;

use crate::{
    application::{
        app::AppService, automation::AutomationService, mcp::McpService, model::ModelService,
        prompt::PromptService, resource::ResourceService, skill::SkillRegistryService,
        task::TaskService, variable::VariableService, workspace::WorkspaceService,
    },
    domain::agent::Tool,
};

pub(super) fn platform_business_tools(
    workspaces: WorkspaceService,
    apps: AppService,
    automations: AutomationService,
    tasks: TaskService,
    models: ModelService,
    skills: SkillRegistryService,
    variables: VariableService,
    resources: ResourceService,
    prompts: PromptService,
    mcp: McpService,
) -> Vec<Arc<dyn Tool>> {
    let mut tools: Vec<Arc<dyn Tool>> = Vec::new();
    tools.extend(workspace::tools(workspaces));
    tools.extend(app::tools(apps));
    tools.extend(automation::tools(automations));
    tools.extend(task::tools(tasks));
    tools.extend(model::tools(models.clone()));
    tools.extend(platform_model::tools(models));
    tools.extend(platform_mcp::tools(mcp));
    tools.extend(platform_prompt::tools(prompts));
    tools.extend(platform_resource::tools(resources.clone()));
    tools.extend(skill::tools(skills));
    tools.extend(platform_variable::tools(variables.clone()));
    tools.extend(variable::tools(variables));
    tools.extend(resource::tools(resources));
    tools
}
