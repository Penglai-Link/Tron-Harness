use crate::{
    application::short_term_memory::ShortTermMemoryService,
    domain::agent::{AgentError, AgentMessage, AgentRunContext},
};

use super::{
    super::{ActiveRunControl, AgentSteerInput, user_prompt_message},
    ModelStreamSink, ReactAgent,
};

impl ReactAgent {
    pub async fn run_with_control(
        &self,
        mut context: AgentRunContext,
        steer_control: Option<ActiveRunControl>,
    ) -> Result<crate::domain::agent::AgentRunResult, AgentError> {
        self.run_with_model(&mut context, None, steer_control).await
    }

    pub async fn run_stream_with_control(
        &self,
        mut context: AgentRunContext,
        sink: ModelStreamSink,
        steer_control: Option<ActiveRunControl>,
    ) -> Result<crate::domain::agent::AgentRunResult, AgentError> {
        self.run_with_model(&mut context, Some(sink), steer_control)
            .await
    }
}

pub(super) fn record_inputs(
    _short_term_memory: Option<&ShortTermMemoryService>,
    _context: &AgentRunContext,
    inputs: Vec<AgentSteerInput>,
) -> Result<Vec<AgentMessage>, AgentError> {
    inputs
        .into_iter()
        .map(|input| Ok(user_prompt_message(input.content, &input.attachments)))
        .collect()
}
