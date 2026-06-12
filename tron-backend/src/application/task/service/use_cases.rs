use super::*;

impl TaskService {
    pub fn options(&self) -> TaskOptions {
        task_options()
    }

    pub fn list_tasks(
        &self,
        query: TaskListQuery,
    ) -> Result<PagedTaskRecords<TaskListItem>, TaskError> {
        let current = query.current.unwrap_or(1).max(1);
        let size = query.size.unwrap_or(20).clamp(1, 100);
        let records = self.filtered_task_items(&query)?;
        let total = records.len();
        let start = (current - 1) * size;
        Ok(PagedTaskRecords {
            current,
            size,
            total,
            records: records.into_iter().skip(start).take(size).collect(),
        })
    }

    pub fn kanban(&self, query: TaskListQuery) -> Result<Vec<TaskKanbanColumn>, TaskError> {
        let include_archived = query.include_archived.unwrap_or(false);
        let mut items = self.filtered_task_items(&query)?;
        sort_kanban_task_items(&mut items);
        let statuses = match query.task_type {
            Some(TaskType::Bug) => {
                task_statuses_for_role(crate::domain::task::TaskRole::Bug).to_vec()
            }
            Some(TaskType::Feature) => {
                task_statuses_for_role(crate::domain::task::TaskRole::FeatureRoot).to_vec()
            }
            None => TASK_STATUSES.to_vec(),
        };
        Ok(statuses
            .into_iter()
            .filter(|status| include_archived || *status != TaskStatus::Archived)
            .map(|status| TaskKanbanColumn {
                status,
                title: status_title(status).into(),
                tasks: items
                    .iter()
                    .filter(|item| item.status == status)
                    .cloned()
                    .collect(),
            })
            .collect())
    }

    pub fn create_task(&self, command: CreateTaskCommand) -> Result<TaskDetail, TaskError> {
        let workspace_code =
            normalized_required(command.workspace_code).ok_or(TaskError::WorkspaceRequired)?;
        self.ensure_active_workspace(&workspace_code)?;
        let title = normalized_required(command.title).ok_or(TaskError::InvalidRequest)?;
        let parent_task_code = optional_trimmed(command.parent_task_code);
        if let Some(parent_code) = parent_task_code.as_deref() {
            self.ensure_valid_parent(parent_code, &workspace_code)?;
        }
        let task_role = infer_task_role(command.task_type, parent_task_code.as_deref());
        let assignees = self.validated_assignees(&workspace_code, command.assignee_user_codes)?;
        let agent_links = normalized_code_links(command.agent_codes);
        let app_links =
            self.validated_app_links(&workspace_code, normalized_code_links(command.app_codes))?;
        let id = self
            .repository
            .lock()
            .expect("task repository mutex poisoned")
            .next_id()?;
        let task_code = format!("TSK-{id}");
        let task = Task {
            id: SnowflakeId::new(id),
            task_code: task_code.clone(),
            workspace_code,
            parent_task_code,
            title,
            description: optional_trimmed(command.description),
            status: task_default_status_for_role(task_role),
            previous_status: None,
            priority: command.priority,
            task_type: command.task_type,
            source_type: command.source_type,
            source_url: optional_source_url(command.source_url),
            progress: 0,
            created_at: current_timestamp(),
            updated_at: current_timestamp(),
            archived_at: None,
        };
        {
            let mut repository = self
                .repository
                .lock()
                .expect("task repository mutex poisoned");
            repository.create_task(&task)?;
            repository.replace_user_assignments(&task_code, &assignees)?;
            repository.replace_agent_links(&task_code, &agent_links)?;
            repository.replace_app_links(&task_code, &app_links)?;
        }
        let detail = self.get_task(&task_code)?;
        self.publish_task_detail_event("task.created", "Created task", &detail);
        Ok(detail)
    }

