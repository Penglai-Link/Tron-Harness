use super::*;

impl TaskService {
    pub(super) fn lifecycle_result(
        &self,
        task_code: &str,
    ) -> Result<TaskLifecycleResult, TaskError> {
        let task = self.get_task_record(task_code)?;
        Ok(TaskLifecycleResult {
            task_code: task.task_code,
            status: task.status,
            previous_status: task.previous_status,
            archived_at: task.archived_at,
        })
    }

    pub(super) fn filtered_task_items(
        &self,
        query: &TaskListQuery,
    ) -> Result<Vec<TaskListItem>, TaskError> {
        let workspace_code = query
            .workspace_code
            .as_deref()
            .and_then(|value| normalized_required(value.to_owned()))
            .ok_or(TaskError::WorkspaceRequired)?;
        self.ensure_active_workspace(&workspace_code)?;
        let include_archived = query.include_archived.unwrap_or(false);
        let all_tasks = self.all_tasks()?;
        let mut records = all_tasks
            .iter()
            .filter(|task| task.workspace_code == workspace_code)
            .filter(|task| include_archived || task.status != TaskStatus::Archived)
            .filter(|task| query.status.map_or(true, |status| task.status == status))
            .filter(|task| {
                query
                    .priority
                    .map_or(true, |priority| task.priority == priority)
            })
            .filter(|task| {
                query
                    .task_type
                    .map_or(true, |task_type| task.task_type == task_type)
            })
            .filter(|task| {
                query
                    .source_type
                    .map_or(true, |source_type| task.source_type == source_type)
            })
            .filter(|task| parent_filter_matches(task, query.parent_task_code.as_deref()))
            .filter(|task| matches_search(task, query.search.as_deref()))
            .map(|task| self.item_from_task(task, &all_tasks))
            .collect::<Result<Vec<_>, _>>()?;
        if let Some(assignee_user_code) = query
            .assignee_user_code
            .as_deref()
            .and_then(|value| normalized_required(value.to_owned()))
        {
            records.retain(|record| record.assignee_user_codes.contains(&assignee_user_code));
        }
        records.sort_by(|left, right| left.task_code.cmp(&right.task_code));
        Ok(records)
    }

    pub(super) fn detail_from_task(&self, task: Task) -> Result<TaskDetail, TaskError> {
        let all_tasks = self.all_tasks()?;
        let parent = task
            .parent_task_code
            .as_deref()
            .and_then(|parent_code| {
                all_tasks
                    .iter()
                    .find(|candidate| candidate.task_code == parent_code)
            })
            .map(|parent| summary_from_task(parent, &all_tasks));
        let mut subtasks = all_tasks
            .iter()
            .filter(|candidate| candidate.parent_task_code.as_deref() == Some(&task.task_code))
            .filter(|candidate| candidate.status != TaskStatus::Archived)
            .map(|candidate| summary_from_task(candidate, &all_tasks))
            .collect::<Vec<_>>();
        subtasks.sort_by(|left, right| left.task_code.cmp(&right.task_code));
        let assignee_user_codes = self
            .raw_user_assignments(&task.task_code)?
            .into_iter()
            .map(|entry| entry.user_code)
            .collect();
        let agent_codes = self
            .raw_agent_links(&task.task_code)?
            .into_iter()
            .map(|entry| entry.code)
            .collect();
        let app_codes = self
            .raw_app_links(&task.task_code)?
            .into_iter()
            .map(|entry| entry.code)
            .collect();
        let progress = effective_progress(&task, &all_tasks);
        Ok(TaskDetail {
            id: task.id.as_decimal_string(),
            task_code: task.task_code,
            workspace_code: task.workspace_code,
            parent_task_code: task.parent_task_code,
            title: task.title,
            description: task.description,
            execution_result: task.execution_result,
            status: task.status,
            previous_status: task.previous_status,
            priority: task.priority,
            task_type: task.task_type,
            source_type: task.source_type,
            source_url: task.source_url,
            agent_code: task.agent_code,
            model_code: task.model_code,
            progress,
            assignee_user_codes,
            agent_codes,
            app_codes,
            parent,
            subtasks,
            created_at: task.created_at,
            updated_at: task.updated_at,
            archived_at: task.archived_at,
        })
    }

    pub(super) fn item_from_task(
        &self,
        task: &Task,
        all_tasks: &[Task],
    ) -> Result<TaskListItem, TaskError> {
        Ok(TaskListItem {
            task_code: task.task_code.clone(),
            workspace_code: task.workspace_code.clone(),
            parent_task_code: task.parent_task_code.clone(),
            title: task.title.clone(),
            status: task.status,
            priority: task.priority,
            progress: effective_progress(task, all_tasks),
            task_type: task.task_type,
            source_type: task.source_type,
            source_url: task.source_url.clone(),
            agent_code: task.agent_code.clone(),
            model_code: task.model_code.clone(),
            assignee_user_codes: self
                .raw_user_assignments(&task.task_code)?
                .into_iter()
                .map(|entry| entry.user_code)
                .collect(),
            agent_codes: self
                .raw_agent_links(&task.task_code)?
                .into_iter()
                .map(|entry| entry.code)
                .collect(),
            app_codes: self
                .raw_app_links(&task.task_code)?
                .into_iter()
                .map(|entry| entry.code)
                .collect(),
            subtask_count: all_tasks
                .iter()
                .filter(|candidate| candidate.parent_task_code.as_deref() == Some(&task.task_code))
                .filter(|candidate| candidate.status != TaskStatus::Archived)
                .count(),
            updated_at: task.updated_at.clone(),
            archived_at: task.archived_at.clone(),
        })
    }
}
