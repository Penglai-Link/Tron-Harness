use std::sync::Arc;

use serde_json::json;

use crate::{
    application::short_term_memory::{ReplaceTodosCommand, ShortTermMemoryService},
    domain::agent::{
        AgentRunContext, Tool, ToolError, ToolExecutionResult, ToolObservation, ToolRequest,
    },
};

pub(super) fn short_term_memory_tools(
    short_term_memory: ShortTermMemoryService,
) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(TodoUpdateTool {
            short_term_memory: short_term_memory.clone(),
            dude_only: false,
        }),
        Arc::new(TodoUpdateTool {
            short_term_memory,
            dude_only: true,
        }),
    ]
}

struct TodoUpdateTool {
    short_term_memory: ShortTermMemoryService,
    dude_only: bool,
}

impl Tool for TodoUpdateTool {
    fn name(&self) -> &str {
        if self.dude_only {
            "dude_todo_update"
        } else {
            "todo_update"
        }
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let requested_workspace = request
            .input
            .get("workspaceCode")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
        if self.dude_only {
            if !context.is_owning_dude()
                || !context.workspace_code.trim().is_empty()
                || request.input.get("workspaceCode").is_some()
            {
                return Err(ToolError::ExecutionFailed(
                    "Dude todo tools require an owning Dude without workspaceCode".into(),
                ));
            }
        } else {
            super::platform_tools::resolve_workspace_code(&requested_workspace, context)?;
        }
        let command: ReplaceTodosCommand = serde_json::from_value(request.input)
            .map_err(|_| ToolError::ExecutionFailed("todo_update input is invalid".into()))?;
        let todos = self
            .short_term_memory
            .replace_todos(context, command.todos)
            .map_err(|error| ToolError::ExecutionFailed(error.to_string()))?;
        let output = serde_json::to_string(&json!({
            "sessionCode": context
                .short_term_memory
                .as_ref()
                .map(|memory| memory.session_code.clone()),
            "count": todos.len(),
            "todos": todos,
        }))
        .map_err(|_| {
            ToolError::ExecutionFailed("todo_update output serialization failed".into())
        })?;
        Ok(ToolExecutionResult::observation(ToolObservation {
            tool_name: self.name().into(),
            output,
        }))
    }
}
