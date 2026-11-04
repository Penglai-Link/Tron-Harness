use std::{
    fs,
    path::{Path, PathBuf},
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use eliteforge_agent_capability_tools::PtyCommand;
use tempfile::TempDir;

/// Hook script sourced by the shell we spawn and injected into subshells we do
/// not own (a remote shell over SSH, a container shell, ...).
const INTEGRATION_SCRIPT: &str = include_str!("shell_integration.sh");
const SSH_HELPER_SCRIPT: &str = include_str!("ssh_helper.sh");
const SSH_BOOTSTRAP_SCRIPT: &str = include_str!("ssh_bootstrap.sh");
const CONTAINER_HELPER_SCRIPT: &str = include_str!("container_helper.sh");

const SSH_CANDIDATES: [&str; 3] = ["/usr/bin/ssh", "/bin/ssh", "/usr/local/bin/ssh"];
const BASH_EXECUTABLE_CANDIDATES: [&str; 3] = ["/bin/bash", "/usr/bin/bash", "/usr/local/bin/bash"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ShellKind {
    Bash,
    Zsh,
    /// A shell without `precmd`/`preexec` hooks. It still runs, but it cannot
    /// report prompt boundaries, so the client degrades to a plain terminal.
    Posix,
    PowerShell,
    Cmd,
}

impl ShellKind {
    pub(super) fn reports_blocks(self) -> bool {
        matches!(self, Self::Bash | Self::Zsh)
    }

    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Bash => "bash",
            Self::Zsh => "zsh",
            Self::Posix => "sh",
            Self::PowerShell => "powershell",
            Self::Cmd => "cmd",
        }
    }

    fn from_program(program: &Path) -> Self {
        let name = program
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_ascii_lowercase();
        match name.trim_end_matches(".exe") {
            "bash" => Self::Bash,
            "zsh" => Self::Zsh,
            "pwsh" | "powershell" => Self::PowerShell,
            "cmd" => Self::Cmd,
            _ => Self::Posix,
        }
    }
}

pub(super) struct ShellIntegration {
    kind: ShellKind,
    program: PathBuf,
    /// Generated rc files, removed when the session ends.
    directory: Option<TempDir>,
}

impl ShellIntegration {
    /// Resolves the shell to spawn and generates the rc files that load the
    /// hook script. Never fails: if the shell cannot report blocks, or the rc
    /// files cannot be written, the session still gets a working terminal
    /// without semantic markers.
    pub(super) fn prepare() -> Self {
        let program = resolve_shell_program();
        let kind = ShellKind::from_program(&program);

        if !kind.reports_blocks() {
            return Self {
                kind,
                program,
                directory: None,
            };
        }

        match write_shell_files(kind) {
            Ok(directory) => Self {
                kind,
                program,
                directory: Some(directory),
            },
            Err(error) => {
                tracing::warn!(
                    "terminal shell integration files could not be written, \
                     falling back to a terminal without command blocks: {error:?}"
                );
                Self {
                    kind: ShellKind::Posix,
                    program,
                    directory: None,
                }
            }
        }
    }

    pub(super) fn kind(&self) -> ShellKind {
        self.kind
    }

    pub(super) fn command(&self) -> PtyCommand {
        let mut command = PtyCommand::new(&self.program);
        let directory = self.directory.as_ref().map(TempDir::path);
        if !matches!(self.kind, ShellKind::PowerShell | ShellKind::Cmd) {
            command.env("__TRON_SUBSHELL_BOOTSTRAP", subshell_bootstrap());
        }
        if let Some(directory) = directory {
            command.env("__TRON_SSH_INTEGRATION", directory.join("integration.sh"));
            command.env("__TRON_SSH_HELPER", directory.join("ssh-helper.sh"));
            command.env("__TRON_SSH_BOOTSTRAP", directory.join("ssh-bootstrap.sh"));
            command.env(
                "__TRON_CONTAINER_HELPER",
                directory.join("container-helper.sh"),
            );
            if let Some(real_ssh) = resolve_ssh_program() {
                command.env("__TRON_REAL_SSH", real_ssh);
            }
            if let Some(real_bash) = resolve_bash_program() {
                command.env("__TRON_CONTAINER_BASH", real_bash);
            }
            if let Some(real_docker) = resolve_program_in_path("docker") {
                command.env("__TRON_REAL_DOCKER", real_docker);
            }
            if let Some(real_podman) = resolve_program_in_path("podman") {
                command.env("__TRON_REAL_PODMAN", real_podman);
            }
        }

        match (self.kind, directory) {
            (ShellKind::Bash, Some(directory)) => {
                command.arg("--init-file");
                command.arg(directory.join("bashrc"));
                command.arg("-i");
            }
            (ShellKind::Zsh, Some(directory)) => {
                if let Some(original) = std::env::var_os("ZDOTDIR") {
                    command.env("__TRON_ORIGINAL_ZDOTDIR", original);
                }
                command.env("ZDOTDIR", directory);
                command.arg("-i");
            }
            (ShellKind::PowerShell, _) => {
                command.arg("-NoLogo");
            }
            (ShellKind::Cmd, _) => {
                command.arg("/D");
            }
            _ => {
                command.arg("-i");
            }
        }

        command
    }
}

