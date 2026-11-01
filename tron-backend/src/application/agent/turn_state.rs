use serde_json::Value;

use super::ToolSchemaCache;

use crate::domain::agent::{
    AgentMessage, AgentModelProtocol, AgentRunContext, ModelRequest, ModelResponse,
    ToolObservation, responses_function_call_item_json, responses_input_item_json,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnStateKind {
    Chat,
    Responses,
}

#[derive(Debug, Clone)]
pub enum TurnStateAdapter {
    Chat(ChatTranscriptState),
    Responses(ResponsesItemState),
}

impl TurnStateAdapter {
    pub fn new(protocol: AgentModelProtocol, responses_stateful_enabled: bool) -> Self {
        match protocol {
            AgentModelProtocol::ChatCompletions
            | AgentModelProtocol::Anthropic
            | AgentModelProtocol::Gemini => Self::Chat(ChatTranscriptState::default()),
            AgentModelProtocol::Responses => {
                if responses_stateful_enabled {
                    Self::Responses(ResponsesItemState::default())
                } else {
                    Self::Chat(ChatTranscriptState::default())
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

    pub fn prepare_tool_schema(
        &mut self,
        protocol: AgentModelProtocol,
        tools: &[crate::domain::agent::EffectiveTool],
        limit_tokens: u32,
    ) -> Result<std::sync::Arc<crate::domain::agent::PreparedToolSchema>, String> {
        match self {
            Self::Chat(state) => state
                .tool_schema_cache
                .prepare(protocol, tools, limit_tokens),
            Self::Responses(state) => {
                state
                    .tool_schema_cache
                    .prepare(protocol, tools, limit_tokens)
            }
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

    /// Persist only provider continuation metadata, never credentials or messages.
    /// The digest prevents replaying a response against edited/compacted history.
    pub(crate) fn checkpoint(
        &mut self,
        messages: &[AgentMessage],
    ) -> Result<Option<Value>, String> {
        let Self::Responses(state) = self else {
            return Ok(None);
        };
        let Some(response_id) = state.previous_response_id.clone() else {
            return Ok(None);
        };
        if state.consumed_message_count > messages.len() {
            return Err("continuation exceeds transcript".into());
        }
        // The chain folds each consumed message exactly once; a turn only
        // hashes the delta since the previous checkpoint.
        state.advance_chain(messages)?;
        Ok(Some(serde_json::json!({
            "responseId": response_id,
            "consumedMessages": state.consumed_message_count,
            "prefixDigest": state.chain_digest_hex(),
            "contextHash": state.context_hash,
        })))
    }

    pub(crate) fn restore_checkpoint(
        &mut self,
        value: &Value,
        messages: &[AgentMessage],
    ) -> Result<bool, String> {
        let Self::Responses(state) = self else {
            return Ok(false);
        };
        let checkpoint: ResponsesCheckpoint = serde_json::from_value(value.clone())
            .map_err(|e| format!("invalid continuation checkpoint: {e}"))?;
        if checkpoint.response_id.is_empty() || checkpoint.consumed_messages > messages.len() {
            *state = ResponsesItemState::default();
            return Ok(false);
        }
        // Restores validate the persisted prefix once, then reuse its digest
        // for subsequent incremental checkpoints.
        let digest = continuation_digest_bytes(&messages[..checkpoint.consumed_messages])?;
        if checkpoint.prefix_digest != hex_digest(&digest) {
            // A user edited history or compaction changed the prefix. Replay it fully.
            *state = ResponsesItemState::default();
            return Ok(false);
        }
        state.previous_response_id = Some(checkpoint.response_id);
        state.consumed_message_count = checkpoint.consumed_messages;
        state.context_hash = checkpoint.context_hash;
        // Seed the running chain so post-restore checkpoints stay incremental.
        state.chain = (checkpoint.consumed_messages > 0).then_some(digest);
        state.chain_count = checkpoint.consumed_messages;
        Ok(true)
    }

    pub fn record_tool_observation(
        &mut self,
        observation: &ToolObservation,
        model_call_id: Option<String>,
    ) -> AgentMessage {
        tool_observation_message(observation, model_call_id)
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResponsesCheckpoint {
    response_id: String,
    consumed_messages: usize,
    prefix_digest: String,
    context_hash: Option<String>,
}

/// Chained digest: each message is folded in exactly once, so maintaining
/// the chain costs the new messages only instead of re-serializing the
/// whole consumed prefix every turn.
const CHAIN_DOMAIN: &[u8] = b"tron.continuation.v2";

type ChainDigest = [u8; 32];

fn empty_chain() -> ChainDigest {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(CHAIN_DOMAIN);
    hasher.finalize().into()
}

struct DigestWriter<'a>(&'a mut sha2::Sha256);

impl std::io::Write for DigestWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        use sha2::Digest;
        self.0.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn chain_step(
    previous: Option<ChainDigest>,
    message: &AgentMessage,
) -> Result<ChainDigest, String> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    match previous {
        Some(digest) => hasher.update(digest),
        None => hasher.update(CHAIN_DOMAIN),
    }
    // Group IDs are local execution identities, not provider input; the same
    // response can be restored with an equivalent framework-assigned ID.
    let mut provider_message = message.clone();
    provider_message.output_group_id = None;
    serde_json::to_writer(DigestWriter(&mut hasher), &provider_message).map_err(|error| error.to_string())?;
    Ok(hasher.finalize().into())
}

/// Full recomputation over a prefix; used once at restore, when no running
/// chain exists yet.
fn continuation_digest_bytes(messages: &[AgentMessage]) -> Result<ChainDigest, String> {
    let mut chain = None;
    for message in messages {
        chain = Some(chain_step(chain, message)?);
    }
    Ok(chain.unwrap_or_else(empty_chain))
}

#[cfg(test)]
fn continuation_digest(messages: &[AgentMessage]) -> Result<String, String> {
    Ok(hex_digest(&continuation_digest_bytes(messages)?))
}

fn hex_digest(digest: &ChainDigest) -> String {
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[derive(Debug, Clone, Default)]
pub struct ChatTranscriptState {
    tool_schema_cache: ToolSchemaCache,
}

impl ChatTranscriptState {
    pub fn build_request(
        &self,
        context: &AgentRunContext,
        messages: &[AgentMessage],
    ) -> ModelRequest {
        ModelRequest {
            messages: messages.to_vec(),
            tools: context.tools.clone(),
            prepared_tool_schema: self.tool_schema_cache.current(),
            system_agent_session_id: context.system_agent_session_id.clone(),
            response_input_items: None,
            previous_response_id: None,
        }
    }

    pub fn apply_model_response(&mut self, _response: &ModelResponse, _message_count: usize) {}
}

#[derive(Debug, Clone, Default)]
pub struct ResponsesItemState {
    tool_schema_cache: ToolSchemaCache,
    previous_response_id: Option<String>,
    consumed_message_count: usize,
    context_hash: Option<String>,
    /// Running chained digest over the consumed prefix. `chain_count` is the
    /// number of transcript positions already folded in; the chain resets on
    /// rebase, so each checkpoint hashes only the delta since the last one.
    chain: Option<ChainDigest>,
    chain_count: usize,
}

impl ResponsesItemState {
    fn reset_continuation(&mut self) {
        self.previous_response_id = None;
        self.consumed_message_count = 0;
        self.chain = None;
        self.chain_count = 0;
    }
    /// Fold newly consumed messages into the running chain. A rebase or a
    /// shrinking consumed count restarts the chain from the new head.
    fn advance_chain(&mut self, messages: &[AgentMessage]) -> Result<(), String> {
        let target = self.consumed_message_count.min(messages.len());
        if self.chain_count > self.consumed_message_count {
            self.chain = None;
            self.chain_count = 0;
        }
        while self.chain_count < target {
            self.chain = Some(chain_step(self.chain, &messages[self.chain_count])?);
            self.chain_count += 1;
        }
        Ok(())
    }
    fn chain_digest_hex(&self) -> String {
        self.chain
            .as_ref()
            .map(|digest| hex_digest(digest))
            .unwrap_or_else(|| hex_digest(&empty_chain()))
    }

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
            self.reset_continuation();
        }
        if let Some(hash) = context_hash {
            self.context_hash = Some(hash.to_string());
        }
        let current_messages = self.current_messages(messages);
        let mut request_messages = Vec::new();
        if context.model.supports_developer_role && self.previous_response_id.is_some() {
            // Responses does not carry top-level instructions through
            // previous_response_id. Reattach only developer-marked instructions;
            // conversation items remain a delta.
            request_messages.extend(
                messages[..self.consumed_message_count.min(messages.len())]
                    .iter()
                    .filter(|message| {
                        message.role == "system" && message.instruction_origin.is_some()
                    })
                    .cloned(),
            );
        }
        request_messages.extend_from_slice(current_messages);
        ModelRequest {
            messages: request_messages,
            tools: context.tools.clone(),
            prepared_tool_schema: self.tool_schema_cache.current(),
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
        .filter(|message| message.role != "system")
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
    use sha2::{Digest, Sha256};

    use crate::domain::agent::{
        AgentExecutionScope, AgentKind, AgentMessage, AgentModelConfig, AgentModelProtocol,
        AgentRunContext, AgentRunLimits, EffectiveSkill, EffectiveTool,
        EffectiveVariable, ModelResponse, ToolObservation,
    };

    use super::{ResponsesItemState, TurnStateAdapter, TurnStateKind};

    fn responses_context() -> AgentRunContext {
        AgentRunContext {
            system_agent_fast_mode: false,
            collaboration_owner: None,
            agent_code: None,
            auto_model_route: None,
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
                request_headers: Default::default(),
                model_name: "qwen-test".into(),
                thinking_enabled: false,
                supports_developer_role: false,
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
            tool_assignment: None,
            execution_scope: AgentExecutionScope::default(),
            limits: AgentRunLimits::default(),
            messages: vec![
                AgentMessage::system("You are helpful."),
                AgentMessage::user("hello"),
            ]
            .into(),
            observations: Vec::new(),
            short_term_memory: None,
            started_at: "2026-06-04T00:00:00Z".into(),
        }
    }

    #[test]
    fn native_provider_protocols_keep_full_history_without_responses_continuation() {
        for protocol in [AgentModelProtocol::Anthropic, AgentModelProtocol::Gemini] {
            let mut context = responses_context();
            context.model.protocol = protocol;
            let mut state = TurnStateAdapter::new(protocol, true);
            assert_eq!(state.kind(), TurnStateKind::Chat);
            state.apply_model_response(
                &ModelResponse::final_output_with_usage_and_response_id(
                    "first answer",
                    None,
                    Some("provider-response-id".into()),
                ),
                context.messages.len(),
            );
            let mut messages = context.messages.as_ref().clone();
            messages.push(AgentMessage::user("next turn"));
            let request = state.build_request(&context, &messages, false, None);
            assert_eq!(request.messages, messages);
            assert_eq!(request.previous_response_id, None);
            assert_eq!(request.response_input_items, None);
        }
    }

    #[test]
    fn shared_transcript_clones_only_when_mutated() {
        let context = responses_context();
        let mut snapshot = context.messages.clone();
        assert!(std::sync::Arc::ptr_eq(&context.messages, &snapshot));
        std::sync::Arc::make_mut(&mut snapshot).push(AgentMessage::user("new turn"));
        assert!(!std::sync::Arc::ptr_eq(&context.messages, &snapshot));
        assert_eq!(context.messages.len(), 2);
        assert_eq!(snapshot.len(), 3);
    }

    #[test]
    fn streamed_chain_hash_matches_serialized_message_bytes() {
        let message = AgentMessage::user("image-sized payload".repeat(1024));
        let mut expected = Sha256::new();
        expected.update(super::CHAIN_DOMAIN);
        expected.update(serde_json::to_vec(&message).unwrap());
        assert_eq!(
            super::chain_step(None, &message).unwrap(),
            <[u8; 32]>::from(expected.finalize())
        );
    }

    #[test]
    fn restored_empty_prefix_uses_the_same_first_chain_step() {
        let mut original = TurnStateAdapter::new(AgentModelProtocol::Responses, true);
        let response = ModelResponse::final_output_with_usage_and_response_id(
            "ok",
            None,
            Some("resp-1".into()),
        );
        original.apply_model_response(&response, 0);
        let initial = original.checkpoint(&[]).unwrap().unwrap();
        let mut restored = TurnStateAdapter::new(AgentModelProtocol::Responses, true);
        assert!(restored.restore_checkpoint(&initial, &[]).unwrap());

        let messages = [AgentMessage::user("first message")];
        original.apply_model_response(&response, 1);
        restored.apply_model_response(&response, 1);
        assert_eq!(
            original.checkpoint(&messages).unwrap(),
            restored.checkpoint(&messages).unwrap()
        );
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

        let mut messages = context.messages.as_ref().clone();
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
    fn responses_state_repeats_developer_instructions_on_continuation() {
        let mut context = responses_context();
        context.model.supports_developer_role = true;
        let mut messages = vec![
            AgentMessage::system("ordinary host context"),
            AgentMessage::system("managed plugin rule")
                .with_instruction_origin(eliteforge_agent_core::InstructionOrigin::Plugin),
            AgentMessage::user("hello"),
        ];
        let mut state = ResponsesItemState::default();
        let first = state.build_request(&context, &messages, false, Some("hash-1"));
        assert_eq!(first.messages, messages);
        state.apply_model_response(
            &ModelResponse::final_output_with_usage_and_response_id(
                "ok",
                None,
                Some("resp-1".into()),
            ),
            messages.len(),
        );
        messages.push(AgentMessage::user("next"));
        let next = state.build_request(&context, &messages, false, Some("hash-1"));
        assert_eq!(next.previous_response_id.as_deref(), Some("resp-1"));
        assert_eq!(next.messages.len(), 2);
        assert_eq!(next.messages[0].plain_text_content(), "managed plugin rule");
        assert_eq!(next.messages[1].plain_text_content(), "next");
        let items = next.response_input_items.expect("stateful input items");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["content"], json!("next"));
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
        let mut messages = context.messages.as_ref().clone();
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
        std::sync::Arc::make_mut(&mut context.messages).push(AgentMessage::assistant_tool_calls(
            vec![(
                Some("call-1".into()),
                "lookup".into(),
                json!({"query": "stm"}),
            )],
        ));
        std::sync::Arc::make_mut(&mut context.messages).push(AgentMessage::tool_result(
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
    #[test]
    fn chained_checkpoints_match_full_recomputation_and_reject_edits() {
        let context = responses_context();
        let mut state = TurnStateAdapter::new(AgentModelProtocol::Responses, true);
        state.apply_model_response(
            &ModelResponse::final_output_with_usage_and_response_id(
                "ok",
                None,
                Some("resp-1".into()),
            ),
            context.messages.len(),
        );
        // First checkpoint folds the whole prefix once.
        let first = state.checkpoint(&context.messages).unwrap().unwrap();
        // Append turns: each checkpoint hashes only the delta, yet must agree
        // with a full recomputation of the same prefix.
        let mut messages = context.messages.as_ref().clone();
        for turn in 0..3 {
            messages.push(AgentMessage::user(format!("turn {turn}")));
            state.apply_model_response(
                &ModelResponse::final_output_with_usage_and_response_id(
                    &format!("ok {turn}"),
                    None,
                    Some(format!("resp-{turn}")),
                ),
                messages.len(),
            );
            let checkpoint = state.checkpoint(&messages).unwrap().unwrap();
            assert_eq!(
                checkpoint["prefixDigest"],
                serde_json::json!(
                    super::continuation_digest(
                        &messages[..checkpoint["consumedMessages"].as_u64().unwrap() as usize]
                    )
                    .unwrap()
                )
            );
        }
        // Restoring against the unedited transcript succeeds and keeps the
        // incremental chain; restoring against an edited prefix is rejected.
        let last = state.checkpoint(&messages).unwrap().unwrap();
        let mut restored = TurnStateAdapter::new(AgentModelProtocol::Responses, true);
        assert!(restored.restore_checkpoint(&last, &messages).unwrap());
        let mut edited = messages.clone();
        edited[0] = AgentMessage::system("New instructions");
        assert!(!restored.restore_checkpoint(&last, &edited).unwrap());
        let _ = first;
    }

    #[test]
    fn durable_continuation_rejects_edited_prefix_and_restores_exact_history() {
        let context = responses_context();
        let mut state = TurnStateAdapter::new(AgentModelProtocol::Responses, true);
        state.apply_model_response(
            &ModelResponse::final_output_with_usage_and_response_id(
                "ok",
                None,
                Some("resp-durable".into()),
            ),
            context.messages.len(),
        );
        let checkpoint = state.checkpoint(&context.messages).unwrap().unwrap();
        assert!(!checkpoint.to_string().contains("sk-test"));
        let mut restored = TurnStateAdapter::new(AgentModelProtocol::Responses, true);
        let mut next = context.messages.as_ref().clone();
        next.push(AgentMessage::user("continue"));
        assert!(restored.restore_checkpoint(&checkpoint, &next).unwrap());
        let request = restored.build_request(&context, &next, false, None);
        assert_eq!(
            request.previous_response_id.as_deref(),
            Some("resp-durable")
        );
        assert_eq!(request.messages.len(), 1);
        next[0] = AgentMessage::system("New instructions");
        assert!(!restored.restore_checkpoint(&checkpoint, &next).unwrap());
        assert!(
            restored
                .build_request(&context, &next, false, None)
                .previous_response_id
                .is_none()
        );
    }
}
