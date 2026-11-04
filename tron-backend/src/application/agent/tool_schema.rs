use std::sync::Arc;

use serde_json::Value;

use crate::domain::agent::{AgentModelProtocol, EffectiveTool, ModelRequest, PreparedToolSchema};

/// Use the SDK's protocol projection for budget estimation; wire encoding stays
/// inside the model plugin, including protocols implemented through GenAI.
pub fn tool_schema_payload(protocol: AgentModelProtocol, tools: &[EffectiveTool]) -> Value {
    let tools = tools
        .iter()
        .map(|tool| eliteforge_agent_official_plugin_model_http::EffectiveTool {
            tool_name: tool.tool_name.clone(),
            description: tool.description.clone(),
            input_schema: tool.input_schema.clone(),
        })
        .collect::<Vec<_>>();
    eliteforge_agent_official_plugin_model_http::tool_schema_payload(protocol.into(), &tools)
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
