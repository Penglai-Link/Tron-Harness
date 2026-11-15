//! Risk classification uses trusted tool identity and actual parameters, never model hints.
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

/// `operation` is host catalog metadata: false = query, true = mutation.
/// Missing metadata is intentionally conservative; MCP descriptions/hints are untrusted.
pub fn identify_tool_risk(
    descriptor: &ToolDescriptor,
    args: &Value,
    operation: Option<bool>,
) -> ToolRisk {
    let id = descriptor.id.as_str();
    if id.starts_with("mcp/")
        || id.starts_with("mcp.")
        || id.starts_with("mcp:")
        || descriptor.call_name.starts_with("mcp__")
    {
        return ToolRisk::High;
    }
    if matches!(
        descriptor.call_name.as_str(),
        "terminal_start" | "terminal_write"
    ) {
        return ToolRisk::High;
    }
    if matches!(
        descriptor.call_name.as_str(),
        "terminal_read" | "terminal_list"
    ) {
        return ToolRisk::Safe;
    }
    match id {
        "system.command.exec" | "framework.terminal.start" => command_risk(args),
        // Input may be interpreted by an already running shell. Its state is
        // unavailable here, so a Critical threshold must still ask for approval.
        "system.command.task.write" | "framework.terminal.write" => ToolRisk::Critical,
        "system.command.task.stop" | "framework.terminal.stop" => ToolRisk::High,
        "system.command.task.read"
        | "system.command.list"
        | "framework.terminal.read"
        | "framework.terminal.list" => ToolRisk::Safe,
        "filesystem.read_text"
        | "filesystem.list"
        | "filesystem.stat"
        | "filesystem.search.grep"
        | "filesystem.search.glob"
        | "filesystem.search.path"
        | "media.read_image"
        | "media.read_pdf" => ToolRisk::Low,
        "filesystem.write_text" => write_risk(args),
        "filesystem.apply_patch" | "filesystem.apply_unified_patch" => patch_risk(args),
        "personai.search"
        | "personai.query_image"
        | "personai.detect_text"
        | "personai.detect_image"
        | "web.fetch" => ToolRisk::Low,
        "personai.generate_image" => ToolRisk::Medium,
        // Remote access changes pairing state and tokens; require the same
        // approval threshold as other host mutations.
        "framework.remote-access.manage" => ToolRisk::High,
        _ => match operation {
            Some(false) => nested_risk(args, 0).max(ToolRisk::Low),
            Some(true) => {
                let base = if descriptor
                    .call_name
                    .split('_')
                    .any(|part| matches!(part, "delete" | "remove" | "destroy" | "drop"))
                {
                    ToolRisk::High
                } else {
                    ToolRisk::Medium
                };
                nested_risk(args, 0).max(base)
            }
            None => nested_risk(args, 0).max(ToolRisk::High),
        },
    }
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
    if input
        .get("env")
        .is_some_and(|v| v.as_object().is_none_or(|v| !v.is_empty()))
        || input.get("interactive").and_then(Value::as_bool) == Some(true)
        || input.get("tty").and_then(Value::as_bool) == Some(true)
        || input
            .get("stdin")
            .is_some_and(|v| v.as_str().is_none_or(|s| !s.is_empty()))
    {
        return ToolRisk::High;
    }
    let Some(command) = input.get("command").and_then(Value::as_str) else {
        return ToolRisk::High;
    };
    let words = if let Some(argv) = input.get("args") {
        let Some(argv) = argv.as_array() else {
            return ToolRisk::High;
        };
        let Some(mut words) = argv
            .iter()
            .map(|v| v.as_str().map(str::to_owned))
            .collect::<Option<Vec<_>>>()
        else {
            return ToolRisk::High;
        };
        // With args present (even []), command is ONE executable name, never shell text.
        words.insert(0, command.to_owned());
        words
    } else {
        let Some(words) = shell_words(command) else {
            return ToolRisk::Critical;
        };
        words
    };
    let risk = argv_risk(&words);
    if risk >= ToolRisk::Medium
        && input
            .get("cwd")
            .and_then(Value::as_str)
            .is_some_and(protected_path)
    {
        ToolRisk::Critical
    } else {
        risk
    }
}

