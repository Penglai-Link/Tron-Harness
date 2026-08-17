use serde_json::Value;

use crate::domain::agent::{
    AgentMessage, AgentModelProtocol, AgentRunContext, ModelRequest, ModelResponse,
    ModelStreamEventKind, ToolObservation, responses_function_call_item_json,
    responses_input_item_json,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnStateKind {
    Chat,
    Responses,
}

impl TurnStateKind {
    pub fn stream_event_name(self, event_kind: ModelStreamEventKind) -> &'static str {
        match event_kind {
            ModelStreamEventKind::Text => "agent.output.delta",
            ModelStreamEventKind::Reasoning => "agent.reasoning.raw_delta",
            ModelStreamEventKind::ReasoningSummary => "agent.reasoning.delta",
        }
    }
}

#[derive(Debug, Clone)]
pub enum TurnStateAdapter {
    Chat(ChatTranscriptState),
    Responses(ResponsesItemState),
}

impl TurnStateAdapter {
    pub fn new(protocol: AgentModelProtocol, responses_stateful_enabled: bool) -> Self {
        match protocol {
            AgentModelProtocol::ChatCompletions => Self::Chat(ChatTranscriptState),
            AgentModelProtocol::Responses => {
                if responses_stateful_enabled {
                    Self::Responses(ResponsesItemState::default())
                } else {
                    Self::Chat(ChatTranscriptState)
                }
            }
            AgentModelProtocol::Embeddings => {
                panic!("agent runtime does not support embeddings protocol")
            }
        }
    }

    pub fn kind(&self) -> TurnStateKind {
        match self {
            Self::Chat(_) => TurnStateKind::Chat,
            Self::Responses(_) => TurnStateKind::Responses,
        }
    }

    pub fn build_request(
        &mut self,
        context: &AgentRunContext,
        messages: &[AgentMessage],
        rebased: bool,
        context_hash: Option<&str>,
    ) -> ModelRequest {
        match self {
            Self::Chat(state) => state.build_request(context, messages),
            Self::Responses(state) => state.build_request(context, messages, rebased, context_hash),
        }
    }

    pub fn apply_model_response(&mut self, response: &ModelResponse, message_count: usize) {
        match self {
            Self::Chat(state) => state.apply_model_response(response, message_count),
            Self::Responses(state) => state.apply_model_response(response, message_count),
        }
    }

    pub fn record_tool_observation(
        &mut self,
        observation: &ToolObservation,
        model_call_id: Option<String>,
    ) -> AgentMessage {
        tool_observation_message(observation, model_call_id)
    }
}

#[derive(Debug, Clone)]
pub struct ChatTranscriptState;

impl ChatTranscriptState {
    pub fn build_request(
        &self,
        context: &AgentRunContext,
        messages: &[AgentMessage],
    ) -> ModelRequest {
        ModelRequest {
            messages: messages.to_vec(),
            tools: context.tools.clone(),
            system_agent_session_id: context.system_agent_session_id.clone(),
            response_input_items: None,
            previous_response_id: None,
        }
    }

    pub fn apply_model_response(&mut self, _response: &ModelResponse, _message_count: usize) {}
}

#[derive(Debug, Clone, Default)]
pub struct ResponsesItemState {
    previous_response_id: Option<String>,
    consumed_message_count: usize,
    context_hash: Option<String>,
}

impl ResponsesItemState {
    pub fn build_request(
        &mut self,
        context: &AgentRunContext,
        messages: &[AgentMessage],
        rebased: bool,
        context_hash: Option<&str>,
    ) -> ModelRequest {
        let context_changed = context_hash
            .map(|hash| self.context_hash.as_deref() != Some(hash))
            .unwrap_or(false);
        let unhashable_rebase = rebased && context_hash.is_none();
        if unhashable_rebase || context_changed || self.consumed_message_count > messages.len() {
            self.previous_response_id = None;
            self.consumed_message_count = 0;
        }
        if let Some(hash) = context_hash {
            self.context_hash = Some(hash.to_string());
        }
        let current_messages = self.current_messages(messages);
        ModelRequest {
            messages: current_messages.to_vec(),
            tools: context.tools.clone(),
            system_agent_session_id: context.system_agent_session_id.clone(),
            response_input_items: Some(responses_input_items_from_messages(current_messages)),
            previous_response_id: self.previous_response_id.clone(),
        }
    }

    pub fn apply_model_response(&mut self, response: &ModelResponse, message_count: usize) {
        if let Some(response_id) = response.response_id() {
            self.previous_response_id = Some(response_id.to_string());
        }
        self.consumed_message_count = message_count;
    }

