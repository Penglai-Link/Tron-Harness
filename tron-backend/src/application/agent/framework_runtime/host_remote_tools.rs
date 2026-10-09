//! Host installation, selection and credential lifecycle for shared Framework tools.
use super::*;
use crate::application::{
    agent::tool_catalog::official_remote_descriptors, model_provider::ModelProviderService,
};
use eliteforge_agent_capability_personai::{
    PersonAiConfig, PersonAiConnection, PersonAiToolProvider,
};
use eliteforge_agent_official_plugin_tools_media::{MediaToolsConfig, MediaToolsPlugin};
use eliteforge_agent_official_plugin_tools_web::{WebFetchConfig, WebToolsPlugin};
use fw::ToolProvider;

#[derive(Default)]
pub(super) struct PersonAiState {
    providers: OnceCell<ModelProviderService>,
    connection: Mutex<Option<(String, Arc<PersonAiConnection>)>>,
}
impl FrameworkHost {
    pub fn bind_personai(&self, providers: ModelProviderService) -> Result<(), String> {
        self.0
            .personai
            .providers
            .set(providers)
            .map_err(|_| "PersonAI provider already bound".into())
    }
    pub(super) async fn close_personai_tools(&self) -> Result<(), String> {
        if let Some((_, connection)) = self.0.personai.connection.lock().await.take() {
            connection.close().await?;
        }
        Ok(())
    }
    async fn personai_connection(
        &self,
        scope: &str,
    ) -> Result<Arc<PersonAiConnection>, fw::ToolError> {
        self.ensure_open().map_err(failed)?;
        let providers = self
            .0
            .personai
            .providers
            .get()
            .ok_or_else(|| failed("PersonAI credentials are not configured"))?;
        let mut cache = self.0.personai.connection.lock().await;
        let (identity, url, context) = providers
            .personai_tool_context(scope)
            .await
            .map_err(failed)?;
        if let Some((key, connection)) = cache
            .as_ref()
            .filter(|(key, connection)| key == &identity && !connection.is_closed())
        {
            let _ = key;
            return Ok(connection.clone());
        }
        if let Some((_, previous)) = cache.take() {
            previous.close().await.map_err(failed)?;
        }
        let connection = PersonAiConnection::connect(PersonAiConfig::new(url), context, None)
            .await
            .map_err(failed)?;
        *cache = Some((identity, connection.clone()));
        Ok(connection)
    }
}
fn failed(error: impl std::fmt::Display) -> fw::ToolError {
    fw::ToolError::Failed(error.to_string())
}

pub(super) struct RemoteTools(pub std::sync::Weak<Host>);
#[async_trait::async_trait]
impl fw::ToolProvider for RemoteTools {
    async fn lease(&self, run: &fw::RunContext) -> Result<fw::ToolLease, fw::AgentError> {
        if run
            .extensions
            .get::<super::super::external::ExternalExecution>()
            .is_some()
        {
            return Ok(fw::ToolLease::new(vec![]));
        }
        let host = FrameworkHost(
            self.0
                .upgrade()
                .ok_or_else(|| fw::AgentError::ToolSetup("host closed".into()))?,
        );
        let execution = HostExecution::from_extensions(&run.extensions)?;
        let context = execution.state.lock().await.context.clone();
        let (packs, _) = host.capability_state();
        let enabled = |id: &str| packs.get(id).copied().unwrap_or(true);
        let allowed = |name: &str| context.is_tool_authorized(name);
        let mut tools = Vec::new();
        if enabled("framework.personai") {
            let scope = host
                .0
                .personai
                .providers
                .get()
                .and_then(|p| p.sdk_model_authorization_scope().ok());
            for descriptor in official_remote_descriptors()
                .into_iter()
                .filter(|d| d.id.starts_with("personai.") && allowed(&d.call_name))
            {
                tools.push(fw::ToolBinding::new(
                    descriptor.clone(),
                    PersonAiTool {
                        host: self.0.clone(),
                        scope: scope.clone(),
                        name: descriptor.call_name,
                    },
                ));
            }
        }
        if enabled("framework.web") && allowed("web_fetch") {
            let lease = WebToolsPlugin::new()
                .fetch(WebFetchConfig::unrestricted())
                .lease(run)
                .await?;
            run.own_resources(lease.resources);
            tools.extend(lease.tools);
        }
        if enabled("framework.media")
            && ["media_read_image", "media_read_pdf"]
                .iter()
                .any(|name| allowed(name))
        {
            let scope = crate::application::agent::business_tools::framework_local_scope(&context)
                .map_err(|e| fw::AgentError::ToolSetup(e.to_string()))?;
            let config = MediaToolsConfig::new(".")
                .with_filesystem_scope(scope)
                .with_image_token_estimate(4096);
            let lease = MediaToolsPlugin::new(config).lease(run).await?;
            run.own_resources(lease.resources);
            tools.extend(lease.tools);
        }
        tools.retain(|binding| allowed(&binding.descriptor.call_name));
        for binding in &mut tools {
            binding.exposed = context
                .tools
                .iter()
                .any(|tool| tool.tool_name == binding.descriptor.call_name);
        }
        Ok(fw::ToolLease::new(tools))
    }
}
struct PersonAiTool {
    host: std::sync::Weak<Host>,
    scope: Option<String>,
    name: String,
}
#[async_trait::async_trait]
impl fw::Tool for PersonAiTool {
    async fn call(
        &self,
        arguments: serde_json::Value,
        run: &fw::RunContext,
    ) -> Result<fw::ToolOutput, fw::ToolError> {
        let host = FrameworkHost(self.host.upgrade().ok_or_else(|| failed("host closed"))?);
        let scope = self
            .scope
            .as_deref()
            .ok_or_else(|| failed("Sign in to PersonAI before using remote tools"))?;
        let connection = host.personai_connection(scope).await?;
        let provider = PersonAiToolProvider::new(connection);
        let lease = provider.lease(run).await.map_err(failed)?;
        let tool = lease
            .tools
            .iter()
            .find(|tool| tool.descriptor.call_name == self.name)
            .ok_or_else(|| failed("PersonAI tool unavailable; sign in again"))?;
        let executor = tool.executor.clone();
        run.own_resources(lease.resources);
        executor.call(arguments, run).await
    }
}
