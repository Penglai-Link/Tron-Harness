use async_trait::async_trait;
use eliteforge_agent_core as fw;

use crate::application::topic::TopicService;

pub(super) struct TopicContributor {
    pub service: Option<TopicService>,
    pub project_code: Option<String>,
    pub user_code: Option<String>,
}

#[async_trait]
impl fw::ContextContributor for TopicContributor {
    fn block_id(&self) -> &str {
        "topic_memo"
    }

    fn layout_metadata(&self) -> fw::ContextBlockMetadata {
        crate::application::short_term_memory::native_context_block_metadata(self.block_id())
            .expect("topic context metadata")
    }

    async fn contribute(
        &self,
        input: &fw::ContextInput<'_>,
    ) -> Result<fw::ContextBlock, fw::AgentError> {
        let mut block = fw::ContextBlock {
            id: self.block_id().into(),
            messages: Vec::new(),
            trust: fw::ContextTrust::Data,
            source_version: None,
        };
        let (Some(service), Some(code), Some(user)) =
            (&self.service, &self.project_code, &self.user_code)
        else {
            return Ok(block);
        };
        if input.budget.max_tokens == 0 {
            return Ok(block);
        }
        input.run.check_cancelled()?;
        let memo = match service.get(user, code) {
            Ok(memo) => memo,
            Err(_) => {
                let message = fw::Message::user("[主题信息不可用，请勿假定已读取主题引用。]");
                if input.estimator.message(input.model, &message).await? <= input.budget.max_tokens
                {
                    block.messages.push(message);
                }
                return Ok(block);
            }
        };
        block.source_version = Some(format!("{}:{}", code, memo.revision));
        let marker = fw::Message::user(
            "[部分主题说明或引用未显示；如有 topic_read 权限，可获取授权范围内的完整信息。]",
        );
        let marker_cost = input.estimator.message(input.model, &marker).await?;
        let budget = input.budget.max_tokens.saturating_sub(marker_cost);
        let head = fw::Message::user(format!("[主题 {code}，版本 {}]", memo.revision));
        let mut used = input.estimator.message(input.model, &head).await?;
        if used <= budget {
            block.messages.push(head);
        } else {
            used = 0;
        }
        let mut omitted = block.messages.is_empty();
        // Reserve room for identifiers before longer labels, URLs or notes.
        // Unavailable, permission-filtered references carry no target details.
        for reference in &memo.references {
            let identity = serde_json::json!({
                "referenceCode": reference.reference_code,
                "kind": reference.kind,
                "targetCode": reference.target_code,
                "connectionCode": reference.connection_code,
                "available": reference.available,
            });
            let entry = fw::Message::user(format!("主题引用标识：{identity}"));
            let cost = input.estimator.message(input.model, &entry).await?;
            if used.saturating_add(cost) <= budget {
                used += cost;
                block.messages.push(entry);
            } else {
                omitted = true;
            }
        }
        for reference in &memo.references {
            let text = serde_json::to_string(reference)
                .map_err(|error| fw::AgentError::InvalidConfiguration(error.to_string()))?;
            let entry = fw::Message::user(format!("主题引用详情：{text}"));
            let cost = input.estimator.message(input.model, &entry).await?;
            if used.saturating_add(cost) <= budget {
                used += cost;
                block.messages.push(entry);
            } else {
                omitted = true;
            }
        }
        let prefix = "主题说明：";
        let characters: Vec<char> = memo.notes.chars().collect();
        let mut low = 0usize;
        let mut high = characters.len();
        while low < high {
            let mid = low + (high - low).div_ceil(2);
            let text = format!("{prefix}{}", characters[..mid].iter().collect::<String>());
            if input
                .estimator
                .message(input.model, &fw::Message::user(text))
                .await?
                .saturating_add(used)
                <= budget
            {
                low = mid;
            } else {
                high = mid - 1;
            }
        }
        if low > 0 {
            let notes = fw::Message::user(format!(
                "{prefix}{}",
                characters[..low].iter().collect::<String>()
            ));
            used += input.estimator.message(input.model, &notes).await?;
            block.messages.push(notes);
        }
        omitted |= low < characters.len();
        if omitted && used.saturating_add(marker_cost) <= input.budget.max_tokens {
            block.messages.push(marker);
        }
        Ok(block)
    }
}
