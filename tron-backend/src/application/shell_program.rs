use std::path::PathBuf;

/// Resolve the user's preferred local shell using the same policy for
/// interactive and Agent command execution.
pub(crate) fn resolve_user_shell() -> PathBuf {
    let configured = std::env::var_os("SHELL").map(PathBuf::from);
    if let Some(configured) = configured
        && matches!(
            configured.file_name().and_then(|name| name.to_str()),
            Some("bash" | "zsh")
        )
        && configured.is_file()
    {
        return configured;
    }

    [
        "/bin/zsh",
        "/usr/bin/zsh",
        "/bin/bash",
        "/usr/bin/bash",
        "/usr/local/bin/bash",
        "/bin/sh",
    ]
    .iter()
    .map(PathBuf::from)
    .find(|candidate| candidate.is_file())
    .unwrap_or_else(|| PathBuf::from("/bin/sh"))
}
