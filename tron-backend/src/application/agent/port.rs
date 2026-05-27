use crate::domain::agent::{AgentRunHistoryError, AgentRunHistoryRecord};

pub trait AgentRunHistoryRepository: Send {
    fn list_runs(&mut self) -> Result<Vec<AgentRunHistoryRecord>, AgentRunHistoryError>;
    fn get_run(&mut self, run_code: &str) -> Result<AgentRunHistoryRecord, AgentRunHistoryError>;
    fn next_id(&mut self) -> Result<i64, AgentRunHistoryError>;
    fn create_run(&mut self, record: &AgentRunHistoryRecord) -> Result<(), AgentRunHistoryError>;
}
