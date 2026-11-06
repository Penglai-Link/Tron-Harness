//! Prepare a local checkout only when the user explicitly requests it.
//! Provider metadata and credentials remain on the host.

use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use base64::{Engine, engine::general_purpose::STANDARD};
use eliteforge_agent_connector::Repository;
use serde::Serialize;
use tokio::process::Command;

use crate::{
    application::{
        connector::IntegrationService,
        directory::DirectoryService,
        topic::{AddTopicReference, TopicService},
    },
    domain::{
        connector::{ConnectionError, ConnectionView},
        directory::{DirectoryError, DirectoryView},
        topic::{TopicError, TopicMemo, TopicReferenceKind},
    },
};

#[derive(Debug)]
pub enum CheckoutError {
    Connection(ConnectionError),
    Topic(TopicError),
    Directory(DirectoryError),
    InvalidPath,
    DifferentRepository,
    UnsupportedRemote,
    GitFailed,
    TimedOut,
}

pub enum CheckoutChoice {
    Existing { path: String },
    Clone { path: String, username: String },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckoutOptions {
    pub provider_id: String,
    pub suggested_path: String,
    pub existing: Vec<DirectoryView>,
    pub clone_supported: bool,
}

fn valid_https_remote(
    connection: &ConnectionView,
    repository: &Repository,
) -> Result<String, CheckoutError> {
    let raw = repository
        .clone_url
        .as_deref()
        .ok_or(CheckoutError::UnsupportedRemote)?;
    let remote = url::Url::parse(raw).map_err(|_| CheckoutError::UnsupportedRemote)?;
    let endpoint =
        url::Url::parse(&connection.endpoint).map_err(|_| CheckoutError::UnsupportedRemote)?;
    let expected_host = match connection.provider_id.as_str() {
        "github" => Some("github.com"),
        "gitee" => Some("gitee.com"),
        "gitlab" => endpoint.host_str(),
        _ => None,
    };
    if remote.scheme() != "https"
        || remote.host_str() != expected_host
        || (connection.provider_id == "gitlab"
            && remote.port_or_known_default() != endpoint.port_or_known_default())
        || (connection.provider_id != "gitlab" && remote.port_or_known_default() != Some(443))
        || !remote.username().is_empty()
        || remote.password().is_some()
        || remote.query().is_some()
        || remote.fragment().is_some()
    {
        return Err(CheckoutError::UnsupportedRemote);
    }
    Ok(remote.into())
}

async fn source(
    integrations: &IntegrationService,
    topics: &TopicService,
    workspace: &str,
    actor: &str,
    project: &str,
    reference_code: &str,
) -> Result<(String, Repository, ConnectionView), CheckoutError> {
    let memo = topics.get(actor, project).map_err(CheckoutError::Topic)?;
    let reference = memo
        .references
        .iter()
        .find(|r| {
            r.reference_code == reference_code
                && r.kind == TopicReferenceKind::Repository
                && r.available
        })
        .ok_or(CheckoutError::Topic(TopicError::NotFound))?;
    let code = reference
        .connection_code
        .as_deref()
        .ok_or(CheckoutError::Topic(TopicError::NotFound))?;
    let connection = integrations
        .list(workspace, actor)
        .map_err(CheckoutError::Connection)?
        .into_iter()
        .find(|item| item.connection_code == code)
        .ok_or(CheckoutError::Connection(ConnectionError::NotFound))?;
    let repository = integrations
        .repository(workspace, actor, code, &reference.target_code)
        .await
        .map_err(CheckoutError::Connection)?;
    if repository.id != reference.target_code {
        return Err(CheckoutError::Topic(TopicError::Conflict));
    }
    Ok((code.to_owned(), repository, connection))
}

pub async fn options(
    integrations: &IntegrationService,
    directories: &DirectoryService,
    topics: &TopicService,
    workspace: &str,
    actor: &str,
    project: &str,
    reference_code: &str,
) -> Result<CheckoutOptions, CheckoutError> {
    let (_, repository, connection) = source(
        integrations,
        topics,
        workspace,
        actor,
        project,
        reference_code,
    )
    .await?;
    let clone_supported = valid_https_remote(&connection, &repository).is_ok();
    let mut path = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or(CheckoutError::InvalidPath)?;
    path.push(".tron");
    path.push("repositories");
    path.push(&connection.provider_id);
    for part in repository.full_name.split('/').take(10) {
        let safe: String = part
            .chars()
            .take(100)
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        path.push(if safe.is_empty() { "_" } else { &safe });
    }
    let mut existing = Vec::new();
    for directory in directories
        .list(workspace)
        .map_err(CheckoutError::Directory)?
    {
        if directory.available && existing_matches(Path::new(&directory.path), &repository).await {
            existing.push(directory);
        }
    }
    Ok(CheckoutOptions {
        provider_id: connection.provider_id,
        suggested_path: path.to_str().ok_or(CheckoutError::InvalidPath)?.to_owned(),
        existing,
        clone_supported,
    })
}

fn normalized_remote(value: &str) -> &str {
    value
        .trim()
        .trim_end_matches('/')
        .strip_suffix(".git")
        .unwrap_or(value.trim().trim_end_matches('/'))
}

fn isolated_git() -> Command {
    let mut command = Command::new("git");
    for (key, _) in std::env::vars_os() {
        if key
            .to_string_lossy()
            .to_ascii_uppercase()
            .starts_with("GIT_")
        {
            command.env_remove(key);
        }
    }
    command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env("GIT_TERMINAL_PROMPT", "0");
    command
}

async fn existing_matches(path: &Path, repository: &Repository) -> bool {
    let output = isolated_git()
        .arg("-C")
        .arg(path)
        .args(["config", "--local", "--get", "remote.origin.url"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .await;
    let Ok(output) = output else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    let Ok(remote) = std::str::from_utf8(&output.stdout) else {
        return false;
    };
    let matches = [
        repository.clone_url.as_deref(),
        repository.ssh_url.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|expected| normalized_remote(expected) == normalized_remote(remote));
    if !matches {
        return false;
    }
    let output = isolated_git()
        .arg("-C")
        .arg(path)
        .args(["rev-parse", "--show-toplevel"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .await;
    let Ok(output) = output else {
        return false;
    };
    let Ok(root) = std::str::from_utf8(&output.stdout) else {
        return false;
    };
    // Git terminates this path with a newline; whitespace within the path,
    // including a trailing space, is part of the directory name.
    let root = root.strip_suffix('\n').unwrap_or(root);
    #[cfg(windows)]
    let root = root.strip_suffix('\r').unwrap_or(root);
    output.status.success()
        && std::fs::canonicalize(root)
            .ok()
            .zip(std::fs::canonicalize(path).ok())
            .is_some_and(|(root, selected)| root == selected)
}

fn reserve_destination(value: &str) -> Result<PathBuf, CheckoutError> {
    let requested = Path::new(value);
    if !requested.is_absolute() || requested.file_name().is_none() {
        return Err(CheckoutError::InvalidPath);
    }
    let parent = requested.parent().ok_or(CheckoutError::InvalidPath)?;
    std::fs::create_dir_all(parent).map_err(|_| CheckoutError::InvalidPath)?;
    let parent = std::fs::canonicalize(parent).map_err(|_| CheckoutError::InvalidPath)?;
    let destination = parent.join(requested.file_name().ok_or(CheckoutError::InvalidPath)?);
    // Reserving the destination atomically prevents overwriting an existing path.
    std::fs::create_dir(&destination).map_err(|_| CheckoutError::InvalidPath)?;
    Ok(destination)
}

async fn clone_repository(
    remote: &str,
    token: &str,
    username: &str,
    path: &Path,
) -> Result<(), CheckoutError> {
    let username = username.trim();
    if username.is_empty() || username.contains([':', '\n', '\r']) {
        return Err(CheckoutError::InvalidPath);
    }
    let header = format!(
        "Authorization: Basic {}",
        STANDARD.encode(format!("{username}:{token}"))
    );
    // Ignore inherited Git overrides and tracing so they cannot redirect or
    // record the scoped authorization header.
    let mut command = isolated_git();
    command
        .args([
            "-c",
            "protocol.allow=never",
            "-c",
            "protocol.https.allow=always",
            "-c",
            "http.followRedirects=false",
            "clone",
            "--template=",
            "--",
            remote,
        ])
        .arg(path)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", format!("http.{remote}.extraheader"))
        .env("GIT_CONFIG_VALUE_0", header)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_LFS_SKIP_SMUDGE", "1")
        .env_remove("GIT_TRACE")
        .env_remove("GIT_TRACE_CURL")
        .env_remove("GIT_CURL_VERBOSE")
        .env_remove("GIT_TRACE_PACKET")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let child = command.spawn().map_err(|_| CheckoutError::GitFailed)?;
    let status = tokio::time::timeout(Duration::from_secs(300), child.wait_with_output())
        .await
        .map_err(|_| CheckoutError::TimedOut)?
        .map_err(|_| CheckoutError::GitFailed)?
        .status;
    if !status.success() {
        return Err(CheckoutError::GitFailed);
    }
    Ok(())
}

pub async fn prepare(
    integrations: &IntegrationService,
    directories: &DirectoryService,
    topics: &TopicService,
    workspace: &str,
    actor: &str,
    project: &str,
    reference_code: &str,
    choice: CheckoutChoice,
) -> Result<TopicMemo, CheckoutError> {
    let (connection_code, repository, connection) = source(
        integrations,
        topics,
        workspace,
        actor,
        project,
        reference_code,
    )
    .await?;
    let directory = match choice {
        CheckoutChoice::Existing { path } => {
            let path = Path::new(&path);
            if !path.is_absolute() || !path.is_dir() {
                return Err(CheckoutError::InvalidPath);
            }
            if !existing_matches(path, &repository).await {
                return Err(CheckoutError::DifferentRepository);
            }
            directories
                .register(workspace, path.to_str().ok_or(CheckoutError::InvalidPath)?)
                .map_err(CheckoutError::Directory)?
        }
        CheckoutChoice::Clone { path, username } => {
            let mut existing = None;
            for entry in directories
                .list(workspace)
                .map_err(CheckoutError::Directory)?
            {
                if entry.available && existing_matches(Path::new(&entry.path), &repository).await {
                    existing = Some(entry);
                    break;
                }
            }
            if let Some(existing) = existing {
                existing
            } else {
                if username.trim().is_empty() || username.contains([':', '\n', '\r']) {
                    return Err(CheckoutError::InvalidPath);
                }
                let remote = valid_https_remote(&connection, &repository)?;
                let (_, token) = integrations
                    .git_credential(workspace, actor, &connection_code)
                    .map_err(CheckoutError::Connection)?;
                let destination = reserve_destination(&path)?;
                clone_repository(&remote, &token, &username, &destination).await?;
                directories
                    .register(
                        workspace,
                        destination.to_str().ok_or(CheckoutError::InvalidPath)?,
                    )
                    .map_err(CheckoutError::Directory)?
            }
        }
    };
    // A long clone must not overwrite concurrent edits or attach a repository
    // reference that has meanwhile been removed or become inaccessible.
    let latest = topics.get(actor, project).map_err(CheckoutError::Topic)?;
    if !latest.references.iter().any(|r| {
        r.reference_code == reference_code
            && r.kind == TopicReferenceKind::Repository
            && r.available
            && r.target_code == repository.id
            && r.connection_code.as_deref() == Some(connection_code.as_str())
    }) {
        return Err(CheckoutError::Topic(TopicError::Conflict));
    }
    topics
        .add_reference(
            actor,
            project,
            latest.revision,
            AddTopicReference {
                kind: TopicReferenceKind::Directory,
                target_code: directory.directory_code,
                connection_code: None,
                label: directory.name,
                url: None,
            },
        )
        .map_err(CheckoutError::Topic)
}
