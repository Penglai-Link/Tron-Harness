//! Product port mapping only; the official capability owns terminal execution.
use crate::{
    application::agent::{AgentTerminal, TerminalSnapshot, TerminalStart},
    domain::agent::ToolError,
};
use eliteforge_agent_capability_tools::{PtyService, PtySnapshot, PtyStart};

#[derive(Default)]
pub struct LocalAgentTerminal(PtyService);
impl LocalAgentTerminal {
    pub fn new(service: PtyService) -> Self {
        Self(service)
    }
}
fn fail(error: impl ToString) -> ToolError {
    ToolError::ExecutionFailed(error.to_string())
}
fn snapshot(value: PtySnapshot) -> TerminalSnapshot {
    TerminalSnapshot {
        terminal_id: value.terminal_id,
        command: value.command,
        args: value.args,
        cwd: value.cwd,
        running: value.running,
        exit_code: value.exit_code,
        output: value.output,
        next_cursor: value.next_cursor,
        truncated: value.truncated,
        output_closed: value.output_closed,
    }
}
impl AgentTerminal for LocalAgentTerminal {
    fn start(&self, request: TerminalStart) -> Result<TerminalSnapshot, ToolError> {
        self.0
            .start(PtyStart {
                owner: request.owner,
                invocation_id: request.invocation_id,
                command: request.command,
                args: request.args,
                program: request.program,
                program_args: request.program_args,
                cwd: request.cwd,
                env: request.env,
            })
            .map(snapshot)
            .map_err(fail)
    }
    fn read(&self, owner: &str, id: &str, cursor: u64) -> Result<TerminalSnapshot, ToolError> {
        self.0.read(owner, id, cursor).map(snapshot).map_err(fail)
    }
    fn write(&self, owner: &str, id: &str, data: &str) -> Result<(), ToolError> {
        self.0.write(owner, id, data).map_err(fail)
    }
    fn stop(&self, owner: &str, id: &str) -> Result<(), ToolError> {
        self.0.stop(owner, id).map_err(fail)
    }
    fn list(&self, owner: &str) -> Result<Vec<TerminalSnapshot>, ToolError> {
        self.0
            .list(owner)
            .map(|values| values.into_iter().map(snapshot).collect())
            .map_err(fail)
    }
}
