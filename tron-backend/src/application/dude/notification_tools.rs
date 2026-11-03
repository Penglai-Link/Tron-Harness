use super::{DudeService, work_changes};
use crate::domain::agent::{
    AgentRunContext, Tool, ToolError, ToolExecutionResult, ToolObservation, ToolRegistry,
    ToolRequest,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

pub(super) fn register(registry: &ToolRegistry, dudes: DudeService) -> Result<(), ToolError> {
    for detail in [false, true] {
        registry.register(Arc::new(NotificationRead {
            dudes: dudes.clone(),
            detail,
        }))?;
    }
    Ok(())
}
struct NotificationRead {
    dudes: DudeService,
    detail: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PageInput {
    current: Option<usize>,
    size: Option<usize>,
    #[serde(default)]
    include_archived: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GetInput {
    notification_id: String,
}
impl Tool for NotificationRead {
    fn name(&self) -> &str {
        if self.detail {
            "dude_notification_get"
        } else {
            "dude_notification_page"
        }
    }
    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let invalid = |e: String| ToolError::ExecutionFailed(e);
        let dude = work_changes::owner(context)?;
        let user = context
            .user_code
            .as_deref()
            .filter(|user| !user.trim().is_empty())
            .ok_or_else(|| invalid("Notification reads require a bound user".into()))?;
        let (id, current, size, archived) = if self.detail {
            let input: GetInput =
                serde_json::from_value(request.input).map_err(|e| invalid(e.to_string()))?;
            if input.notification_id.trim().is_empty() {
                return Err(invalid("notificationId is required".into()));
            }
            (Some(input.notification_id), 1, 1, true)
        } else {
            let input: PageInput =
                serde_json::from_value(request.input).map_err(|e| invalid(e.to_string()))?;
            let (current, size) = (input.current.unwrap_or(1), input.size.unwrap_or(20));
            if current == 0 || current > 1_000_000 || !(1..=100).contains(&size) {
                return Err(invalid(
                    "current must be 1..1000000; size must be 1..100".into(),
                ));
            }
            (None, current, size, input.include_archived)
        };
        let (total, records) = self
            .dudes
            .repo()
            .map_err(|e| invalid(e.to_string()))?
            .scoped_notifications(dude, user, id.as_deref(), current, size, archived)
            .map_err(|e| invalid(e.to_string()))?;
        let records = records
            .into_iter()
            .map(|notice| {
                let mut value = serde_json::to_value(notice).expect("notification serializes");
                // Persistence is known. No browser/push acknowledgement is collected.
                value["delivery"] = json!({"persisted":true,"received":null,"pushed":null});
                value
            })
            .collect::<Vec<_>>();
        let value = if self.detail {
            records
                .into_iter()
                .next()
                .ok_or_else(|| invalid("Notification not found or not accessible".into()))?
        } else {
            json!({"current":current,"size":size,"total":total,"records":records})
        };
        Ok(ToolExecutionResult::observation(ToolObservation {
            tool_name: self.name().into(),
            output: value.to_string(),
        }))
    }
}
