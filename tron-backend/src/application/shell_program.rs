use std::path::PathBuf;

/// Share the framework's native shell selection for command and terminal tools.
pub(crate) fn resolve_user_shell() -> PathBuf {
    eliteforge_agent_official_plugin_tools_local::default_shell_program()
}
