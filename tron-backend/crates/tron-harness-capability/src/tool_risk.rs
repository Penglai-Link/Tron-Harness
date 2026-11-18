//! Risk classification uses trusted tool identity and actual parameters, never model hints.
//!
//! Classification is language independent. [`assess_tool_risk`] returns a
//! structured [`RiskReason`] that the approval/display layer translates with
//! the run locale; logs, approvals and the frontend can reuse the same code
//! without re-deriving the risk.
use eliteforge_agent_core::ToolDescriptor;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ToolRisk {
    Safe,
    #[default]
    Low,
    Medium,
    High,
    Critical,
}

/// Why a command invocation was classified at its risk level.
///
/// This mirrors [`assess_command`] and deliberately carries no localized text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CommandRiskReason {
    /// Environment overrides, an interactive TTY or standard input.
    EnvironmentOrInteractiveInput,
    /// No statically inspectable command string was provided.
    UninspectableCommand,
    /// `args` is present but is not a literal string array.
    UnparsableArguments,
    /// Shell control, expansion or syntax outside the static parser.
    ShellSyntax,
    /// The command mutates data inside a protected directory.
    ProtectedDirectoryMutation,
    /// The command runs an interpreter or dynamic executor.
    InterpreterExecution,
    /// The command deletes files or directories.
    FileDeletion,
    /// The command accesses the network, transfers data or runs remotely.
    NetworkAccess,
    /// Input is written to an already running command or terminal.
    StatefulProcessInput,
    /// The command stops or otherwise controls a running process.
    ProcessControl,
    /// The executable and arguments matched the configured execution rules.
    RiskRuleMatch,
}

impl CommandRiskReason {
    /// Stable, language-independent reason code.
    pub fn code(self) -> &'static str {
        match self {
            Self::EnvironmentOrInteractiveInput => "environmentOrInteractiveInput",
            Self::UninspectableCommand => "uninspectableCommand",
            Self::UnparsableArguments => "unparsableArguments",
            Self::ShellSyntax => "shellSyntax",
            Self::ProtectedDirectoryMutation => "protectedDirectoryMutation",
            Self::InterpreterExecution => "interpreterExecution",
            Self::FileDeletion => "fileDeletion",
            Self::NetworkAccess => "networkAccess",
            Self::StatefulProcessInput => "statefulProcessInput",
            Self::ProcessControl => "processControl",
            Self::RiskRuleMatch => "riskRuleMatch",
        }
    }

    /// Translation key owned by the display layer.
    pub fn message_key(self) -> &'static str {
        match self {
            Self::EnvironmentOrInteractiveInput => "tool.risk.command.environment_or_interactive_input",
            Self::UninspectableCommand => "tool.risk.command.uninspectable",
            Self::UnparsableArguments => "tool.risk.command.unparsable_arguments",
            Self::ShellSyntax => "tool.risk.command.shell_syntax",
            Self::ProtectedDirectoryMutation => "tool.risk.command.protected_directory",
            Self::InterpreterExecution => "tool.risk.command.interpreter",
            Self::FileDeletion => "tool.risk.command.file_deletion",
            Self::NetworkAccess => "tool.risk.command.network",
            Self::StatefulProcessInput => "tool.risk.command.stateful_input",
            Self::ProcessControl => "tool.risk.command.process_control",
            Self::RiskRuleMatch => "tool.risk.command.rule_match",
        }
    }
}

/// Structured, language-independent reason for a tool risk classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RiskReason {
    /// A remote MCP tool whose side effects the host cannot verify.
    RemoteMcp,
    /// A remote image generation task that may incur charges.
    ImageGeneration,
    /// Local file reads or writes, assessed from the actual paths.
    LocalFileAccess,
    /// A persistent terminal session that can run arbitrary commands.
    PersistentTerminal,
    /// The tool changes business data or runtime state.
    BusinessMutation,
    /// The tool deletes business data or runtime state.
    BusinessDeletion,
    /// A query tool whose arguments contain executable or batch operations.
    QueryExecutableArguments,
    /// An unknown native tool whose side effects the host cannot verify.
    UnknownNativeTool,
    /// Nested tool invocations inside the arguments increase the risk.
    NestedToolInvocation,
    /// A call to a remote platform service.
    RemoteServiceCall,
    /// A call that accesses the external network.
    WebAccess,
    /// A change to remote access configuration, pairing or tokens.
    RemoteAccess,
    /// A command invocation with a specific command-level reason.
    Command(CommandRiskReason),
}

