//! Host source mapping into the public SDK assembler. No model loop lives here.
use super::{messages, model::HostExecution};
use crate::{
    application::short_term_memory::{
        ContextProviderPolicy, ContextReductionStrategy as Strategy, PreparedModelContext,
    },
    domain::agent::AgentRunContext,
};
use async_trait::async_trait;
use eliteforge_agent_core as fw;
use eliteforge_agent_core::ContextAssembler;
use eliteforge_agent_memory as memory;
use sha2::{Digest, Sha256};
use std::sync::Arc;

#[path = "context_budget.rs"]
mod context_budget;
use context_budget::SharedSeedBudget;
#[path = "agent_directory_context.rs"]
mod agent_directory_context;
#[path = "knowledge_base_context.rs"]
mod knowledge_base_context;

pub(super) async fn assemble(
    input: &fw::ContextAssemblyInput<'_>,
    host: Arc<HostExecution>,
    context: &AgentRunContext,
    projection: &PreparedModelContext,
    force: bool,
    calls: Arc<std::sync::atomic::AtomicUsize>,
) -> Result<fw::ContextWindow, fw::AgentError> {
    let config = host.harness.framework_context_config();
    config
        .context
        .providers
        .prices
        .validate()
        .map_err(fw::AgentError::InvalidConfiguration)?;
    let plan = config.model_context_budget_plan(
        context.model.context_window_tokens,
        context.model.max_tokens,
    );
    let source = projection.sources.clone().unwrap_or_else(|| {
        let prefix = context
            .messages
            .iter()
            .take_while(|m| matches!(m.role.as_str(), "system" | "developer"))
            .cloned()
            .collect::<Vec<_>>();
        crate::application::short_term_memory::PreparedContextSources {
            revision: memory::MemoryRevision {
                scope: context
                    .memory_scope(
                        context
                            .conversation_code
                            .as_deref()
                            .unwrap_or(&context.run_code),
                    )
                    .expect("validated run scope"),
                generation: input.history.generation.clone(),
                revision: input.history.revision,
            },
            history: context.messages[prefix.len()..].to_vec(),
            prefix,
            checkpoint: vec![],
            working: vec![],
        }
    });
    let mut instructions = source
        .prefix
        .iter()
        .filter(|m| matches!(m.role.as_str(), "system" | "developer"))
        .map(messages::encode)
        .collect::<Result<Vec<_>, _>>()
        .map_err(fw::AgentError::InvalidConfiguration)?;
    for instruction in input.instructions {
        if !instructions.contains(instruction) {
            instructions.push(instruction.clone());
        }
    }
    // Native plugins may introduce domains only after their bindings are leased.
    // Publish that live directory before assembly so it shares the seed budget.
    if context.tool_exposure.tool_exposure_mode
        == Some(crate::domain::agent::ToolExposureMode::DomainDiscovery)
    {
        let directory = serde_json::to_string(&context.tool_exposure.visible_domains)
            .map_err(|error| fw::AgentError::InvalidConfiguration(error.to_string()))?;
        instructions.push(fw::Message::new(
            fw::Role::Developer,
            format!(
                "Current authorized tool domains for this run: {directory}. Use {} to discover tools in these domains. This current directory supersedes any earlier domain listing.",
                crate::application::agent::DISCOVERY_EXPAND_TOOL_DOMAIN_TOOL,
            ),
        ));
    }
    let mut application = source
        .prefix
        .iter()
        .filter(|m| !matches!(m.role.as_str(), "system" | "developer"))
        .cloned()
        .collect::<Vec<_>>();
    if application.is_empty() {
        if let Some(mut scope) =
            crate::application::agent::execution_context_message(&context.execution_scope)
        {
            scope.role = "user".into();
            application.push(scope);
        }
    }
    let history = input.history.clone();
    let budget = fw::ContextBudget {
        max_tokens: input
            .budget
            .max_tokens
            .min(plan.effective_input_budget as usize),
        ..input.budget
    };
    let protocol = input
        .estimator
        .tools(input.model, input.tools)
        .await?
        .saturating_add(input.estimator.options(input.model, input.options).await?);
    let available = fw::ContextBudget {
        max_tokens: budget.max_tokens.checked_sub(protocol).ok_or_else(|| {
            fw::ContextError::BudgetExceeded("protocol exceeds input capacity".into())
        })?,
        ..budget
    };
    let caps = &config.context.blocks;
    let seed = fw::ContextBlockConfig::new(
        fw::INSTRUCTIONS_BLOCK,
        block_budget(&caps.seed_prompt_prefix)?,
    );
    let seed_cap = fw::configured_block_budget(&seed, available)?.max_tokens;
    let working_cap = fw::configured_block_budget(
        &fw::ContextBlockConfig::new(
            "current_session_working_state",
            block_budget(&caps.current_session_working_state)?,
        ),
        available,
    )?
    .max_tokens;
    let visual_enabled = host
        .harness
        .framework_knowledge_bases()
        .is_some_and(|s| s.visual_context_enabled());
    let knowledge_cap = if visual_enabled {
        available.max_tokens.min(16_000)
    } else {
        (working_cap / 2).min(1200)
    };
    let mut contributors: Vec<Arc<dyn fw::ContextContributor>> = Vec::new();
    contributors.push(Arc::new(knowledge_base_context::KnowledgeBaseContributor {
        service: host.harness.framework_knowledge_bases(),
        user: context.user_code.clone(),
        workspace: context.workspace_code.clone(),
        agent: context.agent_code.clone(),
        allowance: crate::application::llm::ContextQueryAllowance {
            calls: calls.clone(),
            limit: config.context.providers.max_auxiliary_calls_per_step,
        },
    }));
    // The runtime-identity contributor feeds the dude blocks; only an owning
    // dude context declares those blocks. Registering it elsewhere bills its
    // instruction contribution against the shared seed without a block.
    if context.is_owning_dude() {
        contributors.push(Arc::new(super::dude_context::DudeContributor { service:host.harness.framework_dudes(),context:context.clone(),id:"runtime_identity" }));
    }
    let mut configs = std::collections::HashMap::new();
    let directory = input
        .run
        .extensions
        .get::<super::host::collaboration::Runtime>();
    if let Some(runtime) = directory.as_ref() {
        contributors.push(Arc::new(
            agent_directory_context::AgentDirectoryContributor((**runtime).clone()),
        ));
    }
    configs.insert(
        "seed_prompt_prefix",
        fw::ContextBlockConfig::new(fw::INSTRUCTIONS_BLOCK, fw::BlockBudget::tokens(seed_cap)),
    );
    for (id, entries, cap, policy) in [
        (
            "application_context",
            &application,
            fw::BlockBudget::tokens(seed_cap),
            None,
        ),
        (
            "current_session_checkpoint",
            &source.checkpoint,
            block_budget(&caps.current_session_checkpoint)?,
            Some(&config.context.providers.checkpoint),
        ),
        (
            "current_session_working_state",
            &source.working,
            block_budget(&caps.current_session_working_state)?,
            Some(&config.context.providers.working),
        ),
    ] {
        let native =
            messages::encode_history(entries).map_err(fw::AgentError::InvalidConfiguration)?;
        let checkpoint =
            (id == "current_session_checkpoint").then_some(&projection.checkpoint_code);
        let generation = fingerprint(&(id, entries, policy, checkpoint, &context.user_code))?;
        let revision = memory::MemoryRevision {
            generation,
            ..source.revision.clone()
        };
        let snapshot = memory::ScopedMemoryContext {
            revision: revision.clone(),
            block: fw::ContextBlock {
                id: id.into(),
                messages: native,
                trust: fw::ContextTrust::Data,
                source_version: None,
            },
        };
        contributors.push(Arc::new(memory::MemoryContextContributor::new(
            id,
            revision.scope.clone(),
            Arc::new(snapshot.clone()),
        )?));
        let mut block = fw::ContextBlockConfig::new(id, cap);
        if let Some(policy) = policy {
            block = compression(block, policy)?;
            let guard: Arc<dyn memory::MemoryContextRevisionGuard> =
                if context.short_term_memory.is_some() {
                    Arc::new(LiveSourceGuard {
                        host: host.clone(),
                        context: context.clone(),
                        id: id.into(),
                        policy: policy.clone(),
                    })
                } else {
                    Arc::new(snapshot)
                };
            block.reducer = reducer(
                policy,
                &revision,
                guard,
                host.clone(),
                context,
                input.limits,
                &input.run.extensions,
                input.model,
            )?;
        }
        configs.insert(id, block);
    }
    let mut knowledge =
        fw::ContextBlockConfig::new("knowledge_base", fw::BlockBudget::tokens(knowledge_cap));
    knowledge.reducer = Arc::new(memory::BoundMultimodalData::default());
    configs.insert("knowledge_base", knowledge);
    let mut block = fw::ContextBlockConfig::new(
        fw::HISTORY_BLOCK,
        fw::BlockBudget::tokens(plan.effective_input_budget as usize),
    );
    if config.compaction.auto_enabled && config.context.providers.history.strategy != Strategy::Keep
    {
        block = compression(block, &config.context.providers.history)?;
        match config.context.providers.history.strategy {
            Strategy::Checkpoint => {
                let model = host.harness.framework_compaction_model_config(&context.model);
                let compressor = memory::ModelCompressor::new(
                    super::model::compaction_model_reference(&input.run.extensions,input.model)?,
                    memory::CompressionOptions { max_input_tokens: model.context_window_tokens.unwrap_or(128_000).saturating_sub(model.max_tokens.unwrap_or(input.limits.max_model_output_tokens)) as usize, max_summary_bytes: model.max_tokens.unwrap_or(input.limits.max_model_output_tokens) as usize, max_model_calls: 16 },
                )?.with_store(host.summaries.clone(), "tron.history.native-v1")?;
                block.reducer = Arc::new(ScopedReduction { inner:Arc::new(compressor), extensions:host.harness.framework_compaction_extensions(&model), limits:input.limits.clone() });
            },
            Strategy::Keep => {},
            _ => return Err(fw::AgentError::InvalidConfiguration("protocol history supports keep or checkpoint; data reducers cannot rewrite tool history".into())),
        }
    }
    configs.insert("current_session_history", block);
    let mut layout = config
        .context
        .providers
        .order
        .iter()
        .map(|id| {
            configs.remove(id.as_str()).ok_or_else(|| {
                fw::AgentError::InvalidConfiguration(format!(
                    "duplicate or unknown context block: {id}"
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if !configs.is_empty() {
        return Err(fw::AgentError::InvalidConfiguration(
            "context layout must include every mandatory and data block exactly once".into(),
        ));
    }
    if directory.is_some() {
        let mut block =
            fw::ContextBlockConfig::new("agent_directory", fw::BlockBudget::tokens(2048));
        block.reducer = Arc::new(agent_directory_context::AgentDirectoryReducer);
        layout.insert(layout.len().saturating_sub(1), block);
    }
    if context.is_owning_dude() {
        for (id,cap) in [("dude_onboarding",1024),("dude_environment",1536),("dude_wakeup",2048)] {
            contributors.push(Arc::new(super::dude_context::DudeContributor {service:host.harness.framework_dudes(),context:context.clone(),id}));
            let mut block=fw::ContextBlockConfig::new(id,fw::BlockBudget::tokens(cap));
            block.reducer=Arc::new(memory::BoundMemoryData::new(|_,_|fw::Message::user("Context data omitted to respect budget.")));
            layout.insert(layout.len().saturating_sub(1),block);
        }
    }
    let base_policy: Arc<dyn fw::ContextBudgetPolicy> = Arc::new(SharedSeedBudget {
        max_tokens: seed_cap,
        working_pool: Some(working_cap),
        knowledge_cap: (working_cap / 2).min(1200),
    });
    let policy: Arc<dyn fw::ContextBudgetPolicy> = if visual_enabled {
        Arc::new(memory::BorrowHistoryForData {
            base: base_policy,
            block_id: "knowledge_base".into(),
            max_tokens: knowledge_cap,
            protected_history_tokens: memory::required_recent_turn_tokens(
                &history.messages,
                input.model,
                input.estimator,
            )
            .await?,
        })
    } else {
        base_policy
    };
    let assembler = fw::DefaultContextAssembler::new(fw::ContextLayout(layout), contributors)?
        .with_policy(policy)
        .with_global_compression(
            config
                .compaction
                .auto_enabled
                .then_some(config.compaction.trigger_ratio_bps.min(10_000) as u16),
            force,
        )?;
    let models = super::context_auxiliary::AuxiliaryAllowance {
        inner: input.models,
        calls,
        limit: config.context.providers.max_auxiliary_calls_per_step,
    };
    let mapped = fw::ContextAssemblyInput {
        instructions: &instructions,
        history: &history,
        models: &models,
        budget,
        ..*input
    };
    assembler.assemble(&mapped).await
}
pub(super) fn block_budget(
    config: &crate::application::short_term_memory::ContextBlockBudgetConfig,
) -> Result<fw::BlockBudget, fw::AgentError> {
    if config.max_ratio_bps.is_some_and(|ratio| ratio > 10_000) {
        return Err(fw::AgentError::InvalidConfiguration(
            "invalid context block percentage".into(),
        ));
    }
    Ok(fw::BlockBudget {
        max_tokens: config.max_tokens.map(|tokens| tokens as usize),
        max_ratio_bps: config.max_ratio_bps.map(|ratio| ratio as u16),
        max_messages: None,
    })
}
fn fingerprint(value: &impl serde::Serialize) -> Result<String, fw::AgentError> {
    serde_json::to_vec(value)
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
        .map_err(|error| fw::AgentError::InvalidConfiguration(error.to_string()))
}
fn compression(
    block: fw::ContextBlockConfig,
    policy: &ContextProviderPolicy,
) -> Result<fw::ContextBlockConfig, fw::AgentError> {
    if policy.trigger_ratio_bps > 10_000
        || policy.target_ratio_bps > policy.trigger_ratio_bps
        || policy.target_ratio_bps == policy.trigger_ratio_bps
    {
        return Err(fw::AgentError::InvalidConfiguration(
            "compression target must be below trigger (hysteresis)".into(),
        ));
    }
    Ok(block
        .compress_at(
            policy.trigger_ratio_bps as u16,
            policy.target_ratio_bps as u16,
        )
        .with_min_compression_gain(policy.min_gain_tokens))
}
fn reducer(
    policy: &ContextProviderPolicy,
    revision: &memory::MemoryRevision,
    source: Arc<dyn memory::MemoryContextRevisionGuard>,
    host: Arc<HostExecution>,
    context: &AgentRunContext,
    limits: &fw::RunLimits,
    extensions: &fw::Extensions,
    main_model: &fw::ModelRef,
) -> Result<Arc<dyn fw::ContextReducer>, fw::AgentError> {
    match policy.strategy {
        Strategy::Keep => Ok(Arc::new(fw::KeepContext)),
        Strategy::Bound => Ok(Arc::new(memory::BoundMemoryData::new(|message, cap| {
            let Ok(native) = messages::decode(message) else {
                return message.clone();
            };
            messages::encode(
                &crate::application::short_term_memory::truncate_checkpoint_message(
                    native,
                    cap.min(u32::MAX as usize) as u32,
                ),
            )
            .unwrap_or_else(|_| message.clone())
        }))),
        Strategy::Semantic => {
            let model = host
                .harness
                .framework_compaction_model_config(&context.model);
            let mut auxiliary_limits = limits.clone();
            auxiliary_limits.max_context_tokens = limits.max_context_tokens.min(
                model
                    .context_window_tokens
                    .unwrap_or(128_000)
                    .saturating_sub(model.max_tokens.unwrap_or(limits.max_model_output_tokens))
                    as usize,
            );
            auxiliary_limits.max_model_output_tokens = limits
                .max_model_output_tokens
                .min(model.max_tokens.unwrap_or(limits.max_model_output_tokens));
            let options = memory::CompressionOptions {
                max_input_tokens: auxiliary_limits.max_context_tokens.max(1),
                max_summary_bytes: auxiliary_limits.max_model_output_tokens as usize,
                max_model_calls: 16,
            };
            // Cache identity follows the actual authorized summary model and settings.
            let model_identity = fingerprint(&(
                &model.model_code,
                &model.model_name,
                &model.base_url,
                &model.api_path,
                model.protocol,
                model.context_window_tokens,
                model.max_tokens,
                model.temperature,
                model.top_p,
            ))?;
            let cache_policy = format!("tron-context-data-v2:{model_identity}");
            let semantic = memory::MemorySemanticDataReducer::new(
                super::model::compaction_model_reference(extensions, main_model)?,
                options,
                revision.clone(),
            )?
            .with_store(host.summaries.clone(), &cache_policy)?
            .with_source_guard(source)
            .with_audit_sink(Arc::new(ReductionAudit {
                host: host.clone(),
                context: context.clone(),
            }));
            Ok(Arc::new(ScopedReduction {
                inner: Arc::new(semantic),
                extensions: host.harness.framework_compaction_extensions(&model),
                limits: auxiliary_limits,
            }))
        }
        Strategy::Checkpoint => Err(fw::AgentError::InvalidConfiguration(
            "checkpoint strategy belongs to builtin history".into(),
        )),
    }
}
struct ScopedReduction {
    inner: Arc<dyn fw::ContextReducer>,
    extensions: fw::Extensions,
    limits: fw::RunLimits,
}
#[async_trait]
impl fw::ContextReducer for ScopedReduction {
    async fn reduce(
        &self,
        input: &fw::ContextReductionInput<'_>,
    ) -> Result<Vec<fw::Message>, fw::AgentError> {
        let mut run = input.run.clone();
        if let Some(model) = self
            .extensions
            .get::<crate::application::agent::runtime::CompactionModelInput>()
        {
            run.extensions.insert((*model).clone());
        }
        self.inner
            .reduce(&fw::ContextReductionInput {
                run: &run,
                limits: &self.limits,
                ..*input
            })
            .await
    }
}
struct LiveSourceGuard {
    host: Arc<HostExecution>,
    context: AgentRunContext,
    id: String,
    policy: ContextProviderPolicy,
}
#[async_trait]
impl memory::MemoryContextRevisionGuard for LiveSourceGuard {
    async fn ensure_current(
        &self,
        expected: &memory::MemoryRevision,
    ) -> Result<(), fw::MemoryError> {
        let host = self.host.clone();
        let context = self.context.clone();
        let id = self.id.clone();
        let policy = self.policy.clone();
        let expected = expected.clone();
        tokio::task::spawn_blocking(move || {
            let (entries, checkpoint_code) = host
                .harness
                .framework_live_memory_source(&context, &id)
                .map_err(|_| fw::MemoryError::Conflict)?;
            let checkpoint = (id == "current_session_checkpoint").then_some(&checkpoint_code);
            let generation =
                fingerprint(&(&id, &entries, Some(&policy), checkpoint, &context.user_code))
                    .map_err(|_| fw::MemoryError::Conflict)?;
            let scope = context.memory_scope(
                &context
                    .short_term_memory
                    .as_ref()
                    .ok_or(fw::MemoryError::Conflict)?
                    .session_code,
            )?;
            if expected.scope == scope && expected.generation == generation {
                Ok(())
            } else {
                Err(fw::MemoryError::Conflict)
            }
        })
        .await
        .map_err(|_| fw::MemoryError::Conflict)?
    }
}

struct ReductionAudit {
    host: Arc<HostExecution>,
    context: AgentRunContext,
}
#[async_trait]
impl memory::MemoryReductionAuditSink for ReductionAudit {
    async fn record(
        &self,
        _run: &fw::RunContext,
        block: &fw::ContextBlock,
        result: &memory::MemoryCompactionResult,
    ) -> Result<(), fw::AgentError> {
        // Reused summary bytes perform zero model work; never duplicate prior usage.
        if result.usage.input_tokens == 0 && result.usage.output_tokens == 0 {
            return Ok(());
        }
        self.host
            .harness
            .framework_memory_reduction_audit(&self.context, block, result)
            .map_err(|error| fw::AgentError::Run(error.to_string()))
    }
}
