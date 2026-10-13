use super::*;

/// Begin a day before a deadline; work without a deadline starts immediately.
/// The policy is internal so creating a task only needs a title and optional deadline.
pub(super) fn default_planned_start(now_ms: i64, due_at: Option<i64>) -> i64 {
    due_at.map_or(now_ms, |due| {
        due.saturating_sub(24 * 60 * 60 * 1000).max(now_ms)
    })
}

impl TaskService {
    pub fn due_dude_tasks(&self, now_ms: i64) -> Result<Vec<TaskDetail>, TaskError> {
        self.all_tasks()?
            .into_iter()
            .filter(|task| {
                task.owner_dude_code.is_some()
                    && task.archived_at.is_none()
                    && task.status != crate::domain::task::CLOSED_STATUS_CODE
                    && task.next_followup_at.is_some_and(|next| next <= now_ms)
            })
            .map(|task| self.detail_from_task(task))
            .collect()
    }

    /// Atomic claim fences concurrent schedulers and invalidates previously queued reminders.
    /// A persisted follow-up serves as a recovery lease if the host exits before enqueueing.
    pub fn claim_dude_task(
        &self,
        task_code: &str,
        expected_version: i64,
        now_ms: i64,
    ) -> Result<bool, TaskError> {
        self.repository
            .lock()
            .expect("task repository mutex poisoned")
            .claim_dude_task(task_code, expected_version, now_ms)
    }

    pub fn bind_dude_work_session(
        &self,
        task_code: &str,
        session_code: &str,
    ) -> Result<bool, TaskError> {
        let session =
            normalized_required(session_code.to_owned()).ok_or(TaskError::InvalidRequest)?;
        self.repository
            .lock()
            .expect("task repository mutex poisoned")
            .bind_dude_work_session(task_code, &session)
    }

    /// Recheck immediately before consuming a reminder: edits/completion invalidate old events.
    pub fn dude_task_reminder_is_current(
        &self,
        task_code: &str,
        expected_version: i64,
    ) -> Result<bool, TaskError> {
        let task = self.get_task_record(task_code)?;
        Ok(task.owner_dude_code.is_some()
            && task.schedule_version == expected_version
            && task.archived_at.is_none()
            && task.status != crate::domain::task::CLOSED_STATUS_CODE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn task_service() -> TaskService {
        use crate::adapters::outbound::persistence::{
            SqliteAppRepository, SqliteTaskRepository, SqliteWorkspaceRepository,
        };
        let workspaces =
            WorkspaceService::new(SqliteWorkspaceRepository::connect(":memory:").unwrap());
        let apps = AppService::new(
            SqliteAppRepository::connect(":memory:").unwrap(),
            workspaces.clone(),
        );
        TaskService::new(
            SqliteTaskRepository::connect(":memory:").unwrap(),
            workspaces,
            apps,
        )
    }

    fn create_platform_task(tasks: &TaskService) -> TaskDetail {
        tasks
            .create_task(CreateTaskCommand {
                workspace_code: None,
                owner_dude_code: Some("main-dude".into()),
                due_at: None,
                parent_task_code: None,
                title: "Prepare report".into(),
                description: None,
                priority: TaskPriority::Medium,
                task_type: TaskType::General,
                source_type: TaskSourceType::Internal,
                source_url: None,
                agent_code: None,
                model_code: None,
                assignee_user_codes: vec![],
                agent_codes: vec![],
                app_codes: vec![],
            })
            .unwrap()
    }

    #[test]
    fn concurrent_dude_sessions_create_distinct_tasks() {
        let tasks = task_service();
        let barrier = Arc::new(std::sync::Barrier::new(8));
        let handles = (0..8)
            .map(|_| {
                let tasks = tasks.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    create_platform_task(&tasks).task_code
                })
            })
            .collect::<Vec<_>>();
        let codes = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<BTreeSet<_>>();
        assert_eq!(codes.len(), 8);
    }

    #[test]
    fn platform_tasks_are_isolated_and_edits_invalidate_claimed_events() {
        let tasks = task_service();
        let task = create_platform_task(&tasks);
        let now = chrono::Utc::now().timestamp_millis();
        assert_eq!(tasks.due_dude_tasks(now).unwrap().len(), 1);
        assert!(
            tasks
                .list_tasks(TaskListQuery::default())
                .unwrap()
                .records
                .is_empty()
        );
        let owned = tasks
            .list_tasks(TaskListQuery {
                owner_dude_code: Some("main-dude".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(owned.total, 1);
        assert_eq!(owned.records[0].workspace_code, None);
        assert!(tasks.claim_dude_task(&task.task_code, 0, now).unwrap());
        assert!(
            tasks
                .dude_task_reminder_is_current(&task.task_code, 1)
                .unwrap()
        );
        tasks
            .update_task(
                &task.task_code,
                UpdateTaskCommand {
                    due_at: Some(Some(now + 3 * 86_400_000)),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(
            !tasks
                .dude_task_reminder_is_current(&task.task_code, 1)
                .unwrap()
        );
        assert!(tasks.due_dude_tasks(now).unwrap().is_empty());
        tasks.archive_task(&task.task_code).unwrap();
        assert!(tasks.due_dude_tasks(i64::MAX).unwrap().is_empty());
        tasks.restore_task(&task.task_code).unwrap();
        assert_eq!(tasks.due_dude_tasks(i64::MAX).unwrap().len(), 1);
        tasks
            .record_execution_result(&task.task_code, "Report prepared".into())
            .unwrap();
        assert_eq!(tasks.due_dude_tasks(i64::MAX).unwrap().len(), 1);
        tasks.update_status(&task.task_code, UpdateTaskStatusCommand {
            status: crate::domain::task::CLOSED_STATUS_CODE.into(), progress: None,
        }).unwrap();
        assert!(tasks.due_dude_tasks(i64::MAX).unwrap().is_empty());
    }

    #[test]
    fn plans_before_deadline_and_starts_overdue_work_now() {
        let now = 5_000_000_000;
        let day = 86_400_000;
        assert_eq!(
            default_planned_start(now, Some(now + day * 3)),
            now + day * 2
        );
        assert_eq!(default_planned_start(now, Some(now - 1)), now);
        assert_eq!(default_planned_start(now, None), now);
    }
}
