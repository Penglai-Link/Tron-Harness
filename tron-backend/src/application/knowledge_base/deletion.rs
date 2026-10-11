use super::*;
use std::collections::BTreeSet;

/// Best-effort work captured in memory by the database deletion transaction.
#[derive(Clone, Debug)]
pub struct DeletionCleanupPlan {
    pub document_id: String,
    pub knowledge_base_id: String,
    pub workspace_code: String,
    pub document_revision: u64,
    pub original_resource_code: String,
    pub resources: Vec<String>,
    pub objects: Vec<String>,
    pub cancellations: Vec<ingestion::IngestionTask>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentDeletion {
    pub document_id: String,
    pub state: &'static str,
    pub original_retained: bool,
    pub mineru_cache_retained: bool,
}

pub struct ApplicationErrorDescriptor {
    pub code: &'static str,
    pub message_key: &'static str,
}

impl KnowledgeBaseError {
    pub fn library_deletion_descriptor(&self) -> ApplicationErrorDescriptor {
        let (code, message_key) = match self {
            Self::NotEmpty => ("101027001", "tron.knowledge_base.not_empty"),
            Self::NotFound => ("101027002", "tron.knowledge_base.not_found"),
            Self::Forbidden => ("101027003", "tron.knowledge_base.forbidden"),
            Self::Conflict => ("101027004", "tron.knowledge_base.conflict"),
            Self::Invalid(_) => ("101027005", "tron.knowledge_base.invalid"),
            Self::Storage(_) => ("101027006", "tron.knowledge_base.storage_failed"),
        };
        ApplicationErrorDescriptor { code, message_key }
    }
}

impl KnowledgeBaseService {
    pub fn remove_library(&self, user: &str, id: &str) -> Result<(), KnowledgeBaseError> {
        self.require_user(user)?;
        let _guard = self.cleanup_gate.lock().map_err(invalid_storage)?;
        self.repository.delete_empty_library(user, id)
    }

    pub(super) fn request_document_deletion(
        &self,
        user: &str,
        workspace: &str,
        kb: &str,
        doc: &str,
    ) -> Result<DocumentDeletion, KnowledgeBaseError> {
        self.owned(user, kb)?;
        // This gate also surrounds creation of derived objects and generation publication.
        let _guard = self.cleanup_gate.lock().map_err(invalid_storage)?;
        let document = self.repository.document(kb, doc)?;
        let mut candidates = BTreeSet::new();
        collect_codes(
            &serde_json::to_value(&document).map_err(invalid_storage)?,
            &mut candidates,
        );
        for task in self
            .queue
            .for_document(doc)
            .map_err(ingestion_worker::queue_error)?
        {
            // Collect identities only, not parser configuration or document text.
            candidates.extend(
                [
                    task.parsed_resource_code,
                    task.chunks_resource_code,
                    task.embedding_resource_code,
                ]
                .into_iter()
                .flatten(),
            );
        }
        candidates.extend(self.repository.derived_resources(doc)?);
        candidates.remove(&document.resource_code);
        // Checkpoint manifests can refer to vectors and earlier parsed generations.
        // Read only this document's reachable, product-owned JSON attachments.
        let mut visited = BTreeSet::new();
        loop {
            let Some(code) = candidates.iter().find(|c| !visited.contains(*c)).cloned() else {
                break;
            };
            visited.insert(code.clone());
            let record = match self.resources.download_binary_to_file(
                user,
                ResourceBinaryDownloadCommand {
                    workspace_code: workspace.into(),
                    resource_code: code.clone(),
                },
            ) {
                Ok((_, record)) => record,
                Err(crate::domain::resource::ResourceError::NotFound) => continue,
                Err(error) => {
                    tracing::error!(
                        "knowledge document {doc}: checkpoint discovery skipped: {error}"
                    );
                    continue;
                }
            };
            if record.description == "知识库解析附件" && record.file_name.ends_with(".json")
            {
                if record.byte_size > 100 * 1024 * 1024 {
                    tracing::warn!(
                        "knowledge document {doc}: oversized cleanup checkpoint skipped"
                    );
                    continue;
                }
                let mut bytes = Vec::new();
                if let Err(error) = self.resources.download_binary_storage(&record, &mut bytes) {
                    tracing::error!(
                        "knowledge document {doc}: checkpoint discovery skipped: {error}"
                    );
                    continue;
                }
                match serde_json::from_slice(&bytes) {
                    Ok(value) => collect_codes(&value, &mut candidates),
                    Err(error) => tracing::error!(
                        "knowledge document {doc}: invalid cleanup checkpoint skipped: {error}"
                    ),
                }
                candidates.remove(&document.resource_code);
            }
        }
        let job = self
            .repository
            .delete_document_transaction(DeletionCleanupPlan {
                document_id: doc.into(),
                knowledge_base_id: kb.into(),
                workspace_code: workspace.into(),
                document_revision: document.revision,
                original_resource_code: document.resource_code,
                resources: candidates.into_iter().collect(),
                objects: vec![],
                cancellations: vec![],
            })?;
        drop(_guard);
        // The response depends only on the committed database transaction.
        // Storage or remote cancellation failure may leave an orphan, never a tombstone.
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            let service = self.clone();
            runtime.spawn(async move {
                service.clean_document_deletion(job).await;
            });
        } else {
            tracing::warn!(
                "knowledge document {doc}: cleanup runtime unavailable; database deletion succeeded"
            );
        }
        Ok(DocumentDeletion {
            document_id: doc.into(),
            state: "DELETED",
            original_retained: true,
            mineru_cache_retained: true,
        })
    }

    pub(crate) async fn clean_document_deletion(&self, job: DeletionCleanupPlan) {
        // Task snapshots survive only in this best-effort call, not in database history.
        for task in &job.cancellations {
            let Some(parser) = self.parser.as_ref() else {
                tracing::warn!(
                    "knowledge document {}: parser unavailable for cancellation",
                    job.document_id
                );
                break;
            };
            match tokio::time::timeout(std::time::Duration::from_secs(3), parser.cancel_task(task))
                .await
            {
                Ok(Ok(_)) => {}
                Ok(Err(error)) => tracing::error!(
                    "knowledge document {}: remote cancellation failed: {error}",
                    job.document_id
                ),
                Err(_) => tracing::warn!(
                    "knowledge document {}: remote cancellation timed out",
                    job.document_id
                ),
            }
        }
        // No database transaction or publication gate is held during physical I/O.
        // Every object is attempted independently; failures do not affect deletion.
        for locator in &job.objects {
            if let Err(error) = self.resources.remove_managed_binary_object(locator) {
                tracing::error!(
                    "knowledge document {}: derived object cleanup failed: {error}",
                    job.document_id
                );
            }
        }
    }
}

pub(crate) fn collect_codes(value: &Value, codes: &mut BTreeSet<String>) {
    match value {
        Value::String(s)
            if s.strip_prefix("KB_")
                .is_some_and(|id| id.len() == 32 && id.bytes().all(|c| c.is_ascii_hexdigit())) =>
        {
            codes.insert(s.clone());
        }
        Value::Array(values) => {
            for v in values {
                collect_codes(v, codes);
            }
        }
        Value::Object(values) => {
            for v in values.values() {
                collect_codes(v, codes);
            }
        }
        _ => {}
    }
}
