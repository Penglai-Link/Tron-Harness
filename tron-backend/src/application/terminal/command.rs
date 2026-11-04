use std::collections::HashMap;

use serde_json::Value;

use crate::domain::prompt::PromptRuntimeProvider;

pub fn terminal_command_runtime_values(
    user_request: &str,
) -> HashMap<PromptRuntimeProvider, Value> {
    HashMap::from([(
        PromptRuntimeProvider::TerminalUserRequest,
        Value::String(user_request.to_string()),
    )])
}

pub fn normalize_terminal_command(output: String) -> String {
    output
        .lines()
        .filter(|line| !line.trim_start().starts_with("```"))
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}