impl RiskReason {
    /// Stable, language-independent reason code for logs and structured output.
    pub fn code(self) -> &'static str {
        match self {
            Self::RemoteMcp => "remoteMcp",
            Self::ImageGeneration => "imageGeneration",
            Self::LocalFileAccess => "localFileAccess",
            Self::PersistentTerminal => "persistentTerminal",
            Self::BusinessMutation => "businessMutation",
            Self::BusinessDeletion => "businessDeletion",
            Self::QueryExecutableArguments => "queryExecutableArguments",
            Self::UnknownNativeTool => "unknownNativeTool",
            Self::NestedToolInvocation => "nestedToolInvocation",
            Self::RemoteServiceCall => "remoteServiceCall",
            Self::WebAccess => "webAccess",
            Self::RemoteAccess => "remoteAccess",
            Self::Command(reason) => reason.code(),
        }
    }

    /// Translation key owned by the display layer.
    pub fn message_key(self) -> &'static str {
        match self {
            Self::RemoteMcp => "tool.risk.remote_mcp",
            Self::ImageGeneration => "tool.risk.image_generation",
            Self::LocalFileAccess => "tool.risk.local_file_access",
            Self::PersistentTerminal => "tool.risk.persistent_terminal",
            Self::BusinessMutation => "tool.risk.business_mutation",
            Self::BusinessDeletion => "tool.risk.business_deletion",
            Self::QueryExecutableArguments => "tool.risk.query_executable_arguments",
            Self::UnknownNativeTool => "tool.risk.unknown_native_tool",
            Self::NestedToolInvocation => "tool.risk.nested_tool_invocation",
            Self::RemoteServiceCall => "tool.risk.remote_service_call",
            Self::WebAccess => "tool.risk.web_access",
            Self::RemoteAccess => "tool.risk.remote_access",
            Self::Command(reason) => reason.message_key(),
        }
    }
}

/// Risk level plus the structured reason for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolRiskAssessment {
    pub risk: ToolRisk,
    pub reason: RiskReason,
}