    pub fn get_task(&self, task_code: &str) -> Result<TaskDetail, TaskError> {
        self.detail_from_task(self.get_task_record(task_code)?)
    }

    pub fn update_task(
        &self,
        task_code: &str,
        command: UpdateTaskCommand,
    ) -> Result<TaskDetail, TaskError> {
        let mut task = self.get_task_record(task_code)?;
        if task.status == TaskStatus::Archived {
            return Err(TaskError::ArchivedMutationDenied);
        }
        if let Some(title) = command.title {
            task.title = normalized_required(title).ok_or(TaskError::InvalidRequest)?;
        }
        if let Some(description) = command.description {
            task.description = description.and_then(|value| optional_trimmed(Some(value)));
        }
        if let Some(priority) = command.priority {
            task.priority = priority;
        }
        if let Some(task_type) = command.task_type {
            let next_role = infer_task_role(task_type, task.parent_task_code.as_deref());
            if !task_status_allowed_for_role(next_role, task.status) {
                return Err(TaskError::StatusRoleInvalid);
            }
            task.task_type = task_type;
        }
        if let Some(source_type) = command.source_type {
            task.source_type = source_type;
        }
        if let Some(source_url) = command.source_url {
            task.source_url = optional_source_url(source_url);
        }
        task.updated_at = current_timestamp();
        let assignees = command
            .assignee_user_codes
            .map(|codes| self.validated_assignees(&task.workspace_code, codes))
            .transpose()?;
        let agent_links = command.agent_codes.map(normalized_code_links);
        let app_links = command
            .app_codes
            .map(|codes| {
                self.validated_app_links(&task.workspace_code, normalized_code_links(codes))
            })
            .transpose()?;
        {
            let mut repository = self
                .repository
                .lock()
                .expect("task repository mutex poisoned");
            repository.update_task(&task)?;
            if let Some(entries) = assignees {
                repository.replace_user_assignments(task_code, &entries)?;
            }
            if let Some(entries) = agent_links {
                repository.replace_agent_links(task_code, &entries)?;
            }
            if let Some(entries) = app_links {
                repository.replace_app_links(task_code, &entries)?;
            }
        }
        let detail = self.get_task(task_code)?;
        self.publish_task_detail_event("task.updated", "Updated task", &detail);
        Ok(detail)
    }

    pub fn update_status(
        &self,
        task_code: &str,
        command: UpdateTaskStatusCommand,
    ) -> Result<TaskDetail, TaskError> {
        if command.status == TaskStatus::Archived {
            return Err(TaskError::InvalidRequest);
        }
        let mut task = self.get_task_record(task_code)?;
        if task.status == TaskStatus::Archived {
            return Err(TaskError::ArchivedMutationDenied);
        }
        let task_role = infer_task_role(task.task_type, task.parent_task_code.as_deref());
        if !task_status_allowed_for_role(task_role, command.status) {
            return Err(TaskError::StatusRoleInvalid);
        }
        task.status = command.status;
        task.progress = if command.status == TaskStatus::Completed {
            100
        } else {
            command.progress.unwrap_or(task.progress).min(100)
        };
        task.updated_at = current_timestamp();
        self.repository
            .lock()
            .expect("task repository mutex poisoned")
            .update_task(&task)?;
        let detail = self.get_task(task_code)?;
        self.publish_task_detail_event("task.status_updated", "Updated task status", &detail);
        Ok(detail)
    }

    pub fn archive_task(&self, task_code: &str) -> Result<TaskLifecycleResult, TaskError> {
        let mut task = self.get_task_record(task_code)?;
        if task.status != TaskStatus::Archived {
            let has_active_subtasks = self.all_tasks()?.iter().any(|candidate| {
                candidate.parent_task_code.as_deref() == Some(&task.task_code)
                    && candidate.status != TaskStatus::Archived
            });
            if has_active_subtasks {
                return Err(TaskError::ActiveSubtaskExists);
            }
            task.previous_status = Some(task.status);
            task.status = TaskStatus::Archived;
            task.archived_at = Some(current_timestamp());
            task.updated_at = current_timestamp();
            self.repository
                .lock()
                .expect("task repository mutex poisoned")
                .update_task(&task)?;
        }
        let result = self.lifecycle_result(task_code)?;
        self.publish_task_lifecycle_event("task.archived", "Archived task", &result);
        Ok(result)
    }