    fn current_messages<'a>(&self, messages: &'a [AgentMessage]) -> &'a [AgentMessage] {
        let start = self.consumed_message_count.min(messages.len());
        &messages[start..]
    }
}

pub(crate) fn responses_input_items_from_messages(messages: &[AgentMessage]) -> Vec<Value> {
    messages
        .iter()
        .filter(|message| !matches!(message.role.as_str(), "system" | "developer"))
        .flat_map(|message| {
            let function_items = message
                .parts
                .iter()
                .filter_map(responses_function_call_item_json)
                .collect::<Vec<_>>();
            let has_text = !message.plain_text_content().trim().is_empty();
            let mut items = Vec::new();
            if has_text || function_items.is_empty() {
                items.push(responses_input_item_json(message));
            }
            items.extend(function_items);
            items
        })
        .collect::<Vec<_>>()
}

fn tool_observation_message(
    observation: &ToolObservation,
    model_call_id: Option<String>,
) -> AgentMessage {
    match model_call_id {
        Some(call_id) => AgentMessage::tool_result(
            observation.tool_name.clone(),
            Some(call_id),
            observation.output.clone(),
        ),
        None => AgentMessage::tool_result(
            observation.tool_name.clone(),
            None,
            format_tool_observation_message(observation),
        ),
    }
}

