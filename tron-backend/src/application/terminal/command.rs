pub fn build_terminal_command_generation_prompt(prompt: String) -> String {
    crate::domain::prompt::builtin_prompt_text("terminal.command-exact", "en-US")
        .expect("terminal command prompt must be bundled")
        .replace("{{user_request}}", &prompt)
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
    fn instruction_requires_exactly_one_shell_command() {
        let prompt = build_terminal_command_generation_prompt("list files".into());
        assert!(prompt.contains("exactly one executable shell command"));
        assert!(prompt.ends_with("User request:\nlist files"));
    }
}
