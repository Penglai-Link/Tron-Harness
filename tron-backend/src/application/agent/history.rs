use std::sync::{Arc, Mutex};

use crate::domain::agent::{
    AgentRunHistoryError, AgentRunHistoryQuery, AgentRunHistoryRecord, PagedAgentRunHistoryRecords,
};

use super::AgentRunHistoryRepository;

#[derive(Clone)]
pub struct AgentRunHistoryService {
    repository: Arc<Mutex<Box<dyn AgentRunHistoryRepository>>>,
}

impl AgentRunHistoryService {
    pub fn new(repository: impl AgentRunHistoryRepository + 'static) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
        }
    }

    pub fn create_run(
        &self,
        mut record: AgentRunHistoryRecord,
    ) -> Result<AgentRunHistoryRecord, AgentRunHistoryError> {
        let mut repository = self
            .repository
            .lock()
            .expect("agent run history repository mutex poisoned");
        record.id = repository.next_id()?;
        repository.create_run(&record)?;
        Ok(record)
    }

    pub fn list_runs(
        &self,
        query: AgentRunHistoryQuery,
    ) -> Result<PagedAgentRunHistoryRecords<AgentRunHistoryRecord>, AgentRunHistoryError> {
        let current = query.current.unwrap_or(1).max(1);
        let size = query.size.unwrap_or(20).clamp(1, 100);
        let mut records = self
            .repository
            .lock()
            .expect("agent run history repository mutex poisoned")
            .list_runs()?;
        if let Some(workspace_code) = normalized_optional(query.workspace_code) {
            records.retain(|record| record.workspace_code == workspace_code);
        }
        if let Some(conversation_code) = normalized_optional(query.conversation_code) {
            records
                .retain(|record| record.conversation_code.as_deref() == Some(&conversation_code));
        }
        records.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then(right.id.cmp(&left.id))
        });
        let total = records.len();
        let start = current.saturating_sub(1).saturating_mul(size);
        Ok(PagedAgentRunHistoryRecords {
            current,
            size,
            total,
            records: records.into_iter().skip(start).take(size).collect(),
        })
    }

    pub fn get_run(&self, run_code: &str) -> Result<AgentRunHistoryRecord, AgentRunHistoryError> {
        self.repository
            .lock()
            .expect("agent run history repository mutex poisoned")
            .get_run(run_code)
    }
}

fn normalized_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