fn format_tool_observation_message(observation: &ToolObservation) -> String {
    format!(
        "tool:{}\nresult:\n{}",
        observation.tool_name, observation.output
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::domain::agent::{
        AgentExecutionScope, AgentKind, AgentMessage, AgentModelConfig, AgentModelProtocol,
        AgentRunContext, AgentRunLimits, AgentToolExposureState, EffectiveSkill, EffectiveTool,
        EffectiveVariable, ModelResponse, ToolObservation,
    };

    use super::{ResponsesItemState, TurnStateAdapter, TurnStateKind};

    fn responses_context() -> AgentRunContext {
        AgentRunContext {
            conversation_code: Some("CONV-1".into()),
            run_code: "RUN-1".into(),
            workspace_code: "WS-1".into(),
            user_code: Some("USR-1".into()),
            locale: "en-US".into(),
            request_id: None,
            stream_key: Some("CONV-1".into()),
            agent_kind: AgentKind::React,
            system_agent_session_id: None,
            model: AgentModelConfig {
                model_code: "MODEL-1".into(),
                protocol: AgentModelProtocol::Responses,
                base_url: "https://provider.test/v1".into(),
                api_path: None,
                api_key: "sk-test".into(),
                model_name: "qwen-test".into(),
                thinking_enabled: false,
                reasoning_effort: None,
                responses_stateful_enabled: true,
                temperature: Some(0.2),
                max_tokens: Some(128),
                context_window_tokens: Some(8_192),
                top_p: Some(1.0),
                max_retries: Some(0),
            },
            variables: vec![EffectiveVariable {
                key: "REGION".into(),
                value: "us".into(),
                source: "LOCAL".into(),
                sensitive: false,
            }],
            skills: vec![EffectiveSkill {
                skill_name: "planner".into(),
                description: "Plan work".into(),
                instruction: Some("Think briefly.".into()),
                tool_names: vec!["lookup".into()],
            }],
            tools: vec![EffectiveTool {
                tool_name: "lookup".into(),
                domain_code: None,
                description: "Lookup".into(),
                input_schema: json!({"type": "object"}),
                skill_name: Some("planner".into()),
                required_variable_keys: vec![],
            }],
            effective_tools: Vec::new(),
            tool_exposure: AgentToolExposureState::default(),
            execution_scope: AgentExecutionScope::default(),
            limits: AgentRunLimits::default(),
            messages: vec![
                AgentMessage::system("You are helpful."),
                AgentMessage::user("hello"),
            ],
            observations: Vec::new(),
            short_term_memory: None,
            started_at: "2026-06-04T00:00:00Z".into(),
        }
    }

    #[test]
    fn responses_state_uses_delta_items_and_previous_response_id() {
        let context = responses_context();
        let mut state = ResponsesItemState::default();
        assert_eq!(
            TurnStateAdapter::Responses(state.clone()).kind(),
            TurnStateKind::Responses
        );

        let request = state.build_request(&context, &context.messages, false, Some("hash-1"));
        assert_eq!(request.previous_response_id, None);
        let items = request.response_input_items.expect("stateful input items");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["type"], json!("message"));
        assert_eq!(items[0]["content"], json!("hello"));

        state.apply_model_response(
            &ModelResponse::tool_calls_with_usage_and_response_id(
                vec![],
                None,
                Some("resp-1".into()),
            ),
            context.messages.len(),
        );

        let message = TurnStateAdapter::Responses(state.clone()).record_tool_observation(
            &ToolObservation {
                tool_name: "lookup".into(),
                output: "found".into(),
            },
            Some("call-1".into()),
        );
        assert_eq!(message.first_tool_call_id(), Some("call-1"));

        let mut messages = context.messages.clone();
        messages.push(message);
        let request = state.build_request(&context, &messages, false, Some("hash-1"));
        assert_eq!(request.previous_response_id.as_deref(), Some("resp-1"));
        let items = request.response_input_items.expect("stateful input items");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["type"], json!("function_call_output"));
        assert_eq!(items[0]["call_id"], json!("call-1"));
        assert_eq!(items[0]["output"], json!("found"));
    }

    #[test]
    fn responses_state_resets_when_context_is_rebased() {
        let context = responses_context();
        let mut state = ResponsesItemState::default();
        state.apply_model_response(
            &ModelResponse::final_output_with_usage_and_response_id(
                "ok",
                None,
                Some("resp-1".into()),
            ),
            context.messages.len(),
        );

        let request = state.build_request(&context, &context.messages, true, None);
        assert_eq!(request.previous_response_id, None);
        let items = request.response_input_items.expect("input items");
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn responses_state_keeps_previous_response_id_when_rebase_hash_is_stable() {
        let context = responses_context();
        let mut state = ResponsesItemState::default();
        let _ = state.build_request(&context, &context.messages, true, Some("base-hash"));
        state.apply_model_response(
            &ModelResponse::tool_calls_with_usage_and_response_id(
                vec![],
                None,
                Some("resp-1".into()),
            ),
            context.messages.len(),
        );
        let mut messages = context.messages.clone();
        messages.push(AgentMessage::tool_result(
            "lookup",
            Some("call-1".into()),
            "found",
        ));

        let request = state.build_request(&context, &messages, true, Some("base-hash"));

        assert_eq!(request.previous_response_id.as_deref(), Some("resp-1"));
        let items = request.response_input_items.expect("input items");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["type"], json!("function_call_output"));
        assert_eq!(items[0]["call_id"], json!("call-1"));
    }

    #[test]
    fn responses_state_resets_when_context_hash_changes() {
        let context = responses_context();
        let mut state = ResponsesItemState::default();
        let _ = state.build_request(&context, &context.messages, false, Some("hash-1"));
        state.apply_model_response(
            &ModelResponse::final_output_with_usage_and_response_id(
                "ok",
                None,
                Some("resp-1".into()),
            ),
            context.messages.len(),
        );

        let request = state.build_request(&context, &context.messages, false, Some("hash-2"));
        assert_eq!(request.previous_response_id, None);
        let items = request.response_input_items.expect("input items");
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn responses_state_replays_tool_calls_after_rebase_without_previous_response() {
        let mut context = responses_context();
        context
            .messages
            .push(AgentMessage::assistant_tool_calls(vec![(
                Some("call-1".into()),
                "lookup".into(),
                json!({"query": "stm"}),
            )]));
        context.messages.push(AgentMessage::tool_result(
            "lookup",
            Some("call-1".into()),
            "found",
        ));
        let mut state = ResponsesItemState::default();
        state.apply_model_response(
            &ModelResponse::final_output_with_usage_and_response_id(
                "ok",
                None,
                Some("resp-1".into()),
            ),
            2,
        );

        let request = state.build_request(&context, &context.messages, true, Some("hash-2"));

        assert_eq!(request.previous_response_id, None);
        let items = request.response_input_items.expect("input items");
        assert_eq!(items.len(), 3);
        assert_eq!(items[0]["type"], json!("message"));
        assert_eq!(items[0]["content"], json!("hello"));
        assert_eq!(items[1]["type"], json!("function_call"));
        assert_eq!(items[1]["call_id"], json!("call-1"));
        assert_eq!(items[1]["name"], json!("lookup"));
        assert_eq!(items[1]["arguments"], json!("{\"query\":\"stm\"}"));
        assert_eq!(items[2]["type"], json!("function_call_output"));
        assert_eq!(items[2]["call_id"], json!("call-1"));
        assert_eq!(items[2]["output"], json!("found"));
    }
}
