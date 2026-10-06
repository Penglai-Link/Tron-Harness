//! Product ACL retrieval is a bounded Data source in the public SDK assembler.
use crate::application::{knowledge_base::KnowledgeBaseService, llm::ContextQueryAllowance};
use async_trait::async_trait;
use eliteforge_agent_core as fw;

pub(super) struct KnowledgeBaseContributor {
    pub service: Option<KnowledgeBaseService>,
    pub user: Option<String>,
    pub workspace: String,
    pub agent: Option<String>,
    pub allowance: ContextQueryAllowance,
}
#[async_trait]
impl fw::ContextContributor for KnowledgeBaseContributor {
    fn block_id(&self) -> &str {
        "knowledge_base"
    }
    async fn contribute(
        &self,
        input: &fw::ContextInput<'_>,
    ) -> Result<fw::ContextBlock, fw::AgentError> {
        input.run.check_cancelled()?;
        let mut block = fw::ContextBlock {
            id: self.block_id().into(),
            messages: vec![],
            trust: fw::ContextTrust::Data,
            source_version: None,
        };
        let (Some(service), Some(user), Some(agent)) = (&self.service, &self.user, &self.agent)
        else {
            return Ok(block);
        };
        let query = input.current_input.text();
        if input.budget.max_tokens == 0 || query.trim().is_empty() {
            return Ok(block);
        }
        let mut run = input.run.clone();
        run.extensions.insert(self.allowance.clone());
        // ACL is checked in the service on every read. Retrieval failures omit this
        // optional source; no stale source or unrestricted fallback is reused.
        if let Ok(context) = service
            .search_for_agent_in_run(
                user,
                &self.workspace,
                agent,
                &query,
                input.budget.max_tokens,
                &run,
            )
            .await
        {
            // The service's explicit default-off gate precedes all image IO.
            // A revoked source also invalidates the previously retrieved text.
            let Ok(visual) =
                service.visual_evidence_for_agent(user, &self.workspace, agent, &context.hits)
            else {
                return Ok(block);
            };
            block.source_version = Some(context.source_version);
            let mut warnings = visual.warnings;
            let mut media_messages = Vec::new();
            for evidence in visual.evidence {
                input.run.check_cancelled()?;
                match eliteforge_agent_memory::prepare_inline_image_evidence(
                    &evidence.bytes,
                    Some(evidence.source_reference.clone()),
                    Default::default(),
                ) {
                    Ok(media) => media_messages.push(fw::Message::user(fw::Content::from(vec![
                        fw::ContentBlock::Text {
                            text: evidence.source_reference,
                        },
                        fw::ContentBlock::Media { media },
                    ]))),
                    Err(_) => warnings.push(format!(
                        "图片资源{}无法在SDK图片限制内处理；当前模型未查看该原图",
                        evidence.resource_code
                    )),
                }
            }
            // Keep visibility limitations before body text so bounding cannot
            // retain a caption while dropping the image-not-viewed notice.
            for warning in warnings {
                block
                    .messages
                    .push(fw::Message::user(format!("[视觉证据未载入: {warning}]")));
            }
            if !context.text.is_empty() {
                block.messages.push(fw::Message::user(context.text));
            }
            block.messages.extend(media_messages);
        }
        input.run.check_cancelled()?;
        Ok(block)
    }
}
