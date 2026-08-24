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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_markdown_wrapped_command_to_one_line() {
        assert_eq!(
            normalize_terminal_command("```bash\nls\n  -la\n```\n".into()),
            "ls -la"
        );
    }

    #[test]
    fn supplies_the_terminal_user_request_runtime_value() {
        let values = terminal_command_runtime_values("list files");
        assert_eq!(
            values.get(&PromptRuntimeProvider::TerminalUserRequest),
            Some(&Value::String("list files".into())),
        );
    }
}
