use super::*;

impl AutomationService {
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
        let start = current.saturating_sub(1).saturating_mul(size);
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
        let start = current.saturating_sub(1).saturating_mul(size);
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
}
