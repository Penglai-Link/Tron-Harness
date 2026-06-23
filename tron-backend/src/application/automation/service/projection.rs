use super::*;

pub(super) fn to_execution_list_item(
    record: AutomationExecutionRecord,
) -> AutomationExecutionListItem {
    AutomationExecutionListItem {
        execution_code: record.execution_code,
        automation_code: record.automation_code,
        workspace_code: record.workspace_code,
        session_code: record.session_code,
        trigger_source: record.trigger_source,
        status: record.status,
        summary: record.summary,
        scheduled_at: record.scheduled_at,
        started_at: record.started_at,
        finished_at: record.finished_at,
        created_at: record.created_at,
    }
}

pub(super) fn record_execution(
    repository: Arc<Mutex<Box<dyn AutomationRepository>>>,
    event_bus: EventBusHandle,
    execution_runner: Arc<Mutex<Option<Arc<dyn AutomationExecutionRunner>>>>,
    automation_code: &str,
    trigger_source: AutomationTriggerSource,
    scheduled_at: Option<String>,
) -> Result<Option<AutomationExecutionRecord>, AutomationError> {
    let automation = {
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
            AutomationExecutionStatus::Running
        }
        (AutomationTriggerSource::Scheduler, _) => AutomationExecutionStatus::Skipped,
        (AutomationTriggerSource::Manual, _) => AutomationExecutionStatus::Running,
    };
    let summary = Some(match execution_status {
        AutomationExecutionStatus::Running => {
            "Main agent execution started for the automation.".to_string()
        }
        AutomationExecutionStatus::Skipped => {
            "Scheduler tick skipped because the automation is not Active.".to_string()
        }
        _ => format!(
            "Execution recorded with status {}.",
            automation_execution_status_to_storage(execution_status)
        ),
    });
    let draft = AutomationExecutionDraft {
        automation_code: automation.automation_code.clone(),
        workspace_code: automation.workspace_code.clone(),
        session_code: None,
        trigger_source,
        status: execution_status,
        summary,
        scheduled_at: if trigger_source == AutomationTriggerSource::Scheduler {
            Some(scheduled_at.unwrap_or_else(|| now.clone()))
        } else {
            None
        },
        started_at: now.clone(),
        finished_at: if execution_status == AutomationExecutionStatus::Skipped {
            Some(now.clone())
        } else {
            None
        },
        created_at: now.clone(),
    };
    let Some(record) = repository
        .lock()
        .expect("automation repository mutex poisoned")
        .record_execution(&automation, &draft)?
    else {
        return Ok(None);
    };
    publish_automation_execution_event(
        event_bus,
        "automation.execution_recorded",
        "Recorded automation execution",
        &record,
    );
    if record.status == AutomationExecutionStatus::Running {
        let runner = execution_runner
            .lock()
            .expect("automation execution runner mutex poisoned")
            .clone();
        if let Some(runner) = runner {
            runner.dispatch(automation, record.clone());
        }
    }
    Ok(Some(record))
}

pub(super) fn normalized_required(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

pub(super) fn normalized_agent_code(value: Option<String>) -> String {
    normalized_required(value).unwrap_or_else(|| DEFAULT_AUTOMATION_AGENT_CODE.to_string())
}

pub(super) fn optional_trimmed(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

pub(super) fn normalized_scheduler_expression(
    cron_expression: &str,
) -> Result<String, AutomationError> {
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

pub(super) fn next_run_preview(
    scheduler_expression: &str,
    status: AutomationStatus,
) -> Option<String> {
    if status != AutomationStatus::Active {
        return None;
    }
    Schedule::from_str(scheduler_expression)
        .ok()?
        .upcoming(Utc)
        .next()
        .map(format_timestamp)
}

pub(super) fn current_timestamp() -> String {
    format_timestamp(Utc::now())
}

pub(super) fn format_timestamp(value: chrono::DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub(super) fn map_workspace_error(error: WorkspaceError) -> AutomationError {
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

pub(super) fn publish_automation_execution_event(
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
