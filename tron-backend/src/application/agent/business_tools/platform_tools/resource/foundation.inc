use std::{future::Future, sync::Arc, thread};

use serde::Deserialize;
use serde_json::Value;

use crate::{
    application::resource::{
        DataSourceCreateCommand, DataSourceUpdateCommand, ResourceBinaryAgentCommand,
        ResourceCreateCommand, ResourceDeleteCommand, ResourceGetQuery, ResourcePageQuery,
        ResourceService, ResourceUpdateCommand, ServerAccountCreateCommand,
        ServerAccountCreateInput, ServerCreateCommand,
    },
    domain::{
        agent::{AgentRunContext, Tool, ToolError, ToolExecutionResult, ToolRequest},
        resource::{ResourceError, ResourceKind, ServerAuthenticationType},
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
        Arc::new(DataSourceCreateTool {
            resources: resources.clone(),
        }),
        Arc::new(DataSourceUpdateTool {
            resources: resources.clone(),
        }),
        Arc::new(ServerCreateTool {
            resources: resources.clone(),
        }),
        Arc::new(ServerAccountCreateTool { resources }),
    ]
}
