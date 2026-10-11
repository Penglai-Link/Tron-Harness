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
        let workspace_code = optional_trimmed(query.workspace_code.clone());
        let owner_dude_code = optional_trimmed(query.owner_dude_code.clone());
        if let Some(workspace) = workspace_code.as_deref() {
            self.ensure_active_workspace(workspace)?;
        }
        let include_archived = query.include_archived.unwrap_or(false);
        let all_tasks = self.all_tasks()?;
        let mut records = all_tasks
            .iter()
            .filter(|task| {
                owner_dude_code
                    .as_ref()
                    .is_some_and(|owner| task.owner_dude_code.as_ref() == Some(owner))
                    || (owner_dude_code.is_none() && task.owner_dude_code.is_none())
            })
            .filter(|task| {
                owner_dude_code.is_some() && workspace_code.is_none()
                    || task.workspace_code == workspace_code
            })
            .filter(|task| include_archived || task.archived_at.is_none())
            .filter(|task| {
                query
                    .status
                    .as_ref()
                    .map_or(true, |status| task.status == *status)
            })
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
            .filter(|candidate| candidate.archived_at.is_none())
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
            owner_dude_code: task.owner_dude_code,
            due_at: task.due_at,
            planned_start_at: task.planned_start_at,
            next_followup_at: task.next_followup_at,
            work_session_code: task.work_session_code,
            schedule_version: task.schedule_version,
            parent_task_code: task.parent_task_code,
            title: task.title,
            description: task.description,
            execution_result: task.execution_result,
            flow_code: task.flow_code,
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
            owner_dude_code: task.owner_dude_code.clone(),
            due_at: task.due_at.clone(),
            planned_start_at: task.planned_start_at.clone(),
            next_followup_at: task.next_followup_at.clone(),
            work_session_code: task.work_session_code.clone(),
            schedule_version: task.schedule_version.clone(),
            parent_task_code: task.parent_task_code.clone(),
            title: task.title.clone(),
            flow_code: task.flow_code.clone(),
            status: task.status.clone(),
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
                .filter(|candidate| candidate.archived_at.is_none())
                .count(),
            updated_at: task.updated_at.clone(),
            archived_at: task.archived_at.clone(),
        })
    }
}
