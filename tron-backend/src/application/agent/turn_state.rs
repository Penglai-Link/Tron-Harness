use serde_json::{Value, json};

use crate::domain::agent::{
    AgentMessage, AgentModelProtocol, AgentRunContext, ModelRequest, ModelResponse,
    ModelStreamEventKind, ToolObservation,
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
    pub fn new(
        protocol: AgentModelProtocol,
        messages: Vec<AgentMessage>,
        responses_stateful_enabled: bool,
    ) -> Self {
        match protocol {
            AgentModelProtocol::ChatCompletions => Self::Chat(ChatTranscriptState::new(messages)),
            AgentModelProtocol::Responses => {
                if responses_stateful_enabled {
                    Self::Responses(ResponsesItemState::new(messages))
                } else {
                    Self::Chat(ChatTranscriptState::new(messages))
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

    pub fn build_request(&self, context: &AgentRunContext) -> ModelRequest {
        match self {
            Self::Chat(state) => state.build_request(context),
            Self::Responses(state) => state.build_request(context),
        }
    }

    pub fn apply_model_response(&mut self, response: &ModelResponse) {
        match self {
            Self::Chat(state) => state.apply_model_response(response),
            Self::Responses(state) => state.apply_model_response(response),
        }
    }

    pub fn record_tool_observation(
        &mut self,
        observation: &ToolObservation,
        model_call_id: Option<String>,
    ) -> AgentMessage {
        match self {
            Self::Chat(state) => state.record_tool_observation(observation, model_call_id),
            Self::Responses(state) => state.record_tool_observation(observation, model_call_id),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatTranscriptState {
    messages: Vec<AgentMessage>,
}

impl ChatTranscriptState {
    pub fn new(messages: Vec<AgentMessage>) -> Self {
        Self { messages }
    }

    pub fn build_request(&self, context: &AgentRunContext) -> ModelRequest {
        ModelRequest {
            messages: self.messages.clone(),
            tools: context.tools.clone(),
            system_agent_session_id: context.system_agent_session_id.clone(),
            response_input_items: None,
            previous_response_id: None,
        }
    }

    pub fn apply_model_response(&mut self, _response: &ModelResponse) {}

    pub fn record_tool_observation(
        &mut self,
        observation: &ToolObservation,
        model_call_id: Option<String>,
    ) -> AgentMessage {
        let message = tool_observation_message(observation, model_call_id);
        self.messages.push(message.clone());
        message
    }
}

#[derive(Debug, Clone)]
pub struct ResponsesItemState {
    messages: Vec<AgentMessage>,
    previous_response_id: Option<String>,
    consumed_message_count: usize,
}

impl ResponsesItemState {
    pub fn new(messages: Vec<AgentMessage>) -> Self {
        Self {
            messages,
            previous_response_id: None,
            consumed_message_count: 0,
        }
    }

    pub fn build_request(&self, context: &AgentRunContext) -> ModelRequest {
        let current_messages = self.current_messages();
        ModelRequest {
            messages: current_messages.to_vec(),
            tools: context.tools.clone(),
            system_agent_session_id: context.system_agent_session_id.clone(),
            response_input_items: Some(responses_input_items_from_messages(current_messages)),
            previous_response_id: self.previous_response_id.clone(),
        }
    }

    pub fn apply_model_response(&mut self, response: &ModelResponse) {
        if let Some(response_id) = response.response_id() {
            self.previous_response_id = Some(response_id.to_string());
        }
        self.consumed_message_count = self.messages.len();
    }

    pub fn record_tool_observation(
        &mut self,
        observation: &ToolObservation,
        model_call_id: Option<String>,
    ) -> AgentMessage {
        let message = tool_observation_message(observation, model_call_id);
        self.messages.push(message.clone());
        message
    }

    fn current_messages(&self) -> &[AgentMessage] {
        let start = self.consumed_message_count.min(self.messages.len());
        &self.messages[start..]
    }
}

pub(crate) fn responses_input_items_from_messages(messages: &[AgentMessage]) -> Vec<Value> {
    messages
        .iter()
        .filter(|message| !matches!(message.role.as_str(), "system" | "developer"))
        .map(responses_input_item_json)
        .collect::<Vec<_>>()
}

pub(crate) fn responses_input_item_json(message: &AgentMessage) -> Value {
    match message.role.as_str() {
        "system" | "developer" => json!({
            "type": "message",
            "role": message.role.as_str(),
            "content": message.content.as_str(),
        }),
        "user" | "assistant" => json!({
            "type": "message",
            "role": message.role.as_str(),
            "content": message.content.as_str(),
        }),
        "tool" => {
            if let Some(call_id) = &message.tool_call_id {
                json!({
                    "type": "function_call_output",
                    "call_id": call_id,
                    "output": message
                        .tool_output
                        .as_deref()
                        .unwrap_or(message.content.as_str()),
                })
            } else {
                json!({
                    "type": "message",
                    "role": "user",
                    "content": message.content.as_str(),
                })
            }
        }
        _ => json!({
            "type": "message",
            "role": "user",
            "content": message.content.as_str(),
        }),
    }
}

fn tool_observation_message(
    observation: &ToolObservation,
    model_call_id: Option<String>,
) -> AgentMessage {
    let content = format_tool_observation_message(observation);
    match model_call_id {
        Some(call_id) => AgentMessage::tool_with_call_id(content, call_id, observation.output.clone()),
        None => AgentMessage::tool(content),
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
            tool_exposure: AgentToolExposureState::default(),
            execution_scope: AgentExecutionScope::default(),
            limits: AgentRunLimits::default(),
            messages: vec![
                AgentMessage::system("You are helpful."),
                AgentMessage::user("hello"),
            ],
            observations: Vec::new(),
            started_at: "2026-06-04T00:00:00Z".into(),
        }
    }

    #[test]
    fn responses_state_uses_delta_items_and_previous_response_id() {
        let context = responses_context();
        let mut state = ResponsesItemState::new(context.messages.clone());
        assert_eq!(TurnStateAdapter::Responses(state.clone()).kind(), TurnStateKind::Responses);

        let request = state.build_request(&context);
        assert_eq!(request.previous_response_id, None);
        let items = request.response_input_items.expect("stateful input items");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["type"], json!("message"));
        assert_eq!(items[0]["content"], json!("hello"));

        state.apply_model_response(&ModelResponse::tool_calls_with_usage_and_response_id(
            vec![],
            None,
            Some("resp-1".into()),
        ));

        let message = state.record_tool_observation(
            &ToolObservation {
                tool_name: "lookup".into(),
                output: "found".into(),
            },
            Some("call-1".into()),
        );
        assert_eq!(message.tool_call_id.as_deref(), Some("call-1"));

        let request = state.build_request(&context);
        assert_eq!(request.previous_response_id.as_deref(), Some("resp-1"));
        let items = request.response_input_items.expect("stateful input items");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["type"], json!("function_call_output"));
        assert_eq!(items[0]["call_id"], json!("call-1"));
        assert_eq!(items[0]["output"], json!("found"));
    }
}
