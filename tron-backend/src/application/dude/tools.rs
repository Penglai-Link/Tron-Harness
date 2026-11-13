//! Dude decides business significance; platform persists changes and delivery receipts.
use super::{
    DudeService,
    work_changes::{self, DudeWorkChangeTool},
};
use crate::{
    application::{agent::FrameworkHost, task::TaskService},
    domain::agent::{
        AgentRunContext, Tool, ToolError, ToolExecutionResult, ToolObservation, ToolRegistry,
        ToolRequest,
    },
};
use serde::Deserialize;
use std::sync::Arc;

pub fn register_dude_tools(
    registry: &ToolRegistry,
    dudes: DudeService,
    tasks: TaskService,
    _host: FrameworkHost,
) -> Result<(), ToolError> {
    registry.register(Arc::new(SoulTool {
        dudes: dudes.clone(),
        update: false,
    }))?;
    registry.register(Arc::new(SoulTool {
        dudes: dudes.clone(),
        update: true,
    }))?;
    registry.register(Arc::new(DudeWorkChangeTool {
        dudes: dudes.clone(),
        tasks: tasks.clone(),
    }))?;
    super::notification_tools::register(registry, dudes.clone())?;
    registry.register(Arc::new(DudeNotify { dudes, tasks }))
}
struct DudeNotify {
    dudes: DudeService,
    tasks: TaskService,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NotifyInput {
    change_id: String,
    title: String,
    body: String,
}
impl Tool for DudeNotify {
    fn name(&self) -> &str {
        "dude_notify"
    }
    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let dude = work_changes::owner(context)?;
        let input: NotifyInput = serde_json::from_value(request.input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid notification: {e}")))?;
        let notice = work_changes::publish(
            &self.dudes,
            &self.tasks,
            dude,
            &input.change_id,
            &input.title,
            &input.body,
        )?;
        Ok(ToolExecutionResult::observation(ToolObservation {
            tool_name: self.name().into(),
            output: serde_json::to_string(&notice)
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?,
        }))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SoulReadInput {}

struct SoulTool {
    dudes: DudeService,
    update: bool,
}
impl Tool for SoulTool {
    fn name(&self) -> &str {
        if self.update {
            "dude_soul_update"
        } else {
            "dude_soul_read"
        }
    }
    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let code = work_changes::owner(context)?;
        let result = if self.update {
            let input = serde_json::from_value(request.input)
                .map_err(|e| ToolError::ExecutionFailed(format!("Invalid soul update: {e}")))?;
            self.dudes.update_soul_from_user(context, input)
        } else {
            let _: SoulReadInput = serde_json::from_value(request.input)
                .map_err(|e| ToolError::ExecutionFailed(format!("Invalid soul read: {e}")))?;
            self.dudes.soul(code)
        }
        .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        // Do not expose the private onboarding session/user to the model.
        Ok(ToolExecutionResult::observation(ToolObservation { tool_name:self.name().into(), output:serde_json::json!({"name":result.name,"markdown":result.markdown,"state":result.state}).to_string() }))
    }
}
