use super::*;
use crate::adapters::outbound::persistence::sqlite::framework_storage::blocking;
use crate::domain::short_term_memory::ShortTermMemoryError;
use eliteforge_agent_core as fw;
use serde_json::Value;

pub struct SavedContextAssembly {
    pub sequence: u64,
    pub recorded_at: u64,
    pub report: fw::ContextAssemblyReport,
    pub budget_breakdown: Value,
}
pub struct ContextAssemblyErrorDescriptor {
    pub code: &'static str,
    pub message_key: &'static str,
}
pub fn context_assembly_error_descriptor(
    error: &ShortTermMemoryError,
) -> ContextAssemblyErrorDescriptor {
    let (code, message_key) = match error {
        ShortTermMemoryError::SessionNotFound => {
            ("101041001", "tron.chat.context_assembly_not_found")
        }
        ShortTermMemoryError::GoalInvalid => {
            ("101041002", "tron.chat.context_assembly_request_invalid")
        }
        _ => ("101041003", "tron.chat.context_assembly_read_failed"),
    };
    ContextAssemblyErrorDescriptor { code, message_key }
}
impl ShortTermMemoryService {
    pub(crate) async fn context_snapshot_event(
        &self,
        id: &str,
        sequence: Option<u64>,
    ) -> Result<Option<fw::SessionEvent>, ShortTermMemoryError> {
        let events = self
            .framework()
            .await?
            .events()
            .map_err(|_| ShortTermMemoryError::StorageFailure)?;
        if let Some(sequence) = sequence {
            if sequence == 0 {
                return Err(ShortTermMemoryError::GoalInvalid);
            }
            Ok(events
                .list(id, sequence - 1, 1)
                .await
                .map_err(|_| ShortTermMemoryError::StorageFailure)?
                .into_iter()
                .find(|event| event.sequence == sequence && event.kind == "tron.context-view"))
        } else {
            events
                .latest_by_kind(id, "tron.context-view")
                .await
                .map_err(|_| ShortTermMemoryError::StorageFailure)
        }
    }
    pub fn saved_context_assembly(
        &self,
        id: &str,
        sequence: Option<u64>,
    ) -> Result<Option<SavedContextAssembly>, ShortTermMemoryError> {
        self.ensure_session(id)?;
        blocking(async {
            let Some(event) = self.context_snapshot_event(id, sequence).await? else {
                return Ok(None);
            };
            let report = serde_json::from_value(event.payload["contextReport"].clone())
                .map_err(|_| ShortTermMemoryError::StorageFailure)?;
            Ok(Some(SavedContextAssembly {
                sequence: event.sequence,
                recorded_at: event.created_at_ms,
                report,
                budget_breakdown: event.payload["budgetBreakdown"].clone(),
            }))
        })
    }
}
