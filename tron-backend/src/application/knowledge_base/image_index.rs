//! Host-owned resource resolution. Only registered, authorized image bytes reach embeddings.
use super::*;
use crate::application::document::ParsedDocument;
use eliteforge_agent_framework::{Content, ContentBlock, MediaContent, MediaSource};
use std::collections::BTreeMap;

pub(crate) fn append_image_chunks(
    parsed: &ParsedDocument,
    title: &str,
    chunks: &mut Vec<LocatedChunk>,
) -> Result<(), KnowledgeBaseError> {
    let source_locations = crate::application::document::source_fact_locations(parsed);
    let mut occurrences = BTreeMap::<&str, usize>::new();
    let mut order = Vec::new();
    let specificity = |index: usize| {
        let location = &parsed.sections[index].location;
        (
            location.block_kind.as_deref() == Some("image"),
            location.image_resource_codes.len() == 1,
        )
    };
    for (section_index, section) in parsed.sections.iter().enumerate() {
        for code in &section.location.image_resource_codes {
            match occurrences.get(code.as_str()).copied() {
                Some(previous) if specificity(section_index) > specificity(previous) => {
                    occurrences.insert(code, section_index);
                }
                None => {
                    order.push(code.as_str());
                    occurrences.insert(code, section_index);
                }
                _ => {}
            }
        }
    }
    for code in order {
        let section_index = occurrences[code];
        let section = &parsed.sections[section_index];
        let description = section
            .location
            .image_annotations
            .iter()
            .filter(|annotation| {
                annotation
                    .resource_code
                    .as_deref()
                    .map_or(section.location.image_resource_codes.len() == 1, |bound| {
                        bound == code
                    })
            })
            .map(|annotation| annotation.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let text = if description.trim().is_empty() {
            format!("{title} 中的图片")
        } else {
            description.chars().take(1200).collect()
        };
        let mut location = section.location.clone();
        location.block_kind = Some("image_vector".into());
        location.image_resource_codes = vec![code.to_owned()];
        chunks.push(LocatedChunk {
            index_text: format!("{title}\n{text}"),
            text,
            title: title.into(),
            heading_path: vec![],
            source_locations: vec![source_locations[section_index].clone()],
            location,
            ordinal: chunks.len(),
            source_spans: vec![],
            context_id: String::new(),
        });
    }
    Ok(())
}



impl KnowledgeBaseService {
    /// Index configuration changes never submit a parser job for existing documents.
    pub fn supplement_corpus_indexes(
        &self,
        user: &str,
        kb: &str,
    ) -> Result<(), KnowledgeBaseError> {
        let detail = self.detail(user, kb)?;
        for doc in detail.documents.iter().filter(|doc| {
            doc.parsing_artifact.is_some()
                || doc
                    .parse_plan
                    .as_ref()
                    .is_some_and(|plan| plan.features.local_text)
        }) {
            for task in self
                .queue
                .for_document(&doc.id)
                .map_err(ingestion_worker::queue_error)?
            {
                if (task.kind == ingestion::IngestionKind::Indexing
                    || task.configuration["reuseParsedOnly"] == true)
                    && matches!(
                        task.state.as_str(),
                        "RUNNING" | "QUEUED" | "WAITING_EMBEDDING" | "WAITING_RETRY"
                    )
                {
                    self.queue
                        .cancel(&task.id)
                        .map_err(ingestion_worker::queue_error)?;
                }
            }
            self.enqueue_index(user, "PLATFORM", kb, &doc.id)?;
        }
        self.start_ingestion_workers()
    }
    pub(crate) fn index_embedding_inputs(
        &self,
        user: &str,
        workspace: &str,
        chunks: &[LocatedChunk],
        model: &str,
    ) -> Result<Vec<Content>, KnowledgeBaseError> {
        let accepts_images = self
            .llm
            .embedding_accepts_images(workspace, model)
            .map_err(|error| KnowledgeBaseError::Invalid(error.to_string()))?;
        let mut total_bytes = 0u64;
        chunks
            .iter()
            .map(|chunk| {
                let text = if chunk.index_text.is_empty() {
                    &chunk.text
                } else {
                    &chunk.index_text
                };
                let mut input = Content::from(text);
                if chunk.location.block_kind.as_deref() != Some("image_vector") {
                    return Ok(input);
                }
                if !accepts_images {
                    return Err(KnowledgeBaseError::Invalid(
                        "当前向量模型不支持原图，请重新补充索引".into(),
                    ));
                }
                let [code] = chunk.location.image_resource_codes.as_slice() else {
                    return Err(KnowledgeBaseError::Invalid(
                        "原图向量必须绑定唯一图片资源".into(),
                    ));
                };
                let (_, record) = self.resources.download_binary_to_file(
                    user,
                    ResourceBinaryDownloadCommand {
                        workspace_code: workspace.into(),
                        resource_code: code.clone(),
                    },
                )?;
                total_bytes = total_bytes.saturating_add(record.byte_size as u64);
                if record.byte_size > 20 * 1024 * 1024 || total_bytes > 100 * 1024 * 1024 {
                    return Err(KnowledgeBaseError::Invalid(
                        "图片向量输入超过大小上限".into(),
                    ));
                }
                let mut bytes = Vec::new();
                self.resources
                    .download_binary_storage(&record, &mut bytes)?;
                let mime = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
                    "image/png"
                } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
                    "image/jpeg"
                } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP".as_slice())
                {
                    "image/webp"
                } else {
                    return Err(KnowledgeBaseError::Invalid(
                        "图片向量输入格式不受支持".into(),
                    ));
                };
                use base64::Engine;
                input.0.push(ContentBlock::Media {
                    media: MediaContent {
                        mime_type: mime.into(),
                        source: MediaSource::Base64(
                            base64::engine::general_purpose::STANDARD.encode(bytes),
                        ),
                        name: None,
                        estimated_tokens: Some(4096),
                    },
                });
                Ok(input)
            })
            .collect()
    }
}
