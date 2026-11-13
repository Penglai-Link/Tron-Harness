//! Complete live authorized Agent metadata; supplied through the configured keep policy.
use super::super::host::collaboration::Runtime;
use eliteforge_agent_core as fw;
use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    agent_code: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    responsibility: String,
}
#[derive(Clone, Serialize)]
struct Directory {
    agents: Vec<Entry>,
}
impl Directory {
    fn message(&self) -> Result<fw::Message, fw::AgentError> {
        serde_json::to_string(self)
            .map(fw::Message::user)
            .map_err(|error| fw::AgentError::InvalidConfiguration(error.to_string()))
    }
}

pub(super) struct AgentDirectoryContributor(pub Runtime);
#[async_trait::async_trait]
impl fw::ContextContributor for AgentDirectoryContributor {
    fn layout_metadata(&self) -> fw::ContextBlockMetadata {
        crate::application::short_term_memory::native_context_block_metadata(self.block_id())
            .expect("native agent directory metadata")
    }
    fn block_id(&self) -> &str {
        "agent_directory"
    }
    async fn contribute(
        &self,
        input: &fw::ContextInput<'_>,
    ) -> Result<fw::ContextBlock, fw::AgentError> {
        input.run.check_cancelled()?;
        let directory = Directory {
            agents: self
                .0
                .directory()?
                .into_iter()
                .map(|agent| Entry {
                    agent_code: agent.agent_code,
                    name: agent.name,
                    responsibility: agent.description,
                })
                .collect(),
        };
        Ok(fw::ContextBlock {
            id: self.block_id().into(),
            trust: fw::ContextTrust::Data,
            source_version: None,
            messages: vec![directory.message()?],
        })
    }
}
