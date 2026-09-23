use std::{
    fs,
    path::{Path, PathBuf},
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use portable_pty::CommandBuilder;
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
}

impl ShellKind {
    pub(super) fn reports_blocks(self) -> bool {
        !matches!(self, Self::Posix)
    }

    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Bash => "bash",
            Self::Zsh => "zsh",
            Self::Posix => "sh",
        }
    }

    fn from_program(program: &Path) -> Self {
        match program.file_name().and_then(|name| name.to_str()) {
            Some("bash") => Self::Bash,
            Some("zsh") => Self::Zsh,
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
                eprintln!(
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

    pub(super) fn command(&self) -> CommandBuilder {
        let mut command = CommandBuilder::new(&self.program);
        let directory = self.directory.as_ref().map(TempDir::path);
        command.env("__TRON_SUBSHELL_BOOTSTRAP", subshell_bootstrap());
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
        ShellKind::Posix => {}
    }

    Ok(directory)
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

#[cfg(all(test, unix))]
#[path = "shell_integration_history_tests.rs"]
mod shell_integration_history_tests;

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;

    #[test]
    fn integration_script_reports_every_semantic_marker() {
        for marker in [
            "133;A",
            "133;B",
            "133;C",
            "133;D;",
            "633;E;",
            "7;file://",
            "?2026l",
        ] {
            assert!(
                INTEGRATION_SCRIPT.contains(marker),
                "integration script should report {marker}"
            );
        }
    }

    #[test]
    fn integration_script_branches_on_the_shell_flavour() {
        assert!(INTEGRATION_SCRIPT.contains("ZSH_VERSION"));
        assert!(INTEGRATION_SCRIPT.contains("BASH_VERSION"));
    }

    #[test]
    fn integration_script_selects_an_installed_utf8_locale() {
        assert!(INTEGRATION_SCRIPT.contains("unset LC_ALL"));
        assert!(INTEGRATION_SCRIPT.contains("export LANG=C"));
        assert!(INTEGRATION_SCRIPT.contains("LC_ALL=C locale -a"));
        assert!(INTEGRATION_SCRIPT.contains("export LC_CTYPE="));
        assert!(!INTEGRATION_SCRIPT.contains("export LC_ALL="));
    }

    #[test]
    fn integration_script_installs_the_local_ssh_helper() {
        assert!(INTEGRATION_SCRIPT.contains("unalias ssh"));
        assert!(INTEGRATION_SCRIPT.contains("ssh()"));
        assert!(INTEGRATION_SCRIPT.contains("$__TRON_SSH_HELPER"));
    }

    #[test]
    fn integration_script_installs_container_helpers_without_prompt_injection() {
        assert!(INTEGRATION_SCRIPT.contains("__TRON_CONTAINER_HELPER"));
        assert!(INTEGRATION_SCRIPT.contains("docker()"));
        assert!(INTEGRATION_SCRIPT.contains("podman()"));
        assert!(CONTAINER_HELPER_SCRIPT.contains("__TRON_REMOTE_SHELL"));
        assert!(CONTAINER_HELPER_SCRIPT.contains("/bin/sh"));
    }

    #[test]
    fn bash_prompt_hook_returns_the_saved_exit_status() {
        assert!(INTEGRATION_SCRIPT.contains("return $__tron_status"));
        assert!(!INTEGRATION_SCRIPT.contains("return $status"));
    }

    #[test]
    fn minified_script_drops_comments_but_keeps_every_statement() {
        let minified = minified_integration_script();

        assert!(!minified.contains("# Tron terminal shell integration."));
        assert!(minified.contains("__tron_precmd"));
        assert!(minified.contains("__tron_preexec"));
        assert!(minified.len() < INTEGRATION_SCRIPT.len());
        assert!(!minified.lines().any(|line| line.trim().is_empty()));
    }

    #[test]
    fn subshell_bootstrap_is_a_single_decodable_line() {
        let bootstrap = subshell_bootstrap();

        assert_eq!(bootstrap.lines().count(), 1);
        assert!(bootstrap.starts_with("eval \"$("));

        let encoded = bootstrap
            .split_once('\'')
            .and_then(|(_, rest)| rest.split_once('\''))
            .map(|(encoded, _)| encoded)
            .expect("bootstrap should carry a single-quoted payload");
        let decoded = STANDARD
            .decode(encoded)
            .expect("bootstrap payload should be valid base64");

        assert_eq!(
            String::from_utf8(decoded).expect("payload should be UTF-8"),
            minified_integration_script()
        );
    }

    #[test]
    fn ssh_bootstrap_restores_the_recursive_ssh_files_on_the_remote_host() {
        for variable in [
            "__TRON_INTEGRATION_BASE64",
            "__TRON_SSH_HELPER_BASE64",
            "__TRON_SSH_BOOTSTRAP_BASE64",
        ] {
            assert!(
                SSH_BOOTSTRAP_SCRIPT.contains(variable),
                "bootstrap should restore {variable}"
            );
        }
        assert!(SSH_BOOTSTRAP_SCRIPT.contains("ssh-helper.sh"));
        assert!(SSH_BOOTSTRAP_SCRIPT.contains("ssh-bootstrap.sh"));
        assert!(SSH_BOOTSTRAP_SCRIPT.contains("container-helper.sh"));
        assert!(SSH_BOOTSTRAP_SCRIPT.contains("--init-file"));
        assert!(SSH_BOOTSTRAP_SCRIPT.contains("ZDOTDIR"));
    }

    #[test]
    fn shell_kind_is_derived_from_the_program_name() {
        assert_eq!(
            ShellKind::from_program(Path::new("/bin/bash")),
            ShellKind::Bash
        );
        assert_eq!(
            ShellKind::from_program(Path::new("/opt/homebrew/bin/zsh")),
            ShellKind::Zsh
        );
        assert_eq!(
            ShellKind::from_program(Path::new("/bin/sh")),
            ShellKind::Posix
        );
        assert!(!ShellKind::Posix.reports_blocks());
        assert!(ShellKind::Bash.reports_blocks());
    }

    #[test]
    fn generated_bash_rc_loads_user_configuration_before_the_hooks() {
        let directory = write_shell_files(ShellKind::Bash).expect("rc files should be written");
        let bashrc =
            fs::read_to_string(directory.path().join("bashrc")).expect("bashrc should exist");

        let user_rc = bashrc.find(".bashrc").expect("user rc should be sourced");
        let hooks = bashrc
            .find("integration.sh")
            .expect("hooks should be sourced");
        assert!(user_rc < hooks, "user configuration must load first");
        assert!(directory.path().join("ssh-helper.sh").is_file());
        assert!(directory.path().join("ssh-bootstrap.sh").is_file());
        assert!(directory.path().join("container-helper.sh").is_file());
    }

    #[test]
    fn generated_zsh_rc_restores_the_original_zdotdir() {
        let directory = write_shell_files(ShellKind::Zsh).expect("rc files should be written");
        let zshrc =
            fs::read_to_string(directory.path().join(".zshrc")).expect(".zshrc should exist");

        assert!(zshrc.contains("__TRON_ORIGINAL_ZDOTDIR"));
        assert!(zshrc.contains("integration.sh"));
        assert!(directory.path().join(".zshenv").is_file());
    }

    #[cfg(unix)]
    #[test]
    fn ssh_helper_injects_only_interactive_sessions_and_clears_locale() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempfile::tempdir().expect("temp directory should be created");
        let fake_ssh = directory.path().join("fake-ssh");
        fs::write(
            &fake_ssh,
            "#!/bin/sh\n\
             for argument do printf 'ARG=%s\\n' \"$argument\"; done\n\
             printf 'LANG=%s\\n' \"${LANG-unset}\"\n\
             printf 'LC_ALL=%s\\n' \"${LC_ALL-unset}\"\n\
             exit \"${TRON_TEST_EXIT:-0}\"\n",
        )
        .expect("fake SSH should be written");
        let mut permissions = fs::metadata(&fake_ssh)
            .expect("fake SSH metadata should exist")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&fake_ssh, permissions).expect("fake SSH should be executable");

        fs::write(directory.path().join("ssh-helper.sh"), SSH_HELPER_SCRIPT)
            .expect("helper should be written");
        fs::write(directory.path().join("integration.sh"), INTEGRATION_SCRIPT)
            .expect("integration should be written");
        fs::write(
            directory.path().join("ssh-bootstrap.sh"),
            SSH_BOOTSTRAP_SCRIPT,
        )
        .expect("bootstrap should be written");
        fs::write(
            directory.path().join("container-helper.sh"),
            CONTAINER_HELPER_SCRIPT,
        )
        .expect("container helper should be written");

        let run = |arguments: &[&str]| {
            Command::new("/bin/sh")
                .arg(directory.path().join("ssh-helper.sh"))
                .args(arguments)
                .env("__TRON_REAL_SSH", &fake_ssh)
                .env(
                    "__TRON_SSH_INTEGRATION",
                    directory.path().join("integration.sh"),
                )
                .env("__TRON_SSH_HELPER", directory.path().join("ssh-helper.sh"))
                .env(
                    "__TRON_SSH_BOOTSTRAP",
                    directory.path().join("ssh-bootstrap.sh"),
                )
                .env(
                    "__TRON_CONTAINER_HELPER",
                    directory.path().join("container-helper.sh"),
                )
                .env("LANG", "invalid.UTF-8")
                .env("LC_ALL", "invalid.UTF-8")
                .output()
                .expect("helper should run")
        };

        let interactive = run(&["-p", "2222", "build@host"]);
        assert!(interactive.status.success());
        let stdout = String::from_utf8(interactive.stdout).expect("output should be UTF-8");
        assert!(stdout.contains("ARG=-t\nARG=-p\nARG=2222\nARG=build@host\n"));
        assert!(stdout.contains("ARG=__TRON_INTEGRATION_BASE64="));
        assert!(stdout.contains("__TRON_SSH_HELPER_BASE64="));
        assert!(stdout.contains("__TRON_SSH_BOOTSTRAP_BASE64="));
        assert!(stdout.contains("__TRON_CONTAINER_HELPER_BASE64="));
        assert!(stdout.contains("LANG=unset\nLC_ALL=unset\n"));

        for arguments in [
            &["host", "uptime"][..],
            &["-T", "host"][..],
            &["-N", "host"][..],
            &["-W", "target:22", "host"][..],
        ] {
            let bypassed = run(arguments);
            let stdout = String::from_utf8(bypassed.stdout).expect("output should be UTF-8");
            assert!(!stdout.contains("ARG=remote bootstrap\n"));
        }
    }

    #[cfg(unix)]
    #[test]
    fn ssh_bootstrap_reinstalls_the_ssh_wrapper_for_the_next_hop() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempfile::tempdir().expect("temp directory should be created");
        let fake_bash = directory.path().join("bash");
        fs::write(
            &fake_bash,
            "#!/bin/sh\n\
             exec /bin/bash --noprofile --norc -i -c '\n\
             . \"$1\"\n\
             test -r \"$__TRON_SSH_INTEGRATION\" &&\n\
             test -r \"$__TRON_SSH_HELPER\" &&\n\
             test -r \"$__TRON_SSH_BOOTSTRAP\" &&\n\
             test \"$(type -t ssh)\" = function\n\
             ' bash \"$2\"\n",
        )
        .expect("fake Bash should be written");
        let mut permissions = fs::metadata(&fake_bash)
            .expect("fake Bash metadata should exist")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&fake_bash, permissions).expect("fake Bash should be executable");

        let launcher = decode_and_eval(&STANDARD.encode(SSH_BOOTSTRAP_SCRIPT));
        let output = Command::new("/bin/sh")
            .arg("-c")
            .arg(launcher)
            .env("HOME", directory.path())
            .env("SHELL", &fake_bash)
            .env("TERM", "dumb")
            .env(
                "__TRON_INTEGRATION_BASE64",
                STANDARD.encode(INTEGRATION_SCRIPT),
            )
            .env(
                "__TRON_SSH_HELPER_BASE64",
                STANDARD.encode(SSH_HELPER_SCRIPT),
            )
            .env(
                "__TRON_SSH_BOOTSTRAP_BASE64",
                STANDARD.encode(SSH_BOOTSTRAP_SCRIPT),
            )
            .env(
                "__TRON_CONTAINER_HELPER_BASE64",
                STANDARD.encode(CONTAINER_HELPER_SCRIPT),
            )
            .output()
            .expect("remote bootstrap should run");

        assert!(
            output.status.success(),
            "remote bootstrap failed; stdout: {}; stderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
