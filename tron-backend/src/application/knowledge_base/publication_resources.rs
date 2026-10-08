//! Keep only resources reachable from the successful parse and its published chunks.
use super::*;
use std::collections::BTreeSet;

impl KnowledgeBaseService {
    pub(super) fn retain_publication_resources(
        &self,
        user: &str,
        workspace: &str,
        document: &mut KnowledgeBaseDocument,
        chunks: &[local_documents::LocatedChunk],
    ) -> Result<(), KnowledgeBaseError> {
        let mut used = BTreeSet::new();
        deletion::collect_codes(
            &serde_json::to_value(chunks).map_err(invalid_storage)?,
            &mut used,
        );
        document
            .image_description_artifacts
            .retain(|artifact| used.contains(&artifact.resource_code));
        deletion::collect_codes(
            &serde_json::to_value(&document.parsing_artifact).map_err(invalid_storage)?,
            &mut used,
        );
        deletion::collect_codes(
            &serde_json::to_value(&document.visual_source).map_err(invalid_storage)?,
            &mut used,
        );
        deletion::collect_codes(
            &serde_json::to_value(&document.image_description_artifacts)
                .map_err(invalid_storage)?,
            &mut used,
        );
        used.remove(&document.resource_code);
        // Audit/current parse files can contain resource identities not present in
        // chunk locations. Traverse only our own registered JSON attachments.
        let mut visited = BTreeSet::new();
        while let Some(code) = used.iter().find(|code| !visited.contains(*code)).cloned() {
            visited.insert(code.clone());
            let (_, record) = self.resources.download_binary_to_file(
                user,
                ResourceBinaryDownloadCommand {
                    workspace_code: workspace.into(),
                    resource_code: code,
                },
            )?;
            if record.description != "知识库解析附件" || !record.file_name.ends_with(".json")
            {
                continue;
            }
            if record.byte_size > local_documents::MAX_EXPANDED_BYTES as u64 {
                return Err(KnowledgeBaseError::Invalid("解析附件超过上限".into()));
            }
            let mut bytes = Vec::new();
            self.resources
                .download_binary_storage(&record, &mut bytes)?;
            let value: Value = serde_json::from_slice(&bytes).map_err(invalid_storage)?;
            deletion::collect_codes(&value, &mut used);
            used.remove(&document.resource_code);
        }
        document.derived_resource_codes = used.into_iter().collect();
        Ok(())
    }
}
