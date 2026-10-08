use super::*;

impl KnowledgeBaseService {
    pub(super) fn recover_ingestion_intents(&self) -> Result<(), KnowledgeBaseError> {
        for (user, workspace, kb, doc, revision, resource_code) in
            self.repository.pending_ingestion_intents(now_ms())?
        {
            // Keep this synchronous recovery step on the observed original instance.
            // Deletion/publication cannot interleave between validation and enqueue/error writes.
            let _guard = self.cleanup_gate.lock().map_err(invalid_storage)?;
            let current = match self.repository.document(&kb, &doc) {
                Ok(current) => current,
                Err(KnowledgeBaseError::NotFound) => continue,
                Err(error) => return Err(error),
            };
            if current.resource_code != resource_code { continue; }
            if current.revision != revision
                || !self
                    .queue
                    .for_document(&doc)
                    .map_err(queue_error)?
                    .is_empty()
            {
                self.repository.finish_ingestion_intent(&doc, revision)?;
                continue;
            }
            if let Err(error) = self.enqueue_index(&user, &workspace, &kb, &doc) {
                let retry = matches!(error, KnowledgeBaseError::Storage(_));
                let reason = IngestionWaitReason::new(match &error {
                    KnowledgeBaseError::Storage(_) => WaitReasonCode::IntakeStorage,
                    KnowledgeBaseError::Invalid(_) => WaitReasonCode::IntakeInvalid,
                    KnowledgeBaseError::Forbidden => WaitReasonCode::IntakeForbidden,
                    KnowledgeBaseError::Conflict | KnowledgeBaseError::NotEmpty => WaitReasonCode::IntakeConflict,
                    KnowledgeBaseError::NotFound => WaitReasonCode::IntakeMissing,
                });
                self.repository.fail_ingestion_intent(
                    &doc,
                    revision,
                    &format!("创建入库任务失败：{error}"),
                    &reason,
                    retry.then(|| {
                        now_ms().saturating_add(
                            self.active_ingestion_config().retry_backoff_ms.max(1000),
                        )
                    }),
                )?;
            }
        }
        Ok(())
    }
}
