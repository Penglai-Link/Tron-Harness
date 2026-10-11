use super::service::*;
use crate::domain::dude_memory::*;
use std::collections::BTreeMap;

impl DudeMemoryService {
    /// Sources are supplied by the host from durable history; user/model input must not forge lineage.
    pub fn prepare_batch(
        &self,
        dude: &str,
        user: &str,
        sources: Vec<MemorySource>,
    ) -> Result<Option<MemoryBatch>, DudeMemoryError> {
        let mut repository = self.repository.lock().unwrap();
        if let Some(batch) = repository
            .list_batches(user, dude)?
            .into_iter()
            .find(|b| b.status != MemoryBatchStatus::Completed)
        {
            return Ok(Some(batch));
        }
        let cursors = repository.cursor_versions(user, dude)?;
        let mut unique = BTreeMap::new();
        for source in sources.into_iter().filter(|s| s.eligible_for(dude)) {
            if cursors.get(&source.identity()) != Some(&source.version) {
                let key = source.identity();
                if let Some(previous) = unique.insert(key, source.clone()) {
                    if previous != source {
                        return Err(DudeMemoryError::Invalid(
                            "source snapshot contains conflicting versions".into(),
                        ));
                    }
                }
            }
        }
        let sources: Vec<_> = unique.into_values().collect();
        if sources.is_empty() {
            return Ok(None);
        }
        let batch = MemoryBatch {
            batch_code: new_code("memory-batch"),
            dude_code: dude.into(),
            user_code: user.into(),
            session_code: None,
            status: MemoryBatchStatus::Prepared,
            sources,
            memory_receipts: Vec::new(),
            summary: None,
            revision: 1,
            created_at: now(),
            updated_at: now(),
        };
        match repository.save_batch(None, &batch, false) {
            Ok(()) => Ok(Some(batch)),
            Err(DudeMemoryError::Conflict) => Ok(repository
                .list_batches(user, dude)?
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
    ) -> Result<MemoryBatch, DudeMemoryError> {
        let mut repository = self.repository.lock().unwrap();
        let previous = repository.get_batch(code)?;
        if previous.user_code != user {
            return Err(DudeMemoryError::NotFound);
        }
        if previous.status == MemoryBatchStatus::Completed
            || previous
                .session_code
                .as_deref()
                .is_some_and(|s| s != session)
        {
            return Err(DudeMemoryError::Conflict);
        }
        let mut batch = previous.clone();
        batch.session_code = Some(session.into());
        batch.status = MemoryBatchStatus::Running;
        batch.revision += 1;
        batch.updated_at = now();
        repository.save_batch(Some(&previous), &batch, false)?;
        Ok(batch)
    }
    pub fn list_batches(
        &self,
        user: &str,
        dude: &str,
    ) -> Result<Vec<MemoryBatch>, DudeMemoryError> {
        self.repository.lock().unwrap().list_batches(user, dude)
    }
    pub fn complete_batch(
        &self,
        user: &str,
        code: &str,
        session: &str,
        receipts: Vec<MemoryWriteReceipt>,
        summary: String,
    ) -> Result<MemoryBatch, DudeMemoryError> {
        let mut repository = self.repository.lock().unwrap();
        let previous = repository.get_batch(code)?;
        if previous.user_code != user {
            return Err(DudeMemoryError::NotFound);
        }
        if previous.status == MemoryBatchStatus::Completed {
            return Ok(previous);
        }
        if previous.session_code.as_deref() != Some(session) {
            return Err(DudeMemoryError::Conflict);
        }
        for receipt in &receipts {
            if receipt.semantic_key.trim().is_empty()
                || receipt.memory_code.trim().is_empty()
                || receipt.source_codes.is_empty()
                || receipt.source_codes.iter().any(|code| {
                    !previous.sources.iter().any(|source| {
                        source.source_code == *code
                            && source.workspace_code == receipt.workspace_code
                    })
                })
            {
                return Err(DudeMemoryError::Invalid(
                    "memory writes must retain their workspace and source evidence".into(),
                ));
            }
            if receipt.claim_kind == MemoryClaimKind::Inference
                && receipt.conflict_with_memory_code.is_some()
            {
                return Err(DudeMemoryError::Invalid(
                    "an inference cannot replace conflicting established memory".into(),
                ));
            }
        }
        let mut batch = previous.clone();
        batch.status = MemoryBatchStatus::Completed;
        batch.memory_receipts = receipts;
        batch.summary = Some(summary);
        batch.revision += 1;
        batch.updated_at = now();
        repository.save_batch(Some(&previous), &batch, true)?;
        Ok(batch)
    }
}
