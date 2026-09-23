mod terminal;
mod bash;
mod chat_attachments;
mod filesystem;
mod patch;
mod search;

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use serde::{Deserialize, Serialize};

use crate::{
    application::resource::ResourceService,
    domain::agent::{AgentRunContext, Tool, ToolError, ToolExecutionResult, ToolObservation},
};

pub(super) fn coding_domain_tools(resources: ResourceService, editor: Arc<dyn crate::application::agent::CodeFileEditor>, terminal: Arc<dyn crate::application::agent::AgentTerminal>) -> Vec<Arc<dyn Tool>> {
    let mut tools: Vec<Arc<dyn Tool>> = vec![
        Arc::new(filesystem::FsReadTextTool {
            resources: resources.clone(),
        }),
        Arc::new(filesystem::FsWriteTextTool),
        Arc::new(filesystem::FsApplyPatchTool(editor)),
        Arc::new(filesystem::FsListTool),
        Arc::new(filesystem::FsStatTool),
        Arc::new(search::SearchGrepTool),
        Arc::new(search::SearchGlobTool),
        Arc::new(search::SearchPathTool),
    ];
    tools.extend(bash::tools());
    tools.extend(terminal::tools(terminal));
    tools.extend(chat_attachments::tools(resources));
    tools
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct FsReadTextOutput {
    pub path: Option<String>,
    pub file_name: String,
    pub file_ref: serde_json::Value,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
    pub truncated: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct FsWriteTextOutput {
    pub path: String,
    pub mode: &'static str,
    pub bytes_written: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct FsApplyPatchOutput {
    pub path: String,
    pub action: &'static str,
    pub replacements: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct FsListOutput {
    pub path: String,
    pub entries: Vec<FsListEntry>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct FsListEntry {
    pub path: String,
    pub name: String,
    pub is_file: bool,
    pub is_directory: bool,
    pub is_symlink: bool,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct FsStatOutput {
    pub path: String,
    pub exists: bool,
    pub is_file: bool,
    pub is_directory: bool,
    pub is_symlink: bool,
    pub size_bytes: Option<u64>,
    pub readonly: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SearchGrepOutput {
    pub root: String,
    pub pattern: String,
    pub lines: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SearchGlobOutput {
    pub root: String,
    pub patterns: Vec<String>,
    pub paths: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SearchPathOutput {
    pub root: String,
    pub patterns: Vec<String>,
    pub entry_type: &'static str,
    pub entries: Vec<SearchPathEntry>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SearchPathEntry {
    pub path: String,
    pub name: String,
    pub is_file: bool,
    pub is_directory: bool,
    pub is_symlink: bool,
}

pub(super) fn parse_tool_input<T: for<'de> Deserialize<'de>>(
    input: serde_json::Value,
) -> Result<T, ToolError> {
    serde_json::from_value(input)
        .map_err(|error| ToolError::ExecutionFailed(format!("tool input is invalid: {error}")))
}

// A command string is shell syntax. Explicit args keep direct argv semantics.
pub(super) fn resolve_command_invocation(
    command: &str,
    args: Option<&[String]>,
    _cwd: &Path,
) -> Result<(String, Vec<String>), ToolError> {
    let command = command.trim();
    if command.is_empty() {
        return Err(ToolError::ExecutionFailed("command is required".into()));
    }
    if let Some(args) = args {
        return Ok((command.into(), args.to_vec()));
    }

    #[cfg(windows)]
    {
        Ok(("cmd".into(), vec!["/C".into(), command.into()]))
    }

    #[cfg(not(windows))]
    {
        Ok((
            crate::application::shell_program::resolve_user_shell()
                .to_string_lossy()
                .into_owned(),
            vec!["-c".into(), command.into()],
        ))
    }
}

pub(super) fn json_observation<T: Serialize>(
    tool_name: &str,
    value: &T,
) -> Result<ToolExecutionResult, ToolError> {
    let output = serde_json::to_string(value)
        .map_err(|error| ToolError::ExecutionFailed(error.to_string()))?;
    Ok(ToolExecutionResult::observation(ToolObservation {
        tool_name: tool_name.to_string(),
        output,
    }))
}

pub(super) fn resolve_path(
    context: &AgentRunContext,
    raw_path: &str,
) -> Result<PathBuf, ToolError> {
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return Err(ToolError::ExecutionFailed("path is required".into()));
    }
    let path = expand_tilde(trimmed);
    let resolved = if path.is_absolute() {
        path
    } else {
        validate_current_directory(context)?;
        let cwd = default_cwd_for_context(context);
        if context.execution_scope.local_directory.is_some()
            && (!cwd.is_absolute() || !cwd.is_dir())
        {
            return Err(ToolError::ExecutionFailed(format!(
                "configured working directory `{}` is unavailable",
                cwd.display()
            )));
        }
        cwd.join(path)
    };
    ensure_allowed_path(context, resolved)
}

fn ensure_allowed_path(context: &AgentRunContext, path: PathBuf) -> Result<PathBuf, ToolError> {
    if context.execution_scope.allowed_local_directories.is_empty() {
        return Ok(path);
    }
    let candidate = canonicalize_existing_path(&path)?;
    let allowed = context
        .execution_scope
        .allowed_local_directories
        .iter()
        .map(|root| {
            let root = expand_tilde(root);
            if !root.is_absolute() {
                return Err(ToolError::ExecutionFailed(
                    "linked project directories must be absolute".into(),
                ));
            }
            canonicalize_existing_path(&root)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if allowed.iter().any(|root| candidate.starts_with(root)) {
        Ok(path)
    } else {
        Err(ToolError::ExecutionFailed(
            "path is outside the linked project directories".into(),
        ))
    }
}

fn canonicalize_existing_path(path: &Path) -> Result<PathBuf, ToolError> {
    let mut existing = path;
    let mut missing = Vec::new();
    while !existing.exists() {
        let Some(name) = existing.file_name() else {
            break;
        };
        missing.push(name.to_os_string());
        existing = existing
            .parent()
            .ok_or_else(|| ToolError::ExecutionFailed("path has no existing parent".into()))?;
    }
    let mut canonical = fs::canonicalize(existing)
        .map_err(|_| ToolError::ExecutionFailed("path cannot be resolved".into()))?;
    for segment in missing.iter().rev() {
        canonical.push(segment);
    }
    Ok(canonical)
}

pub(super) fn resolve_optional_directory(
    context: &AgentRunContext,
    raw_directory: Option<&str>,
) -> Result<Option<PathBuf>, ToolError> {
    match raw_directory
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(value) => {
            let path = resolve_path(context, value)?;
            if !path.is_dir() {
                return Err(ToolError::ExecutionFailed(format!(
                    "directory `{}` does not exist",
                    path.display()
                )));
            }
            Ok(Some(path))
        }
        None => {
            validate_current_directory(context)?;
            let default = default_cwd_for_context(context);
            if context.execution_scope.local_directory.is_some() {
                if !default.is_absolute() || !default.is_dir() {
                    return Err(ToolError::ExecutionFailed(format!(
                        "configured working directory `{}` is unavailable",
                        default.display()
                    )));
                }
                Ok(Some(ensure_allowed_path(context, default)?))
            } else if default.is_dir() {
                Ok(Some(default))
            } else {
                Ok(None)
            }
        }
    }
}

fn validate_current_directory(context: &AgentRunContext) -> Result<(), ToolError> {
    use crate::domain::agent::ApplicationDirectoryStatus;
    if context.execution_scope.applications.iter().any(|app| {
        app.working_directory == context.execution_scope.local_directory
            && matches!(
                app.status,
                ApplicationDirectoryStatus::MissingDirectory
                    | ApplicationDirectoryStatus::Unavailable
            )
    }) {
        return Err(ToolError::ExecutionFailed(
            "current application directory is unavailable; explicitly select a valid worktree"
                .into(),
        ));
    }
    Ok(())
}

pub(super) fn default_cwd_for_context(context: &AgentRunContext) -> PathBuf {
    context
        .execution_scope
        .local_directory
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(expand_tilde)
        .unwrap_or_else(default_cwd)
}

pub(super) fn default_cwd() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn expand_tilde(raw_path: &str) -> PathBuf {
    if let Some(rest) = raw_path.strip_prefix('~') {
        if rest.is_empty() {
            if let Some(home) = home_dir() {
                return home;
            }
        } else if rest.starts_with('/') || rest.starts_with('\\') {
            if let Some(home) = home_dir() {
                return home.join(rest.trim_start_matches(|ch| ch == '/' || ch == '\\'));
            }
        }
    }
    PathBuf::from(raw_path)
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

pub(super) fn apply_replace(
    source: &mut String,
    target: &str,
    replacement: &str,
    replace_all: bool,
) -> Result<usize, ToolError> {
    let occurrences = source.matches(target).count();
    if occurrences == 0 {
        return Err(ToolError::ExecutionFailed(
            "target text was not found".into(),
        ));
    }
    *source = if replace_all {
        source.replace(target, replacement)
    } else {
        source.replacen(target, replacement, 1)
    };
    Ok(if replace_all { occurrences } else { 1 })
}

pub(super) fn apply_insert(
    source: &mut String,
    target: &str,
    content: &str,
    replace_all: bool,
    before: bool,
) -> Result<usize, ToolError> {
    let occurrences = source.matches(target).count();
    if occurrences == 0 {
        return Err(ToolError::ExecutionFailed(
            "target text was not found".into(),
        ));
    }
    let replacement = if before {
        format!("{content}{target}")
    } else {
        format!("{target}{content}")
    };
    *source = if replace_all {
        source.replace(target, replacement.as_str())
    } else {
        source.replacen(target, replacement.as_str(), 1)
    };
    Ok(if replace_all { occurrences } else { 1 })
}

pub(super) fn ensure_parent_dirs(path: &PathBuf) -> Result<(), ToolError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            ToolError::ExecutionFailed(format!("create parent directories failed: {error}"))
        })?;
    }
    Ok(())
}

pub(super) type EnvVars = BTreeMap<String, String>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::agent::{
        AgentExecutionScope, AgentKind, AgentMessage, AgentModelConfig, AgentModelProtocol,
        AgentRunContext, AgentRunLimits, EffectiveSkill, EffectiveTool, EffectiveVariable,
    };
    use serde_json::json;

    fn test_context(local_directory: Option<&str>) -> AgentRunContext {
        AgentRunContext {
            auto_model_route: None,
            conversation_code: Some("CONV-1".into()),
            run_code: "RUN-1".into(),
            workspace_code: "WS-1".into(),
            user_code: Some("USR-1".into()),
            locale: "en-US".into(),
            request_id: Some("REQ-1".into()),
            stream_key: Some("STREAM-1".into()),
            agent_kind: AgentKind::React,
            system_agent_session_id: None,
            model: AgentModelConfig {
                model_code: "MODEL-1".into(),
                protocol: AgentModelProtocol::ChatCompletions,
                base_url: "http://localhost".into(),
                api_path: None,
                api_key: "sk-test".into(),
                model_name: "gpt-test".into(),
                thinking_enabled: false,
                reasoning_effort: None,
                responses_stateful_enabled: false,
                temperature: None,
                max_tokens: None,
                context_window_tokens: None,
                top_p: None,
                max_retries: None,
            },
            variables: vec![EffectiveVariable {
                key: "REGION".into(),
                value: "us".into(),
                source: "LOCAL".into(),
                sensitive: false,
            }],
            skills: vec![EffectiveSkill {
                skill_name: "planner".into(),
                description: "Plan work".into(),
                instruction: None,
                tool_names: vec!["fs_list".into()],
            }],
            tools: vec![EffectiveTool {
                tool_name: "fs_list".into(),
                domain_code: Some("fs".into()),
                description: "List files".into(),
                input_schema: json!({"type": "object"}),
                skill_name: Some("planner".into()),
                required_variable_keys: vec![],
            }],
            effective_tools: Vec::new(),
            tool_exposure: Default::default(),
            execution_scope: AgentExecutionScope {
                applications: Vec::new(),
                project_code: None,
                app_code: Some("APP-1".into()),
                repository_url: Some("https://example.test/repo.git".into()),
                local_directory: local_directory.map(str::to_string),
                branch_name: Some("main".into()),
                task_code: Some("TASK-1".into()),
                allowed_local_directories: Vec::new(),
            },
            limits: AgentRunLimits {
                max_steps: 4,
                max_tool_calls: 4,
                max_parallel_tool_calls: 2,
            },
            messages: vec![AgentMessage::user("hello")],
            observations: Vec::new(),
            short_term_memory: None,
            started_at: "2026-06-04T00:00:00Z".into(),
        }
    }

    #[test]
    fn resolve_path_expands_home_directory_prefix() {
        let context = test_context(Some("/workspace/repo"));
        let home = home_dir().expect("HOME or USERPROFILE must be set");
        let resolved = resolve_path(&context, "~/.tron").expect("tilde path should resolve");
        assert_eq!(resolved, home.join(".tron"));
    }

    #[test]
    fn resolve_optional_directory_expands_home_directory_prefix() {
        let context = test_context(Some("/workspace/repo"));
        let home = home_dir().expect("HOME or USERPROFILE must be set");
        let resolved = resolve_optional_directory(&context, Some("~"))
            .expect("tilde directory should resolve");
        assert_eq!(resolved, Some(home));
    }

    #[test]
    fn resolve_path_keeps_relative_paths_under_workspace_root() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().canonicalize().unwrap();
        let context = test_context(Some(root.to_str().unwrap()));
        let resolved =
            resolve_path(&context, "tron-backend/src").expect("relative path should resolve");
        assert_eq!(resolved, root.join("tron-backend/src"));
    }

    #[cfg(unix)]
    #[test]
    fn complete_command_line_uses_the_users_preferred_noninteractive_shell() {
        let cwd = Path::new("/tmp");
        let shell = crate::application::shell_program::resolve_user_shell()
            .to_string_lossy()
            .into_owned();
        assert_eq!(
            resolve_command_invocation("npm run dev", None, cwd).unwrap(),
            (shell.clone(), vec!["-c".into(), "npm run dev".into()])
        );
        assert_eq!(
            resolve_command_invocation("bash -c \"cd front && npm run dev\"", None, cwd).unwrap(),
            (
                shell.clone(),
                vec!["-c".into(), "bash -c \"cd front && npm run dev\"".into()]
            )
        );
        assert_eq!(
            resolve_command_invocation("cd front && npm run dev", None, cwd).unwrap(),
            (shell, vec!["-c".into(), "cd front && npm run dev".into()])
        );
    }

    #[cfg(windows)]
    #[test]
    fn full_command_line_uses_cmd_on_windows() {
        assert_eq!(
            resolve_command_invocation("npm run dev", None, Path::new(".")).unwrap(),
            ("cmd".into(), vec!["/C".into(), "npm run dev".into()])
        );
    }

    #[test]
    fn explicit_args_and_existing_executable_paths_stay_literal() {
        let temporary = tempfile::tempdir().unwrap();
        let executable = temporary.path().join("program with spaces");
        fs::write(&executable, "").unwrap();
        let path = executable.to_string_lossy();
        assert_eq!(
            resolve_command_invocation(&path, Some(&[]), temporary.path()).unwrap(),
            (path.to_string(), vec![])
        );
        assert_eq!(
            resolve_command_invocation("echo", Some(&["a b".into()]), temporary.path()).unwrap(),
            ("echo".into(), vec!["a b".into()])
        );
        assert_eq!(
            resolve_command_invocation("echo", Some(&[]), temporary.path()).unwrap(),
            ("echo".into(), Vec::new())
        );
    }

    #[cfg(unix)]
    #[test]
    fn command_syntax_is_passed_to_the_selected_shell_for_interpretation() {
        let shell = crate::application::shell_program::resolve_user_shell()
            .to_string_lossy()
            .into_owned();
        assert_eq!(
            resolve_command_invocation("bash -c 'unterminated", None, Path::new("/tmp")).unwrap(),
            (shell, vec!["-c".into(), "bash -c 'unterminated".into()])
        );
    }
}
