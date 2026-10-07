//! Drivers have no ReAct tool-policy plugins. Their storage services are borrowed
//! from the profile Framework; only the profile owner closes those services.
use super::*;
use eliteforge_agent_official_plugin_storage_local::{ArtifactStoreService, SummaryStoreService};

struct ProfileStorage(eliteforge_agent_plugin_core::ServiceRegistry);
#[async_trait::async_trait]
impl fw::Plugin for ProfileStorage {
    fn id(&self) -> &str {
        "tron.profile-storage"
    }
    async fn install(&self, context: &mut fw::PluginContext<'_>) -> Result<(), fw::PluginError> {
        // These tuple adapters are public contracts, not a new storage backend.
        let source = &self.0;
        macro_rules! install {
            ($($service:ident),*) => { $(
                let service = source.get::<$service>().ok_or_else(|| fw::PluginError::message(concat!("missing profile ", stringify!($service))))?;
                context.provide($service(service.0.clone()))?;
                context.contribute_extension(service.0.clone())?;
            )* };
        }
        use fw::{
            ConversationStoreService, InteractionStoreService, LongTermMemoryService,
            ModelCatalogStoreService, SessionStoreService, StateStoreService,
        };
        install!(
            ConversationStoreService,
            StateStoreService,
            InteractionStoreService,
            SessionStoreService,
            LongTermMemoryService,
            ModelCatalogStoreService,
            SummaryStoreService,
            ArtifactStoreService
        );
        Ok(())
    }
}
impl FrameworkHost {
    pub(super) async fn build_driver(&self, external: bool) -> Result<Agent, fw::AgentError> {
        let profile = self.framework().await?;
        let mut builder = Framework::builder()
            .default_models(false)
            .plugin(ProfileStorage(profile.services()?))
            .plugin(eliteforge_agent_official_plugin_agui::AgUiPlugin::default());
        let selection = if external {
            builder = builder
                .plugin(fw::AgentLoopPlugin::new(
                    eliteforge_agent_framework::ExternalAgentLoop,
                ))
                .plugin(fw::ExternalAgentPlugin::new(
                    super::super::external::HostExternalAgent,
                ));
            AgentSelection {
                agent_loop: Some("framework.loop".into()),
                external_agent: Some("framework.external-agent".into()),
                ..AgentSelection::default()
            }
        } else {
            builder = builder.plugin(fw::AgentLoopPlugin::new(
                eliteforge_agent_framework::ContinuationLoop,
            ));
            AgentSelection {
                agent_loop: Some("framework.loop".into()),
                ..AgentSelection::default()
            }
        };
        let framework = builder.build().await?;
        match framework.agent(selection).build().await {
            Ok(agent) => Ok(agent),
            Err(error) => {
                framework.close().await;
                Err(error)
            }
        }
    }
}