/// `operation` is host catalog metadata: false = query, true = mutation.
/// Missing metadata is intentionally conservative; MCP descriptions/hints are untrusted.
pub fn assess_tool_risk(
    descriptor: &ToolDescriptor,
    args: &Value,
    operation: Option<bool>,
) -> ToolRiskAssessment {
    use CommandRiskReason as Command;
    use RiskReason as Reason;

    let id = descriptor.id.as_str();
    if id.starts_with("mcp/")
        || id.starts_with("mcp.")
        || id.starts_with("mcp:")
        || descriptor.call_name.starts_with("mcp__")
    {
        return assessment(ToolRisk::High, Reason::RemoteMcp);
    }
    if matches!(
        descriptor.call_name.as_str(),
        "terminal_start" | "terminal_write"
    ) {
        return assessment(ToolRisk::High, Reason::PersistentTerminal);
    }
    if matches!(
        descriptor.call_name.as_str(),
        "terminal_read" | "terminal_list"
    ) {
        return assessment(ToolRisk::Safe, Reason::LocalFileAccess);
    }
    match id {
        "system.command.exec" | "framework.terminal.start" => command_assessment(args),
        // Input may be interpreted by an already running shell. Its state is
        // unavailable here, so a Critical threshold must still ask for approval.
        "system.command.task.write" | "framework.terminal.write" => assessment(
            ToolRisk::Critical,
            Reason::Command(Command::StatefulProcessInput),
        ),
        "system.command.task.stop" | "framework.terminal.stop" => assessment(
            ToolRisk::High,
            Reason::Command(Command::ProcessControl),
        ),
        "system.command.task.read"
        | "system.command.list"
        | "framework.terminal.read"
        | "framework.terminal.list" => assessment(ToolRisk::Safe, Reason::LocalFileAccess),
        "filesystem.read_text"
        | "filesystem.list"
        | "filesystem.stat"
        | "filesystem.search.grep"
        | "filesystem.search.glob"
        | "filesystem.search.path"
        | "media.read_image"
        | "media.read_pdf" => assessment(ToolRisk::Low, Reason::LocalFileAccess),
        "filesystem.write_text" => {
            assessment(write_risk(args), Reason::LocalFileAccess)
        }
        "filesystem.apply_patch" | "filesystem.apply_unified_patch" => {
            assessment(patch_risk(args), Reason::LocalFileAccess)
        }
        "personai.search" | "personai.query_image" | "personai.detect_text"
        | "personai.detect_image" => assessment(ToolRisk::Low, Reason::RemoteServiceCall),
        "web.fetch" => assessment(ToolRisk::Low, Reason::WebAccess),
        "personai.generate_image" => assessment(ToolRisk::Medium, Reason::ImageGeneration),
        // Remote access changes pairing state and tokens; require the same
        // approval threshold as other host mutations.
        "framework.remote-access.manage" => assessment(ToolRisk::High, Reason::RemoteAccess),
        _ => match operation {
            Some(false) => {
                let nested = nested_risk(args, 0);
                let risk = nested.max(ToolRisk::Low);
                let reason = if nested > ToolRisk::Low {
                    Reason::NestedToolInvocation
                } else {
                    Reason::QueryExecutableArguments
                };
                assessment(risk, reason)
            }
            Some(true) => {
                let deleting = descriptor
                    .call_name
                    .split('_')
                    .any(|part| matches!(part, "delete" | "remove" | "destroy" | "drop"));
                let base = if deleting {
                    ToolRisk::High
                } else {
                    ToolRisk::Medium
                };
                let nested = nested_risk(args, 0);
                let reason = if nested > base {
                    Reason::NestedToolInvocation
                } else if deleting {
                    Reason::BusinessDeletion
                } else {
                    Reason::BusinessMutation
                };
                assessment(nested.max(base), reason)
            }
            None => assessment(
                nested_risk(args, 0).max(ToolRisk::High),
                Reason::UnknownNativeTool,
            ),
        },
    }
}

/// Compatibility wrapper: callers that only need the level use this.
pub fn identify_tool_risk(
    descriptor: &ToolDescriptor,
    args: &Value,
    operation: Option<bool>,
) -> ToolRisk {
    assess_tool_risk(descriptor, args, operation).risk
}

fn assessment(risk: ToolRisk, reason: RiskReason) -> ToolRiskAssessment {
    ToolRiskAssessment { risk, reason }
}

// A wrapper cannot hide a dangerous command behind query metadata. Nested tool
// declarations may escalate risk but can never grant a lower risk classification.
fn nested_risk(value: &Value, depth: usize) -> ToolRisk {
    if depth > 16 {
        return ToolRisk::High;
    }
    let mut risk = ToolRisk::Safe;
    for key in ["sql", "statement", "statements"] {
        if let Some(sql) = value.get(key) {
            if !sql.is_null() {
                risk = risk.max(database_risk(&[sql.to_string()]));
            }
        }
    }
    if let Some(object) = value.as_object() {
        for child in object.values().filter(|child| child.is_object()) {
            risk = risk.max(nested_risk(child, depth + 1));
        }
    }
    if value.get("command").is_some() {
        risk = command_risk(value);
    }
    for key in ["tool_calls", "tool_uses", "calls", "actions", "operations"] {
        if let Some(items) = value.get(key).and_then(Value::as_array) {
            risk = risk.max(ToolRisk::High);
            for item in items {
                risk = risk.max(nested_risk(item, depth + 1));
                for key in ["arguments", "args", "parameters"] {
                    if let Some(child) = item.get(key) {
                        risk = risk.max(nested_risk(child, depth + 1));
                    }
                }
            }
        }
    }
    risk
}

fn protected_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    normalized.split('/').any(|part| part == "..")
        || [
            "/etc", "/System", "/usr", "/bin", "/sbin", "/boot", "/proc", "/sys", "/dev",
        ]
        .iter()
        .any(|root| normalized == *root || normalized.starts_with(&format!("{root}/")))
        || normalized
            .split('/')
            .any(|part| matches!(part, ".ssh" | ".gnupg" | ".git"))
        || normalized == "/"
}

