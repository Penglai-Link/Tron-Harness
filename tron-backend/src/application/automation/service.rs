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

mod projection;
mod runtime;
mod use_cases;

use self::{projection::*, runtime::AutomationRuntime};

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
