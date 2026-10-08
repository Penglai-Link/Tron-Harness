//! ACL checked Resource bytes. Default-off gate precedes all image storage IO.
use super::*;
use std::{collections::BTreeSet, io};
#[derive(Debug)]
pub struct KnowledgeBaseVisualEvidence {
    pub resource_code: String,
    pub source_reference: String,
    pub mime_type: String,
    pub bytes: Vec<u8>,
}
#[derive(Debug, Default)]
pub struct KnowledgeBaseVisualContext {
    pub evidence: Vec<KnowledgeBaseVisualEvidence>,
    pub warnings: Vec<String>,
}
struct BoundedBytes(Vec<u8>);
impl io::Write for BoundedBytes {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if self.0.len().saturating_add(data.len()) > 20 * 1024 * 1024 {
            return Err(io::Error::other("visual resource exceeds 20 MiB"));
        }
        self.0.extend_from_slice(data);
        Ok(data.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl KnowledgeBaseService {
    fn validate_visual_hits(
        &self,
        user: &str,
        workspace: &str,
        agent: &str,
        hits: &[KnowledgeBaseHit],
    ) -> Result<Vec<KnowledgeBase>, KnowledgeBaseError> {
        let allowed = self.allowed(user, workspace, agent)?;
        if hits.iter().any(|hit| {
            !allowed.iter().any(|kb| {
                kb.id == hit.knowledge_base_id
                    && kb.documents.iter().any(|d| {
                        !super::fidelity::blocked(d) && d.id == hit.document_id
                            && d.resource_code == hit.resource_code
                            && d.resource_revision == hit.resource_revision
                            && d.active_source_code.as_ref() == Some(&hit.source_code)
                    })
            })
        }) {
            return Err(KnowledgeBaseError::Forbidden);
        }
        Ok(allowed)
    }
    /// Only registered images linked by a current published hit are eligible.
    /// Recheck the grant and published source before and after bounded storage IO.
    pub fn visual_evidence_for_agent(
        &self,
        user: &str,
        workspace: &str,
        agent: &str,
        hits: &[KnowledgeBaseHit],
    ) -> Result<KnowledgeBaseVisualContext, KnowledgeBaseError> {
        let allowed = self.validate_visual_hits(user, workspace, agent, hits)?;
        if !self.visual_context_enabled() {
            return Ok(KnowledgeBaseVisualContext {
                evidence: vec![],
                warnings: if hits.iter().any(|h| {
                    h.location["imageResourceCodes"]
                        .as_array()
                        .is_some_and(|a| !a.is_empty())
                }) {
                    vec!["图片上下文未启用；当前回答模型仅收到检索文本，未查看原图".into()]
                } else {
                    vec![]
                },
            });
        }
        let resources = self
            .resources
            .for_agent_in_workspace(Some(agent), workspace);
        let mut output = KnowledgeBaseVisualContext::default();
        let mut seen = BTreeSet::new();
        let mut total = 0usize;
        for hit in hits {
            let doc = allowed
                .iter()
                .find(|k| k.id == hit.knowledge_base_id)
                .and_then(|k| k.documents.iter().find(|d| d.id == hit.document_id))
                .ok_or(KnowledgeBaseError::Forbidden)?;
            let registered = doc
                .parsing_artifact
                .as_ref()
                .map(|a| a.image_resource_codes.as_slice())
                .unwrap_or(&[]);
            for code in hit.location["imageResourceCodes"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
            {
                if !registered.iter().any(|r| r == code) {
                    return Err(KnowledgeBaseError::Forbidden);
                }
                if !seen.insert(code.to_owned()) {
                    continue;
                }
                if output.evidence.len() >= 4 {
                    output
                        .warnings
                        .push("部分视觉证据超过单步4张图片上限，未载入".into());
                    break;
                }
                self.validate_visual_hits(user, workspace, agent, hits)?;
                let (_, record) = match resources.download_binary_to_file(
                    user,
                    ResourceBinaryDownloadCommand {
                        workspace_code: workspace.into(),
                        resource_code: code.into(),
                    },
                ) {
                    Ok(value) => value,
                    Err(crate::domain::resource::ResourceError::Forbidden) => {
                        return Err(KnowledgeBaseError::Forbidden);
                    }
                    Err(_) => {
                        output
                            .warnings
                            .push(format!("视觉资源{code}读取失败，未载入；可重试"));
                        continue;
                    }
                };
                if record.byte_size > 20 * 1024 * 1024 {
                    output
                        .warnings
                        .push(format!("视觉资源{code}超过大小上限，未载入"));
                    continue;
                }
                let mut bytes = BoundedBytes(Vec::new());
                if resources
                    .download_binary_storage(&record, &mut bytes)
                    .is_err()
                {
                    output
                        .warnings
                        .push(format!("视觉资源{code}读取失败，未载入；可重试"));
                    continue;
                }
                total = total.saturating_add(bytes.0.len());
                if total > 32 * 1024 * 1024 {
                    output
                        .warnings
                        .push("视觉证据超过单步读取大小上限，未载入".into());
                    break;
                }
                let mime = if bytes.0.starts_with(b"\x89PNG\r\n\x1a\n") {
                    "image/png"
                } else if bytes.0.starts_with(&[0xff, 0xd8, 0xff]) {
                    "image/jpeg"
                } else if bytes.0.starts_with(b"RIFF")
                    && bytes.0.get(8..12) == Some(b"WEBP".as_slice())
                {
                    "image/webp"
                } else {
                    output
                        .warnings
                        .push(format!("视觉资源{code}格式不支持，未载入"));
                    continue;
                };
                output.evidence.push(KnowledgeBaseVisualEvidence {
                    resource_code: code.into(),
                    source_reference: format!(
                        "[文档:{} ID:{} 图片资源:{} 索引:{} 位置:{}]",
                        doc.name,
                        doc.id,
                        code,
                        hit.source_code,
                        compact_location(&hit.location)
                    ),
                    mime_type: mime.into(),
                    bytes: bytes.0,
                });
            }
        }
        self.validate_visual_hits(user, workspace, agent, hits)?;
        output.warnings.sort();
        output.warnings.dedup();
        Ok(output)
    }
}
