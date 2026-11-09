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

/// Roll back staged filesystem changes when a container transaction fails.
pub(crate) struct StagedSessionDirectories {
    paths: Vec<(PathBuf, PathBuf)>,
}

impl StagedSessionDirectories {
    pub(crate) fn commit(mut self) -> Result<(), String> {
        let mut failure = None;
        for (_, staged) in self.paths.drain(..) {
            let result = if staged.is_symlink() { fs::remove_file(&staged) } else { fs::remove_dir_all(&staged) };
            if let Err(error) = result {
                tracing::error!(path = %staged.display(), %error, "could not remove staged directory after committed deletion");
                failure = Some(format!("{}: {error}", staged.display()));
            }
        }
        failure.map_or(Ok(()), Err)
    }
}

impl Drop for StagedSessionDirectories {
    fn drop(&mut self) {
        for (original, staged) in self.paths.iter().rev() {
            if let Err(error) = fs::rename(staged, original) {
                tracing::error!(path = %original.display(), staged = %staged.display(), %error, "could not restore session directory after rolled back deletion");
            }
        }
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

    pub(crate) async fn stage_session_working_directories(
        &self,
        sessions: Vec<(String, Option<String>)>,
    ) -> Result<StagedSessionDirectories, String> {
        let root = self.0.path.with_extension("workspaces");
        let directories = sessions.into_iter().map(|(session, user)| self.managed_working_directory("sessions", user.as_deref(), &session)).collect::<Vec<_>>();
        tokio::task::spawn_blocking(move || {
            let mut staged = StagedSessionDirectories { paths: Vec::new() };
            'directory: for directory in directories {
                for parent in [root.clone(), root.join("sessions"), directory.parent().unwrap().to_owned()] {
                    match fs::symlink_metadata(&parent) {
                        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
                        Ok(_) => return Err(format!("session working directory parent is not managed: {}", parent.display())),
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue 'directory,
                        Err(error) => return Err(format!("could not inspect session working directory: {error}")),
                    }
                }
                match fs::symlink_metadata(&directory) {
                    Ok(metadata) if metadata.is_dir() || metadata.file_type().is_symlink() => {}
                    Ok(_) => return Err(format!("session working path is not a directory: {}", directory.display())),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(error) => return Err(format!("could not inspect session working directory: {error}")),
                }
                let target = directory.with_file_name(format!(".deleting-{:032x}", rand::random::<u128>()));
                match fs::rename(&directory, &target) {
                    Ok(()) => staged.paths.push((directory, target)),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(format!("could not stage session working directory: {error}")),
                }
            }
            Ok(staged)
        }).await.map_err(|error| error.to_string())?
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