fn write_risk(args: &Value) -> ToolRisk {
    match args.get("path").and_then(Value::as_str) {
        Some(path) if protected_path(path) => ToolRisk::Critical,
        Some(_) => ToolRisk::Medium,
        None => ToolRisk::High,
    }
}

fn patch_risk(args: &Value) -> ToolRisk {
    // A dry-run still reads files but cannot write. It is a native implementation flag.
    if args.get("dryRun").and_then(Value::as_bool) == Some(true)
        && args.get("patch").is_some_and(Value::is_string)
    {
        return ToolRisk::Low;
    }
    if args
        .get("cwd")
        .and_then(Value::as_str)
        .is_some_and(protected_path)
    {
        return ToolRisk::Critical;
    }
    if let Some(patch) = args.get("patch").and_then(Value::as_str) {
        let mut risk = ToolRisk::Medium;
        let mut paths = 0;
        for line in patch.lines() {
            for prefix in [
                "*** Add File: ",
                "*** Update File: ",
                "*** Delete File: ",
                "*** Move to: ",
                "+++ ",
                "--- ",
            ] {
                if let Some(path) = line.strip_prefix(prefix) {
                    paths += 1;
                    if protected_path(path) {
                        return ToolRisk::Critical;
                    }
                    if prefix == "*** Delete File: " || path == "/dev/null" {
                        risk = ToolRisk::High;
                    }
                }
            }
        }
        return if paths > 20 { ToolRisk::Critical } else { risk };
    }
    let risk = write_risk(args);
    if args.get("action").and_then(Value::as_str) == Some("delete") {
        risk.max(ToolRisk::High)
    } else {
        risk
    }
}

fn command_risk(input: &Value) -> ToolRisk {
    command_assessment(input).risk
}

/// Classify one command invocation and explain the classification in the same
/// interpretation, so approval text can never drift from the risk level.
pub fn command_assessment(input: &Value) -> ToolRiskAssessment {
    use CommandRiskReason as Command;
    use RiskReason as Reason;

    if input
        .get("env")
        .is_some_and(|v| v.as_object().is_none_or(|v| !v.is_empty()))
        || input.get("interactive").and_then(Value::as_bool) == Some(true)
        || input.get("tty").and_then(Value::as_bool) == Some(true)
        || input
            .get("stdin")
            .is_some_and(|v| v.as_str().is_none_or(|s| !s.is_empty()))
    {
        return assessment(
            ToolRisk::High,
            Reason::Command(Command::EnvironmentOrInteractiveInput),
        );
    }
    let Some(command) = input.get("command").and_then(Value::as_str) else {
        return assessment(
            ToolRisk::High,
            Reason::Command(Command::UninspectableCommand),
        );
    };
    let words = if let Some(argv) = input.get("args") {
        let Some(argv) = argv.as_array() else {
            return assessment(
                ToolRisk::High,
                Reason::Command(Command::UnparsableArguments),
            );
        };
        let Some(mut words) = argv
            .iter()
            .map(|v| v.as_str().map(str::to_owned))
            .collect::<Option<Vec<_>>>()
        else {
            return assessment(
                ToolRisk::High,
                Reason::Command(Command::UnparsableArguments),
            );
        };
        // With args present (even []), command is ONE executable name, never shell text.
        words.insert(0, command.to_owned());
        words
    } else {
        let Some(words) = shell_words(command) else {
            return assessment(ToolRisk::Critical, Reason::Command(Command::ShellSyntax));
        };
        words
    };
    let (risk, reason) = argv_assessment(&words);
    if risk >= ToolRisk::Medium
        && input
            .get("cwd")
            .and_then(Value::as_str)
            .is_some_and(protected_path)
    {
        return assessment(
            ToolRisk::Critical,
            Reason::Command(Command::ProtectedDirectoryMutation),
        );
    }
    assessment(risk, Reason::Command(reason))
}

