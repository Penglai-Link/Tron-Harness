use super::*;

impl TaskService {
    pub(super) fn get_task_record(&self, task_code: &str) -> Result<Task, TaskError> {
        let normalized = task_code.trim();
        if normalized.is_empty() {
            return Err(TaskError::NotFound);
        }
        self.reader.get_task(normalized)
    }
    pub(super) fn all_tasks(&self) -> Result<Vec<Task>, TaskError> {
        self.reader.list_tasks()
    }
    pub(super) fn task_relations(
        &self,
        codes: &[String],
    ) -> Result<std::collections::BTreeMap<String, TaskRelations>, TaskError> {
        self.reader.relations(codes)
    }
}