/// Explain the same command interpretation used by the risk classifier.
pub(crate) fn command_approval_reason(input: &Value) -> (&'static str, &'static str) {
    if input
        .get("env")
        .is_some_and(|v| v.as_object().is_none_or(|v| !v.is_empty()))
        || input.get("interactive").and_then(Value::as_bool) == Some(true)
        || input.get("tty").and_then(Value::as_bool) == Some(true)
        || input
            .get("stdin")
            .is_some_and(|v| v.as_str().is_none_or(|s| !s.is_empty()))
    {
        return (
            "调用包含环境变量、交互终端或标准输入，无法仅凭命令名确认执行影响",
            "The call includes environment overrides, an interactive terminal, or standard input; its effects cannot be determined from the executable alone",
        );
    }
    let Some(command) = input.get("command").and_then(Value::as_str) else {
        return (
            "调用未提供可静态检查的命令，需要人工确认",
            "No statically inspectable command was provided; manual approval is required",
        );
    };
    let words = if let Some(args) = input.get("args") {
        let Some(mut words) = args.as_array().and_then(|args| {
            args.iter()
                .map(|v| v.as_str().map(str::to_owned))
                .collect::<Option<Vec<_>>>()
        }) else {
            return (
                "命令参数无法静态解析",
                "The command arguments cannot be statically parsed",
            );
        };
        words.insert(0, command.to_owned());
        words
    } else {
        let Some(words) = shell_words(command) else {
            return (
                "命令包含 shell 控制、展开或无法静态解析的语法",
                "The command contains shell control, expansion, or syntax outside the static parser",
            );
        };
        words
    };
    if argv_risk(&words) >= ToolRisk::Medium
        && input
            .get("cwd")
            .and_then(Value::as_str)
            .is_some_and(protected_path)
    {
        return (
            "命令在受保护目录中执行且可能修改数据",
            "The command runs in a protected directory and may modify data",
        );
    }
    let name = words[0]
        .strip_prefix("/bin/")
        .or_else(|| words[0].strip_prefix("/usr/bin/"))
        .unwrap_or(&words[0]);
    match name {
        "sh" | "bash" | "zsh" | "fish" | "python" | "python3" | "node" | "ruby" | "perl"
        | "eval" | "exec" | "env" | "xargs" => (
            "命令通过解释器或动态执行程序运行，可执行任意脚本",
            "The command uses an interpreter or dynamic executor and can run arbitrary scripts",
        ),
        "rm" | "rmdir" => (
            "命令会删除文件或目录",
            "The command deletes files or directories",
        ),
        "curl" | "wget" | "scp" | "sftp" | "ssh" | "rsync" | "nc" | "ncat" | "telnet" => (
            "命令会访问网络、传输数据或执行远端操作",
            "The command can access the network, transfer data, or run remote operations",
        ),
        _ => (
            "命令的可执行程序与参数命中了当前执行风险规则，达到你配置的确认阈值",
            "The executable and arguments match the execution risk rules and meet your configured approval threshold",
        ),
    }
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

fn argv_risk(words: &[String]) -> ToolRisk {
    let Some(command) = words.first() else {
        return ToolRisk::High;
    };
    // Arbitrary executables named `ls` outside standard locations are not trusted queries.
    let name = if command.contains('/') {
        let Some(name) = command
            .strip_prefix("/bin/")
            .or_else(|| command.strip_prefix("/usr/bin/"))
        else {
            return ToolRisk::High;
        };
        if name.contains('/') {
            return ToolRisk::High;
        }
        name
    } else {
        command.as_str()
    };
    let args = &words[1..];
    match name {
        "sudo" | "su" | "doas" | "dd" | "mkfs" | "shutdown" | "reboot" | "launchctl"
        | "systemctl" | "diskutil" | "mount" | "umount" => ToolRisk::Critical,
        "rm" | "rmdir" => {
            if args.iter().any(|a| {
                protected_path(a) || a == "--recursive" || a.starts_with('-') && a.contains('r')
            }) {
                ToolRisk::Critical
            } else {
                ToolRisk::High
            }
        }
        "chmod" | "chown" | "chgrp" | "kill" | "killall" | "pkill" => ToolRisk::High,
        "curl" | "wget" | "scp" | "sftp" | "ssh" | "rsync" | "nc" | "ncat" | "telnet" => {
            ToolRisk::High
        }
        "sh" | "bash" | "zsh" | "fish" | "python" | "python3" | "node" | "ruby" | "perl"
        | "eval" | "exec" | "env" | "xargs" => ToolRisk::Critical,
        "git" => git_risk(args),
        "mysql" | "psql" | "sqlite3" => database_risk(args),
        "npm" | "pnpm" | "yarn" | "cargo" | "pip" | "pip3" | "docker" | "kubectl" | "make" => {
            if args.iter().any(|a| {
                matches!(
                    a.as_str(),
                    "publish" | "push" | "deploy" | "release" | "delete" | "prune"
                )
            }) {
                ToolRisk::Critical
            } else {
                ToolRisk::High
            }
        }
        "cp" | "mv" | "mkdir" | "touch" | "tee" => {
            if args.iter().any(|a| protected_path(a)) {
                ToolRisk::Critical
            } else {
                ToolRisk::Medium
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
                ToolRisk::High
            } else {
                ToolRisk::Low
            }
        }
        "rg" => {
            if args.iter().any(|a| {
                a == "--pre"
                    || a.starts_with("--pre=")
                    || a == "--hostname-bin"
                    || a.starts_with("--hostname-bin=")
            }) {
                ToolRisk::High
            } else {
                ToolRisk::Low
            }
        }
        "hostname" => {
            if args.is_empty() {
                ToolRisk::Safe
            } else {
                ToolRisk::High
            }
        }
        "file" => {
            if args.iter().any(|arg| arg == "-C" || arg == "--compile") {
                ToolRisk::Medium
            } else {
                ToolRisk::Low
            }
        }
        "sed" | "awk" | "gawk" | "sort" => ToolRisk::High, // Scripts/output options can execute or overwrite.
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
                ToolRisk::Safe
            } else {
                ToolRisk::High
            }
        }
        "echo" | "printf" | "pwd" | "whoami" | "uname" | "true" | "false" => ToolRisk::Safe,
        "ls" | "cat" | "head" | "tail" | "wc" | "stat" | "du" | "df" | "grep" | "readlink"
        | "realpath" | "which" | "type" | "ps" | "id" => ToolRisk::Low,
        _ => ToolRisk::High,
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