/// Parse only a deliberately small shell subset. Any expansion, control operator,
/// redirect or malformed quote stays high risk, including inside double quotes.
fn shell_words(command: &str) -> Option<Vec<String>> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut started = false;
    for ch in command.chars() {
        if matches!(ch, '$' | '`' | '\n' | '\r' | '\\') {
            return None;
        }
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            } else {
                word.push(ch);
            }
            started = true;
        } else if matches!(ch, '\'' | '"') {
            quote = Some(ch);
            started = true;
        } else if matches!(
            ch,
            ';' | '|' | '&' | '>' | '<' | '(' | ')' | '*' | '?' | '[' | ']' | '{' | '}' | '~' | '#'
        ) {
            return None;
        } else if ch.is_whitespace() {
            if started {
                words.push(std::mem::take(&mut word));
                started = false;
            }
        } else {
            word.push(ch);
            started = true;
        }
    }
    if quote.is_some() {
        return None;
    }
    if started {
        words.push(word);
    }
    (!words.is_empty()).then_some(words)
}

fn argv_assessment(words: &[String]) -> (ToolRisk, CommandRiskReason) {
    use CommandRiskReason::{FileDeletion, InterpreterExecution, NetworkAccess, RiskRuleMatch};
    let Some(command) = words.first() else {
        return (ToolRisk::High, RiskRuleMatch);
    };
    // Arbitrary executables named `ls` outside standard locations are not trusted queries.
    let name = if command.contains('/') {
        let Some(name) = command
            .strip_prefix("/bin/")
            .or_else(|| command.strip_prefix("/usr/bin/"))
        else {
            return (ToolRisk::High, RiskRuleMatch);
        };
        if name.contains('/') {
            return (ToolRisk::High, RiskRuleMatch);
        }
        name
    } else {
        command.as_str()
    };
    let args = &words[1..];
    match name {
        "sudo" | "su" | "doas" | "dd" | "mkfs" | "shutdown" | "reboot" | "launchctl"
        | "systemctl" | "diskutil" | "mount" | "umount" => (ToolRisk::Critical, RiskRuleMatch),
        "rm" | "rmdir" => {
            if args.iter().any(|a| {
                protected_path(a) || a == "--recursive" || a.starts_with('-') && a.contains('r')
            }) {
                (ToolRisk::Critical, FileDeletion)
            } else {
                (ToolRisk::High, FileDeletion)
            }
        }
        "chmod" | "chown" | "chgrp" | "kill" | "killall" | "pkill" => {
            (ToolRisk::High, RiskRuleMatch)
        }
        "curl" | "wget" | "scp" | "sftp" | "ssh" | "rsync" | "nc" | "ncat" | "telnet" => {
            (ToolRisk::High, NetworkAccess)
        }
        "sh" | "bash" | "zsh" | "fish" | "python" | "python3" | "node" | "ruby" | "perl"
        | "eval" | "exec" | "env" | "xargs" => (ToolRisk::Critical, InterpreterExecution),
        "git" => (git_risk(args), RiskRuleMatch),
        "mysql" | "psql" | "sqlite3" => (database_risk(args), RiskRuleMatch),
        "npm" | "pnpm" | "yarn" | "cargo" | "pip" | "pip3" | "docker" | "kubectl" | "make" => {
            if args.iter().any(|a| {
                matches!(
                    a.as_str(),
                    "publish" | "push" | "deploy" | "release" | "delete" | "prune"
                )
            }) {
                (ToolRisk::Critical, RiskRuleMatch)
            } else {
                (ToolRisk::High, RiskRuleMatch)
            }
        }
        "cp" | "mv" | "mkdir" | "touch" | "tee" => {
            if args.iter().any(|a| protected_path(a)) {
                (ToolRisk::Critical, RiskRuleMatch)
            } else {
                (ToolRisk::Medium, RiskRuleMatch)
            }
        }
        "find" => {
            if args.iter().any(|a| {
                matches!(
                    a.as_str(),
                    "-exec"
                        | "-execdir"
                        | "-ok"
                        | "-okdir"
                        | "-delete"
                        | "-fprint"
                        | "-fprint0"
                        | "-fprintf"
                )
            }) {
                (ToolRisk::High, RiskRuleMatch)
            } else {
                (ToolRisk::Low, RiskRuleMatch)
            }
        }
        "rg" => {
            if args.iter().any(|a| {
                a == "--pre"
                    || a.starts_with("--pre=")
                    || a == "--hostname-bin"
                    || a.starts_with("--hostname-bin=")
            }) {
                (ToolRisk::High, RiskRuleMatch)
            } else {
                (ToolRisk::Low, RiskRuleMatch)
            }
        }
        "hostname" => {
            if args.is_empty() {
                (ToolRisk::Safe, RiskRuleMatch)
            } else {
                (ToolRisk::High, RiskRuleMatch)
            }
        }
        "file" => {
            if args.iter().any(|arg| arg == "-C" || arg == "--compile") {
                (ToolRisk::Medium, RiskRuleMatch)
            } else {
                (ToolRisk::Low, RiskRuleMatch)
            }
        }
        "sed" | "awk" | "gawk" | "sort" => (ToolRisk::High, RiskRuleMatch), // Scripts/output options can execute or overwrite.
        "date" => {
            if args.is_empty()
                || args.iter().all(|a| {
                    a.starts_with('+')
                        || matches!(
                            a.as_str(),
                            "-u" | "--utc" | "--iso-8601" | "--rfc-3339=seconds"
                        )
                })
            {
                (ToolRisk::Safe, RiskRuleMatch)
            } else {
                (ToolRisk::High, RiskRuleMatch)
            }
        }
        "echo" | "printf" | "pwd" | "whoami" | "uname" | "true" | "false" => {
            (ToolRisk::Safe, RiskRuleMatch)
        }
        "ls" | "cat" | "head" | "tail" | "wc" | "stat" | "du" | "df" | "grep" | "readlink"
        | "realpath" | "which" | "type" | "ps" | "id" => (ToolRisk::Low, RiskRuleMatch),
        _ => (ToolRisk::High, RiskRuleMatch),
    }
}

