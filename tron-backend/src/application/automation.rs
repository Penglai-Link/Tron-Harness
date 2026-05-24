use std::{
    collections::BTreeSet,
    str::FromStr,
    sync::{Arc, Mutex},
};

use chrono::{SecondsFormat, Utc};
use cron::Schedule;
use tokio_cron::{Job, Scheduler};

use crate::{
    application::workspace::WorkspaceService,
    domain::{
        automation::{
            Automation, AutomationDetail, AutomationError, AutomationExecutionListItem,
            AutomationExecutionListQuery, AutomationExecutionRecord, AutomationExecutionStatus,
            AutomationLifecycleResult, AutomationListItem, AutomationListQuery, AutomationOptions,
            AutomationStatus, AutomationTriggerSource, PagedAutomationRecords,
            automation_execution_status_to_storage, automation_options,
        },
        workspace::{WorkspaceError, WorkspaceStatus},
    },
    shared::{
        event_bus::{BusEventDraft, EventBusHandle},
        id::SnowflakeId,
    },
};

#[derive(Debug, Clone)]
pub struct CreateAutomationCommand {
    pub workspace_code: String,
    pub name: String,
    pub description: Option<String>,
    pub prompt: String,
    pub cron_expression: String,
    pub status: Option<AutomationStatus>,
}

#[derive(Debug, Clone)]
pub struct UpdateAutomationCommand {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub prompt: Option<String>,
    pub cron_expression: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateAutomationStatusCommand {
    pub status: AutomationStatus,
}

pub trait AutomationRepository: Send {
    fn list_automations(&mut self) -> Result<Vec<Automation>, AutomationError>;
    fn next_id(&mut self) -> Result<i64, AutomationError>;
    fn create_automation(&mut self, automation: &Automation) -> Result<(), AutomationError>;
    fn update_automation(&mut self, automation: &Automation) -> Result<(), AutomationError>;
    fn list_execution_records(
        &mut self,
        automation_code: &str,
    ) -> Result<Vec<AutomationExecutionRecord>, AutomationError>;
    fn create_execution_record(
        &mut self,
        record: &AutomationExecutionRecord,
    ) -> Result<(), AutomationError>;
}

#[derive(Clone)]
struct AutomationRuntime {
    registered_codes: Arc<Mutex<BTreeSet<String>>>,
    scheduler: Arc<Mutex<Scheduler<Utc>>>,
}

impl AutomationRuntime {
    fn new() -> Self {
        Self {
            registered_codes: Arc::new(Mutex::new(BTreeSet::new())),
            scheduler: Arc::new(Mutex::new(Scheduler::utc())),
        }
    }

    fn register(
        &self,
        automation_code: &str,
        scheduler_expression: &str,
        repository: Arc<Mutex<Box<dyn AutomationRepository>>>,
        event_bus: EventBusHandle,
    ) -> Result<(), AutomationError> {
        self.unregister(automation_code);
        let code_for_name = automation_code.to_string();
        let code_for_run = code_for_name.clone();
        let job = Job::named(
            &code_for_name,
            scheduler_expression.to_string(),
            move || {
                let repository = repository.clone();
                let event_bus = event_bus.clone();
                let automation_code = code_for_run.clone();
                async move {
                    let _ = record_execution(
                        repository,
                        event_bus,
                        &automation_code,
                        AutomationTriggerSource::Scheduler,
                    );
                }
            },
        );
        self.scheduler
            .lock()
            .expect("automation scheduler mutex poisoned")
            .add(job);
        self.registered_codes
            .lock()
            .expect("automation scheduler registry mutex poisoned")
            .insert(code_for_name);
        Ok(())
    }

    fn unregister(&self, automation_code: &str) {
        self.scheduler
            .lock()
            .expect("automation scheduler mutex poisoned")
            .cancel_by_name(automation_code);
        self.registered_codes
            .lock()
            .expect("automation scheduler registry mutex poisoned")
            .remove(automation_code);
    }

