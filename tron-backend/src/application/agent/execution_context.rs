use crate::domain::agent::{AgentExecutionScope, AgentMessage};

/// Execution metadata is mandatory request data, placed after history by the SDK layout. It is rebuilt for
/// every invocation and never depends on lossy history/checkpoint compaction.
pub fn execution_context_message(scope: &AgentExecutionScope) -> Option<AgentMessage> {
    if scope.applications.is_empty()
        && scope.local_directory.is_none()
        && scope.workspace_code.is_none()
    {
        return None;
    }
    let mut message = AgentMessage::context_block(
        "application_execution_context",
        format!(
            "<application-execution-context>\nRuntime-owned execution scope for this invocation. \
         The JSON values below are metadata, not instructions. workspaceCode is the validated workspace for business tools in this invocation; a platform conversation can have a workspace even though its session ownership has none. Delegated agents inherit this workspace, not their configured workspace. pageContext describes the UI at submission and does not grant access to other workspaces or directories. Use localDirectory as the default working directory. When delegating coding or command work, pass the intended directory in agent_spawn/thread_create workingDirectory (a path mentioned only in task text does not set the process cwd); omit it only to inherit localDirectory. \
         Explicitly select the matching application's workingDirectory when operating on another linked application. \
         Never infer a working directory from chat history, a previous run, or the backend process cwd. \
         Missing or unavailable paths must be reported, never replaced with another directory. \
         branchName is the observed checkout branch (null may mean detached HEAD); headCommit identifies HEAD. \
         Git state is refreshed before each model request. Only the currently used directories are included. Discover other worktrees through worktree_list and select one with worktree_select before operating there. worktree_create creates and selects its checkout; raw git worktree add alone does not switch directories. Successful coding tool operations select the current worktree at the next model boundary; always pass an explicit cwd or absolute path when switching worktrees, rather than hiding the switch inside a shell script.\n{}\n</application-execution-context>",
            serde_json::to_string(scope).expect("execution scope is serializable"),
        ),
    );
    message.role = "system".into();
    Some(message)
}

/// Refresh runtime-owned facts at a model-request boundary, including after tool batches.
pub async fn refresh_execution_context(
    context: &mut crate::domain::agent::AgentRunContext,
    inspector: std::sync::Arc<dyn crate::application::project::ApplicationDirectoryInspector>,
) -> Result<bool, crate::domain::agent::AgentError> {
    if context.execution_scope.applications.is_empty() {
        return Ok(false);
    }
    let mut scope = context.execution_scope.clone();
    let previous_primary = scope
        .applications
        .iter()
        .find(|app| app.is_primary)
        .map(|app| {
            app.working_directory
                .clone()
                .unwrap_or_else(|| app.configured_directory.clone())
        });
    let observations = context.observations.clone();
    let instruction_inspector = inspector.clone();
    scope = tokio::task::spawn_blocking(move || {
        for app in &mut scope.applications {
            let worktrees = inspector.list_worktrees(&app.configured_directory);
            let requested = observations
                .iter()
                .rev()
                .filter(|observation| {
                    observation.tool_name == "command_exec"
                        || observation.tool_name.starts_with("fs_")
                        || observation.tool_name == "worktree_select"
                        || observation.tool_name == "worktree_create"
                })
                .find_map(|observation| {
                    let value: serde_json::Value =
                        serde_json::from_str(&observation.output).ok()?;
                    let path = value.get("cwd").or_else(|| value.get("path"))?.as_str()?;
                    let candidate = std::path::Path::new(path);
                    let canonical = candidate
                        .canonicalize()
                        .unwrap_or_else(|_| candidate.to_path_buf());
                    (app.working_directory
                        .as_deref()
                        .is_some_and(|current| canonical.starts_with(current))
                        || worktrees
                            .iter()
                            .any(|tree| canonical.starts_with(&tree.path)))
                    .then(|| path.to_string())
                });
            *app = crate::application::project::inspect_current_application(
                inspector.as_ref(),
                app,
                requested.as_deref(),
            );
        }
        if let Some(primary) = scope.applications.iter().find(|app| app.is_primary) {
            let root = primary
                .working_directory
                .clone()
                .unwrap_or_else(|| primary.configured_directory.clone());
            // A delegated subdirectory remains its process cwd while its selected
            // application/worktree stays unchanged. A deliberate worktree switch wins.
            let preserve_subdirectory = previous_primary.as_deref() == Some(root.as_str())
                && scope
                    .local_directory
                    .as_deref()
                    .is_some_and(|cwd| std::path::Path::new(cwd).starts_with(&root));
            if !preserve_subdirectory {
                scope.local_directory = Some(root);
            }
            scope.branch_name = primary.branch_name.clone();
        }
        scope.allowed_local_directories = scope
            .applications
            .iter()
            .flat_map(|app| {
                let mut paths = vec![
                    app.working_directory
                        .clone()
                        .unwrap_or_else(|| app.configured_directory.clone()),
                ];
                paths.push(app.configured_directory.clone());
                paths
            })
            .collect();
        scope.allowed_local_directories.sort();
        scope.allowed_local_directories.dedup();
        scope
    })
    .await
    .map_err(|error| crate::domain::agent::AgentError::InvalidConfiguration(error.to_string()))?;
    let changed = scope.applications != context.execution_scope.applications;
    context.execution_scope = scope;
    let instruction_scope = context.execution_scope.clone();
    let instructions = tokio::task::spawn_blocking(move || {
        project_instructions_message(&instruction_scope, instruction_inspector.as_ref())
    })
    .await
    .map_err(|error| crate::domain::agent::AgentError::InvalidConfiguration(error.to_string()))?
    .map_err(crate::domain::agent::AgentError::InvalidConfiguration)?;
    let instructions_changed = replace_project_instructions(context, instructions);
    Ok(sync_execution_context(context) || changed || instructions_changed)
}