fn git_risk(args: &[String]) -> ToolRisk {
    // Global flags can inject config, replace the executable or enable pagers/hooks.
    let Some(subcommand) = args.first().map(String::as_str) else {
        return ToolRisk::Low;
    };
    if subcommand.starts_with('-') {
        return ToolRisk::High;
    }
    let rest = &args[1..];
    if rest.iter().any(|a| {
        a == "--output"
            || a.starts_with("--output=")
            || a == "--ext-diff"
            || a == "--textconv"
            || a == "--exec"
            || a.starts_with("--exec=")
    }) {
        return ToolRisk::High;
    }
    match subcommand {
        "push" => ToolRisk::Critical,
        "reset" | "clean" | "rebase" | "filter-branch" | "filter-repo" | "gc" => ToolRisk::High,
        "status" | "log" | "show" | "diff" | "rev-parse" | "ls-files" | "ls-tree" | "describe"
        | "shortlog" | "blame" => ToolRisk::Low,
        "branch" | "tag" => {
            if rest.is_empty()
                || rest.iter().all(|a| {
                    matches!(
                        a.as_str(),
                        "--list" | "-l" | "-a" | "-r" | "--all" | "--remote" | "-v" | "-vv"
                    )
                })
            {
                ToolRisk::Low
            } else {
                ToolRisk::Medium
            }
        }
        "add" | "commit" | "checkout" | "switch" | "restore" | "merge" | "cherry-pick"
        | "stash" | "fetch" | "pull" | "clone" => ToolRisk::High,
        _ => ToolRisk::High,
    }
}

fn database_risk(args: &[String]) -> ToolRisk {
    let joined = args.join(" ").to_ascii_lowercase();
    if [
        "drop ",
        "truncate ",
        "outfile",
        "dumpfile",
        "load_file",
        "copy ",
        "\\!",
        "system ",
        "shutdown",
        "grant ",
        "revoke ",
    ]
    .iter()
    .any(|s| joined.contains(s))
    {
        return ToolRisk::Critical;
    }
    // SQL permits functions, extensions and stacked statements. Only help/version
    // is whitelisted; a SELECT prefix cannot prove a read-only operation.
    if args.len() == 1 && matches!(args[0].as_str(), "--version" | "--help") {
        ToolRisk::Safe
    } else {
        ToolRisk::High
    }
}