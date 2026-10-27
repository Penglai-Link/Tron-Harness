//! Stable product-owned working directories; never borrow the backend process cwd.
use super::*;
use crate::domain::agent::{AgentExecutionScope, AgentRunContext};

fn component(value: &str) -> String {
    if !value.is_empty()
        && value != "."
        && value != ".."
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
    {
        value.to_owned()
    } else {
        use sha2::{Digest, Sha256};
        format!("{:x}", Sha256::digest(value.as_bytes()))
    }
}

impl FrameworkHost {
    pub(super) fn delegated_execution_scope(
        &self,
        context: &AgentRunContext,
        raw: &str,
    ) -> Result<AgentExecutionScope, String> {
        let directory =
            crate::application::agent::business_tools::delegated_working_directory(context, raw)
                .map_err(|error| error.to_string())?;
        let mut scope = context.execution_scope.clone();
        let selected = std::path::Path::new(&directory);
        let inspector = self
            .0
            .collaboration_builder
            .get()
            .and_then(std::sync::Weak::upgrade)
            .and_then(|builder| builder.directory_inspector());
        let primary = scope
            .applications
            .iter()
            .enumerate()
            .flat_map(|(index, app)| {
                let mut roots = vec![
                    app.working_directory
                        .clone()
                        .unwrap_or_else(|| app.configured_directory.clone()),
                    app.configured_directory.clone(),
                ];
                if let Some(inspector) = &inspector {
                    roots.extend(
                        inspector
                            .list_worktrees(&app.configured_directory)
                            .into_iter()
                            .map(|tree| tree.path),
                    );
                }
                roots.into_iter().map(move |root| (index, root))
            })
            .filter_map(|(index, root)| {
                let root = Path::new(&root)
                    .canonicalize()
                    .unwrap_or_else(|_| PathBuf::from(root));
                selected.starts_with(&root).then_some((index, root))
            })
            .max_by_key(|(_, root)| root.components().count());
        if let Some((index, root)) = primary {
            if let Some(inspector) = &inspector {
                scope.applications[index] =
                    crate::application::project::inspect_current_application(
                        inspector.as_ref(),
                        &scope.applications[index],
                        root.to_str(),
                    );
            }
            for (current, app) in scope.applications.iter_mut().enumerate() {
                app.is_primary = current == index;
            }
            scope.app_code = Some(scope.applications[index].app_code.clone());
            scope.branch_name = scope.applications[index].branch_name.clone();
        }
        scope.local_directory = Some(directory);
        Ok(scope)
    }
    pub(crate) fn prepare_working_directory(
        &self,
        context: &mut AgentRunContext,
    ) -> Result<(), String> {
        if let Some(directory) = &context.execution_scope.local_directory {
            if directory.trim().is_empty() {
                return Err("configured working directory must not be empty".into());
            }
            // An explicitly selected directory is authoritative, even if missing.
            // The executor must report that error instead of silently replacing it.
            return Ok(());
        }
        if let Some(primary) = context
            .execution_scope
            .applications
            .iter()
            .find(|app| app.is_primary)
        {
            context.execution_scope.local_directory = Some(
                primary
                    .working_directory
                    .clone()
                    .unwrap_or_else(|| primary.configured_directory.clone()),
            );
            return Ok(());
        }
        let user = component(context.user_code.as_deref().unwrap_or("system"));
        let (kind, id) = match context
            .collaboration_owner
            .as_ref()
            .filter(|owner| owner.kind == "DUDE")
        {
            Some(owner) => ("dudes", owner.code.as_str()),
            None => (
                "sessions",
                context
                    .conversation_code
                    .as_deref()
                    .unwrap_or(&context.run_code),
            ),
        };
        let directory = self
            .0
            .path
            .with_extension("workspaces")
            .join(kind)
            .join(user)
            .join(component(id));
        fs::create_dir_all(&directory)
            .map_err(|error| format!("could not prepare session working directory: {error}"))?;
        let directory = directory
            .canonicalize()
            .map_err(|error| format!("could not resolve session working directory: {error}"))?;
        let directory = directory.to_string_lossy().into_owned();
        if !context.execution_scope.allowed_local_directories.is_empty() {
            context
                .execution_scope
                .allowed_local_directories
                .push(directory.clone());
        }
        context.execution_scope.local_directory = Some(directory);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::agent::{AgentCollaborationOwner, AgentExecutionScope};
    fn context(session: &str) -> AgentRunContext {
        serde_json::from_value(serde_json::json!({
            "runCode":fw::RunId::new_v4().to_string(),"conversationCode":session,"workspaceCode":"workspace","userCode":"user",
            "locale":"en-US","agentKind":"REACT","model":{"modelCode":"m","protocol":"CHAT_COMPLETIONS","baseUrl":"http://fixture","apiKey":"fixture","modelName":"m"},
            "variables":[],"skills":[],"tools":[],"limits":{"maxSteps":100,"maxToolCalls":100},"messages":[],"observations":[],"startedAt":""
        })).unwrap()
    }
    #[test]
    fn sessions_are_stable_isolated_and_not_the_backend_cwd() {
        let host = FrameworkHost::temporary(None).unwrap();
        let mut first = context("first");
        host.prepare_working_directory(&mut first).unwrap();
        let path = first.execution_scope.local_directory.clone().unwrap();
        assert!(Path::new(&path).is_dir());
        assert_ne!(Path::new(&path), std::env::current_dir().unwrap());
        let mut again = context("first");
        host.prepare_working_directory(&mut again).unwrap();
        assert_eq!(first.execution_scope, again.execution_scope);
        let mut other = context("second");
        host.prepare_working_directory(&mut other).unwrap();
        assert_ne!(Some(path), other.execution_scope.local_directory);
        let mut other_user = context("first");
        other_user.user_code = Some("another-user".into());
        host.prepare_working_directory(&mut other_user).unwrap();
        assert_ne!(
            first.execution_scope.local_directory,
            other_user.execution_scope.local_directory
        );
    }
    #[test]
    fn dude_directory_survives_session_rotation_and_is_separate_from_chat() {
        let host = FrameworkHost::temporary(None).unwrap();
        let mut first = context("wake-first");
        first.collaboration_owner = Some(AgentCollaborationOwner {
            kind: "DUDE".into(),
            code: "main-dude".into(),
            origin_session_code: Some("wake-first".into()),
            task_code: None,
        });
        host.prepare_working_directory(&mut first).unwrap();
        let mut next = context("wake-next");
        next.collaboration_owner = first.collaboration_owner.clone();
        host.prepare_working_directory(&mut next).unwrap();
        assert_eq!(
            first.execution_scope.local_directory,
            next.execution_scope.local_directory
        );
        let mut chat = context("wake-first");
        host.prepare_working_directory(&mut chat).unwrap();
        assert_ne!(
            first.execution_scope.local_directory,
            chat.execution_scope.local_directory
        );
    }
    #[test]
    fn delegated_relative_directory_is_created_and_cannot_escape_existing_grants() {
        let host = FrameworkHost::temporary(None).unwrap();
        let mut parent = context("parent");
        host.prepare_working_directory(&mut parent).unwrap();
        let root = parent.execution_scope.local_directory.clone().unwrap();
        parent.execution_scope.allowed_local_directories = vec![root.clone()];
        let child = host
            .delegated_execution_scope(&parent, "tasks/small-script")
            .unwrap();
        assert_eq!(
            child.local_directory,
            Some(
                Path::new(&root)
                    .join("tasks/small-script")
                    .to_string_lossy()
                    .into_owned()
            )
        );
        assert!(Path::new(child.local_directory.as_deref().unwrap()).is_dir());
        assert!(
            host.delegated_execution_scope(&parent, "../../another-session")
                .is_err()
        );
        assert_eq!(parent.execution_scope.local_directory, Some(root));
    }
    #[test]
    fn invalid_explicit_directory_is_preserved_instead_of_creating_a_default() {
        let host = FrameworkHost::temporary(None).unwrap();
        let mut ctx = context("explicit");
        ctx.execution_scope = AgentExecutionScope {
            local_directory: Some("/missing/explicit-directory".into()),
            ..Default::default()
        };
        host.prepare_working_directory(&mut ctx).unwrap();
        assert_eq!(
            ctx.execution_scope.local_directory.as_deref(),
            Some("/missing/explicit-directory")
        );
    }
    #[tokio::test]
    async fn context_refresh_keeps_a_delegated_subdirectory() {
        use crate::application::project::ApplicationDirectoryInspector;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let child = root.join("child-task");
        std::fs::create_dir(&child).unwrap();
        let inspector =
            crate::adapters::outbound::application_directory::GitApplicationDirectoryInspector;
        let mut app = inspector.inspect(root.to_str().unwrap());
        app.is_primary = true;
        let mut ctx = context("subdirectory");
        ctx.execution_scope.local_directory = Some(child.to_string_lossy().into_owned());
        ctx.execution_scope.applications = vec![app];
        ctx.execution_scope.allowed_local_directories = vec![root.to_string_lossy().into_owned()];
        crate::application::agent::refresh_execution_context(&mut ctx, Arc::new(inspector))
            .await
            .unwrap();
        assert_eq!(
            ctx.execution_scope.local_directory.as_deref(),
            child.to_str()
        );
    }
}
