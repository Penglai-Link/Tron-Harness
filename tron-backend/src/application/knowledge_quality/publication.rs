use super::*;
use super::{
    candidate::{CandidateIndex, persist_command,quality_configuration},
    snapshot::QualitySnapshot,
};
use crate::{
    adapters::outbound::persistence::sqlite::OptimizedDocumentPublication,
    application::{
        document::ChunkConfig,
        knowledge_base::{
            hash,
            ingestion::{IngestionKind, IngestionStage, IngestionTask},
        },
        rag::RagIngestRepository,
    },
};

impl KnowledgeQualityService {
    pub(crate) fn validate_snapshot(
        &self,
        user: &str,
        snapshot: &QualitySnapshot,
    ) -> Result<(), KnowledgeBaseError> {
        self.host.owned(user, &snapshot.library.id)?;
        let current = self
            .host
            .repository
            .documents(&snapshot.library.id)?
            .into_iter()
            .filter(|doc| doc.active_source_code.is_some())
            .collect::<Vec<_>>();
        if current.len() != snapshot.documents.len()
            || snapshot.documents.iter().any(|old| {
                current
                    .iter()
                    .find(|doc| doc.id == old.document.id)
                    .is_none_or(|doc| {
                        doc.revision != old.document.revision
                            || doc.resource_revision != old.document.resource_revision
                            || doc.resource_code != old.document.resource_code
                            || doc.active_source_code != old.document.active_source_code
                            || doc.status != "READY"
                    })
            })
        {
            return Err(KnowledgeBaseError::Conflict);
        }
        if serde_json::to_value(self.host.rag.settings_for_corpus(&snapshot.library.id)?)
            .map_err(invalid)?
            != serde_json::to_value(&snapshot.settings).map_err(invalid)?
        {
            return Err(KnowledgeBaseError::Conflict);
        }
        Ok(())
    }
    pub(crate) async fn publish_candidate(
        &self,
        user: &str,
        snapshot: &QualitySnapshot,
        run: &mut QualityOptimizationRun,
        config: &ChunkConfig,
        candidate: CandidateIndex,
    ) -> Result<(), KnowledgeBaseError> {
        self.validate_snapshot(user, snapshot)?;
        let mut publications = Vec::new();
        let repository = self.host.rag.repository_handle();
        // All models and vectors are the ones already used by the experiment.
        // Staging cannot be queried through the published document pointers.
        for tested in candidate.documents {
            let mut ready = tested.original.clone();
            ready.status = "READY".into();
            ready.error = None;
            ready.chunk_config = Some(config.clone());
            ready.chunk_count = tested.chunks.len();
            ready.revision += 1;
            ready.index_revision += 1;
            ready.active_source_code = Some(tested.source.clone());
            ready.embedding_model_code = tested.prepared.vectors.model_code.clone();
            ready.embedding_dimension = candidate
                .settings
                .vector
                .then_some(candidate.settings.embedding_dimension);
            ready.embedding_space_fingerprint = tested.prepared.vectors.fingerprint.clone();
            ready.embedding_provider_model = tested.prepared.vectors.provider_model.clone();
            ready.task_id = Some(tested.task_id.clone());
            ready.stage = Some(IngestionStage::Publishing);
            let configuration = quality_configuration(config,&run.id,&candidate.settings,&ready,ready.embedding_model_code.as_deref(),ready.embedding_space_fingerprint.as_deref());
            let mut task = IngestionTask::queued(
                tested.task_id,
                snapshot.library.id.clone(),
                ready.id.clone(),
                user.into(),
                "PLATFORM".into(),
                tested.original.revision,
                ready.resource_revision,
                hash(&configuration.to_string()),
                configuration,
            );
            if let Some(id)=&tested.graph_id {
                if tested.graph_context.configuration_hash!=task.input_config_hash{return Err(KnowledgeBaseError::Conflict);}
                let graph=candidate.rag.graph_infrastructure().ok_or_else(||invalid("候选图谱未配置"))?;
                self.host.rag.copy_graph_generation("PLATFORM",graph,id,&tested.graph_context,&tested.prepared,&super::runtime::QualityCheckpoint(candidate.rag.runtime_handle().budget)).map_err(invalid)?;
            }
            repository.persist_ingest(persist_command(user,&snapshot.library.id,&tested.original,&tested.source,&task.id,&candidate.settings,
                &tested.prepared,"STAGING",tested.graph_id.as_deref().map(|id|(id,&tested.graph_context))))?;
            task.kind = IngestionKind::Indexing;
            task.state = "READY".into();
            task.stage = IngestionStage::Publishing;
            task.index_stage = Some("PUBLISHED".into());
            task.attempt = 1;
            task.total_units = ready.chunk_count;
            task.completed_units = ready.chunk_count;
            publications.push(OptimizedDocumentPublication {
                original: tested.original,
                ready,
                contexts: tested.contexts,
                task,
            });
        }
        self.validate_snapshot(user, snapshot)?;
        run.status = "PUBLISHED".into();
        run.stage = "COMPLETED".into();
        run.published = true;
        run.updated_at = chrono::Utc::now().timestamp_millis();
        let cleanups = {
            let _guard = self.host.cleanup_gate.lock().map_err(invalid)?;
            self.host.repository.publish_quality_candidate(
                user,
                &snapshot.library.id,
                run.clone(),
                config.clone(),
                snapshot.corpus.capabilities.clone(),
                publications,
            )?
        };
        let host = self.host.clone();
        tokio::spawn(async move {
            for cleanup in cleanups {
                host.clean_document_deletion(cleanup).await;
            }
        });
        Ok(())
    }
}
