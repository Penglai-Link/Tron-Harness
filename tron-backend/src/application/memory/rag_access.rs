use super::MemoryRecordRepository;
use crate::application::rag::*;
use crate::domain::rag::RagError;
use serde_json::json;

pub struct MemoryOnlyAccess;
impl RagAccessResolver for MemoryOnlyAccess {
    fn resolve(&self, _: &RagIdentity) -> Result<Vec<AuthorizedRagRange>, RagError> {
        Ok(vec![])
    }
    fn recheck(&self, _: &RagIdentity, _: &mut RagEvidenceResult) -> Result<(), RagError> {
        Ok(())
    }
}
pub(crate) struct MemoryRagAccess<'a, R, E> {
    pub base: &'a dyn RagAccessResolver,
    pub rag: &'a RagService<R, E>,
    pub store: Option<&'a dyn MemoryRecordRepository>,
}
impl<R: RagSearchRepository + Clone + Send + Sync, E: RagSearchRuntime> RagAccessResolver
    for MemoryRagAccess<'_, R, E>
{
    fn resolve(&self, i: &RagIdentity) -> Result<Vec<AuthorizedRagRange>, RagError> {
        let mut ranges = self.base.resolve(i)?;
        ranges.retain(|r| r.corpus_id != "memory-card");
        if let Some(store) = self.store {
            let corpus = self
                .rag
                .repository_handle()
                .get_corpus("memory-card")?
                .ok_or(RagError::CorpusNotFound)?;
            let settings = RagSettings::from_capabilities(&corpus.capabilities)?;
            let records = store.recallable(i).map_err(|_| RagError::StorageFailure)?;
            let mut scopes = std::collections::BTreeMap::<(String, String), Vec<String>>::new();
            for r in records {
                scopes
                    .entry((r.scope.rag_type().into(), r.scope_code))
                    .or_default()
                    .push(r.memory_code);
            }
            // An empty authorized range still permits an explicit memory-only query.
            if scopes.is_empty() {
                scopes.insert(("USER".into(), i.user_code.clone()), vec![]);
            }
            for ((scope_type, scope_code), codes) in scopes {
                ranges.extend(self.rag.partition_ranges_for_corpus(
                    AuthorizedRagRange {
                        corpus_id: "memory-card".into(),
                        source_version: "v1".into(),
                        scopes: vec![RagSearchScope {
                            scope_type,
                            scope_code,
                        }],
                        filters: json!({"sourceCode":{"op":"in","value":codes}}),
                        embedding_model_code: settings.embedding_model_code.clone(),
                        embedding_dimension: Some(settings.embedding_dimension),
                        provider_model: None,
                        embedding_fingerprint: None,
                        return_fields: vec![],
                        fulltext: settings.fulltext,
                        vector: settings.vector,
                        graph: settings.graph,
                    },
                    &corpus,
                )?);
            }
        }
        Ok(ranges)
    }
    fn recheck_sources(
        &self,
        i: &RagIdentity,
        result: &mut RagEvidenceResult,
    ) -> Result<(), RagError> {
        let mut memory = vec![];
        result.hits.retain(|hit| {
            if hit.row.corpus_id == "memory-card" {
                memory.push(hit.clone());
                false
            } else {
                true
            }
        });
        self.base.recheck_sources(i, result)?;
        if let Some(store) = self.store {
            let current = store.recallable(i).map_err(|_| RagError::StorageFailure)?;
            memory.retain_mut(|hit| {
                if let Some(record) = current.iter().find(|r| {
                    r.memory_code == hit.row.source_code
                        && Some(r.version as u64)
                            == hit.row.source_metadata["recordVersion"].as_u64()
                }) {
                    hit.row.source_metadata["sourceRefs"] = json!(record.source_refs);
                    hit.row.source_metadata["sourceLabel"] = json!(record.source_label);
                    true
                } else {
                    false
                }
            });
            result.hits.extend(memory);
        }
        Ok(())
    }
    fn recheck(&self, i: &RagIdentity, r: &mut RagEvidenceResult) -> Result<(), RagError> {
        self.recheck_sources(i, r)
    }
    fn assemble_context(&self, i: &RagIdentity, r: &mut RagEvidenceResult) -> Result<(), RagError> {
        self.base.assemble_context(i, r)
    }
}
