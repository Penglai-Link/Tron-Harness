//! Dude orchestration ports; the host retains execution independently of UI streams.
mod notification_tools;
pub(crate) mod soul;
mod tools;
mod work_changes;
use crate::application::agent::{
    AgentManagementService, CreateWorkspaceInternalAgentCommand,
    UpdateWorkspaceInternalAgentDefinitionCommand,
};
use crate::domain::agent::InternalAgentToolPolicy;
use crate::domain::dude::*;
use serde_json::Value;
use std::sync::{Arc, Mutex};
pub use tools::register_dude_tools;

pub trait DudeRepository: Send {
    fn soul(&mut self, _code: &str) -> Result<DudeSoul, DudeError> {
        Err(DudeError::Storage)
    }
    fn save_soul(&mut self, _soul: &DudeSoul) -> Result<(), DudeError> {
        Err(DudeError::Storage)
    }

    fn create(
        &mut self,
        dude: &Dude,
        definition: &crate::domain::agent::PlatformInternalAgentConfig,
    ) -> Result<(), DudeError>;

    fn list(&mut self) -> Result<Vec<Dude>, DudeError>;
    fn save(&mut self, dude: &Dude) -> Result<(), DudeError>;
    /// Atomically remove the Dude, its soul, Agent definition, and resource grants.
    fn delete(&mut self, code: &str) -> Result<(), DudeError>;
    fn designate_main(&mut self, code: &str) -> Result<(), DudeError>;
    fn resolve_session(&mut self, session: &DudeSession) -> Result<DudeSession, DudeError>;
    fn sessions(&mut self, code: &str) -> Result<Vec<DudeSession>, DudeError>;
    /// Old heartbeat sessions eligible for retirement, excluding active or referenced work.
    fn expired_heartbeat_sessions(&mut self) -> Result<Vec<String>, DudeError> {
        Ok(Vec::new())
    }
    fn delete_empty_heartbeat_session(&mut self, _code: &str) -> Result<(), DudeError> {
        Ok(())
    }
    fn session(&mut self, code: &str) -> Result<Option<DudeSession>, DudeError>;
    fn enqueue(&mut self, event: &DudeEvent) -> Result<DudeEvent, DudeError>;
    fn events(&mut self, code: &str) -> Result<Vec<DudeEvent>, DudeError>;
    fn activity_events(&mut self, code: &str) -> Result<Vec<DudeEvent>, DudeError>;
    /// Retire old finished heartbeat events without touching pending work.
    fn prune_heartbeat_events(&mut self) -> Result<usize, DudeError>;
    fn claim(&mut self, now: i64, lease_ms: i64) -> Result<Option<DudeEvent>, DudeError>;
    fn finish(
        &mut self,
        id: &str,
        token: &str,
        status: DudeEventStatus,
        result: &str,
        now: i64,
    ) -> Result<(), DudeError>;
    fn attach_session(&mut self, id: &str, token: &str, session: &str) -> Result<(), DudeError>;
    fn renew(&mut self, event: &DudeEvent, now: i64, lease_ms: i64) -> Result<(), DudeError>;
    fn retry(
        &mut self,
        event: &DudeEvent,
        now: i64,
        delay_ms: i64,
        error: &str,
    ) -> Result<(), DudeError>;
    fn defer_busy(&mut self, event: &DudeEvent, now: i64, delay_ms: i64) -> Result<(), DudeError> {
        self.retry(event, now, delay_ms, "Waiting for the current Dude run")
    }
    fn resolve_awaiting_input(
        &mut self,
        session: &str,
        summary: &str,
        now: i64,
    ) -> Result<(), DudeError>;
    fn schedule_heartbeats(&mut self, now: i64) -> Result<(), DudeError>;
    fn notify(
        &mut self,
        notification: &PlatformNotification,
    ) -> Result<PlatformNotification, DudeError>;
    /// Persist the new notice and retire prior actionable notices in one unit.
    fn notify_replacing(
        &mut self,
        notification: &PlatformNotification,
        prior_kinds: &[String],
    ) -> Result<PlatformNotification, DudeError> {
        let _ = prior_kinds;
        self.notify(notification)
    }
    fn record_work_change(
        &mut self,
        change: &DudeWorkChange,
        fingerprint: &str,
    ) -> Result<DudeWorkChange, DudeError>;
    fn scoped_pending_work_changes(
        &mut self,
        _dude: &str,
        _user: &str,
    ) -> Result<Vec<DudeWorkChange>, DudeError> {
        Err(DudeError::Storage)
    }
    /// Recent assessments and delivery receipts, restricted to the session owner.
    fn scoped_recent_work_changes(
        &mut self,
        _dude: &str,
        _user: &str,
    ) -> Result<Vec<DudeWorkChange>, DudeError> {
        Err(DudeError::Storage)
    }
    fn pending_work_changes(&mut self, dude: &str) -> Result<Vec<DudeWorkChange>, DudeError>;
    fn work_changes(&mut self, dude: &str, task: &str) -> Result<Vec<DudeWorkChange>, DudeError>;
    fn work_change(&mut self, dude: &str, id: &str) -> Result<DudeWorkChange, DudeError>;
    fn resolve_work_change(
        &mut self,
        dude: &str,
        id: &str,
        resolution: &str,
        evidence: &[String],
        now: i64,
    ) -> Result<DudeWorkChange, DudeError>;
    fn notify_work_change(
        &mut self,
        dude: &str,
        id: &str,
        title: &str,
        body: &str,
        now: i64,
    ) -> Result<PlatformNotification, DudeError>;
    fn notifications(&mut self) -> Result<Vec<PlatformNotification>, DudeError>;
    /// Current notification records visible to this Dude and session owner.
    fn scoped_notifications(
        &mut self,
        _dude: &str,
        _user: &str,
        _id: Option<&str>,
        _current: usize,
        _size: usize,
        _include_archived: bool,
    ) -> Result<(usize, Vec<PlatformNotification>), DudeError> {
        Err(DudeError::Storage)
    }
    fn update_notification(
        &mut self,
        id: &str,
        read: bool,
        archive: bool,
        now: i64,
    ) -> Result<(), DudeError>;
    fn delete_archived_notification(&mut self, id: &str) -> Result<(), DudeError>;
    fn delete_archived_notifications(&mut self) -> Result<usize, DudeError>;
}
#[derive(Debug, Clone)]
pub struct CreateDudeCommand {
    pub definition: CreateWorkspaceInternalAgentCommand,
    pub kind: DudeKind,
    pub enabled: bool,
    pub heartbeat_minutes: u32,
    pub memory_auto_approve: bool,
}
#[derive(Debug, Clone, Default)]
pub struct UpdateDudeCommand {
    pub definition: UpdateWorkspaceInternalAgentDefinitionCommand,
    pub enabled: Option<bool>,
    pub heartbeat_minutes: Option<u32>,
    pub memory_auto_approve: Option<bool>,
}
#[derive(Debug, Clone)]
pub struct DudeEventInput {
    pub dude_code: String,
    pub source: DudeEventSource,
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
    pub dude_code: Option<String>,
    pub title: String,
    pub body: String,
    pub category: String,
    pub target: Value,
}
#[derive(Clone)]
pub struct DudeService {
    repository: Arc<Mutex<Box<dyn DudeRepository>>>,
    management: AgentManagementService,
}
impl DudeService {
    pub fn new(
        repository: impl DudeRepository + 'static,
        management: AgentManagementService,
    ) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
            management,
        }
    }
    fn repo(&self) -> Result<std::sync::MutexGuard<'_, Box<dyn DudeRepository>>, DudeError> {
        self.repository.lock().map_err(|_| DudeError::Storage)
    }
    pub fn list(&self) -> Result<Vec<Dude>, DudeError> {
        self.repo()?.list()
    }
    pub fn get(&self, code: &str) -> Result<Dude, DudeError> {
        self.list()?
            .into_iter()
            .find(|d| d.dude_code == code)
            .ok_or(DudeError::NotFound)
    }
    pub fn definition(&self, code: &str) -> Result<InternalAgentToolPolicy, DudeError> {
        self.get(code)?;
        self.management
            .all_platform_internal_agents()
            .map_err(|_| DudeError::Storage)?
            .into_iter()
            .find(|d| d.agent_code == code)
            .ok_or(DudeError::NotFound)
    }
    pub fn bootstrap(&self) -> Result<(), DudeError> {
        let main = self
            .management
            .all_platform_internal_agents()
            .map_err(|_| DudeError::Storage)?
            .into_iter()
            .find(|a| a.agent_code == "main-agent")
            .ok_or(DudeError::Storage)?;
        for (code,name,kind,prompt) in [
            ("main-dude","好兄弟",DudeKind::Main,main.role_prompt.clone()),
            ("memory-dude","记忆整理",DudeKind::Memory,"整理当前用户所有工作区的稳定事实和已验证经验，排除自身及自身派发的记录。经验转为 Skill 时必须加载并遵循已安装启用的 skill-creator；产物进入待批准区，用户批准后安装。保留来源、避免重复，不将临时进度写入长期记忆。".into())
        ] {
            if self.get(code).is_ok() { continue; }
            self.management.initialize_dude_definition(code,name,Some(prompt)).map_err(|_|DudeError::Storage)?;
            let now=chrono::Utc::now().timestamp_millis();
            self.repo()?.save(&Dude{dude_code:code.into(),kind,enabled:true,heartbeat_minutes:if kind==DudeKind::Memory{360}else{5},memory_auto_approve:false,is_main:false,created_at:now,updated_at:now})?;
        }
        if self.main().is_err() {
            self.designate_main("main-dude")?;
        }
        Ok(())
    }
    pub fn create(&self, mut command: CreateDudeCommand) -> Result<Dude, DudeError> {
        if command.heartbeat_minutes == 0 || command.heartbeat_minutes > 10080 {
            return Err(DudeError::Invalid);
        }
        command.definition.group_name = None;
        let definition = self
            .management
            .prepare_platform_internal_agent(command.definition)
            .map_err(|_| DudeError::Invalid)?;
        let now = chrono::Utc::now().timestamp_millis();
        let dude = Dude {
            dude_code: definition.agent_code.clone(),
            kind: command.kind,
            enabled: command.enabled,
            heartbeat_minutes: command.heartbeat_minutes,
            memory_auto_approve: command.memory_auto_approve,
            is_main: false,
            created_at: now,
            updated_at: now,
        };
        self.repo()?.create(&dude, &definition)?;
        Ok(dude)
    }
    pub fn update(&self, code: &str, command: UpdateDudeCommand) -> Result<Dude, DudeError> {
        let mut dude = self.get(code)?;
        if command.definition.name.is_some() {
            return Err(DudeError::Invalid);
        }
        if command
            .heartbeat_minutes
            .is_some_and(|v| v == 0 || v > 10080)
        {
            return Err(DudeError::Invalid);
        }
        self.management
            .update_platform_internal_agent_definition(code, command.definition)
            .map_err(|_| DudeError::Invalid)?;
        if let Some(v) = command.enabled {
            dude.enabled = v
        }
        if let Some(v) = command.heartbeat_minutes {
            dude.heartbeat_minutes = v
        }
        if let Some(v) = command.memory_auto_approve {
            dude.memory_auto_approve = v
        }
        dude.updated_at = chrono::Utc::now().timestamp_millis();
        self.repo()?.save(&dude)?;
        Ok(dude)
    }
    pub fn delete(&self, code: &str) -> Result<(), DudeError> {
        let d = self.get(code)?;
        if d.is_main || d.kind != DudeKind::Custom {
            return Err(DudeError::Conflict);
        }
        self.repo()?.delete(code)
    }
    pub fn designate_main(&self, code: &str) -> Result<(), DudeError> {
        self.get(code)?;
        if self.main().is_ok_and(|main| main.dude_code != code) {
            return Err(DudeError::Conflict);
        }
        self.repo()?.designate_main(code)
    }
    pub fn main(&self) -> Result<Dude, DudeError> {
        self.list()?
            .into_iter()
            .find(|d| d.is_main)
            .ok_or(DudeError::NotFound)
    }
    pub fn resolve_session(
        &self,
        code: &str,
        purpose: DudeSessionPurpose,
        key: &str,
        workspace: Option<String>,
    ) -> Result<DudeSession, DudeError> {
        self.get(code)?;
        let now = chrono::Utc::now().timestamp_millis();
        self.repo()?.resolve_session(&DudeSession {
            session_code: eliteforge_agent_core::SessionRecord::new("").id,
            dude_code: code.into(),
            purpose,
            correlation_key: key.into(),
            title: format!("{code} · {purpose:?}"),
            workspace_code: workspace,
            created_at: now,
            updated_at: now,
        })
    }
    pub fn sessions(&self, code: &str) -> Result<Vec<DudeSession>, DudeError> {
        self.get(code)?;
        self.repo()?.sessions(code)
    }
    pub fn expired_heartbeat_sessions(&self) -> Result<Vec<String>, DudeError> {
        self.repo()?.expired_heartbeat_sessions()
    }
    pub fn delete_empty_heartbeat_session(&self, code: &str) -> Result<(), DudeError> {
        self.repo()?.delete_empty_heartbeat_session(code)
    }
    pub(crate) fn work_execution_context(
        &self,
        dude: &str,
        user: &str,
    ) -> Result<Vec<serde_json::Value>, DudeError> {
        let mut repo = self.repo()?;
        let pending = repo.scoped_pending_work_changes(dude, user)?;
        let recent = repo.scoped_recent_work_changes(dude, user)?;
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
    pub fn session(&self, code: &str) -> Result<Option<DudeSession>, DudeError> {
        self.repo()?.session(code)
    }
    pub fn publish_event(&self, input: DudeEventInput) -> Result<DudeEvent, DudeError> {
        self.get(&input.dude_code)?;
        if input.dedup_key.trim().is_empty() {
            return Err(DudeError::Invalid);
        }
        if let Some(session) = &input.session_code {
            let s = self.session(session)?.ok_or(DudeError::NotFound)?;
            if s.dude_code != input.dude_code {
                return Err(DudeError::NotFound);
            }
        }
        let session_code = match input.session_code {
            Some(code) => Some(code),
            None => {
                let (purpose, key) = match input.source {
                    DudeEventSource::UserMessage => {
                        (DudeSessionPurpose::Interactive, "interactive".into())
                    }
                    DudeEventSource::TaskReminder => (
                        DudeSessionPurpose::Task,
                        input.task_code.clone().ok_or(DudeError::Invalid)?,
                    ),
                    DudeEventSource::ThreadFeedback => {
                        if let Some(task) = &input.task_code {
                            (DudeSessionPurpose::Task, task.clone())
                        } else {
                            (
                                DudeSessionPurpose::Feedback,
                                input.payload["sessionCode"]
                                    .as_str()
                                    .map(|id| format!("thread:{id}"))
                                    .unwrap_or_else(|| input.dedup_key.clone()),
                            )
                        }
                    }
                    DudeEventSource::Heartbeat => {
                        (DudeSessionPurpose::Heartbeat, input.dedup_key.clone())
                    }
                    DudeEventSource::MemoryBatch => {
                        (DudeSessionPurpose::MemoryBatch, input.dedup_key.clone())
                    }
                };
                Some(
                    self.resolve_session(
                        &input.dude_code,
                        purpose,
                        &key,
                        input.workspace_code.clone(),
                    )?
                    .session_code,
                )
            }
        };
        self.repo()?.enqueue(&DudeEvent {
            event_id: eliteforge_agent_core::SessionRecord::new("").id,
            dude_code: input.dude_code,
            source: input.source,
            dedup_key: input.dedup_key,
            session_code,
            task_code: input.task_code,
            workspace_code: input.workspace_code,
            payload: input.payload,
            status: DudeEventStatus::Queued,
            available_at: input.available_at,
            lease_until: None,
            lease_token: None,
            attempts: 0,
            result: None,
            created_at: chrono::Utc::now().timestamp_millis(),
        })
    }
    pub fn events(&self, code: &str) -> Result<Vec<DudeEvent>, DudeError> {
        self.get(code)?;
        self.repo()?.events(code)
    }
    pub fn activity_events(&self, code: &str) -> Result<Vec<DudeEvent>, DudeError> {
        self.get(code)?;
        self.repo()?.activity_events(code)
    }
    pub fn prune_heartbeat_events(&self) -> Result<usize, DudeError> {
        self.repo()?.prune_heartbeat_events()
    }
    pub fn claim(&self, now: i64, lease_ms: i64) -> Result<Option<DudeEvent>, DudeError> {
        self.repo()?.claim(now, lease_ms)
    }
    pub fn finish(
        &self,
        event: &DudeEvent,
        status: DudeEventStatus,
        result: &str,
        now: i64,
    ) -> Result<(), DudeError> {
        self.repo()?.finish(
            &event.event_id,
            event.lease_token.as_deref().ok_or(DudeError::Conflict)?,
            status,
            result,
            now,
        )
    }
    pub fn attach_session(&self, event: &DudeEvent, session: &str) -> Result<(), DudeError> {
        self.repo()?.attach_session(
            &event.event_id,
            event.lease_token.as_deref().ok_or(DudeError::Conflict)?,
            session,
        )
    }
    pub fn renew(&self, event: &DudeEvent, now: i64, lease_ms: i64) -> Result<(), DudeError> {
        self.repo()?.renew(event, now, lease_ms)
    }
    pub fn retry(
        &self,
        event: &DudeEvent,
        now: i64,
        delay_ms: i64,
        error: &str,
    ) -> Result<(), DudeError> {
        self.repo()?.retry(event, now, delay_ms, error)
    }
    pub fn defer_busy(&self, event: &DudeEvent, now: i64, delay_ms: i64) -> Result<(), DudeError> {
        self.repo()?.defer_busy(event, now, delay_ms)
    }
    pub fn resolve_awaiting_input(&self, session: &str, summary: &str) -> Result<(), DudeError> {
        self.repo()?
            .resolve_awaiting_input(session, summary, chrono::Utc::now().timestamp_millis())
    }
    pub fn schedule_heartbeats(&self, now: i64) -> Result<(), DudeError> {
        self.repo()?.schedule_heartbeats(now)
    }
    pub fn notify(&self, input: NotificationInput) -> Result<PlatformNotification, DudeError> {
        self.notify_replacing(input, &[])
    }
    pub fn notify_replacing(
        &self,
        input: NotificationInput,
        prior_kinds: &[&str],
    ) -> Result<PlatformNotification, DudeError> {
        let prior_kinds = prior_kinds
            .iter()
            .map(|kind| (*kind).to_owned())
            .collect::<Vec<_>>();
        self.repo()?.notify_replacing(
            &PlatformNotification {
                id: eliteforge_agent_core::SessionRecord::new("").id,
                dedup_key: input.dedup_key,
                dude_code: input.dude_code,
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
    pub fn notifications(&self) -> Result<Vec<PlatformNotification>, DudeError> {
        self.repo()?.notifications()
    }
    pub fn update_notification(
        &self,
        id: &str,
        read: bool,
        archive: bool,
    ) -> Result<(), DudeError> {
        self.repo()?
            .update_notification(id, read, archive, chrono::Utc::now().timestamp_millis())
    }
    pub fn delete_archived_notification(&self, id: &str) -> Result<(), DudeError> {
        self.repo()?.delete_archived_notification(id)
    }
    pub fn delete_archived_notifications(&self) -> Result<usize, DudeError> {
        self.repo()?.delete_archived_notifications()
    }
}

mod runtime;
pub use runtime::{DudeDispatcher, DudeExecutionOutcome, DudeExecutor};

mod errors;
pub use errors::{ApplicationErrorDescriptor, error_descriptor};
