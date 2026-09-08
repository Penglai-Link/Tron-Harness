use std::sync::Arc;

use serde_json::{Value, json};

use crate::domain::agent::{AgentModelProtocol, EffectiveTool, ModelRequest, PreparedToolSchema};

/// Shared by measurement and protocol adapters so they use the same fields and wrappers.
pub fn tool_schema_payload(protocol: AgentModelProtocol, tools: &[EffectiveTool]) -> Value {
    Value::Array(
        tools
            .iter()
            .map(|tool| match protocol {
                AgentModelProtocol::ChatCompletions => json!({
                    "type": "function",
                    "function": {
                        "name": tool.tool_name,
                        "description": tool.description,
                        "parameters": tool.input_schema,
                    },
                }),
                AgentModelProtocol::Responses => json!({
                    "type": "function",
                    "name": tool.tool_name,
                    "description": tool.description,
                    "parameters": tool.input_schema,
                }),
                AgentModelProtocol::Embeddings => Value::Null,
            })
            .collect(),
    )
}

pub fn request_tool_schema_payload(protocol: AgentModelProtocol, request: &ModelRequest) -> Value {
    request
        .prepared_tool_schema
        .as_ref()
        .filter(|snapshot| snapshot.matches(protocol, &request.tools))
        .map(|snapshot| snapshot.payload().clone())
        .unwrap_or_else(|| tool_schema_payload(protocol, &request.tools))
}

/// Owned by one runtime turn-state, never shared across independent runs.
#[derive(Debug, Clone, Default)]
pub struct ToolSchemaCache {
    current: Option<Arc<PreparedToolSchema>>,
}

impl ToolSchemaCache {
    pub fn current(&self) -> Option<Arc<PreparedToolSchema>> {
        self.current.clone()
    }

    /// Compare the ordered definitions without serializing or hashing them on a hit.
    /// A rejected candidate does not replace the last accepted snapshot.
    pub fn prepare(
        &mut self,
        protocol: AgentModelProtocol,
        tools: &[EffectiveTool],
        limit_tokens: u32,
    ) -> Result<Arc<PreparedToolSchema>, String> {
        if protocol == AgentModelProtocol::Embeddings {
            return Err("tool schemas are unsupported for embeddings".into());
        }
        let snapshot = match self
            .current
            .as_ref()
            .filter(|item| item.matches(protocol, tools))
        {
            Some(snapshot) => Arc::clone(snapshot),
            None => Arc::new(PreparedToolSchema::new(
                protocol,
                tools.to_vec(),
                tool_schema_payload(protocol, tools),
                self.current
                    .as_ref()
                    .map_or(1, |item| item.revision().saturating_add(1)),
            )?),
        };
        if snapshot.estimated_tokens() > limit_tokens {
            return Err(format!(
                "TOOL_SCHEMA_OVER_BUDGET: estimatedTokens={}, limitTokens={}, toolCount={}; tool exposure was not changed",
                snapshot.estimated_tokens(),
                limit_tokens,
                tools.len(),
            ));
        }
        self.current = Some(Arc::clone(&snapshot));
        Ok(snapshot)
    }
}
