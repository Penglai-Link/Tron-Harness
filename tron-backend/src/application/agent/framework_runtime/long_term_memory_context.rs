use crate::application::{llm::ContextQueryAllowance, memory::MemoryRunContext};
use async_trait::async_trait;
use eliteforge_agent_core as fw;
pub(super) struct LongTermMemoryContributor {
    pub service: Option<crate::app::AppMemoryService>,
    pub user: Option<String>,
    pub workspace: String,
    pub locale: String,
    pub allowance: ContextQueryAllowance,
}
#[async_trait]
impl fw::ContextContributor for LongTermMemoryContributor {
    fn block_id(&self) -> &str {
        "long_term_memory"
    }
    fn layout_metadata(&self) -> fw::ContextBlockMetadata {
        crate::application::short_term_memory::native_context_block_metadata(self.block_id())
            .expect("memory metadata")
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
        let (Some(service), Some(user)) = (&self.service, &self.user) else {
            return Ok(block);
        };
        let query = input.current_input.text();
        if input.budget.max_tokens == 0 || query.trim().is_empty() {
            return Ok(block);
        }
        let mut run = input.run.clone();
        run.extensions.insert(self.allowance.clone());
        let binding = service.bind_backend_in_run(
            MemoryRunContext::new(user, &self.workspace),
            None,
            None,
            Some(run),
        );
        let memory_query = fw::MemoryQuery {
            text: query,
            limit: 8,
            max_bytes: 2 * 1024 * 1024,
        };
        let search = binding.backend.search(&binding.namespace, &memory_query);
        let result = tokio::select! {_ = input.run.cancelled()=>return Err(fw::AgentError::Cancelled),r=search=>r};
        input.run.check_cancelled()?;
        let result = match result {
            Ok(r) => r,
            Err(error) => {
                tracing::warn!(%error,"Long-term memory retrieval failed");
                input.run.emit_output(fw::OutputEvent::Custom{name:"tron.memory.retrieval.failed".into(),value:serde_json::json!({"blockId":"long_term_memory","error":error.to_string()})});
                let messages = vec![fw::Message::user(
                    tron_i18n::t!(
                        "agent_runtime.context.memory.retrieval_failed",
                        locale = self.locale.as_str()
                    )
                    .into_owned(),
                )];
                if fw::context_tokens(&messages, input.model, input.estimator).await?
                    <= input.budget.max_tokens
                {
                    block.messages = messages;
                }
                return Ok(block);
            }
        };
        let warnings = binding
            .diagnostics
            .lock()
            .map(|v| v.clone())
            .unwrap_or_default();
        for warning in warnings {
            let message = fw::Message::user(tron_i18n::t!(
                "agent_runtime.context.memory.retrieval_diagnostic",
                locale = self.locale.as_str(),
                warning = warning
            ));
            let mut proposed = block.messages.clone();
            proposed.push(message);
            if fw::context_tokens(&proposed, input.model, input.estimator).await?
                <= input.budget.max_tokens
            {
                block.messages = proposed;
            }
        }
        let diagnostic_count = block.messages.len();
        let mut selected = vec![];
        for item in result {
            input.run.check_cancelled()?;
            let Some((memory_code, version)) = item.version.rsplit_once(':') else {
                continue;
            };
            let Ok(version) = version.parse::<u32>() else {
                continue;
            };
            let text = tron_i18n::t!(
                "agent_runtime.context.memory.entry",
                locale = self.locale.as_str(),
                memory_code = memory_code,
                source = item.value.source.id,
                content = item.value.content
            );
            let mut proposed = block.messages.clone();
            proposed.push(fw::Message::user(text));
            if fw::context_tokens(&proposed, input.model, input.estimator).await?
                <= input.budget.max_tokens
            {
                block.messages = proposed;
                selected.push((memory_code.to_owned(), version));
            }
        }
        let current = match service
            .recallable_versions(&MemoryRunContext::new(user, &self.workspace))
        {
            Ok(current) => current,
            Err(error) => {
                tracing::warn!(%error,"Memory final validity check failed");
                block.messages.clear();
                input.run.emit_output(fw::OutputEvent::Custom{name:"tron.memory.retrieval.failed".into(),value:serde_json::json!({"blockId":"long_term_memory","error":error.to_string()})});
                return Ok(block);
            }
        };
        let mut index = 0;
        let mut versions = vec![];
        block.messages.retain(|_| {
            let keep = if index < diagnostic_count {
                true
            } else {
                let version = &selected[index - diagnostic_count];
                let valid = current.contains(version);
                if valid {
                    versions.push(format!("{}:{}", version.0, version.1));
                }
                valid
            };
            index += 1;
            keep
        });
        block.source_version = Some(versions.join(","));
        input.run.check_cancelled()?;
        Ok(block)
    }
}
