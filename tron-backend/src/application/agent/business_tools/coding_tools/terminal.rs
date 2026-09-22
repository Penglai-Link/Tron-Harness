use super::{json_observation, resolve_optional_directory};
use crate::{
    application::agent::{AgentTerminal, TerminalStart},
    domain::agent::{AgentRunContext, Tool, ToolError, ToolExecutionResult, ToolRequest},
};
use serde::Deserialize;
use serde_json::json;
use std::{collections::BTreeMap, sync::Arc};
pub(super) struct TerminalTool(pub &'static str, pub Arc<dyn AgentTerminal>);
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Start {
    command: String,
    #[serde(default)]
    args: Vec<String>,
    cwd: Option<String>,
    #[serde(default)]
    env: BTreeMap<String, String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Read {
    terminal_id: String,
    #[serde(default)]
    cursor: u64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Write {
    terminal_id: String,
    data: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Stop {
    terminal_id: String,
}
fn fail(error: impl ToString) -> ToolError {
    ToolError::ExecutionFailed(error.to_string())
}
impl Tool for TerminalTool {
    fn name(&self) -> &str {
        self.0
    }
    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let owner = crate::application::agent::terminal_owner(
            &context.workspace_code,
            context.user_code.as_deref(),
            context
                .conversation_code
                .as_deref()
                .unwrap_or(&context.run_code),
        );
        let value = match self.0 {
            "terminal_start" => {
                let input: Start = serde_json::from_value(request.input).map_err(fail)?;
                if input.command.trim().is_empty() {
                    return Err(fail("command is required"));
                }
                let cwd = resolve_optional_directory(context, input.cwd.as_deref())?
                    .ok_or_else(|| fail("working directory unavailable"))?
                    .canonicalize()
                    .map_err(fail)?;
                serde_json::to_value(self.1.start(TerminalStart {
                    owner,
                    command: input.command,
                    args: input.args,
                    cwd,
                    env: input.env,
                })?)
                .map_err(fail)?
            }
            "terminal_read" => {
                let input: Read = serde_json::from_value(request.input).map_err(fail)?;
                serde_json::to_value(self.1.read(&owner, &input.terminal_id, input.cursor)?)
                    .map_err(fail)?
            }
            "terminal_write" => {
                let input: Write = serde_json::from_value(request.input).map_err(fail)?;
                self.1.write(&owner, &input.terminal_id, &input.data)?;
                json!({"terminalId":input.terminal_id,"written":true})
            }
            "terminal_stop" => {
                let input: Stop = serde_json::from_value(request.input).map_err(fail)?;
                self.1.stop(&owner, &input.terminal_id)?;
                json!({"terminalId":input.terminal_id,"stopped":true})
            }
            "terminal_list" => {
                if request.input != json!({}) {
                    return Err(fail("terminal_list takes no parameters"));
                }
                serde_json::to_value(self.1.list(&owner)?).map_err(fail)?
            }
            _ => return Err(fail("unknown terminal operation")),
        };
        json_observation(self.0, &value)
    }
}
pub(super) fn tools(terminal: Arc<dyn AgentTerminal>) -> Vec<Arc<dyn Tool>> {
    [
        "terminal_start",
        "terminal_read",
        "terminal_write",
        "terminal_stop",
        "terminal_list",
    ]
    .into_iter()
    .map(|name| Arc::new(TerminalTool(name, terminal.clone())) as Arc<dyn Tool>)
    .collect()
}
