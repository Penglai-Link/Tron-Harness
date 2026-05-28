use super::*;

pub(super) fn to_execution_list_item(
    record: AutomationExecutionRecord,
) -> AutomationExecutionListItem {
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

pub(super) fn record_execution(
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

pub(super) fn normalized_required(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
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
