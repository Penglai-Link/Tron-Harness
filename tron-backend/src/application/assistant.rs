//! PersonAI orchestration ports; the host retains execution independently of UI streams.
mod notification_tools;
pub(crate) mod soul;
mod tools;
mod work_changes;
use crate::application::agent::{
    AgentManagementService, CreateWorkspaceInternalAgentCommand,
    UpdateWorkspaceInternalAgentDefinitionCommand,
};
use crate::domain::agent::InternalAgentToolPolicy;
use crate::domain::assistant::*;
use serde_json::Value;
use std::sync::{Arc, Mutex};
pub use tools::register_assistant_tools;

pub trait AssistantRepository: Send {
    fn soul(&mut self, _code: &str) -> Result<AssistantSoul, AssistantError> {
        Err(AssistantError::Storage)
    }
    fn save_soul(&mut self, _soul: &AssistantSoul) -> Result<(), AssistantError> {
        Err(AssistantError::Storage)
    }

    fn create(
        &mut self,
        assistant: &Assistant,
        definition: &crate::domain::agent::PlatformInternalAgentConfig,
    ) -> Result<(), AssistantError>;

    fn list(&mut self) -> Result<Vec<Assistant>, AssistantError>;
    fn save(&mut self, assistant: &Assistant) -> Result<(), AssistantError>;
    /// Atomically remove the PersonAI, its soul, Agent definition, and resource grants.
    fn delete(&mut self, code: &str) -> Result<(), AssistantError>;
    fn designate_main(&mut self, code: &str) -> Result<(), AssistantError>;
    fn resolve_session(
        &mut self,
        session: &AssistantSession,
    ) -> Result<AssistantSession, AssistantError>;
    fn sessions(&mut self, code: &str) -> Result<Vec<AssistantSession>, AssistantError>;
    /// Old heartbeat sessions eligible for retirement, excluding active or referenced work.
    fn expired_heartbeat_sessions(&mut self) -> Result<Vec<String>, AssistantError> {
        Ok(Vec::new())
    }
    fn delete_empty_heartbeat_session(&mut self, _code: &str) -> Result<(), AssistantError> {
        Ok(())
    }
    fn session(&mut self, code: &str) -> Result<Option<AssistantSession>, AssistantError>;
    fn enqueue(&mut self, event: &AssistantEvent) -> Result<AssistantEvent, AssistantError>;
    fn events(&mut self, code: &str) -> Result<Vec<AssistantEvent>, AssistantError>;
    fn activity_events(&mut self, code: &str) -> Result<Vec<AssistantEvent>, AssistantError>;
    /// Retire old finished heartbeat events without touching pending work.
    fn prune_heartbeat_events(&mut self) -> Result<usize, AssistantError>;
    fn claim(&mut self, now: i64, lease_ms: i64) -> Result<Option<AssistantEvent>, AssistantError>;
    fn finish(
        &mut self,
        id: &str,
        token: &str,
        status: AssistantEventStatus,
        result: &str,
        now: i64,
    ) -> Result<(), AssistantError>;
    fn attach_session(
        &mut self,
        id: &str,
        token: &str,
        session: &str,
    ) -> Result<(), AssistantError>;
    fn renew(
        &mut self,
        event: &AssistantEvent,
        now: i64,
        lease_ms: i64,
    ) -> Result<(), AssistantError>;
    fn retry(
        &mut self,
        event: &AssistantEvent,
        now: i64,
        delay_ms: i64,
        error: &str,
    ) -> Result<(), AssistantError>;
    fn defer_busy(
        &mut self,
        event: &AssistantEvent,
        now: i64,
        delay_ms: i64,
    ) -> Result<(), AssistantError> {
        self.retry(event, now, delay_ms, "Waiting for the current PersonAI run")
    }
    fn resolve_awaiting_input(
        &mut self,
        session: &str,
        summary: &str,
        now: i64,
    ) -> Result<(), AssistantError>;
    fn schedule_heartbeats(&mut self, now: i64) -> Result<(), AssistantError>;
    fn notify(
        &mut self,
        notification: &PlatformNotification,
    ) -> Result<PlatformNotification, AssistantError>;
    /// Persist the new notice and retire prior actionable notices in one unit.
    fn notify_replacing(
        &mut self,
        notification: &PlatformNotification,
        prior_kinds: &[String],
    ) -> Result<PlatformNotification, AssistantError> {
        let _ = prior_kinds;
        self.notify(notification)
    }
    fn record_work_change(
        &mut self,
        change: &AssistantWorkChange,
        fingerprint: &str,
    ) -> Result<AssistantWorkChange, AssistantError>;
    fn scoped_pending_work_changes(
        &mut self,
        _assistant: &str,
        _user: &str,
    ) -> Result<Vec<AssistantWorkChange>, AssistantError> {
        Err(AssistantError::Storage)
    }
    /// Recent assessments and delivery receipts, restricted to the session owner.
    fn scoped_recent_work_changes(
        &mut self,
        _assistant: &str,
        _user: &str,
    ) -> Result<Vec<AssistantWorkChange>, AssistantError> {
        Err(AssistantError::Storage)
    }
    fn pending_work_changes(
        &mut self,
        assistant: &str,
    ) -> Result<Vec<AssistantWorkChange>, AssistantError>;
    fn work_changes(
        &mut self,
        assistant: &str,
        task: &str,
    ) -> Result<Vec<AssistantWorkChange>, AssistantError>;
    fn work_change(
        &mut self,
        assistant: &str,
        id: &str,
    ) -> Result<AssistantWorkChange, AssistantError>;
    fn resolve_work_change(
        &mut self,
        assistant: &str,
        id: &str,
        resolution: &str,
        evidence: &[String],
        now: i64,
    ) -> Result<AssistantWorkChange, AssistantError>;
    fn notify_work_change(
        &mut self,
        assistant: &str,
        id: &str,
        title: &str,
        body: &str,
        now: i64,
    ) -> Result<PlatformNotification, AssistantError>;
    fn notifications(&mut self) -> Result<Vec<PlatformNotification>, AssistantError>;
    /// Current notification records visible to this PersonAI and session owner.
    fn scoped_notifications(
        &mut self,
        _assistant: &str,
        _user: &str,
        _id: Option<&str>,
        _current: usize,
        _size: usize,
        _include_archived: bool,
    ) -> Result<(usize, Vec<PlatformNotification>), AssistantError> {
        Err(AssistantError::Storage)
    }
    fn update_notification(
        &mut self,
        id: &str,
        read: bool,
        archive: bool,
        now: i64,
    ) -> Result<(), AssistantError>;
    fn update_notifications(
        &mut self,
        _ids: &[String],
        _archive: bool,
        _now: i64,
    ) -> Result<usize, AssistantError> {
        Err(AssistantError::Storage)
    }
    fn delete_archived_notification(&mut self, id: &str) -> Result<(), AssistantError>;
    fn delete_archived_notifications(&mut self) -> Result<usize, AssistantError>;
}
#[derive(Debug, Clone)]
pub struct CreateAssistantCommand {
    pub definition: CreateWorkspaceInternalAgentCommand,
    pub kind: AssistantKind,
    pub enabled: bool,
    pub heartbeat_minutes: u32,
    pub memory_auto_approve: bool,
}
#[derive(Debug, Clone, Default)]
pub struct UpdateAssistantCommand {
    pub definition: UpdateWorkspaceInternalAgentDefinitionCommand,
    pub enabled: Option<bool>,
    pub heartbeat_minutes: Option<u32>,
    pub memory_auto_approve: Option<bool>,
}
#[derive(Debug, Clone)]
pub struct AssistantEventInput {
    pub assistant_code: String,
    pub source: AssistantEventSource,
    pub dedup_key: String,
    pub session_code: Option<String>,
    pub task_code: Option<String>,
    pub workspace_code: Option<String>,
    pub payload: Value,
    pub available_at: i64,
}
#[derive(Debug, Clone)]
pub struct NotificationInput {
    pub dedup_key: String,
    pub assistant_code: Option<String>,
    pub title: String,
    pub body: String,
    pub category: String,
    pub target: Value,
}
#[derive(Clone)]
pub struct AssistantService {
    repository: Arc<Mutex<Box<dyn AssistantRepository>>>,
    management: AgentManagementService,
}
impl AssistantService {
    pub fn new(
        repository: impl AssistantRepository + 'static,
        management: AgentManagementService,
    ) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
            management,
        }
    }
    fn repo(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, Box<dyn AssistantRepository>>, AssistantError> {
        self.repository.lock().map_err(|_| AssistantError::Storage)
    }
    pub fn list(&self) -> Result<Vec<Assistant>, AssistantError> {
        self.repo()?.list()
    }
    pub fn get(&self, code: &str) -> Result<Assistant, AssistantError> {
        self.list()?
            .into_iter()
            .find(|d| d.assistant_code == code)
            .ok_or(AssistantError::NotFound)
    }
    pub fn definition(&self, code: &str) -> Result<InternalAgentToolPolicy, AssistantError> {
        self.get(code)?;
        self.management
            .all_platform_internal_agents()
            .map_err(|_| AssistantError::Storage)?
            .into_iter()
            .find(|d| d.agent_code == code)
            .ok_or(AssistantError::NotFound)
    }
    pub fn bootstrap(&self) -> Result<(), AssistantError> {
        let main = self
            .management
            .all_platform_internal_agents()
            .map_err(|_| AssistantError::Storage)?
            .into_iter()
            .find(|a| a.agent_code == "main-agent")
            .ok_or(AssistantError::Storage)?;
        for (code,name,kind,prompt) in [
            ("main-assistant","PersonAI",AssistantKind::Main,main.role_prompt.clone()),
            ("memory-assistant","记忆整理",AssistantKind::Memory,"整理当前用户所有工作区的稳定事实和已验证经验，排除自身及自身派发的记录。经验转为 Skill 时必须加载并遵循已安装启用的 skill-creator；产物进入待批准区，用户批准后安装。保留来源、避免重复，不将临时进度写入长期记忆。".into())
        ] {
            if self.get(code).is_ok() { continue; }
            self.management.initialize_assistant_definition(code,name,Some(prompt)).map_err(|_|AssistantError::Storage)?;
            let now=chrono::Utc::now().timestamp_millis();
            self.repo()?.save(&Assistant{assistant_code:code.into(),kind,enabled:true,heartbeat_minutes:if kind==AssistantKind::Memory{360}else{5},memory_auto_approve:false,is_main:false,created_at:now,updated_at:now})?;
        }
        if self.main().is_err() {
            self.designate_main("main-assistant")?;
        }
        Ok(())
    }
    pub fn create(&self, mut command: CreateAssistantCommand) -> Result<Assistant, AssistantError> {
        if command.heartbeat_minutes == 0 || command.heartbeat_minutes > 10080 {
            return Err(AssistantError::Invalid);
        }
        command.definition.group_name = None;
        let definition = self
            .management
            .prepare_platform_internal_agent(command.definition)
            .map_err(|_| AssistantError::Invalid)?;
        let now = chrono::Utc::now().timestamp_millis();
        let assistant = Assistant {
            assistant_code: definition.agent_code.clone(),
            kind: command.kind,
            enabled: command.enabled,
            heartbeat_minutes: command.heartbeat_minutes,
            memory_auto_approve: command.memory_auto_approve,
            is_main: false,
            created_at: now,
            updated_at: now,
        };
        self.repo()?.create(&assistant, &definition)?;
        Ok(assistant)
    }
    pub fn update(
        &self,
        code: &str,
        command: UpdateAssistantCommand,
    ) -> Result<Assistant, AssistantError> {
        let mut assistant = self.get(code)?;
        if command.definition.name.is_some() {
            return Err(AssistantError::Invalid);
        }
        if command
            .heartbeat_minutes
            .is_some_and(|v| v == 0 || v > 10080)
        {
            return Err(AssistantError::Invalid);
        }
        self.management
            .update_platform_internal_agent_definition(code, command.definition)
            .map_err(|_| AssistantError::Invalid)?;
        if let Some(v) = command.enabled {
            assistant.enabled = v
        }
        if let Some(v) = command.heartbeat_minutes {
            assistant.heartbeat_minutes = v
        }
        if let Some(v) = command.memory_auto_approve {
            assistant.memory_auto_approve = v
        }
        assistant.updated_at = chrono::Utc::now().timestamp_millis();
        self.repo()?.save(&assistant)?;
        Ok(assistant)
    }
    pub fn delete(&self, code: &str) -> Result<(), AssistantError> {
        let d = self.get(code)?;
        if d.is_main || d.kind != AssistantKind::Custom {
            return Err(AssistantError::Conflict);
        }
        self.repo()?.delete(code)
    }
    pub fn designate_main(&self, code: &str) -> Result<(), AssistantError> {
        self.get(code)?;
        if self.main().is_ok_and(|main| main.assistant_code != code) {
            return Err(AssistantError::Conflict);
        }
        self.repo()?.designate_main(code)
    }
    pub fn main(&self) -> Result<Assistant, AssistantError> {
        self.list()?
            .into_iter()
            .find(|d| d.is_main)
            .ok_or(AssistantError::NotFound)
    }
    pub fn resolve_session(
        &self,
        code: &str,
        purpose: AssistantSessionPurpose,
        key: &str,
        workspace: Option<String>,
    ) -> Result<AssistantSession, AssistantError> {
        self.get(code)?;
        let now = chrono::Utc::now().timestamp_millis();
        self.repo()?.resolve_session(&AssistantSession {
            session_code: eliteforge_agent_core::SessionRecord::new("").id,
            assistant_code: code.into(),
            purpose,
            correlation_key: key.into(),
            title: format!("{code} · {purpose:?}"),
            workspace_code: workspace,
            created_at: now,
            updated_at: now,
        })
    }
    pub fn sessions(&self, code: &str) -> Result<Vec<AssistantSession>, AssistantError> {
        self.get(code)?;
        self.repo()?.sessions(code)
    }
    pub fn expired_heartbeat_sessions(&self) -> Result<Vec<String>, AssistantError> {
        self.repo()?.expired_heartbeat_sessions()
    }
    pub fn delete_empty_heartbeat_session(&self, code: &str) -> Result<(), AssistantError> {
        self.repo()?.delete_empty_heartbeat_session(code)
    }
    pub(crate) fn work_execution_context(
        &self,
        assistant: &str,
        user: &str,
    ) -> Result<Vec<serde_json::Value>, AssistantError> {
        let mut repo = self.repo()?;
        let pending = repo.scoped_pending_work_changes(assistant, user)?;
        let recent = repo.scoped_recent_work_changes(assistant, user)?;
        let mut seen = std::collections::BTreeSet::new();
        let changes: Vec<_> = pending
            .into_iter()
            .chain(recent)
            .filter(|change| seen.insert(change.change_id.clone()))
            .map(|change| {
                serde_json::json!({
                    "changeId": change.change_id,
                    "taskCode": change.task_code,
                    "kind": change.kind,
                    "changeKey": change.change_key,
                    "summary": change.summary,
                    "requestedAction": change.requested_action,
                    "evidence": change.evidence,
                    "originSessionCode": change.session_code,
                    "resolvedAt": change.resolved_at,
                    "resolution": change.resolution,
                    "notification": change.notification.map(|notice| serde_json::json!({
                        "id": notice.id, "title": notice.title, "createdAt": notice.created_at,
                    })),
                })
            })
            .collect();
        Ok(changes)
    }
    pub fn session(&self, code: &str) -> Result<Option<AssistantSession>, AssistantError> {
        self.repo()?.session(code)
    }
    pub fn publish_event(
        &self,
        input: AssistantEventInput,
    ) -> Result<AssistantEvent, AssistantError> {
        self.get(&input.assistant_code)?;
        if input.dedup_key.trim().is_empty() {
            return Err(AssistantError::Invalid);
        }
        if let Some(session) = &input.session_code {
            let s = self.session(session)?.ok_or(AssistantError::NotFound)?;
            if s.assistant_code != input.assistant_code {
                return Err(AssistantError::NotFound);
            }
        }
        let session_code = match input.session_code {
            Some(code) => Some(code),
            None => {
                let (purpose, key) = match input.source {
                    AssistantEventSource::UserMessage => {
                        (AssistantSessionPurpose::Interactive, "interactive".into())
                    }
                    AssistantEventSource::TaskReminder => (
                        AssistantSessionPurpose::Task,
                        input.task_code.clone().ok_or(AssistantError::Invalid)?,
                    ),
                    AssistantEventSource::ThreadFeedback => {
                        if let Some(task) = &input.task_code {
                            (AssistantSessionPurpose::Task, task.clone())
                        } else {
                            (
                                AssistantSessionPurpose::Feedback,
                                input.payload["sessionCode"]
                                    .as_str()
                                    .map(|id| format!("thread:{id}"))
                                    .unwrap_or_else(|| input.dedup_key.clone()),
                            )
                        }
                    }
                    AssistantEventSource::Heartbeat => {
                        (AssistantSessionPurpose::Heartbeat, input.dedup_key.clone())
                    }
                    AssistantEventSource::MemoryBatch => (
                        AssistantSessionPurpose::MemoryBatch,
                        input.dedup_key.clone(),
                    ),
                };
                Some(
                    self.resolve_session(
                        &input.assistant_code,
                        purpose,
                        &key,
                        input.workspace_code.clone(),
                    )?
                    .session_code,
                )
            }
        };
        self.repo()?.enqueue(&AssistantEvent {
            event_id: eliteforge_agent_core::SessionRecord::new("").id,
            assistant_code: input.assistant_code,
            source: input.source,
            dedup_key: input.dedup_key,
            session_code,
            task_code: input.task_code,
            workspace_code: input.workspace_code,
            payload: input.payload,
            status: AssistantEventStatus::Queued,
            available_at: input.available_at,
            lease_until: None,
            lease_token: None,
            attempts: 0,
            result: None,
            created_at: chrono::Utc::now().timestamp_millis(),
        })
    }
    pub fn events(&self, code: &str) -> Result<Vec<AssistantEvent>, AssistantError> {
        self.get(code)?;
        self.repo()?.events(code)
    }
    pub fn activity_events(&self, code: &str) -> Result<Vec<AssistantEvent>, AssistantError> {
        self.get(code)?;
        self.repo()?.activity_events(code)
    }
    pub fn prune_heartbeat_events(&self) -> Result<usize, AssistantError> {
        self.repo()?.prune_heartbeat_events()
    }
    pub fn claim(&self, now: i64, lease_ms: i64) -> Result<Option<AssistantEvent>, AssistantError> {
        self.repo()?.claim(now, lease_ms)
    }
    pub fn finish(
        &self,
        event: &AssistantEvent,
        status: AssistantEventStatus,
        result: &str,
        now: i64,
    ) -> Result<(), AssistantError> {
        self.repo()?.finish(
            &event.event_id,
            event
                .lease_token
                .as_deref()
                .ok_or(AssistantError::Conflict)?,
            status,
            result,
            now,
        )
    }
    pub fn attach_session(
        &self,
        event: &AssistantEvent,
        session: &str,
    ) -> Result<(), AssistantError> {
        self.repo()?.attach_session(
            &event.event_id,
            event
                .lease_token
                .as_deref()
                .ok_or(AssistantError::Conflict)?,
            session,
        )
    }
    pub fn renew(
        &self,
        event: &AssistantEvent,
        now: i64,
        lease_ms: i64,
    ) -> Result<(), AssistantError> {
        self.repo()?.renew(event, now, lease_ms)
    }
    pub fn retry(
        &self,
        event: &AssistantEvent,
        now: i64,
        delay_ms: i64,
        error: &str,
    ) -> Result<(), AssistantError> {
        self.repo()?.retry(event, now, delay_ms, error)
    }
    pub fn defer_busy(
        &self,
        event: &AssistantEvent,
        now: i64,
        delay_ms: i64,
    ) -> Result<(), AssistantError> {
        self.repo()?.defer_busy(event, now, delay_ms)
    }
    pub fn resolve_awaiting_input(
        &self,
        session: &str,
        summary: &str,
    ) -> Result<(), AssistantError> {
        self.repo()?
            .resolve_awaiting_input(session, summary, chrono::Utc::now().timestamp_millis())
    }
    pub fn schedule_heartbeats(&self, now: i64) -> Result<(), AssistantError> {
        self.repo()?.schedule_heartbeats(now)
    }
    pub fn notify(&self, input: NotificationInput) -> Result<PlatformNotification, AssistantError> {
        self.notify_replacing(input, &[])
    }
    pub fn notify_replacing(
        &self,
        input: NotificationInput,
        prior_kinds: &[&str],
    ) -> Result<PlatformNotification, AssistantError> {
        let prior_kinds = prior_kinds
            .iter()
            .map(|kind| (*kind).to_owned())
            .collect::<Vec<_>>();
        self.repo()?.notify_replacing(
            &PlatformNotification {
                id: eliteforge_agent_core::SessionRecord::new("").id,
                dedup_key: input.dedup_key,
                assistant_code: input.assistant_code,
                title: input.title,
                body: input.body,
                category: input.category,
                target: input.target,
                read_at: None,
                resolved_at: None,
                archived_at: None,
                created_at: chrono::Utc::now().timestamp_millis(),
            },
            &prior_kinds,
        )
    }
    pub fn notifications(&self) -> Result<Vec<PlatformNotification>, AssistantError> {
        self.repo()?.notifications()
    }
    pub fn update_notification(
        &self,
        id: &str,
        read: bool,
        archive: bool,
    ) -> Result<(), AssistantError> {
        self.repo()?
            .update_notification(id, read, archive, chrono::Utc::now().timestamp_millis())
    }
    pub fn update_notifications(
        &self,
        ids: Vec<String>,
        archive: bool,
    ) -> Result<usize, AssistantError> {
        if ids.is_empty() || ids.len() > 500 || ids.iter().any(|id| id.is_empty() || id.len() > 128)
        {
            return Err(AssistantError::Invalid);
        }
        let ids = ids
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        self.repo()?
            .update_notifications(&ids, archive, chrono::Utc::now().timestamp_millis())
    }
    pub fn delete_archived_notification(&self, id: &str) -> Result<(), AssistantError> {
        self.repo()?.delete_archived_notification(id)
    }
    pub fn delete_archived_notifications(&self) -> Result<usize, AssistantError> {
        self.repo()?.delete_archived_notifications()
    }
}

mod runtime;
pub use runtime::{AssistantDispatcher, AssistantExecutionOutcome, AssistantExecutor};

mod errors;
pub use errors::{ApplicationErrorDescriptor, error_descriptor};