/// A single line that installs the same hooks into a shell this process did not
/// spawn. Interactive `bash`/`zsh` read through readline in non-canonical mode,
/// so the line is not subject to the 4 KiB canonical input limit.
pub(super) fn subshell_bootstrap() -> String {
    let encoded = STANDARD.encode(minified_integration_script());
    decode_and_eval(&encoded)
}

fn decode_and_eval(encoded: &str) -> String {
    // BSD `base64` predates `-d`, GNU `base64` has never had `-D`.
    format!("eval \"$(printf %s '{encoded}' | {{ base64 -d 2>/dev/null || base64 -D; }})\"")
}

/// Drops comments and blank lines so the bootstrap line stays as short as
/// possible. Only whole-line comments are removed, never a trailing `#`.
fn minified_integration_script() -> String {
    INTEGRATION_SCRIPT
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.is_empty() && !trimmed.starts_with('#')
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn resolve_shell_program() -> PathBuf {
    crate::application::shell_program::resolve_user_shell()
}

fn resolve_ssh_program() -> Option<PathBuf> {
    resolve_program_in_path("ssh").or_else(|| {
        SSH_CANDIDATES
            .iter()
            .map(PathBuf::from)
            .find(|candidate| candidate.is_file())
    })
}

fn resolve_bash_program() -> Option<PathBuf> {
    BASH_EXECUTABLE_CANDIDATES
        .iter()
        .map(PathBuf::from)
        .find(|candidate| candidate.is_file())
}

fn resolve_program_in_path(program: &str) -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("PATH") {
        for directory in std::env::split_paths(&path) {
            let candidate = directory.join(program);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

fn write_shell_files(kind: ShellKind) -> std::io::Result<TempDir> {
    let directory = tempfile::Builder::new()
        .prefix("tron-terminal-")
        .tempdir()?;
    let script_path = directory.path().join("integration.sh");
    fs::write(&script_path, INTEGRATION_SCRIPT)?;
    fs::write(directory.path().join("ssh-helper.sh"), SSH_HELPER_SCRIPT)?;
    fs::write(
        directory.path().join("ssh-bootstrap.sh"),
        SSH_BOOTSTRAP_SCRIPT,
    )?;
    fs::write(
        directory.path().join("container-helper.sh"),
        CONTAINER_HELPER_SCRIPT,
    )?;
    let script = shell_single_quote(&script_path.to_string_lossy());

    match kind {
        ShellKind::Bash => {
            // `--init-file` replaces ~/.bashrc, so load the user's own
            // configuration first and install the hooks on top of it.
            fs::write(
                directory.path().join("bashrc"),
                format!(
                    "# Generated by Tron.\n\
                     if [ -f /etc/bash.bashrc ]; then . /etc/bash.bashrc; fi\n\
                     if [ -f \"$HOME/.bashrc\" ]; then . \"$HOME/.bashrc\"; fi\n\
                     . {script}\n"
                ),
            )?;
        }
        ShellKind::Zsh => {
            // ZDOTDIR points here, so both startup files must hand control back
            // to the user's real ones before the hooks are installed.
            fs::write(
                directory.path().join(".zshenv"),
                "# Generated by Tron.\n\
                 if [ -f \"${__TRON_ORIGINAL_ZDOTDIR:-$HOME}/.zshenv\" ]; then\n\
                 \x20 . \"${__TRON_ORIGINAL_ZDOTDIR:-$HOME}/.zshenv\"\n\
                 fi\n",
            )?;
            fs::write(
                directory.path().join(".zshrc"),
                format!(
                    "# Generated by Tron.\n\
                     if [ -n \"${{__TRON_ORIGINAL_ZDOTDIR-}}\" ]; then\n\
                     \x20 ZDOTDIR=\"$__TRON_ORIGINAL_ZDOTDIR\"\n\
                     \x20 unset __TRON_ORIGINAL_ZDOTDIR\n\
                     else\n\
                     \x20 unset ZDOTDIR\n\
                     fi\n\
                     if [ -f \"${{ZDOTDIR:-$HOME}}/.zshrc\" ]; then . \"${{ZDOTDIR:-$HOME}}/.zshrc\"; fi\n\
                     . {script}\n"
                ),
            )?;
        }
        ShellKind::Posix | ShellKind::PowerShell | ShellKind::Cmd => {}
    }

    Ok(directory)
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}