    fn snapshot(&self) -> Vec<String> {
        self.registered_codes
            .lock()
            .expect("automation scheduler registry mutex poisoned")
            .iter()
            .cloned()
            .collect()
    }
}

#[derive(Clone)]
pub struct AutomationService {
    repository: Arc<Mutex<Box<dyn AutomationRepository>>>,
    runtime: AutomationRuntime,
    workspaces: WorkspaceService,
    event_bus: EventBusHandle,
}

impl AutomationService {
    pub fn new(
        repository: impl AutomationRepository + 'static,
        workspaces: WorkspaceService,
    ) -> Self {
        Self::with_event_bus(repository, workspaces, EventBusHandle::disabled())
    }

    pub fn with_event_bus(
        repository: impl AutomationRepository + 'static,
        workspaces: WorkspaceService,
        event_bus: EventBusHandle,
    ) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
            runtime: AutomationRuntime::new(),
            workspaces,
            event_bus,
        }
    }

    pub fn bootstrap_scheduler(&self) -> Result<(), AutomationError> {
        for automation in self
            .all_automations()?
            .into_iter()
            .filter(|item| item.status == AutomationStatus::Active)
        {
            self.runtime.register(
                &automation.automation_code,
                &automation.scheduler_expression,
                self.repository.clone(),
                self.event_bus.clone(),
            )?;
        }
        Ok(())
    }

    pub fn options(&self) -> AutomationOptions {
        automation_options()
    }

    pub fn list_automations(
        &self,
        query: AutomationListQuery,
    ) -> Result<PagedAutomationRecords<AutomationListItem>, AutomationError> {
        let workspace_code = normalized_required(query.workspace_code.clone())
            .ok_or(AutomationError::WorkspaceRequired)?;
        self.ensure_active_workspace(&workspace_code)?;
        let current = query.current.unwrap_or(1).max(1);
        let size = query.size.unwrap_or(20).clamp(1, 100);
        let records = self.filtered_automation_items(&workspace_code, &query)?;
        let total = records.len();
        let start = (current - 1) * size;
        Ok(PagedAutomationRecords {
            current,
            size,
            total,
            records: records.into_iter().skip(start).take(size).collect(),
        })
    }

    pub fn create_automation(
        &self,
        command: CreateAutomationCommand,
    ) -> Result<AutomationDetail, AutomationError> {
        let workspace_code = normalized_required(Some(command.workspace_code))
            .ok_or(AutomationError::WorkspaceRequired)?;
        self.ensure_active_workspace(&workspace_code)?;
        let name = normalized_required(Some(command.name)).ok_or(AutomationError::NameRequired)?;
        let prompt =
            normalized_required(Some(command.prompt)).ok_or(AutomationError::PromptRequired)?;
        let description = optional_trimmed(command.description);
        let cron_expression = normalized_required(Some(command.cron_expression))
            .ok_or(AutomationError::InvalidCron)?;
        let scheduler_expression = normalized_scheduler_expression(&cron_expression)?;
        let now = current_timestamp();
        let status = command.status.unwrap_or(AutomationStatus::Paused);
        if status == AutomationStatus::Archived {
            return Err(AutomationError::InvalidStatus);
        }
        let id = self
            .repository
            .lock()
            .expect("automation repository mutex poisoned")
            .next_id()?;
        let automation_code = format!("AUT-{id}");
        let automation = Automation {
            id: SnowflakeId::new(id),
            automation_code: automation_code.clone(),
            workspace_code,
            name,
            description,
            prompt_text: prompt,
            cron_expression,
            scheduler_expression,
            timezone: "UTC".into(),
            status,
            previous_status: None,
            last_execution_status: None,
            last_execution_code: None,
            last_execution_summary: None,
            last_started_at: None,
            last_finished_at: None,
            created_at: now.clone(),
            updated_at: now,
            archived_at: None,
        };
        self.repository
            .lock()
            .expect("automation repository mutex poisoned")
            .create_automation(&automation)?;
        if automation.status == AutomationStatus::Active {
            self.runtime.register(
                &automation.automation_code,
                &automation.scheduler_expression,
                self.repository.clone(),
                self.event_bus.clone(),
            )?;
        }
        let detail = self.get_automation(&automation_code)?;
        self.publish_automation_detail_event("automation.created", "Created automation", &detail);
        Ok(detail)
    }

    pub fn get_automation(
        &self,
        automation_code: &str,
    ) -> Result<AutomationDetail, AutomationError> {
        self.detail_from_automation(self.get_automation_record(automation_code)?)
    }

    pub fn update_automation(
        &self,
        automation_code: &str,
        command: UpdateAutomationCommand,
    ) -> Result<AutomationDetail, AutomationError> {
        let mut automation = self.get_automation_record(automation_code)?;
        if automation.status == AutomationStatus::Archived {
            return Err(AutomationError::ArchivedMutationDenied);
        }
        if let Some(name) = command.name {
            automation.name =
                normalized_required(Some(name)).ok_or(AutomationError::NameRequired)?;
        }
        if let Some(description) = command.description {
            automation.description = description.and_then(|value| optional_trimmed(Some(value)));
        }
        if let Some(prompt) = command.prompt {
            automation.prompt_text =
                normalized_required(Some(prompt)).ok_or(AutomationError::PromptRequired)?;
        }
        if let Some(cron_expression) = command.cron_expression {
            let normalized =
                normalized_required(Some(cron_expression)).ok_or(AutomationError::InvalidCron)?;
            automation.scheduler_expression = normalized_scheduler_expression(&normalized)?;
            automation.cron_expression = normalized;
        }
        automation.updated_at = current_timestamp();
        self.repository
            .lock()
            .expect("automation repository mutex poisoned")
            .update_automation(&automation)?;
        self.sync_runtime(&automation)?;
        let detail = self.get_automation(automation_code)?;
        self.publish_automation_detail_event("automation.updated", "Updated automation", &detail);
        Ok(detail)
    }

    pub fn update_status(
        &self,
        automation_code: &str,
        command: UpdateAutomationStatusCommand,
    ) -> Result<AutomationDetail, AutomationError> {
        let mut automation = self.get_automation_record(automation_code)?;
        if automation.status == AutomationStatus::Archived {
            return Err(AutomationError::ArchivedMutationDenied);
        }
        if command.status == AutomationStatus::Archived {
            return Err(AutomationError::InvalidStatus);
        }
        automation.status = command.status;
        automation.updated_at = current_timestamp();
        self.repository
            .lock()
            .expect("automation repository mutex poisoned")
            .update_automation(&automation)?;
        self.sync_runtime(&automation)?;
        let detail = self.get_automation(automation_code)?;
        self.publish_automation_detail_event(
            "automation.status_updated",
            "Updated automation status",
            &detail,
        );
        Ok(detail)
    }

    pub fn archive_automation(
        &self,
        automation_code: &str,
    ) -> Result<AutomationLifecycleResult, AutomationError> {
        let mut automation = self.get_automation_record(automation_code)?;
        if automation.status != AutomationStatus::Archived {
            automation.previous_status = Some(automation.status);
            automation.status = AutomationStatus::Archived;
            automation.archived_at = Some(current_timestamp());
            automation.updated_at = current_timestamp();
            self.repository
                .lock()
                .expect("automation repository mutex poisoned")
                .update_automation(&automation)?;
        }
        self.runtime.unregister(&automation.automation_code);
        let result = self.lifecycle_result(&automation);
        self.publish_automation_lifecycle_event(
            "automation.archived",
            "Archived automation",
            &result,
        );
        Ok(result)
    }

    pub fn restore_automation(
        &self,
        automation_code: &str,
    ) -> Result<AutomationLifecycleResult, AutomationError> {
        let mut automation = self.get_automation_record(automation_code)?;
        if automation.status == AutomationStatus::Archived {
            automation.status = automation
                .previous_status
                .unwrap_or(AutomationStatus::Paused);
            automation.previous_status = None;
            automation.archived_at = None;
            automation.updated_at = current_timestamp();
            self.repository
                .lock()
                .expect("automation repository mutex poisoned")
                .update_automation(&automation)?;
            self.sync_runtime(&automation)?;
        }
        let result = self.lifecycle_result(&automation);
        self.publish_automation_lifecycle_event(
            "automation.restored",
            "Restored automation",
            &result,
        );
        Ok(result)
    }

    pub fn execute_now(&self, automation_code: &str) -> Result<AutomationDetail, AutomationError> {
        self.execute_internal(automation_code, AutomationTriggerSource::Manual)?;
        let detail = self.get_automation(automation_code)?;
        self.publish_automation_detail_event(
            "automation.executed",
            "Executed automation manually",
            &detail,
        );
        Ok(detail)
    }

    pub fn list_execution_records(
        &self,
        automation_code: &str,
        query: AutomationExecutionListQuery,
    ) -> Result<PagedAutomationRecords<AutomationExecutionListItem>, AutomationError> {
        let _automation = self.get_automation_record(automation_code)?;
        let current = query.current.unwrap_or(1).max(1);
        let size = query.size.unwrap_or(20).clamp(1, 100);
        let records = self
            .repository
            .lock()
            .expect("automation repository mutex poisoned")
            .list_execution_records(automation_code)?
            .into_iter()
            .map(to_execution_list_item)
            .collect::<Vec<_>>();
        let total = records.len();
        let start = (current - 1) * size;
        Ok(PagedAutomationRecords {
            current,
            size,
            total,
            records: records.into_iter().skip(start).take(size).collect(),
        })
    }

    pub fn scheduler_snapshot(&self) -> Vec<String> {
        self.runtime.snapshot()
    }

    fn execute_internal(
        &self,
        automation_code: &str,
        trigger_source: AutomationTriggerSource,
    ) -> Result<AutomationExecutionRecord, AutomationError> {
        record_execution(
            self.repository.clone(),
            self.event_bus.clone(),
            automation_code,
            trigger_source,
        )
    }

    fn sync_runtime(&self, automation: &Automation) -> Result<(), AutomationError> {
        if automation.status == AutomationStatus::Active {
            self.runtime.register(
                &automation.automation_code,
                &automation.scheduler_expression,
                self.repository.clone(),
                self.event_bus.clone(),
            )?;
        } else {
            self.runtime.unregister(&automation.automation_code);
        }
        Ok(())
    }

    fn filtered_automation_items(
        &self,
        workspace_code: &str,
        query: &AutomationListQuery,
    ) -> Result<Vec<AutomationListItem>, AutomationError> {
        let include_archived = query.include_archived.unwrap_or(false);
        let search = query
            .search
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .to_lowercase();
        let status = query.status;
        let mut records = self
            .all_automations()?
            .into_iter()
            .filter(|automation| automation.workspace_code == workspace_code)
            .filter(|automation| {
                if let Some(status) = status {
                    return automation.status == status;
                }
                include_archived || automation.status != AutomationStatus::Archived
            })
            .filter(|automation| {
                if search.is_empty() {
                    return true;
                }
                [
                    automation.automation_code.as_str(),
                    automation.name.as_str(),
                    automation.description.as_deref().unwrap_or(""),
                    automation.last_execution_summary.as_deref().unwrap_or(""),
                ]
                .join(" ")
                .to_lowercase()
                .contains(&search)
            })
            .map(|automation| AutomationListItem {
                automation_code: automation.automation_code,
                workspace_code: automation.workspace_code,
                name: automation.name,
                description: automation.description,
                cron_expression: automation.cron_expression,
                timezone: automation.timezone,
                status: automation.status,
                last_execution_status: automation.last_execution_status,
                last_execution_summary: automation.last_execution_summary,
                last_execution_code: automation.last_execution_code,
                last_started_at: automation.last_started_at,
                last_finished_at: automation.last_finished_at,
                next_run_at: next_run_preview(&automation.scheduler_expression, automation.status),
                updated_at: automation.updated_at,
                archived_at: automation.archived_at,
            })
            .collect::<Vec<_>>();
        records.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        Ok(records)
    }

    fn detail_from_automation(
        &self,
        automation: Automation,
    ) -> Result<AutomationDetail, AutomationError> {
        Ok(AutomationDetail {
            id: automation.id.as_decimal_string(),
            automation_code: automation.automation_code,
            workspace_code: automation.workspace_code,
            name: automation.name,
            description: automation.description,
            prompt: automation.prompt_text,
            cron_expression: automation.cron_expression,
            scheduler_expression: automation.scheduler_expression.clone(),
            timezone: automation.timezone,
            status: automation.status,
            previous_status: automation.previous_status,
            last_execution_status: automation.last_execution_status,
            last_execution_code: automation.last_execution_code,
            last_execution_summary: automation.last_execution_summary,
            last_started_at: automation.last_started_at,
            last_finished_at: automation.last_finished_at,
            next_run_at: next_run_preview(&automation.scheduler_expression, automation.status),
            created_at: automation.created_at,
            updated_at: automation.updated_at,
            archived_at: automation.archived_at,
        })
    }

    fn lifecycle_result(&self, automation: &Automation) -> AutomationLifecycleResult {
        AutomationLifecycleResult {
            archived_at: automation.archived_at.clone(),
            automation_code: automation.automation_code.clone(),
            previous_status: automation.previous_status,
            status: automation.status,
        }
    }

    fn get_automation_record(&self, automation_code: &str) -> Result<Automation, AutomationError> {
        let code = automation_code.trim();
        if code.is_empty() {
            return Err(AutomationError::NotFound);
        }
        self.all_automations()?
            .into_iter()
            .find(|automation| automation.automation_code == code)
            .ok_or(AutomationError::NotFound)
    }

    fn all_automations(&self) -> Result<Vec<Automation>, AutomationError> {
        self.repository
            .lock()
            .expect("automation repository mutex poisoned")
            .list_automations()
    }

    fn ensure_active_workspace(&self, workspace_code: &str) -> Result<(), AutomationError> {
        let detail = self
            .workspaces
            .get_workspace(workspace_code)
            .map_err(map_workspace_error)?;
        if detail.status != WorkspaceStatus::Active {
            return Err(AutomationError::InvalidWorkspace);
        }
        Ok(())
    }

    fn publish_automation_detail_event(
        &self,
        event_name: &'static str,
        summary: &'static str,
        detail: &AutomationDetail,
    ) {
        let _ = self.event_bus.publish(
            BusEventDraft::audit(event_name, "automation_service")
                .subject("automation", detail.automation_code.clone())
                .summary(summary)
                .attr("workspaceCode", detail.workspace_code.clone())
                .attr("status", format!("{:?}", detail.status))
                .attr("cronExpression", detail.cron_expression.clone())
                .attr("timezone", detail.timezone.clone()),
        );
    }

    fn publish_automation_lifecycle_event(
        &self,
        event_name: &'static str,
        summary: &'static str,
        result: &AutomationLifecycleResult,
    ) {
        let _ = self.event_bus.publish(
            BusEventDraft::audit(event_name, "automation_service")
                .subject("automation", result.automation_code.clone())
                .summary(summary)
                .attr("status", format!("{:?}", result.status))
                .attr(
                    "previousStatus",
                    result
                        .previous_status
                        .map(|status| format!("{status:?}"))
                        .unwrap_or_else(|| "None".into()),
                ),
        );
    }
}

