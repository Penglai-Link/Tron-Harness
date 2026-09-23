use crate::domain::agent::ToolError;
use serde::Serialize;
use std::{collections::BTreeMap, path::PathBuf};

pub struct TerminalStart {
    pub owner: String,
    pub invocation_id: Option<String>,
    pub command: String,
    pub args: Vec<String>,
    pub program: String,
    pub program_args: Vec<String>,
    pub cwd: PathBuf,
    pub env: BTreeMap<String, String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalSnapshot {
    pub terminal_id: String,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub running: bool,
    pub exit_code: Option<u32>,
    pub output: String,
    pub next_cursor: u64,
    pub truncated: bool,
    pub output_closed: bool,
}
/// Backend-owned execution sessions, independent of frontend socket lifetime.
pub trait AgentTerminal: Send + Sync {
    fn start(&self, request: TerminalStart) -> Result<TerminalSnapshot, ToolError>;
    fn read(&self, owner: &str, id: &str, cursor: u64) -> Result<TerminalSnapshot, ToolError>;
    fn write(&self, owner: &str, id: &str, data: &str) -> Result<(), ToolError>;
    fn stop(&self, owner: &str, id: &str) -> Result<(), ToolError>;
    fn list(&self, owner: &str) -> Result<Vec<TerminalSnapshot>, ToolError>;
}

pub fn terminal_owner(workspace: &str, user: Option<&str>, conversation: &str) -> String {
    serde_json::to_string(&(workspace, user, conversation))
        .expect("terminal owner fields serialize")
}
