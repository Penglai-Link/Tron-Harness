//! Notifications are model-callable, but ownership and dedup are host authority.
use super::{DudeService, NotificationInput};
use crate::{
    application::{agent::FrameworkHost, task::TaskService},
    domain::agent::{
        AgentRunContext, Tool, ToolError, ToolExecutionControl, ToolExecutionFuture,
        ToolExecutionResult, ToolObservation, ToolRegistry, ToolRequest,
    },
};
use serde::Deserialize;
use std::sync::Arc;

pub fn register_dude_tools(
    registry: &ToolRegistry,
    dudes: DudeService,
    tasks: TaskService,
    host: FrameworkHost,
) -> Result<(), ToolError> {
    registry.register(Arc::new(DudeNotify { dudes, tasks, host }))
}
struct DudeNotify {
    dudes: DudeService,
    tasks: TaskService,
    host: FrameworkHost,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NotifyInput {
    title: String,
    body: String,
    thread_code: Option<String>,
    task_code: Option<String>,
}
impl Tool for DudeNotify {
    fn name(&self) -> &str {
        "dude_notify"
    }
    fn exec(&self, _: ToolRequest, _: &AgentRunContext) -> Result<ToolExecutionResult, ToolError> {
        Err(ToolError::ExecutionFailed(
            "Dude notifications require the native asynchronous runtime".into(),
        ))
    }
    fn exec_async<'a>(
        &'a self,
        request: ToolRequest,
        context: &'a AgentRunContext,
        control: &'a dyn ToolExecutionControl,
    ) -> Option<ToolExecutionFuture<'a>> {
        Some(Box::pin(async move {
            control.check_active()?;
            let owner = context
                .collaboration_owner
                .as_ref()
                .filter(|owner| owner.kind.eq_ignore_ascii_case("DUDE"))
                .ok_or_else(|| ToolError::UnauthorizedTool(self.name().into()))?;
            self.dudes
                .get(&owner.code)
                .map_err(|_| ToolError::UnauthorizedTool(self.name().into()))?;
            let invocation = request
                .invocation_id
                .filter(|id| !id.is_empty())
                .ok_or_else(|| {
                    ToolError::ExecutionFailed("Native invocation identity is required".into())
                })?;
            let input: NotifyInput = serde_json::from_value(request.input)
                .map_err(|e| ToolError::ExecutionFailed(format!("Invalid notification: {e}")))?;
            if input.title.trim().is_empty()
                || input.title.len() > 800
                || input.body.trim().is_empty()
                || input.body.len() > 32000
            {
                return Err(ToolError::ExecutionFailed(
                    "Notification title/body is empty or too long".into(),
                ));
            }
            if let Some(task) = input.task_code.as_deref() {
                let task = self
                    .tasks
                    .get_task(task)
                    .map_err(|_| ToolError::ExecutionFailed("Task is not accessible".into()))?;
                if task.owner_dude_code.as_deref() != Some(&owner.code) {
                    return Err(ToolError::ExecutionFailed(
                        "Task is outside the current Dude".into(),
                    ));
                }
            }
            if let Some(thread) = input.thread_code.as_deref() {
                if !self
                    .host
                    .dude_thread_catalog(&owner.code)
                    .await
                    .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?
                    .iter()
                    .any(|item| item.summary.thread_id == thread)
                {
                    return Err(ToolError::ExecutionFailed(
                        "Thread is outside the current Dude".into(),
                    ));
                }
            }
            let session = owner
                .origin_session_code
                .as_deref()
                .map(|id| self.dudes.session(id))
                .transpose()
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?
                .flatten()
                .filter(|session| session.dude_code == owner.code)
                .map(|session| session.session_code);
            control.check_active()?;
            let notice = self.dudes.notify(NotificationInput {
                dedup_key: format!("dude-notify:{}:{invocation}", owner.code),
                dude_code: Some(owner.code.clone()), title: input.title.trim().into(),
                body: input.body.trim().into(), category: "Dude".into(),
                target: serde_json::json!({"sessionCode":session,"threadCode":input.thread_code,"taskCode":input.task_code}),
            }).map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
            Ok(ToolExecutionResult::observation(ToolObservation {
                tool_name: self.name().into(),
                output: serde_json::to_string(&notice)
                    .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?,
            }))
        }))
    }
}
