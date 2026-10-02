//! Run-scoped authorization and native SDK model bindings. No transport retry loop.
use super::*;
use crate::domain::agent::{AgentError, AgentModelConfig, AgentRunContext};
use eliteforge_agent_framework as sdk;
use std::collections::BTreeMap;

#[derive(Clone)]
struct Binding {
    config: AgentModelConfig,
}
#[derive(Clone)]
struct Bindings {
    models: BTreeMap<fw::ModelRef, Binding>,
    summaries: BTreeMap<fw::ModelRef, fw::ModelRef>,
}
pub(crate) struct NativeModelScope {
    execution: sdk::ModelExecutionScope,
    bindings: Bindings,
    primary: Option<fw::ModelRef>,
}
pub(super) fn protocol(config: &AgentModelConfig) -> &'static str {
    match config.protocol {
        crate::domain::agent::AgentModelProtocol::Responses => "openai.responses",
        crate::domain::agent::AgentModelProtocol::ChatCompletions => "openai.chat-completions",
        crate::domain::agent::AgentModelProtocol::Embeddings => "openai.embeddings",
    }
}
fn reference(
    context: &AgentRunContext,
    config: &AgentModelConfig,
    scope: Option<&str>,
    auxiliary: bool,
) -> fw::ModelRef {
    use sha2::{Digest, Sha256};
    // Product credentials and run/conversation IDs do not become public identities.
    let endpoint_path = if config.model_code.starts_with("PERSON_AI:") {
        None
    } else {
        config.api_path.as_deref()
    };
    let identity = serde_json::json!([
        context.user_code,
        context.workspace_code,
        config.base_url,
        endpoint_path,
        protocol(config),
        config.model_code,
        scope,
        auxiliary
    ]);
    fw::ModelRef::new(
        format!(
            "product-{:x}",
            Sha256::digest(identity.to_string().as_bytes())
        ),
        &config.model_code,
    )
}
impl NativeModelScope {
    pub(crate) fn new(harness: &ReactAgent, context: &AgentRunContext) -> Result<Self, AgentError> {
        let configs = harness.framework_authorized_models(context)?;
        let runtime = sdk::ModelRuntime::new(sdk::ModelRuntimeUpdate::new("", 0, vec![]))
            .map_err(|e| AgentError::InvalidConfiguration(e.to_string()))?;
        let mut bindings = Bindings {
            models: BTreeMap::new(),
            summaries: BTreeMap::new(),
        };
        let mut order = vec![];
        let mut refreshers: Vec<Arc<dyn sdk::ModelCatalogRefresh>> = vec![];
        for config in configs {
            let main = add_binding(
                &runtime,
                &mut bindings.models,
                harness,
                context,
                config.clone(),
                false,
            )?;
            let summary_config = harness.framework_compaction_model_config(&config);
            let summary = add_binding(
                &runtime,
                &mut bindings.models,
                harness,
                context,
                summary_config,
                true,
            )?;
            bindings.summaries.insert(main.clone(), summary);
            refreshers.push(Arc::new(AuthorizedRefresh {
                reference: main.clone(),
                harness: harness.clone(),
                context: {
                    let mut snapshot = context.clone();
                    snapshot.model = config.clone();
                    snapshot
                },
                scope: harness
                    .framework_model_authorization_scope(&config)
                    .map_err(AgentError::InvalidConfiguration)?,
            }));
            order.push(main);
        }
        let primary = order.first().cloned().ok_or_else(|| {
            AgentError::InvalidConfiguration("no authorized model bindings".into())
        })?;
        let mut execution = sdk::ModelExecutionScope::new(
            runtime,
            Arc::new(AuthorizedSelection {
                order: order.clone(),
            }),
        )
        .map_err(|e| AgentError::InvalidConfiguration(e.to_string()))?;
        execution.refreshers = refreshers;
        if context.auto_model_route.is_some() {
            execution.recovery = sdk::ModelRecoveryPolicy {
                candidates: order,
                allow_explicit_model_replacement: false,
            };
        }
        Ok(Self {
            execution,
            bindings,
            primary: if context.auto_model_route.is_some() {
                None
            } else {
                Some(primary)
            },
        })
    }
    pub(crate) fn summary_reference(
        &self,
        config: &AgentModelConfig,
    ) -> Result<fw::ModelRef, AgentError> {
        self.bindings
            .summaries
            .iter()
            .find(|(main, _)| {
                self.bindings.models.get(*main).is_some_and(|binding| {
                    binding.config.model_code == config.model_code
                        && binding.config.protocol == config.protocol
                        && binding.config.base_url == config.base_url
                        && binding.config.model_name == config.model_name
                })
            })
            .map(|(_, summary)| summary.clone())
            .ok_or_else(|| {
                AgentError::InvalidConfiguration("summary model is not authorized".into())
            })
    }
    pub(crate) fn attach(self, request: &mut fw::RunRequest) {
        request.model = self.primary;
        self.execution.attach(request);
        request.extensions.insert(self.bindings);
    }
}
fn add_binding(
    runtime: &sdk::ModelRuntime,
    bindings: &mut BTreeMap<fw::ModelRef, Binding>,
    harness: &ReactAgent,
    context: &AgentRunContext,
    config: AgentModelConfig,
    auxiliary: bool,
) -> Result<fw::ModelRef, AgentError> {
    let scope = harness
        .framework_model_authorization_scope(&config)
        .map_err(AgentError::InvalidConfiguration)?;
    let model = reference(context, &config, scope.as_deref(), auxiliary);
    if bindings.contains_key(&model) {
        return Ok(model);
    }
    let client = harness.framework_bound_model_client(&config)?;
    let wire = crate::adapters::outbound::model::http_model_client::sdk_config(&config)
        .map_err(AgentError::InvalidConfiguration)?;
    let descriptor = eliteforge_agent_official_plugin_model_http::model_descriptor(&wire);
    let connection = fw::ModelConnection::new(
        &model.connection,
        Arc::new(TronChatModel {
            reference: model.clone(),
            config: config.clone(),
            client: client.clone(),
            auxiliary,
        }),
        vec![descriptor],
    )
    .with_protocol(protocol(&config))
    .with_token_estimator(Arc::new(super::super::context::TronTokenEstimator));
    runtime
        .replace_connection(
            &model.connection,
            sdk::ModelRuntimeUpdate::new(
                scope.unwrap_or_else(|| model.connection.clone()),
                0,
                vec![connection],
            ),
        )
        .map_err(|e| AgentError::InvalidConfiguration(e.to_string()))?;
    bindings.insert(model.clone(), Binding { config });
    Ok(model)
}
struct AuthorizedSelection {
    order: Vec<fw::ModelRef>,
}
#[async_trait]
impl fw::ModelSelectionStrategy for AuthorizedSelection {
    async fn select(
        &self,
        context: &fw::ModelSelectionContext<'_>,
        catalog: &fw::ModelCatalog,
    ) -> Result<fw::ModelRef, fw::AgentError> {
        if context.transient_failures > 2 {
            return Err(fw::AgentError::InvalidConfiguration(
                "Auto fallback allowance exhausted".into(),
            ));
        }
        let index = context.transient_failures;
        self.order
            .iter()
            .filter(|m| {
                catalog
                    .get(m)
                    .is_some_and(|d| d.accepts(context.messages, context.tools, context.options))
            })
            .nth(index)
            .cloned()
            .ok_or_else(|| {
                fw::AgentError::InvalidConfiguration(
                    "no authorized compatible model remains".into(),
                )
            })
    }
}
pub(crate) fn synchronize_selected_model(
    state: &mut HostRunState,
    reference: &fw::ModelRef,
    extensions: &fw::Extensions,
    options: &fw::GenerationOptions,
) -> Result<(), fw::AgentError> {
    let bindings = extensions.get::<Bindings>().ok_or_else(|| {
        fw::AgentError::InvalidConfiguration("native model bindings missing".into())
    })?;
    let binding = bindings.models.get(reference).ok_or_else(|| {
        fw::AgentError::InvalidConfiguration("selected model is outside authorized bindings".into())
    })?;
    if state.context.model.model_code != binding.config.model_code
        || state.context.model.protocol != binding.config.protocol
        || state.context.model.base_url != binding.config.base_url
    {
        state.turn = TurnStateAdapter::new(
            binding.config.protocol,
            binding.config.responses_stateful_enabled,
        );
        state.prepared = None;
    }
    state.context.model = binding.config.clone();
    state.context.model.max_tokens = options.max_output_tokens.or(state.context.model.max_tokens);
    state.context.model.temperature = options.temperature.or(state.context.model.temperature);
    state.context.model.top_p = options.top_p.or(state.context.model.top_p);
    Ok(())
}
pub(crate) fn compaction_model_reference(
    extensions: &fw::Extensions,
    main: &fw::ModelRef,
) -> Result<fw::ModelRef, fw::AgentError> {
    extensions
        .get::<Bindings>()
        .and_then(|b| b.summaries.get(main).cloned())
        .ok_or_else(|| {
            fw::AgentError::InvalidConfiguration("authorized summary model binding missing".into())
        })
}

