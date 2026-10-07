use crate::{
    application::agent::{ModelClient, runtime::CompactionModelInput},
    domain::agent::AgentModelConfig,
};
use eliteforge_agent_core as fw;
use eliteforge_agent_framework as sdk;
use std::sync::Arc;
struct AuxiliaryModel(CompactionModelInput);
#[async_trait::async_trait]
impl fw::ChatModel for AuxiliaryModel {
    async fn generate(
        &self,
        _: &str,
        request: fw::ChatRequest,
        _: &dyn fw::ChatStream,
    ) -> Result<fw::ChatResponse, fw::ModelError> {
        self.0.generate(&request).await
    }
}
struct Selection(fw::ModelRef);
#[async_trait::async_trait]
impl fw::ModelSelectionStrategy for Selection {
    async fn select(
        &self,
        _: &fw::ModelSelectionContext<'_>,
        _: &fw::ModelCatalog,
    ) -> Result<fw::ModelRef, fw::AgentError> {
        Ok(self.0.clone())
    }
}
pub(crate) fn attach_auxiliary_model(
    request: &mut fw::RunRequest,
    model: &AgentModelConfig,
    client: Arc<dyn ModelClient>,
) -> Result<(), fw::AgentError> {
    let wire = crate::adapters::outbound::model::http_model_client::sdk_config(model)
        .map_err(fw::AgentError::InvalidConfiguration)?;
    let reference = fw::ModelRef::new("tron.auxiliary", &model.model_code);
    let connection = fw::ModelConnection::new(
        &reference.connection,
        Arc::new(AuxiliaryModel(CompactionModelInput::new(
            client,
            model.clone(),
        ))),
        vec![eliteforge_agent_official_plugin_model_http::model_descriptor(&wire)],
    );
    let runtime = sdk::ModelRuntime::new(sdk::ModelRuntimeUpdate::new(
        "tron.auxiliary",
        0,
        vec![connection],
    ))?;
    sdk::ModelExecutionScope::new(runtime, Arc::new(Selection(reference.clone())))?.attach(request);
    request.model = Some(reference);
    Ok(())
}
