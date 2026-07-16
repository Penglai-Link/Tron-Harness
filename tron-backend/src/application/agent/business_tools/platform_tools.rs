mod app;
mod automation;
mod common;
mod model;
mod resource;
mod skill;
mod task;
mod variable;
mod workspace;

use std::sync::Arc;

use crate::{
    application::{
        app::AppService, automation::AutomationService, model::ModelService,
        resource::ResourceService, skill::SkillRegistryService, task::TaskService,
        variable::VariableService, workspace::WorkspaceService,
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
) -> Vec<Arc<dyn Tool>> {
    let mut tools: Vec<Arc<dyn Tool>> = Vec::new();
    tools.extend(workspace::tools(workspaces));
    tools.extend(app::tools(apps));
    tools.extend(automation::tools(automations));
    tools.extend(task::tools(tasks));
    tools.extend(model::tools(models));
    tools.extend(skill::tools(skills));
    tools.extend(variable::tools(variables));
    tools.extend(resource::tools(resources));
    tools
}
