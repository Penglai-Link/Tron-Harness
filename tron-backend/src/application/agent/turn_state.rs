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
