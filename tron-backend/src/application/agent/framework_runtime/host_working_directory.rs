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
    /// dsh's immutable session cwd identifies the shared UI group. The CLI
    /// adapter sets effective cwd and standing write root to the actual task.
    pub(crate) fn dsh_workspace_options(
        &self,
    ) -> eliteforge_agent_capability_external::DshWorkspaceOptions {
        eliteforge_agent_capability_external::DshWorkspaceOptions {
            directory: self.0.path.with_extension("workspaces").join("sessions"),
            title: "Tron harness".into(),
        }
    }

    fn managed_working_directory(&self, kind: &str, user: Option<&str>, id: &str) -> PathBuf {
        self.0
            .path
            .with_extension("workspaces")
            .join(kind)
            .join(component(user.unwrap_or("system")))
            .join(component(id))
    }

    /// Delete only this session's managed directory, never its selected project
    /// or a Dude's shared directory. Reuse the creation path and reject symlinked
    /// parents so a replaced path cannot redirect cleanup outside the managed root.
    pub(crate) async fn delete_session_working_directory(
        &self,
        session: &str,
        user: Option<&str>,
    ) -> Result<(), String> {
        let root = self.0.path.with_extension("workspaces");
        let directory = self.managed_working_directory("sessions", user, session);
        tokio::task::spawn_blocking(move || {
            for parent in [
                root.clone(),
                root.join("sessions"),
                directory.parent().unwrap().to_owned(),
            ] {
                let metadata = match fs::symlink_metadata(&parent) {
                    Ok(metadata) => metadata,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                    Err(error) => {
                        return Err(format!(
                            "could not inspect session working directory: {error}"
                        ));
                    }
                };
                if !metadata.is_dir() || metadata.file_type().is_symlink() {
                    return Err(format!(
                        "session working directory parent is not a managed directory: {}",
                        parent.display()
                    ));
                }
            }
            match fs::remove_dir_all(&directory) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(format!(
                    "could not remove session working directory {}: {error}",
                    directory.display()
                )),
            }
        })
        .await
        .map_err(|error| format!("session working directory cleanup failed: {error}"))?
    }

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
        let directory = self.managed_working_directory(kind, context.user_code.as_deref(), id);
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
    fn dsh_group_is_the_shared_session_parent_without_changing_task_directories() {
        let host = FrameworkHost::temporary(None).unwrap();
        let group = host.dsh_workspace_options();
        assert_eq!(group.title, "Tron harness");
        assert_eq!(
            group.directory,
            host.0.path.with_extension("workspaces").join("sessions")
        );
        let mut first = context("first");
        let mut other = context("second");
        host.prepare_working_directory(&mut first).unwrap();
        host.prepare_working_directory(&mut other).unwrap();
        for ctx in [&first, &other] {
            assert!(
                Path::new(ctx.execution_scope.local_directory.as_ref().unwrap())
                    .starts_with(group.directory.canonicalize().unwrap())
            );
        }
        assert_ne!(
            first.execution_scope.local_directory,
            other.execution_scope.local_directory
        );
        let project = tempfile::tempdir().unwrap();
        let mut selected = context("project");
        selected.execution_scope.local_directory = Some(project.path().to_str().unwrap().into());
        host.prepare_working_directory(&mut selected).unwrap();
        assert_eq!(
            selected.execution_scope.local_directory.as_deref(),
            project.path().to_str()
        );
        assert_eq!(host.dsh_workspace_options().directory, group.directory);
        assert_ne!(
            FrameworkHost::temporary(None)
                .unwrap()
                .dsh_workspace_options()
                .directory,
            group.directory
        );
    }
    #[tokio::test]
    async fn deletion_removes_only_the_session_owned_directory() {
        let host = FrameworkHost::temporary(None).unwrap();
        let mut deleted = context("deleted");
        host.prepare_working_directory(&mut deleted).unwrap();
        let directory = PathBuf::from(deleted.execution_scope.local_directory.as_ref().unwrap());
        fs::create_dir(directory.join("nested")).unwrap();
        fs::write(directory.join("nested/result.txt"), "session output").unwrap();
        let mut other = context("other");
        host.prepare_working_directory(&mut other).unwrap();
        let mut other_user = context("deleted");
        other_user.user_code = Some("another-user".into());
        host.prepare_working_directory(&mut other_user).unwrap();
        let mut dude = context("deleted");
        dude.collaboration_owner = Some(AgentCollaborationOwner {
            kind: "DUDE".into(),
            code: "main-dude".into(),
            origin_session_code: None,
            task_code: None,
        });
        host.prepare_working_directory(&mut dude).unwrap();
        let project = tempfile::tempdir().unwrap();
        fs::write(project.path().join("source.rs"), "project source").unwrap();
        let mut selected = context("project-session");
        selected.execution_scope.local_directory =
            Some(project.path().to_string_lossy().into_owned());
        host.prepare_working_directory(&mut selected).unwrap();

        host.delete_session_working_directory("deleted", deleted.user_code.as_deref())
            .await
            .unwrap();
        assert!(!directory.exists());
        for survivor in [other, other_user, dude, selected] {
            assert!(Path::new(survivor.execution_scope.local_directory.as_ref().unwrap()).is_dir());
        }
        host.delete_session_working_directory("project-session", Some("user"))
            .await
            .unwrap();
        assert!(project.path().join("source.rs").is_file());
        host.delete_session_working_directory("deleted", deleted.user_code.as_deref())
            .await
            .unwrap();
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn deletion_does_not_follow_session_or_parent_symlinks() {
        use std::os::unix::fs::symlink;
        let host = FrameworkHost::temporary(None).unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("keep.txt"), "external data").unwrap();
        let directory = host.managed_working_directory("sessions", Some("user"), "linked");
        let parent = directory.parent().unwrap();
        fs::create_dir_all(parent).unwrap();
        symlink(outside.path(), &directory).unwrap();
        host.delete_session_working_directory("linked", Some("user"))
            .await
            .unwrap();
        assert!(fs::symlink_metadata(&directory).is_err());
        assert!(outside.path().join("keep.txt").is_file());
        fs::remove_dir(parent).unwrap();
        symlink(outside.path(), parent).unwrap();
        assert!(
            host.delete_session_working_directory("linked", Some("user"))
                .await
                .is_err()
        );
        assert!(outside.path().join("keep.txt").is_file());
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