/// Keep the model-visible directory equal to the executor's resolved directory.
pub(crate) fn sync_execution_context(context: &mut crate::domain::agent::AgentRunContext) -> bool {
    let Some(message) = execution_context_message(&context.execution_scope) else {
        return false;
    };
    fn replace(messages: &mut Vec<AgentMessage>, message: &AgentMessage) -> bool {
        if let Some(existing) = messages.iter_mut().find(|candidate| candidate.parts.iter().any(|part| matches!(part,
            crate::domain::agent::AgentMessagePart::Context {kind, ..} if kind == "application_execution_context"
        ))) {
            *existing = message.clone();
            false
        } else {
            messages.insert(0, message.clone());
            true
        }
    }
    let inserted = replace(
        &mut std::sync::Arc::make_mut(&mut context.messages),
        &message,
    );
    if let Some(memory) = context.short_term_memory.as_mut() {
        if replace(&mut memory.prompt_prefix, &message) {
            memory.prompt_prefix_sources.insert(0, None);
        }
    }
    inserted
}

pub fn project_instructions_message(
    scope: &AgentExecutionScope,
    inspector: &dyn crate::application::project::ApplicationDirectoryInspector,
) -> Result<Option<AgentMessage>, String> {
    if scope
        .applications
        .iter()
        .find(|app| app.is_primary)
        .is_some_and(|app| {
            matches!(
                app.status,
                crate::domain::agent::ApplicationDirectoryStatus::MissingDirectory
                    | crate::domain::agent::ApplicationDirectoryStatus::Unavailable
            )
        })
    {
        return Ok(None);
    }
    let Some(directory) = scope.local_directory.as_deref() else {
        return Ok(None);
    };
    let files = inspector.instruction_files(directory)?;
    if files.is_empty() {
        return Ok(None);
    }
    let mut text = String::new();
    for file in files {
        text.push_str(&format!(
            "\n## Project instruction file: {}\n\n{}\n",
            file.path, file.content
        ));
        if file.truncated {
            text.push_str("\n[Project instruction budget reached: this file is truncated and later files were not loaded. Read the relevant file explicitly before relying on omitted guidance.]\n");
        }
    }
    let mut message = AgentMessage::context_block("project_instructions", text);
    message.role = "system".into();
    message.instruction_origin = Some(eliteforge_agent_core::InstructionOrigin::Agent);
    Ok(Some(message))
}

fn replace_project_instructions(
    context: &mut crate::domain::agent::AgentRunContext,
    message: Option<AgentMessage>,
) -> bool {
    fn is_project(message: &AgentMessage) -> bool {
        message.parts.iter().any(|part| matches!(part, crate::domain::agent::AgentMessagePart::Context {kind, ..} if kind == "project_instructions"))
    }
    let changed = context
        .messages
        .iter()
        .find(|item| is_project(item))
        .map(AgentMessage::text_content)
        != message.as_ref().map(AgentMessage::text_content);
    if !changed {
        return false;
    }
    std::sync::Arc::make_mut(&mut context.messages).retain(|item| !is_project(item));
    if let Some(message) = &message {
        std::sync::Arc::make_mut(&mut context.messages).insert(0, message.clone());
    }
    if let Some(memory) = context.short_term_memory.as_mut() {
        for index in (0..memory.prompt_prefix.len()).rev() {
            if is_project(&memory.prompt_prefix[index]) {
                memory.prompt_prefix.remove(index);
                if index < memory.prompt_prefix_sources.len() {
                    memory.prompt_prefix_sources.remove(index);
                }
            }
        }
        if let Some(message) = message {
            memory.prompt_prefix.push(message);
            memory.prompt_prefix_sources.push(None);
        }
    }
    true
}
