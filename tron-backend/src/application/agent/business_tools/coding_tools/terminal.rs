use super::{json_observation, resolve_command_invocation, resolve_optional_directory};
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
    command: Option<String>,
    #[serde(default)]
    args: Option<Vec<String>>,
    #[serde(default)]
    interactive: bool,
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
                let cwd = resolve_optional_directory(context, input.cwd.as_deref())?
                    .ok_or_else(|| fail("working directory unavailable"))?
                    .canonicalize()
                    .map_err(fail)?;
                let (command, args, program, program_args) = if input.interactive {
                    if input.command.is_some() || input.args.is_some() {
                        return Err(fail(
                            "interactive terminal_start cannot include command or args",
                        ));
                    }
                    let shell = crate::application::shell_program::resolve_user_shell();
                    let command = shell
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("sh")
                        .to_string();
                    (
                        command,
                        Vec::new(),
                        shell.to_string_lossy().into_owned(),
                        Vec::new(),
                    )
                } else {
                    let command = input
                        .command
                        .as_deref()
                        .ok_or_else(|| fail("command is required unless interactive is true"))?;
                    if is_bare_shell_invocation(command, input.args.as_deref()) {
                        return Err(fail("starting an empty shell requires interactive: true"));
                    }
                    let (program, program_args) =
                        resolve_command_invocation(command, input.args.as_deref(), &cwd)?;
                    (
                        command.trim().to_string(),
                        input.args.clone().unwrap_or_default(),
                        program,
                        program_args,
                    )
                };
                serde_json::to_value(self.1.start(TerminalStart {
                    owner,
                    invocation_id: request.invocation_id,
                    command,
                    args,
                    program,
                    program_args,
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

fn is_bare_shell_invocation(command: &str, args: Option<&[String]>) -> bool {
    let words = if let Some(args) = args {
        let mut words = vec![command.to_string()];
        words.extend(args.iter().cloned());
        words
    } else if let Ok(words) = shell_words::split(command) {
        words
    } else {
        return false;
    };
    let Some((program, arguments)) = words.split_first() else {
        return false;
    };
    matches!(
        std::path::Path::new(program)
            .file_name()
            .and_then(|name| name.to_str()),
        Some("bash" | "zsh" | "sh" | "dash" | "ksh" | "fish" | "pwsh" | "powershell")
    ) && !arguments.iter().any(|argument| {
        matches!(argument.as_str(), "--help" | "--version" | "-h" | "-V")
            || argument == "-c"
            || argument == "--command"
            || argument.starts_with("--command=")
            || argument.starts_with("-c")
            || !argument.starts_with('-')
    })
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

#[cfg(test)]
mod tests {
    use super::is_bare_shell_invocation;

    #[test]
    fn empty_shell_commands_require_explicit_interactive_mode() {
        assert!(is_bare_shell_invocation("bash", None));
        assert!(is_bare_shell_invocation("/bin/zsh", Some(&[])));
        assert!(is_bare_shell_invocation(
            "bash",
            Some(&["--noprofile".into(), "--norc".into()])
        ));
        assert!(!is_bare_shell_invocation("bash -c 'npm run dev'", None));
        assert!(!is_bare_shell_invocation("bash --version", None));
        assert!(!is_bare_shell_invocation(
            "/bin/bash",
            Some(&["-c".into(), "npm run dev".into()])
        ));
    }
}
