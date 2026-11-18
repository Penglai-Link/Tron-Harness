//! Session management use cases: cross-workspace listing, search, status
//! filtering, batch archive/delete/move and guaranteed run stopping.
use super::ChatService;
use crate::{
    application::agent::{ActiveRunError, ActiveRunState},
    domain::chat::{ChatError, ChatSession, ChatSessionSourceType, ChatSessionStatus},
};

/// Listing filters accepted by the management page. `None` means "all".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatSessionManagementQuery {
    pub workspace_code: Option<String>,
    pub status: Option<ChatSessionStatus>,
    pub source_type: Option<ChatSessionSourceType>,
    pub search: Option<String>,
    pub current: u32,
    pub size: u32,
}

/// A management row: the session projection plus the activity signal the page
/// needs to decide which destructive operations are safe.
#[derive(Debug, Clone)]
pub struct ManagedChatSession {
    pub session: ChatSession,
    pub active_run_code: Option<String>,
    pub active_run_state: Option<ActiveRunState>,
}

impl ManagedChatSession {
    pub fn running(&self) -> bool {
        self.active_run_code.is_some()
            && self
                .active_run_state
                .is_none_or(|state| state == ActiveRunState::Running)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatSessionStopError {
    SessionNotFound,
    StopTimeout,
    Storage,
}

/// Result of one item in a batch operation.
#[derive(Debug, Clone)]
pub struct ChatSessionBatchItem {
    pub session_code: String,
    pub success: bool,
    pub error_code: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChatSessionBatchResult {
    pub succeeded: u32,
    pub failed: u32,
    pub items: Vec<ChatSessionBatchItem>,
}

/// Cross-workspace move command. `target_project_code` is the topic in the
/// target workspace; `None` moves to the workspace's quick chats.
#[derive(Debug, Clone)]
pub struct ChatSessionMoveCommand {
    pub session_codes: Vec<String>,
    pub target_workspace_code: String,
    pub target_project_code: Option<String>,
}

const STOP_SETTLE_TIMEOUT_MS: u64 = 15_000;
const STOP_SETTLE_POLL_MS: u64 = 250;

impl ChatService {
    /// Paged management listing across every workspace.
    pub fn list_managed_sessions(
        &self,
        query: ChatSessionManagementQuery,
    ) -> Result<(Vec<ManagedChatSession>, u64), ChatError> {
        let current = query.current.max(1);
        let size = query.size.clamp(1, 100);
        let offset = current.saturating_sub(1).saturating_mul(size);
        let search = query
            .search
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let (sessions, total) = self.repository().list_managed_sessions(
            query
                .workspace_code
                .as_deref()
                .filter(|code| !code.is_empty()),
            query.status,
            query.source_type,
            search,
            offset,
            size,
        )?;
        let mut rows = Vec::with_capacity(sessions.len());
        for session in sessions {
            let snapshot = self
                .active_runs
                .snapshot(&session.session_code)
                .map_err(|_| ChatError::StorageFailure)?;
            rows.push(ManagedChatSession {
                active_run_code: snapshot.as_ref().map(|run| run.run_code.clone()),
                active_run_state: snapshot.as_ref().map(|run| run.state),
                session,
            });
        }
        Ok((rows, total))
    }

    /// Stop every active run of the given sessions: main run, subagents,
    /// collaboration threads and detached children. Waits until all runs
    /// leave the registry or the settle timeout expires.
    pub async fn stop_sessions(
        &self,
        session_codes: &[String],
    ) -> Result<Vec<String>, ChatSessionStopError> {
        let codes: Vec<String> = session_codes
            .iter()
            .map(|code| code.trim().to_owned())
            .filter(|code| !code.is_empty())
            .collect();
        if codes.is_empty() {
            return Ok(Vec::new());
        }
        let existing = self
            .repository()
            .sessions_by_codes(&codes)
            .map_err(|_| ChatSessionStopError::Storage)?;
        if existing.len() != codes.len() {
            return Err(ChatSessionStopError::SessionNotFound);
        }
        self.stop_sessions_unchecked(&codes).await
    }

    /// Same as [`Self::stop_sessions`] but skips the existence check; callers
    /// that already loaded the sessions use this directly.
    pub async fn stop_sessions_unchecked(
        &self,
        codes: &[String],
    ) -> Result<Vec<String>, ChatSessionStopError> {
        // Main runs stop through the registry; a RunMismatch from a concurrent
        // replacement is retried on the next iteration.
        let mut stopped: Vec<String> = Vec::new();
        for code in codes {
            loop {
                let snapshot = self
                    .active_runs
                    .snapshot(code)
                    .map_err(|_| ChatSessionStopError::Storage)?;
                let Some(run) = snapshot else { break };
                if run.state == ActiveRunState::Finishing {
                    break;
                }
                match self.active_runs.request_stop(code, &run.run_code) {
                    Ok(_) => {
                        stopped.push(code.clone());
                        break;
                    }
                    Err(ActiveRunError::NoActiveRun) | Err(ActiveRunError::RunMismatch { .. }) => {
                        continue;
                    }
                    Err(_) => return Err(ChatSessionStopError::Storage),
                }
            }
        }
        // Subagent child runs stop with their parent run; the request above
        // also cancels the continuation group root. Detached children and
        // Threads own independent lifetimes and must be stopped directly.
        if let Some(framework) = self.framework_host() {
            let threads = framework
                .cancel_threads_for_sessions(codes)
                .await
                .map_err(|_| ChatSessionStopError::Storage)?;
            if threads > 0 {
                tracing::info!(
                    sessions = codes.len(),
                    threads,
                    "management stopped threads"
                );
            }
            let children = framework.cancel_children_for_sessions(codes);
            if children > 0 {
                tracing::info!(
                    sessions = codes.len(),
                    children,
                    "management stopped detached children"
                );
            }
        }
        // Wait for the registry to drain so archive/delete cannot race a run
        // that is still persisting its terminal state.
        let deadline = tokio::time::Instant::now()
            + tokio::time::Duration::from_millis(STOP_SETTLE_TIMEOUT_MS);
        loop {
            let busy = codes
                .iter()
                .filter(|code| {
                    self.active_runs
                        .snapshot(code)
                        .map(|snapshot| snapshot.is_some())
                        .unwrap_or(true)
                })
                .count();
            if busy == 0 {
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                tracing::warn!(
                    sessions = codes.len(),
                    busy,
                    "session stop settle timed out"
                );
                return Err(ChatSessionStopError::StopTimeout);
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(STOP_SETTLE_POLL_MS)).await;
        }
        Ok(stopped)
    }

    /// Ensure the sessions are quiescent before a destructive mutation.
    /// Cancels ACTIVE goals because they admit new runs autonomously.
    pub async fn ensure_sessions_stopped(&self, codes: &[String]) -> Result<(), ChatError> {
        for code in codes {
            if let Some(memory) = self.execution_memory.as_ref() {
                let goal = memory
                    .get_active_goal(code)
                    .map_err(|_| ChatError::StorageFailure)?;
                if goal.is_some_and(|goal| goal.goal.status == "ACTIVE") {
                    memory
                        .cancel_goal(crate::application::short_term_memory::GoalSessionCommand {
                            session_code: code.clone(),
                            goal_code: None,
                        })
                        .map_err(|_| ChatError::StorageFailure)?;
                }
            }
        }
        self.stop_sessions_unchecked(codes)
            .await
            .map_err(|_| ChatError::SessionRunActive)?;
        Ok(())
    }

    /// Archive sessions in batch. Sessions that still run after the caller's
    /// stop attempt fail per item instead of blocking the whole batch.
    pub fn archive_sessions(
        &self,
        session_codes: &[String],
    ) -> Result<ChatSessionBatchResult, ChatError> {
        self.batch_sessions(session_codes, "archive", |service, code| {
            service.archive_session(code).map(|_session| ())
        })
    }

    /// Permanently delete sessions in batch, including archived ones.
    pub fn delete_sessions(
        &self,
        session_codes: &[String],
    ) -> Result<ChatSessionBatchResult, ChatError> {
        self.batch_sessions(session_codes, "delete", |service, code| {
            service.delete_session(code).map(|_session| ())
        })
    }

    /// Move manual chat sessions to a target workspace/topic. Directory-bound
    /// sessions cannot leave their directory and fail per item.
    pub fn move_sessions(
        &self,
        command: ChatSessionMoveCommand,
    ) -> Result<ChatSessionBatchResult, ChatError> {
        let target_workspace = command.target_workspace_code.trim().to_owned();
        if target_workspace.is_empty() {
            return Err(ChatError::WorkspaceRequired);
        }
        let target_project = command
            .target_project_code
            .as_deref()
            .map(str::trim)
            .filter(|code| !code.is_empty())
            .map(str::to_owned);
        self.batch_sessions(&command.session_codes, "move", |service, code| {
            service
                .move_managed_session(code, &target_workspace, target_project.as_deref())
                .map(|_session| ())
        })
    }

    /// Move a manual chat session to a target workspace (and optionally a
    /// topic inside it). Cross-workspace moves rewrite the owning workspace
    /// link; directory-bound chats refuse to move.
    pub fn move_managed_session(
        &self,
        session_code: &str,
        target_workspace: &str,
        target_project: Option<&str>,
    ) -> Result<ChatSession, ChatError> {
        let _guard =
            self.active_runs
                .idle_session_guard(session_code)
                .map_err(|error| match error {
                    ActiveRunError::AlreadyActive => ChatError::SessionRunActive,
                    ActiveRunError::SessionArchived => ChatError::SessionArchived,
                    _ => ChatError::StorageFailure,
                })?;
        if self
            .execution_memory
            .as_ref()
            .map(|memory| memory.get_active_goal(session_code))
            .transpose()
            .map_err(|_| ChatError::StorageFailure)?
            .flatten()
            .is_some_and(|goal| goal.goal.status == "ACTIVE")
        {
            return Err(ChatError::SessionRunActive);
        }
        let session = self.repository().get_session(session_code)?;
        if session.status != ChatSessionStatus::Active {
            return Err(ChatError::SessionArchived);
        }
        if session.source_type != ChatSessionSourceType::Manual
            || session.source_code.as_deref().is_some_and(|source| {
                source.starts_with("__assistant:") || source.starts_with("__knowledge:")
            })
        {
            return Err(ChatError::WorkspaceForbidden);
        }
        if session.directory_code.is_some() {
            return Err(ChatError::DirectoryFixed);
        }
        if let Some(code) = target_project {
            self.projects
                .as_ref()
                .ok_or(ChatError::ProjectInvalid)?
                .resolve_execution_context(target_workspace, code)
                .map_err(|_| ChatError::ProjectInvalid)?;
        }
        // A working directory from the old workspace cannot resolve in the
        // new one; a cross-workspace move clears it.
        let keep_working = session.workspace_code == target_workspace;
        let working_directory = if keep_working {
            session.working_directory_code.as_deref()
        } else {
            None
        };
        let reference = format!(
            "TOPIC-REF-{}",
            self.id_provider
                .next_id()
                .map_err(|_| ChatError::StorageFailure)?
                .value()
        );
        self.repository().update_managed_session_location(
            session_code,
            target_workspace,
            target_project,
            working_directory,
            &reference,
        )?;
        self.repository().get_session(session_code)
    }

    /// Shared per-item executor for the batch endpoints. Each item resolves
    /// independently so one failure never rolls back the others. The HTTP
    /// layer validates the request shape; this only guards against misuse.
    fn batch_sessions(
        &self,
        session_codes: &[String],
        operation: &'static str,
        mut execute: impl FnMut(&Self, &str) -> Result<(), ChatError>,
    ) -> Result<ChatSessionBatchResult, ChatError> {
        let mut codes: Vec<String> = Vec::with_capacity(session_codes.len());
        let mut seen = std::collections::BTreeSet::new();
        for code in session_codes {
            let code = code.trim().to_owned();
            if !code.is_empty() && seen.insert(code.clone()) {
                codes.push(code);
            }
        }
        if codes.is_empty() {
            return Err(ChatError::SessionNotFound);
        }
        let mut items = Vec::with_capacity(codes.len());
        let mut succeeded = 0u32;
        for code in &codes {
            // Re-check run admission: a session that could not be stopped
            // fails this item instead of blocking the whole batch.
            let admission = self
                .active_runs
                .snapshot(code)
                .map_err(|_| ChatError::StorageFailure)?;
            if admission.is_some() {
                items.push(ChatSessionBatchItem {
                    session_code: code.clone(),
                    success: false,
                    error_code: Some("CHAT_SESSION_RUN_ACTIVE".into()),
                    message: None,
                });
                continue;
            }
            match execute(self, code) {
                Ok(()) => {
                    succeeded += 1;
                    items.push(ChatSessionBatchItem {
                        session_code: code.clone(),
                        success: true,
                        error_code: None,
                        message: None,
                    });
                }
                Err(error) => items.push(failed_item(code.clone(), error)),
            }
        }
        tracing::info!(
            operation,
            total = codes.len(),
            succeeded,
            "session management batch"
        );
        Ok(ChatSessionBatchResult {
            succeeded,
            failed: codes.len() as u32 - succeeded,
            items,
        })
    }
}

fn failed_item(session_code: String, error: ChatError) -> ChatSessionBatchItem {
    let (code, message) = match error {
        ChatError::SessionNotFound => ("CHAT_SESSION_NOT_FOUND", "chat session was not found"),
        ChatError::SessionRunActive => (
            "CHAT_SESSION_RUN_ACTIVE",
            "chat session still has an active run",
        ),
        ChatError::SessionArchived => ("CHAT_SESSION_ARCHIVED", "chat session is archived"),
        ChatError::WorkspaceForbidden => (
            "CHAT_WORKSPACE_FORBIDDEN",
            "chat session cannot leave its workspace",
        ),
        ChatError::DirectoryFixed => (
            "CHAT_DIRECTORY_FIXED",
            "directory chats must stay in their directory",
        ),
        ChatError::ProjectInvalid => ("CHAT_PROJECT_INVALID", "target topic is invalid"),
        ChatError::DirectoryInvalid => ("CHAT_DIRECTORY_INVALID", "target directory is invalid"),
        _ => (
            "CHAT_SESSION_MANAGEMENT_FAILED",
            "session management operation failed; retry this item",
        ),
    };
    ChatSessionBatchItem {
        session_code,
        success: false,
        error_code: Some(code.to_owned()),
        message: Some(message.to_owned()),
    }
}
