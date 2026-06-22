mod app;
mod common;
mod model;
mod skill;
mod task;
mod variable;
mod workspace;

use std::sync::Arc;

use crate::{
    application::{
        app::AppService, model::ModelService, skill::SkillRegistryService, task::TaskService,
        variable::VariableService, workspace::WorkspaceService,
    },
    domain::agent::Tool,
};

pub(super) fn platform_business_tools(
    workspaces: WorkspaceService,
    apps: AppService,
    tasks: TaskService,
    models: ModelService,
    skills: SkillRegistryService,
    variables: VariableService,
) -> Vec<Arc<dyn Tool>> {
    let mut tools: Vec<Arc<dyn Tool>> = Vec::new();
    tools.extend(workspace::tools(workspaces));
    tools.extend(app::tools(apps));
    tools.extend(task::tools(tasks));
    tools.extend(model::tools(models));
    tools.extend(skill::tools(skills));
    tools.extend(variable::tools(variables));
    tools
}
