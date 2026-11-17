use super::service::*;
use crate::domain::assistant_memory::*;
use std::collections::BTreeMap;

impl AssistantMemoryService {
    /// Sources are supplied by the host from durable history; user/model input must not forge lineage.
    pub fn prepare_batch(
        &self,
        assistant: &str,
        user: &str,
        sources: Vec<MemorySource>,
    ) -> Result<Option<MemoryBatch>, AssistantMemoryError> {
        let mut repository = self.repository.lock().unwrap();
        if let Some(batch) = repository
            .list_batches(user, assistant)?
            .into_iter()
            .find(|b| b.status != MemoryBatchStatus::Completed)
        {
            return Ok(Some(batch));
        }
        let cursors = repository.cursor_versions(user, assistant)?;
        let mut unique = BTreeMap::new();
        for source in sources.into_iter().filter(|s| s.eligible_for(assistant)) {
            if cursors.get(&source.identity()) != Some(&source.version) {
                let key = source.identity();
                if let Some(previous) = unique.insert(key, source.clone()) {
                    if previous != source {
                        return Err(AssistantMemoryError::Invalid(
                            "source snapshot contains conflicting versions".into(),
                        ));
                    }
                }
            }
        }
        let mut source_bytes = 0;
        let sources: Vec<_> = unique
            .into_values()
            .take_while(|source| {
                source_bytes += source.excerpt.len();
                source_bytes <= 128 * 1024
            })
            .take(64)
            .collect();
        if sources.is_empty() {
            return Ok(None);
        }
        let batch = MemoryBatch {
            batch_code: new_code("memory-batch"),
            assistant_code: assistant.into(),
            user_code: user.into(),
            session_code: None,
            status: MemoryBatchStatus::Prepared,
            sources,
            memory_receipts: Vec::new(),
            saved_count: 0,
            duplicate_skipped_count: 0,
            unreliable_skipped_count: 0,
            consolidation_plan: None,
            summary: None,
            revision: 1,
            created_at: now(),
            updated_at: now(),
        };
        match repository.save_batch(None, &batch, false) {
            Ok(()) => Ok(Some(batch)),
            Err(AssistantMemoryError::Conflict) => Ok(repository
                .list_batches(user, assistant)?
                .into_iter()
                .find(|b| b.status != MemoryBatchStatus::Completed)),
            Err(error) => Err(error),
        }
    }
    pub fn attach_batch_session(
        &self,
        user: &str,
        code: &str,
        session: &str,
    ) -> Result<MemoryBatch, AssistantMemoryError> {
        let mut repository = self.repository.lock().unwrap();
        let previous = repository.get_batch(code)?;
        if previous.user_code != user {
            return Err(AssistantMemoryError::NotFound);
        }
        if previous.status == MemoryBatchStatus::Completed
            || previous
                .session_code
                .as_deref()
                .is_some_and(|s| s != session)
        {
            return Err(AssistantMemoryError::Conflict);
        }
        let mut batch = previous.clone();
        batch.session_code = Some(session.into());
        batch.status = MemoryBatchStatus::Running;
        batch.revision += 1;
        batch.updated_at = now();
        repository.save_batch(Some(&previous), &batch, false)?;
        Ok(batch)
    }
    pub fn cursor_snapshot(
        &self,
        user: &str,
        assistant: &str,
    ) -> Result<BTreeMap<String, String>, AssistantMemoryError> {
        self.repository
            .lock()
            .unwrap()
            .cursor_versions(user, assistant)
    }
    pub fn get_batch(&self, user: &str, code: &str) -> Result<MemoryBatch, AssistantMemoryError> {
        let batch = self.repository.lock().unwrap().get_batch(code)?;
        if batch.user_code != user {
            return Err(AssistantMemoryError::NotFound);
        }
        Ok(batch)
    }
    pub fn record_plan(
        &self,
        user: &str,
        code: &str,
        session: &str,
        plan: serde_json::Value,
    ) -> Result<MemoryBatch, AssistantMemoryError> {
        let mut repository = self.repository.lock().unwrap();
        let previous = repository.get_batch(code)?;
        if previous.user_code != user
            || previous.session_code.as_deref() != Some(session)
            || previous.status != MemoryBatchStatus::Running
        {
            return Err(AssistantMemoryError::Conflict);
        }
        if previous.consolidation_plan.is_some() {
            return Ok(previous);
        }
        let mut next = previous.clone();
        next.consolidation_plan = Some(plan);
        next.revision += 1;
        next.updated_at = now();
        repository.save_batch(Some(&previous), &next, false)?;
        Ok(next)
    }
    pub fn record_memory_receipt(
        &self,
        user: &str,
        code: &str,
        session: &str,
        receipt: MemoryWriteReceipt,
    ) -> Result<MemoryBatch, AssistantMemoryError> {
        let mut repository = self.repository.lock().unwrap();
        let previous = repository.get_batch(code)?;
        if previous.user_code != user
            || previous.session_code.as_deref() != Some(session)
            || previous.status != MemoryBatchStatus::Running
        {
            return Err(AssistantMemoryError::Conflict);
        }
        if previous
            .memory_receipts
            .iter()
            .any(|old| old.proposal_key == receipt.proposal_key)
        {
            return Ok(previous);
        }
        let mut next = previous.clone();
        next.memory_receipts.push(receipt);
        next.revision += 1;
        next.updated_at = now();
        repository.save_batch(Some(&previous), &next, false)?;
        Ok(next)
    }
    pub fn list_batches(
        &self,
        user: &str,
        assistant: &str,
    ) -> Result<Vec<MemoryBatch>, AssistantMemoryError> {
        self.repository
            .lock()
            .unwrap()
            .list_batches(user, assistant)
    }
    pub fn complete_batch(
        &self,
        user: &str,
        code: &str,
        session: &str,
        receipts: Vec<MemoryWriteReceipt>,
        summary: String,
    ) -> Result<MemoryBatch, AssistantMemoryError> {
        let mut repository = self.repository.lock().unwrap();
        let previous = repository.get_batch(code)?;
        if previous.user_code != user {
            return Err(AssistantMemoryError::NotFound);
        }
        if previous.status == MemoryBatchStatus::Completed {
            return Ok(previous);
        }
        if previous.session_code.as_deref() != Some(session) {
            return Err(AssistantMemoryError::Conflict);
        }
        for receipt in &receipts {
            if receipt.outcome == MemoryWriteOutcome::UnreliableSkipped {
                continue;
            }
            if receipt.semantic_key.trim().is_empty()
                || (receipt.outcome == MemoryWriteOutcome::Saved
                    && receipt.memory_code.trim().is_empty())
                || receipt.source_codes.is_empty()
                || receipt.source_codes.iter().any(|code| {
                    !previous.sources.iter().any(|source| {
                        source.source_code == *code
                            && source.workspace_code == receipt.workspace_code
                    })
                })
            {
                return Err(AssistantMemoryError::Invalid(
                    "memory writes must retain their workspace and source evidence".into(),
                ));
            }
        }
        let mut batch = previous.clone();
        batch.status = MemoryBatchStatus::Completed;
        batch.saved_count = receipts
            .iter()
            .filter(|r| r.outcome == MemoryWriteOutcome::Saved)
            .count();
        batch.duplicate_skipped_count = receipts
            .iter()
            .filter(|r| r.outcome == MemoryWriteOutcome::DuplicateSkipped)
            .count();
        batch.unreliable_skipped_count = receipts
            .iter()
            .filter(|r| r.outcome == MemoryWriteOutcome::UnreliableSkipped)
            .count();
        batch.memory_receipts = receipts;
        batch.summary = Some(summary);
        batch.revision += 1;
        batch.updated_at = now();
        repository.save_batch(Some(&previous), &batch, true)?;
        Ok(batch)
    }
}
