use std::sync::Arc;

use serde::Deserialize;

use crate::{
    application::resource::{
        ResourceBinaryAgentCommand,
        ResourceCreateCommand, ResourceDeleteCommand, ResourceGetQuery, ResourcePageQuery,
        ResourceService, ResourceUpdateCommand,
    },
    domain::{
        agent::{AgentRunContext, Tool, ToolError, ToolExecutionResult, ToolRequest},
        resource::ResourceKind,
    },
};

use super::common::{
    json_observation, parse_tool_input, resolve_workspace_code, service_tool_error,
};

pub(super) fn tools(resources: ResourceService) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(ResourceQueryTool {
            resources: resources.clone(),
        }),
        Arc::new(ResourceGetTool {
            resources: resources.clone(),
        }),
        Arc::new(ResourceCreateTool {
            resources: resources.clone(),
        }),
        Arc::new(ResourceUpdateTool {
            resources: resources.clone(),
        }),
        Arc::new(ResourceDeleteTool {
            resources: resources.clone(),
        }),
        Arc::new(ResourceBinaryTool {
            resources: resources.clone(),
        }),
    ]
}
