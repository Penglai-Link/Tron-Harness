use crate::{
    application::short_term_memory::ShortTermMemoryService,
    domain::agent::{AgentError, AgentMessage, AgentRunContext, ModelResponse},
};

use super::{
    super::{ActiveRunControl, AgentSteerInput, TurnStateKind, user_prompt_message},
    ModelStreamSink, ReactAgent,
    runtime_helpers::short_term_memory_error,
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

pub(super) struct RuntimeSteering {
    control: Option<ActiveRunControl>,
    pending_full_messages: Vec<AgentMessage>,
}

impl RuntimeSteering {
    pub(super) fn new(control: Option<ActiveRunControl>) -> Self {
        Self {
            control,
            pending_full_messages: Vec::new(),
        }
    }

    pub(super) fn inject_at_loop_boundary(
        &mut self,
        context: &mut AgentRunContext,
        short_term_memory: Option<&ShortTermMemoryService>,
    ) -> Result<(), AgentError> {
        let uses_short_term_memory = context.short_term_memory.is_some();
        let messages = record_inputs(short_term_memory, context, self.take_pending()?)?;
        if uses_short_term_memory {
            self.pending_full_messages.extend(messages);
        } else {
            context.messages.extend(messages);
        }
        Ok(())
    }

    pub(super) fn inject_before_model_request(
        &mut self,
        context: &mut AgentRunContext,
        short_term_memory: Option<&ShortTermMemoryService>,
    ) -> Result<(), AgentError> {
        replace_pending_messages(&mut context.messages, &mut self.pending_full_messages);
        let messages = record_inputs(short_term_memory, context, self.take_pending()?)?;
        context.messages.extend(messages);
        Ok(())
    }

    pub(super) fn continue_after_final_response(
        &mut self,
        context: &mut AgentRunContext,
        short_term_memory: Option<&ShortTermMemoryService>,
        turn_state_kind: TurnStateKind,
        response: &ModelResponse,
    ) -> Result<bool, AgentError> {
        let inputs = self.take_pending_or_begin_finishing()?;
        if inputs.is_empty() {
            return Ok(false);
        }

        let uses_short_term_memory = context.short_term_memory.is_some();
        if !uses_short_term_memory
            && turn_state_kind == TurnStateKind::Chat
            && !response.assistant.parts.is_empty()
        {
            context.messages.push(response.assistant.clone());
        }
        let messages = record_inputs(short_term_memory, context, inputs)?;
        if uses_short_term_memory {
            self.pending_full_messages.extend(messages);
        } else {
            context.messages.extend(messages);
        }
        Ok(true)
    }

    fn take_pending(&self) -> Result<Vec<AgentSteerInput>, AgentError> {
        self.control
            .as_ref()
            .map(ActiveRunControl::take_pending)
            .transpose()
            .map_err(control_error)
            .map(Option::unwrap_or_default)
    }

    fn take_pending_or_begin_finishing(&self) -> Result<Vec<AgentSteerInput>, AgentError> {
        self.control
            .as_ref()
            .map(ActiveRunControl::take_pending_or_begin_finishing)
            .transpose()
            .map_err(control_error)
            .map(Option::unwrap_or_default)
    }
}

fn record_inputs(
    short_term_memory: Option<&ShortTermMemoryService>,
    context: &AgentRunContext,
    inputs: Vec<AgentSteerInput>,
) -> Result<Vec<AgentMessage>, AgentError> {
    inputs
        .into_iter()
        .map(|input| {
            if let Some(short_term_memory) = short_term_memory {
                short_term_memory
                    .record_steer_input(context, &input.content, &input.steer_code)
                    .map_err(short_term_memory_error)?;
            }
            Ok(user_prompt_message(input.content, &input.attachments))
        })
        .collect()
}

fn replace_pending_messages(
    messages: &mut Vec<AgentMessage>,
    replacements: &mut Vec<AgentMessage>,
) {
    let mut search_end = messages.len();
    let mut unmatched = Vec::new();
    for replacement in replacements.iter().rev() {
        let replacement_text = replacement.plain_text_content();
        let matching_index = messages[..search_end].iter().rposition(|message| {
            message.role == "user" && message.plain_text_content() == replacement_text
        });
        if let Some(index) = matching_index {
            messages[index] = replacement.clone();
            search_end = index;
        } else {
            unmatched.push(replacement.clone());
        }
    }
    messages.extend(unmatched.into_iter().rev());
    replacements.clear();
}

fn control_error(error: impl ToString) -> AgentError {
    AgentError::InvalidConfiguration(error.to_string())
}