fn to_execution_list_item(record: AutomationExecutionRecord) -> AutomationExecutionListItem {
    AutomationExecutionListItem {
        execution_code: record.execution_code,
        automation_code: record.automation_code,
        workspace_code: record.workspace_code,
        trigger_source: record.trigger_source,
        status: record.status,
        summary: record.summary,
        scheduled_at: record.scheduled_at,
        started_at: record.started_at,
        finished_at: record.finished_at,
        created_at: record.created_at,
    }
}

fn record_execution(
    repository: Arc<Mutex<Box<dyn AutomationRepository>>>,
    event_bus: EventBusHandle,
    automation_code: &str,
    trigger_source: AutomationTriggerSource,
) -> Result<AutomationExecutionRecord, AutomationError> {
    let mut automation = {
        let mut repository = repository
            .lock()
            .expect("automation repository mutex poisoned");
        repository
            .list_automations()?
            .into_iter()
            .find(|automation| automation.automation_code == automation_code.trim())
            .ok_or(AutomationError::NotFound)?
    };
    if trigger_source == AutomationTriggerSource::Manual
        && automation.status == AutomationStatus::Archived
    {
        return Err(AutomationError::ArchivedMutationDenied);
    }
    let now = current_timestamp();
    let execution_status = match (trigger_source, automation.status) {
        (AutomationTriggerSource::Scheduler, AutomationStatus::Active) => {
            AutomationExecutionStatus::Success
        }
        (AutomationTriggerSource::Scheduler, _) => AutomationExecutionStatus::Skipped,
        (AutomationTriggerSource::Manual, _) => AutomationExecutionStatus::Success,
    };
    let summary = Some(match execution_status {
        AutomationExecutionStatus::Success => {
            "Prompt dispatch recorded for the automation.".to_string()
        }
        AutomationExecutionStatus::Skipped => {
            "Scheduler tick skipped because the automation is not Active.".to_string()
        }
        _ => format!(
            "Execution recorded with status {}.",
            automation_execution_status_to_storage(execution_status)
        ),
    });
    let next_id = repository
        .lock()
        .expect("automation repository mutex poisoned")
        .next_id()?;
    let record = AutomationExecutionRecord {
        id: SnowflakeId::new(next_id),
        execution_code: format!("AEX-{next_id}"),
        automation_code: automation.automation_code.clone(),
        workspace_code: automation.workspace_code.clone(),
        trigger_source,
        status: execution_status,
        summary: summary.clone(),
        scheduled_at: (trigger_source == AutomationTriggerSource::Scheduler).then(|| now.clone()),
        started_at: now.clone(),
        finished_at: Some(now.clone()),
        created_at: now.clone(),
    };
    automation.last_execution_status = Some(execution_status);
    automation.last_execution_code = Some(record.execution_code.clone());
    automation.last_execution_summary = summary;
    automation.last_started_at = Some(record.started_at.clone());
    automation.last_finished_at = record.finished_at.clone();
    automation.updated_at = now;
    {
        let mut repository = repository
            .lock()
            .expect("automation repository mutex poisoned");
        repository.create_execution_record(&record)?;
        repository.update_automation(&automation)?;
    }
    publish_automation_execution_event(
        event_bus,
        "automation.execution_recorded",
        "Recorded automation execution",
        &record,
    );
    Ok(record)
}

