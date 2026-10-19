use super::*;
use crate::application::rag::{RagConfigRepository, RagSettings};
use crate::application::{
    document::{ParsedDocument, chunk_document_input},
    knowledge_base::{KnowledgeBase, KnowledgeBaseDocument},
};
use crate::domain::rag::RagCorpusRecord;

pub(crate) struct SnapshotDocument {
    pub document: KnowledgeBaseDocument,
    pub parsed: ParsedDocument,
    pub input: ChunkDocument,
}
pub(crate) struct QualitySnapshot {
    pub library: KnowledgeBase,
    pub documents: Vec<SnapshotDocument>,
    pub fingerprint: String,
    pub settings: RagSettings,
    pub corpus: RagCorpusRecord,
}

impl KnowledgeQualityService {
    pub(crate) async fn snapshot(
        &self,
        user: &str,
        kb: &str,
    ) -> Result<QualitySnapshot, KnowledgeBaseError> {
        let library = self.host.owned(user, kb)?;
        let documents = self.host.repository.documents(kb)?;
        let documents = documents
            .into_iter()
            .filter(|doc| doc.active_source_code.is_some())
            .collect::<Vec<_>>();
        if documents.is_empty() {
            return Err(KnowledgeBaseError::Invalid(
                "请先完成至少一份资料的解析和入库".into(),
            ));
        }
        if documents.len() > 16 {
            return Err(KnowledgeBaseError::Invalid(
                "一次优化最多处理16份资料，请分开建立知识库或缩小评测范围".into(),
            ));
        }
        let corpus = self
            .host
            .rag
            .repository_handle()
            .get_corpus(kb)?
            .ok_or(KnowledgeBaseError::NotFound)?;
        let settings = RagSettings::from_capabilities(&corpus.capabilities)?;
        let mut snapshots = Vec::new();
        let mut total = 0;
        for document in documents {
            if document.status != "READY" {
                return Err(KnowledgeBaseError::Invalid(
                    "资料正在处理或重建失败，请先恢复稳定的已发布索引".into(),
                ));
            }
            let parsed = self.host.existing_parsed_document(user, &document)?;
            let (parsed, _) = self
                .host
                .supplement_image_descriptions(user, "PLATFORM", &library, &document, parsed, false)
                .await?;
            total += parsed
                .sections
                .iter()
                .map(|section| section.text.len())
                .sum::<usize>();
            if total > 4 * 1024 * 1024 {
                return Err(KnowledgeBaseError::Invalid(
                    "本轮评测正文超过4 MiB上限".into(),
                ));
            }
            let input = chunk_document_input(&parsed, &document.id, &document.file_name, 6);
            snapshots.push(SnapshotDocument {
                document,
                parsed,
                input,
            });
        }
        let identity = snapshots
            .iter()
            .map(|snapshot| {
                (
                    &snapshot.document.id,
                    &snapshot.document.resource_code,
                    snapshot.document.resource_revision,
                    &snapshot.input,
                )
            })
            .collect::<Vec<_>>();
        let fingerprint = crate::application::knowledge_base::hash(
            &serde_json::to_string(&identity)
                .map_err(|error| KnowledgeBaseError::Storage(error.to_string()))?,
        );
        Ok(QualitySnapshot {
            library,
            documents: snapshots,
            fingerprint,
            settings,
            corpus,
        })
    }
    pub async fn source_excerpts(
        &self,
        user: &str,
        kb: &str,
        offset: usize,
        limit: usize,
    ) -> Result<(String, Vec<QualitySourceExcerpt>), KnowledgeBaseError> {
        if limit == 0 || limit > 24 || offset > 10000 {
            return Err(KnowledgeBaseError::Invalid("评测原文分页超出范围".into()));
        }
        let snapshot = self.snapshot(user, kb).await?;
        let excerpts = excerpts(&snapshot, (offset + limit).min(10024))
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect();
        Ok((snapshot.fingerprint, excerpts))
    }
}

pub(crate) fn excerpts(snapshot: &QualitySnapshot, max: usize) -> Vec<QualitySourceExcerpt> {
    let mut result = Vec::new();
    for document in &snapshot.documents {
        for block in &document.input.blocks {
            if matches!(block.kind, ChunkBlockKind::Heading | ChunkBlockKind::Image)
                || block.text.chars().count() < 40
            {
                continue;
            }
            let chars = block.text.chars().collect::<Vec<_>>();
            for start in (0..chars.len()).step_by(700).take(3) {
                let end = (start + 700).min(chars.len());
                if end - start < 40 {
                    continue;
                }
                result.push(QualitySourceExcerpt {
                    document_id: document.document.id.clone(),
                    document_name: document.document.file_name.clone(),
                    block_id: block.id.clone(),
                    start,
                    end,
                    text: chars[start..end].iter().collect(),
                    location: block.location.clone(),
                });
                if result.len() >= max {
                    return result;
                }
            }
        }
    }
    result
}
