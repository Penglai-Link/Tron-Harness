use super::*;

impl TaskService {
    pub(super) fn get_task_record(&self, task_code: &str) -> Result<Task, TaskError> {
        let normalized = task_code.trim();
        if normalized.is_empty() {
            return Err(TaskError::NotFound);
        }
        self.all_tasks()?
            .into_iter()
            .find(|task| task.task_code == normalized)
            .ok_or(TaskError::NotFound)
    }
    pub(super) fn all_tasks(&self) -> Result<Vec<Task>, TaskError> {
        self.repository
            .lock()
            .expect("task repository mutex poisoned")
            .list_tasks()
    }
    pub(super) fn raw_user_assignments(
        &self,
        task_code: &str,
    ) -> Result<Vec<TaskUserAssignment>, TaskError> {
        self.repository
            .lock()
            .expect("task repository mutex poisoned")
            .list_user_assignments(task_code)
    }
    pub(super) fn raw_agent_links(&self, task_code: &str) -> Result<Vec<TaskCodeLink>, TaskError> {
        self.repository
            .lock()
            .expect("task repository mutex poisoned")
            .list_agent_links(task_code)
    }
    pub(super) fn raw_app_links(&self, task_code: &str) -> Result<Vec<TaskCodeLink>, TaskError> {
        self.repository
            .lock()
            .expect("task repository mutex poisoned")
            .list_app_links(task_code)
    }
}