/// Re-read host authorization only. The SDK owns refresh fencing and recovery.
/// No inference is sent by discovery, and a switched account cannot publish here.
struct AuthorizedRefresh {
    reference: fw::ModelRef,
    harness: ReactAgent,
    context: AgentRunContext,
    scope: Option<String>,
}
#[async_trait]
impl sdk::ModelCatalogRefresh for AuthorizedRefresh {
    fn connection_id(&self) -> &str {
        &self.reference.connection
    }
    fn account_identity(&self) -> Option<sdk::ModelAccountIdentity> {
        self.harness
            .framework_model_authorization_scope(&self.context.model)
            .ok()
            .map(|scope| sdk::ModelAccountIdentity {
                scope: scope.unwrap_or_else(|| self.reference.connection.clone()),
                revision: 0,
            })
    }
    async fn refresh(&self) -> Result<sdk::ModelRuntimeUpdate, fw::ModelError> {
        let expected = sdk::ModelAccountIdentity {
            scope: self
                .scope
                .clone()
                .unwrap_or_else(|| self.reference.connection.clone()),
            revision: 0,
        };
        if self.account_identity() != Some(expected.clone()) {
            return Err(account_changed());
        }
        let config = self
            .harness
            .framework_authorized_models(&self.context)
            .map_err(|e| fw::ModelError::Fatal(e.to_string()))?
            .into_iter()
            .find(|config| config.model_code == self.reference.model);
        let mut chat = vec![];
        if let Some(config) = config {
            if config != self.context.model {
                return Err(fw::ModelError::Unavailable(
                    "authorized model configuration changed; start a new model execution scope"
                        .into(),
                ));
            }
            if reference(&self.context, &config, self.scope.as_deref(), false) != self.reference {
                return Err(account_changed());
            }
            let client = self
                .harness
                .framework_bound_model_client(&config)
                .map_err(|e| fw::ModelError::Fatal(e.to_string()))?;
            let wire = crate::adapters::outbound::model::http_model_client::sdk_config(&config)
                .map_err(fw::ModelError::Fatal)?;
            chat.push(
                fw::ModelConnection::new(
                    &self.reference.connection,
                    Arc::new(TronChatModel {
                        reference: self.reference.clone(),
                        config: config.clone(),
                        client,
                        auxiliary: false,
                    }),
                    vec![eliteforge_agent_official_plugin_model_http::model_descriptor(&wire)],
                )
                .with_protocol(protocol(&config))
                .with_token_estimator(Arc::new(super::super::context::TronTokenEstimator)),
            );
        }
        if self.account_identity() != Some(expected.clone()) {
            return Err(account_changed());
        }
        Ok(sdk::ModelRuntimeUpdate::new(
            expected.scope,
            expected.revision,
            chat,
        ))
    }
}
fn account_changed() -> fw::ModelError {
    fw::ModelError::Provider(Box::new(fw::ProviderModelError {
        authorization_attempt: None,
        provider: "authorized-model-runtime".into(),
        category: fw::ProviderErrorCategory::AccountChanged,
        http_status: None,
        business_code: None,
        error_type: None,
        model: None,
        message: "model authorization changed during discovery".into(),
    }))
}
