use super::*;

#[derive(Debug)]
pub enum CancelDudeTaskError {
    Completed,
    Task(TaskError),
}

impl From<TaskError> for CancelDudeTaskError {
    fn from(error: TaskError) -> Self {
        Self::Task(error)
    }
}

impl CancelDudeTaskError {
    pub fn descriptor(&self) -> crate::application::dude::ApplicationErrorDescriptor {
        let (code, message_key) = match self {
            Self::Completed => ("101018006", "tron.dude.task_completed"),
            Self::Task(TaskError::ActiveSubtaskExists) => {
                ("101018007", "tron.dude.task_has_active_subtasks")
            }
            Self::Task(TaskError::NotFound) => ("101018001", "tron.dude.not_found"),
            Self::Task(TaskError::InvalidRequest) => ("101018003", "tron.dude.state_changed"),
            Self::Task(TaskError::StorageFailure) => ("101018004", "tron.dude.storage_failed"),
            Self::Task(_) => ("101018002", "tron.dude.request_invalid"),
        };
        crate::application::dude::ApplicationErrorDescriptor { code, message_key }
    }
}

/// Begin a day before a deadline; work without a deadline starts immediately.
/// The policy is internal so creating a task only needs a title and optional deadline.
pub(super) fn default_planned_start(now_ms: i64, due_at: Option<i64>) -> i64 {
    due_at.map_or(now_ms, |due| {
        due.saturating_sub(24 * 60 * 60 * 1000).max(now_ms)
    })
}

impl TaskService {
    /// Retain completed or cancelled Dude tasks for three days after their last update.
    pub fn prune_finished_dude_tasks(&self, now_ms: i64) -> Result<usize, TaskError> {
        const RETENTION_MS: i64 = 3 * 24 * 60 * 60 * 1000;
        self.repository
            .lock()
            .expect("task repository mutex poisoned")
            .prune_finished_dude_tasks(now_ms.saturating_sub(RETENTION_MS))
    }

    pub fn cancel_dude_task(
        &self,
        dude_code: &str,
        task_code: &str,
    ) -> Result<(), CancelDudeTaskError> {
        let task = self.get_task_record(task_code)?;
        if task.owner_dude_code.as_deref() != Some(dude_code) {
            return Err(TaskError::NotFound.into());
        }
        if task.archived_at.is_none() && task.status == crate::domain::task::CLOSED_STATUS_CODE {
            return Err(CancelDudeTaskError::Completed);
        }
        self.archive_task_record(task)?;
        Ok(())
    }

    pub fn due_dude_tasks(&self, now_ms: i64) -> Result<Vec<TaskDetail>, TaskError> {
        let all = self.all_tasks()?;
        let hierarchy = TaskHierarchy::new(&all);
        let selected = all
            .iter()
            .filter(|task| {
                task.owner_dude_code.is_some()
                    && task.archived_at.is_none()
                    && task.status != crate::domain::task::CLOSED_STATUS_CODE
                    && task.next_followup_at.is_some_and(|next| next <= now_ms)
            })
            .collect::<Vec<_>>();
        let codes = selected
            .iter()
            .map(|task| task.task_code.clone())
            .collect::<Vec<_>>();
        let mut relations = self
            .repository
            .lock()
            .expect("task repository mutex poisoned")
            .relations(&codes)?;
        selected
            .into_iter()
            .map(|task| {
                self.detail_with_relations(
                    task.clone(),
                    &hierarchy,
                    relations.remove(&task.task_code).unwrap_or_default(),
                )
            })
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