    pub fn restore_task(&self, task_code: &str) -> Result<TaskLifecycleResult, TaskError> {
        let mut task = self.get_task_record(task_code)?;
        if task.status == TaskStatus::Archived {
            let task_role = infer_task_role(task.task_type, task.parent_task_code.as_deref());
            task.status = task
                .previous_status
                .filter(|status| task_status_allowed_for_role(task_role, *status))
                .unwrap_or_else(|| task_default_status_for_role(task_role));
            task.previous_status = None;
            task.archived_at = None;
            task.updated_at = current_timestamp();
            self.repository
                .lock()
                .expect("task repository mutex poisoned")
                .update_task(&task)?;
        }
        let result = self.lifecycle_result(task_code)?;
        self.publish_task_lifecycle_event("task.restored", "Restored task", &result);
        Ok(result)
    }
}

fn sort_kanban_task_items(items: &mut [TaskListItem]) {
    items.sort_by(compare_kanban_task_items);
}

fn compare_kanban_task_items(left: &TaskListItem, right: &TaskListItem) -> std::cmp::Ordering {
    kanban_priority_rank(left.priority)
        .cmp(&kanban_priority_rank(right.priority))
        .then_with(|| {
            match (
                task_code_sequence(&left.task_code),
                task_code_sequence(&right.task_code),
            ) {
                (Some(left_sequence), Some(right_sequence)) => right_sequence.cmp(&left_sequence),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => right.task_code.cmp(&left.task_code),
            }
        })
        .then_with(|| right.task_code.cmp(&left.task_code))
}

fn kanban_priority_rank(priority: TaskPriority) -> u8 {
    match priority {
        TaskPriority::High => 0,
        TaskPriority::Medium => 1,
        TaskPriority::Low => 2,
    }
}

fn task_code_sequence(task_code: &str) -> Option<i64> {
    let digits = task_code
        .chars()
        .rev()
        .take_while(|character| character.is_ascii_digit())
        .collect::<Vec<_>>();
    if digits.is_empty() {
        return None;
    }
    digits
        .into_iter()
        .rev()
        .collect::<String>()
        .parse::<i64>()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list_item(priority: TaskPriority, task_code: &str) -> TaskListItem {
        TaskListItem {
            task_code: task_code.into(),
            workspace_code: "TRON_CORE".into(),
            parent_task_code: None,
            title: task_code.into(),
            status: TaskStatus::Open,
            priority,
            progress: 0,
            task_type: TaskType::Feature,
            source_type: TaskSourceType::Internal,
            source_url: Some(String::new()),
            assignee_user_codes: vec![],
            agent_codes: vec![],
            app_codes: vec![],
            subtask_count: 0,
            updated_at: "2026-06-09T00:00:00Z".into(),
            archived_at: None,
        }
    }

    #[test]
    fn sort_kanban_task_items_prioritizes_priority_then_descending_task_code() {
        let mut items = vec![
            list_item(TaskPriority::Medium, "TSK-12"),
            list_item(TaskPriority::High, "TSK-10"),
            list_item(TaskPriority::Low, "TSK-999"),
            list_item(TaskPriority::High, "TSK-20"),
            list_item(TaskPriority::Medium, "TSK-30"),
        ];

        sort_kanban_task_items(&mut items);

        let task_codes = items
            .iter()
            .map(|item| item.task_code.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            task_codes,
            vec!["TSK-20", "TSK-10", "TSK-30", "TSK-12", "TSK-999"]
        );
    }
}
