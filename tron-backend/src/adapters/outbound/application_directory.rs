mod instruction_files;
use std::{
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

use crate::{
    application::project::ApplicationDirectoryInspector,
    domain::agent::{ApplicationDirectoryContext, ApplicationDirectoryStatus},
};

pub struct GitApplicationDirectoryInspector;

impl ApplicationDirectoryInspector for GitApplicationDirectoryInspector {
    fn instruction_files(
        &self,
        directory: &str,
    ) -> Result<Vec<crate::application::project::ProjectInstructionFile>, String> {
        instruction_files::read(directory)
    }

    fn inspect(&self, directory: &str) -> ApplicationDirectoryContext {
        let mut result = ApplicationDirectoryContext {
            app_code: String::new(),
            app_name: String::new(),
            is_primary: false,
            configured_directory: directory.into(),
            working_directory: None,
            repository_root: None,
            branch_name: None,
            head_commit: None,
            worktree_path: None,
            is_worktree: false,
            status: ApplicationDirectoryStatus::MissingDirectory,
        };
        let path = Path::new(directory);
        if !path.is_absolute() || !path.is_dir() {
            return result;
        }
        let Ok(path) = path.canonicalize() else {
            return result;
        };
        result.working_directory = Some(path.to_string_lossy().into_owned());
        result.status = ApplicationDirectoryStatus::Unavailable;
        match git(&path, &["rev-parse", "--is-inside-work-tree"]) {
            Ok(Some(value)) if value == "true" => {}
            Ok(_) => {
                result.status = ApplicationDirectoryStatus::NotGit;
                return result;
            }
            Err(()) => return result,
        }
        let (Ok(Some(root)), Ok(Some(git_dir)), Ok(Some(common_dir)), Ok(branch), Ok(head)) = (
            git(&path, &["rev-parse", "--show-toplevel"]),
            git(&path, &["rev-parse", "--absolute-git-dir"]),
            git(
                &path,
                &["rev-parse", "--path-format=absolute", "--git-common-dir"],
            ),
            git(&path, &["symbolic-ref", "--quiet", "--short", "HEAD"]),
            git(&path, &["rev-parse", "--verify", "HEAD"]),
        ) else {
            return result;
        };
        result.is_worktree = Path::new(&git_dir) != Path::new(&common_dir);
        result.worktree_path = Some(root.clone());
        result.repository_root = Some(root);
        result.branch_name = branch;
        result.head_commit = head;
        result.status = ApplicationDirectoryStatus::Ready;
        result
    }
    fn list_worktrees(&self, directory: &str) -> Vec<crate::domain::agent::GitWorktreeContext> {
        let path = Path::new(directory);
        let mut worktrees = Vec::new();
        if let Ok(Some(list)) = git(&path, &["worktree", "list", "--porcelain", "-z"]) {
            let mut current: Option<crate::domain::agent::GitWorktreeContext> = None;
            for field in list.split('\0') {
                if let Some(path) = field.strip_prefix("worktree ") {
                    if let Some(item) = current.take() {
                        worktrees.push(item);
                    }
                    current = Some(crate::domain::agent::GitWorktreeContext {
                        path: path.into(),
                        branch_name: None,
                        head_commit: None,
                    });
                } else if let Some(item) = current.as_mut() {
                    if let Some(branch) = field.strip_prefix("branch refs/heads/") {
                        item.branch_name = Some(branch.into());
                    }
                    if let Some(head) = field.strip_prefix("HEAD ") {
                        item.head_commit = Some(head.into());
                    }
                }
            }
            if let Some(item) = current {
                worktrees.push(item);
            }
        }
        worktrees
    }
}

fn git(path: &Path, args: &[&str]) -> Result<Option<String>, ()> {
    let mut command = Command::new("git");
    // Inherited Git environment overrides can point at a different checkout.
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("GIT_") {
            command.env_remove(name);
        }
    }
    let mut child = command
        .arg("-C")
        .arg(path)
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| ())?;
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(5)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(());
            }
        }
    }
    let output = child.wait_with_output().map_err(|_| ())?;
    if !output.status.success() {
        return Ok(None);
    }
    let value = String::from_utf8(output.stdout).map_err(|_| ())?;
    Ok(Some(value.trim_end_matches(['\r', '\n']).to_string()).filter(|value| !value.is_empty()))
}
