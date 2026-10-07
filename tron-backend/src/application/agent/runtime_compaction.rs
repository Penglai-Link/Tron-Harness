//! Product compaction selection and persistence. Framework executes the model
//! and controls reassembly, cancellation and bounded overflow recovery.
use super::{ModelClient, ReactAgent};
use crate::domain::agent::{AgentModelConfig, ModelRequest};
use eliteforge_agent_core as fw;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct CompactionModelInput {
    client: Arc<dyn ModelClient>,
    model: AgentModelConfig,
}
impl CompactionModelInput {
    pub(crate) fn new(client: Arc<dyn ModelClient>, model: AgentModelConfig) -> Self {
        Self { client, model }
    }
    pub(crate) fn with_client(&self, client: Arc<dyn ModelClient>) -> Self {
        Self {
            client,
            model: self.model.clone(),
        }
    }
    pub(crate) async fn generate(
        &self,
        request: &fw::ChatRequest,
    ) -> Result<fw::ChatResponse, fw::ModelError> {
        let mut model = self.model.clone();
        model.temperature = request.options.temperature;
        model.top_p = request.options.top_p;
        model.max_tokens = request.options.max_output_tokens;
        let native = eliteforge_agent_official_plugin_model_http::request_from_framework(request)?;
        let native: ModelRequest =
            crate::adapters::outbound::model::http_model_client::convert(&native)
                .map_err(fw::ModelError::Fatal)?;
        let response = self
            .client
            .generate_once(&model, &native)
            .await
            .map_err(|error| {
                eliteforge_agent_official_plugin_model_http::decode_model_error(&error)
                    .unwrap_or(fw::ModelError::Fatal(error))
            })?;
        if response.finish != crate::domain::agent::ModelFinish::Final {
            return Err(fw::ModelError::Fatal(
                "compaction returned tool execution".into(),
            ));
        }
        eliteforge_agent_official_plugin_model_http::response_to_framework(
            crate::adapters::outbound::model::http_model_client::convert(&response)
                .map_err(fw::ModelError::Fatal)?,
        )
    }
}
impl ReactAgent {
    pub(crate) fn framework_compaction_extensions(
        &self,
        model: &AgentModelConfig,
    ) -> fw::Extensions {
        let mut extensions = fw::Extensions::default();
        extensions.insert(CompactionModelInput {
            client: self.model_client.clone(),
            model: model.clone(),
        });
        extensions
    }
}