fn normalized_required(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn optional_trimmed(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn normalized_scheduler_expression(cron_expression: &str) -> Result<String, AutomationError> {
    let parts = cron_expression
        .split_whitespace()
        .filter(|item| !item.trim().is_empty())
        .collect::<Vec<_>>();
    let normalized = match parts.len() {
        5 => format!("0 {}", parts.join(" ")),
        6 => parts.join(" "),
        _ => return Err(AutomationError::InvalidCron),
    };
    Schedule::from_str(&normalized).map_err(|_| AutomationError::InvalidCron)?;
    Ok(normalized)
}

fn next_run_preview(scheduler_expression: &str, status: AutomationStatus) -> Option<String> {
    if status != AutomationStatus::Active {
        return None;
    }
    Schedule::from_str(scheduler_expression)
        .ok()?
        .upcoming(Utc)
        .next()
        .map(format_timestamp)
}

fn current_timestamp() -> String {
    format_timestamp(Utc::now())
}

fn format_timestamp(value: chrono::DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn map_workspace_error(error: WorkspaceError) -> AutomationError {
    match error {
        WorkspaceError::NotFound
        | WorkspaceError::InvalidWorkspaceCode
        | WorkspaceError::ArchivedMutationDenied => AutomationError::InvalidWorkspace,
        WorkspaceError::StorageFailure => AutomationError::StorageFailure,
        WorkspaceError::DuplicateWorkspaceCode
        | WorkspaceError::WorkspaceNameRequired
        | WorkspaceError::InvalidHierarchy
        | WorkspaceError::InvalidAccessUser
        | WorkspaceError::InvalidAccessRole
        | WorkspaceError::DuplicateAccessGrant => AutomationError::InvalidWorkspace,
    }
}

fn publish_automation_execution_event(
    event_bus: EventBusHandle,
    event_name: &'static str,
    summary: &'static str,
    record: &AutomationExecutionRecord,
) {
    let _ = event_bus.publish(
        BusEventDraft::audit(event_name, "automation_service")
            .subject("automation", record.automation_code.clone())
            .summary(summary)
            .attr("executionCode", record.execution_code.clone())
            .attr("workspaceCode", record.workspace_code.clone())
            .attr("triggerSource", format!("{:?}", record.trigger_source))
            .attr("status", format!("{:?}", record.status)),
    );
}
