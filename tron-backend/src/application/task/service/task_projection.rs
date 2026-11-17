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
        self.filtered_task_page(query, None)
            .map(|(records, _)| records)
    }

    pub(super) fn filtered_task_page(
        &self,
        query: &TaskListQuery,
        window: Option<(usize, usize)>,
    ) -> Result<(Vec<TaskListItem>, usize), TaskError> {
        let workspace_code = optional_trimmed(query.workspace_code.clone());
        if let Some(workspace) = workspace_code.as_deref() {
            self.ensure_active_workspace(workspace)?;
        }
        let mut page = self.reader.page(query, window)?;
        let hierarchy = TaskHierarchy::new(&page.tasks);
        let records = page
            .page_codes
            .iter()
            .map(|code| {
                let task = hierarchy.task(code).ok_or(TaskError::StorageFailure)?;
                self.item_from_task(
                    task,
                    &hierarchy,
                    page.relations.remove(code).unwrap_or_default(),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok((records, page.total))
    }

    pub(super) fn detail_from_task(&self, task: Task) -> Result<TaskDetail, TaskError> {
        let all_tasks = self.all_tasks()?;
        let mut relations = self.task_relations(&[task.task_code.clone()])?;
        let links = relations.remove(&task.task_code).unwrap_or_default();
        self.detail_with_relations(task, &TaskHierarchy::new(&all_tasks), links)
    }

    pub(super) fn detail_with_relations(
        &self,
        task: Task,
        hierarchy: &TaskHierarchy<'_>,
        links: TaskRelations,
    ) -> Result<TaskDetail, TaskError> {
        let parent = task
            .parent_task_code
            .as_deref()
            .and_then(|code| hierarchy.task(code))
            .map(|parent| summary_from_task(parent, hierarchy));
        let mut subtasks = hierarchy
            .children(&task.task_code)
            .iter()
            .map(|child| summary_from_task(child, hierarchy))
            .collect::<Vec<_>>();
        subtasks.sort_by(|left, right| left.task_code.cmp(&right.task_code));
        let assignee_user_codes = links
            .assignees
            .into_iter()
            .map(|entry| entry.user_code)
            .collect();
        let agent_codes = links.agents.into_iter().map(|entry| entry.code).collect();
        let app_codes = links.apps.into_iter().map(|entry| entry.code).collect();
        let progress = hierarchy.progress(&task);
        Ok(TaskDetail {
            id: task.id.as_decimal_string(),
            task_code: task.task_code,
            workspace_code: task.workspace_code,
            owner_assistant_code: task.owner_assistant_code,
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
        hierarchy: &TaskHierarchy<'_>,
        links: TaskRelations,
    ) -> Result<TaskListItem, TaskError> {
        Ok(TaskListItem {
            task_code: task.task_code.clone(),
            workspace_code: task.workspace_code.clone(),
            owner_assistant_code: task.owner_assistant_code.clone(),
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
            progress: hierarchy.progress(task),
            task_type: task.task_type,
            source_type: task.source_type,
            source_url: task.source_url.clone(),
            agent_code: task.agent_code.clone(),
            model_code: task.model_code.clone(),
            assignee_user_codes: links
                .assignees
                .into_iter()
                .map(|entry| entry.user_code)
                .collect(),
            agent_codes: links.agents.into_iter().map(|entry| entry.code).collect(),
            app_codes: links.apps.into_iter().map(|entry| entry.code).collect(),
            subtask_count: hierarchy.children(&task.task_code).len(),
            updated_at: task.updated_at.clone(),
            archived_at: task.archived_at.clone(),
        })
    }
}
