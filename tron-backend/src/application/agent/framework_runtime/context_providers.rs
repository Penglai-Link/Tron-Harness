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

#[path = "agent_directory_context.rs"]
mod agent_directory_context;
#[path = "knowledge_base_context.rs"]
mod knowledge_base_context;
#[path = "page_context.rs"]
mod page_context;

pub(super) async fn assemble(
    input: &fw::ContextAssemblyInput<'_>,
    host: Arc<HostExecution>,
    context: &AgentRunContext,
    projection: &PreparedModelContext,
    force: bool,
    calls: Arc<std::sync::atomic::AtomicUsize>,
    onboarding: bool,
) -> Result<fw::ContextWindow, fw::AgentError> {
    let config = host.harness.framework_context_config();
    config
        .context
        .providers
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
            .take_while(|message| messages::is_prompt_prefix(message))
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
    let contains_context_data = |message: &crate::domain::agent::AgentMessage| {
        message
            .parts
            .iter()
            .any(|part| matches!(part, crate::domain::agent::AgentMessagePart::Context { .. }))
    };
    let mut instructions = source
        .prefix
        .iter()
        .filter(|m| m.role == "system" && !contains_context_data(m))
        .map(messages::encode)
        .collect::<Result<Vec<_>, _>>()
        .map_err(fw::AgentError::InvalidConfiguration)?;
    for instruction in input.instructions {
        if messages::decode(instruction).is_ok_and(|message| contains_context_data(&message)) {
            continue;
        }
        if !instructions.contains(instruction) {
            instructions.push(instruction.clone());
        }
    }
    let is_tool_domains = |message: &crate::domain::agent::AgentMessage| {
        message.parts.iter().any(|part| matches!(part, crate::domain::agent::AgentMessagePart::Context { kind, .. } if kind == "tool_domains"))
    };
    let tool_domains = source
        .prefix
        .iter()
        .filter(|m| !onboarding && is_tool_domains(m))
        .cloned()
        .collect::<Vec<_>>();
    let mut application = source
        .prefix
        .iter()
        .filter(|m| !is_tool_domains(m) && (m.role != "system" || contains_context_data(m)))
        .map(|message| {
            let mut message = message.clone();
            message.role = "user".into();
            message
        })
        .collect::<Vec<_>>();
    // Execution scope is independent of project instructions and other context.
    // Refresh it at every boundary, even when another application entry exists.
    application.retain(|message| !message.parts.iter().any(|part| matches!(part,
        crate::domain::agent::AgentMessagePart::Context { kind, .. } if kind == "application_execution_context"
    )));
    if let Some(mut scope) =
        crate::application::agent::execution_context_message(&context.execution_scope)
    {
        scope.role = "user".into();
        application.push(scope);
    }
    let mut history = input.history.clone();
    let page = page_context::project(&mut history)?;
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
    fw::configured_block_budget(&seed, available)?;
    let knowledge_cap = config.context.providers.data_blocks["knowledge_base"].max_tokens.map_or(available.max_tokens, |n| n as usize);
    let mut contributors: Vec<Arc<dyn fw::ContextContributor>> = Vec::new();
    contributors.push(Arc::new(knowledge_base_context::KnowledgeBaseContributor {
        service: (!onboarding)
            .then(|| host.harness.framework_knowledge_bases())
            .flatten(),
        user: context.user_code.clone(),
        workspace: context.workspace_code.clone(),
        agent: context.agent_code.clone(),
        allowance: crate::application::llm::ContextQueryAllowance {
            calls: calls.clone(),
            limit: config.context.providers.max_auxiliary_calls_per_step,
        },
    }));
    if onboarding {
        instructions.clear();
    }
    if context.is_owning_dude() {
        contributors.push(Arc::new(super::dude_context::DudeGuidance {
            locale: context.locale.clone(),
            onboarding,
        }));
    }
    let mut configs: std::collections::HashMap<String, fw::ContextBlockConfig> =
        std::collections::HashMap::new();
    let directory = input
        .run
        .extensions
        .get::<super::host::collaboration::Runtime>();
    if let Some(runtime) = directory.as_ref() {
        let key = "agent.directory-guidance";
        let fallback = crate::domain::prompt::builtin_prompt_text(key, &context.locale)
            .ok_or_else(|| {
                fw::AgentError::InvalidConfiguration(format!(
                    "bundled developer prompt is missing: {key}"
                ))
            })?;
        let guidance = fw::ManagedDeveloperPrompts::resolve(input.run, key, fallback)?;
        contributors.push(Arc::new(fw::FixedInstructions::new(
            "agent.directory-guidance",
            guidance,
        )));
        contributors.push(Arc::new(
            agent_directory_context::AgentDirectoryContributor((**runtime).clone()),
        ));
    }
    configs.insert(
        "seed_prompt_prefix".into(),
        seed,
    );
    for (id, entries, cap, policy) in [
        (
            crate::application::agent::page_context::PAGE_CONTEXT_BLOCK,
            &page,
            fw::BlockBudget::tokens(available.max_tokens),
            None,
        ),
        (
            "tool_domains",
            &tool_domains,
            fw::BlockBudget::tokens(
                config.context.providers.data_blocks["tool_domains"].max_tokens.map_or(available.max_tokens, |n| n as usize),
            ),
            None,
        ),
        (
            "application_context",
            &application,
            fw::BlockBudget::tokens(
                config.context.providers.data_blocks["application_context"].max_tokens.map_or(available.max_tokens, |n| n as usize),
            ),
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
            // The history coordinator writes the summary once under this block's
            // own budget; a generic data reducer must not truncate it separately.
            if id == "current_session_checkpoint" {
                if policy.strategy != Strategy::Keep { block = compression(block, policy)?; }
                configs.insert(id.into(), block);
                continue;
            }
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
        configs.insert(id.into(), block);
    }
    let knowledge =
        fw::ContextBlockConfig::new("knowledge_base", fw::BlockBudget::tokens(knowledge_cap));
    configs.insert("knowledge_base".into(), knowledge);
    let mut block = fw::ContextBlockConfig::new(
        fw::HISTORY_BLOCK,
        fw::BlockBudget::tokens(plan.effective_input_budget as usize),
    );
    if config.context.providers.history.strategy != Strategy::Keep {
        if config.compaction.auto_enabled || force { block = compression(block, &config.context.providers.history)?; }
        match config.context.providers.history.strategy {
            Strategy::Checkpoint => {
                let model = host.harness.framework_compaction_model_config(&context.model);
                let compressor = memory::ModelCompressor::new(
                    super::model::compaction_model_reference(&input.run.extensions,input.model)?,
                    memory::CompressionOptions { max_input_tokens: model.context_window_tokens.unwrap_or(128_000).saturating_sub(model.max_tokens.unwrap_or(input.limits.max_model_output_tokens)) as usize, max_summary_bytes: memory::summary_byte_guard(plan.block_budgets.current_session_checkpoint.effective_limit_tokens as usize), max_model_calls: 16 },
                )?.with_store(host.summaries.clone(), "tron.history.native-v1")?;
                block.reducer = Arc::new(ScopedReduction { inner:Arc::new(compressor), extensions:host.harness.framework_compaction_extensions(&model), limits:input.limits.clone() });
            },
            Strategy::Keep => {},
            _ => return Err(fw::AgentError::InvalidConfiguration("protocol history supports keep or checkpoint; data reducers cannot rewrite tool history".into())),
        }
    }
    configs.insert("current_session_history".into(), block);
    if directory.is_some() {
        let block = fw::ContextBlockConfig::new(
            "agent_directory",
            fw::BlockBudget::tokens(
                config.context.providers.data_blocks["agent_directory"].max_tokens.map_or(available.max_tokens, |n| n as usize),
            ),
        );
        configs.insert("agent_directory".into(), block);
    }
    if context.is_owning_dude() {
        for id in [
            "runtime_identity",
            "dude_onboarding",
            "dude_environment",
            "dude_wakeup",
        ] {
            let cap = config.context.providers.data_blocks[id].max_tokens.map_or(available.max_tokens, |n| n as usize);
            contributors.push(Arc::new(super::dude_context::DudeContributor {
                service: host.harness.framework_dudes(),
                context: context.clone(),
                id,
            }));
            let block = fw::ContextBlockConfig::new(id, fw::BlockBudget::tokens(cap));
            configs.insert(id.into(), block);
        }
    }
    if let Some(services) = input.run.extensions.get::<fw::ScopedAgentServices>() {
        for source in services
            .0
            .context_sources
            .iter()
            .filter(|source| source.kind() == fw::ContextSourceKind::Data)
        {
            if onboarding {
                continue;
            }
            let id = source.block_id();
            let policy = config
                .context
                .providers
                .data_blocks
                .get(id)
                .ok_or_else(|| {
                    fw::AgentError::InvalidConfiguration(format!(
                        "unconfigured context contributor: {id}"
                    ))
                })?;
            if configs
                .insert(
                    id.into(),
                    fw::ContextBlockConfig::new(
                        id,
                        fw::BlockBudget::tokens(policy.max_tokens.map_or(available.max_tokens, |n| n as usize)),
                    ),
                )
                .is_some()
            {
                return Err(fw::AgentError::InvalidConfiguration(format!(
                    "duplicate context contributor: {id}"
                )));
            }
            contributors.push(source.clone());
        }
    }
    for (id, policy) in &config.context.providers.data_blocks {
        if let Some(block) = configs.get_mut(id.as_str()) {
            block.budget = policy.max_tokens.map_or_else(|| fw::BlockBudget::ratio(10_000), |n| fw::BlockBudget::tokens(n as usize));
            if policy.policy.strategy == Strategy::Bound {
                *block = compression(block.clone(), &policy.policy)?;
                block.reducer = bound_data_reducer(policy.max_tokens.map_or(available.max_tokens, |n| n as usize));
            } else {
                block.reducer = Arc::new(fw::KeepContext);
                block.compression = None;
            }
        }
    }
    let layout = config
        .context
        .providers
        .order
        .iter()
        .filter_map(|id| configs.remove(id.as_str()))
        .collect::<Vec<_>>();
    if !configs.is_empty() {
        return Err(fw::AgentError::InvalidConfiguration(
            "every active contributor must occur in the configured layout".into(),
        ));
    }
    let policy: Arc<dyn fw::ContextBudgetPolicy> = Arc::new(memory::BorrowUnusedMemoryBudget);
    let assembler = fw::DefaultContextAssembler::new(fw::ContextLayout(layout), contributors)?;
    let assembler = if config.context.providers.history.strategy == Strategy::Checkpoint {
        assembler.with_history_summary_block("current_session_checkpoint", config.context.providers.history_recent_turns)?
    } else { assembler };
    let assembler = assembler
        .with_policy(policy)
        .with_reduction_cache(Arc::new(memory::MemoryContextReductionCache::new(host.summaries.clone(), fingerprint(&("tron.context-reducers.v2", &config.context.providers))?)))
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
        Strategy::Bound => Ok(bound_memory_reducer()),
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
                max_summary_bytes: memory::summary_byte_guard(
                    limits.max_context_tokens
                ),
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
fn bound_data_reducer(max_tokens: usize) -> Arc<dyn fw::ContextReducer> {
    Arc::new(HostDataBounding(memory::BoundMultimodalData {
        max_media: usize::MAX,
        text_tokens: max_tokens,
    }))
}

fn bound_memory_reducer() -> Arc<dyn fw::ContextReducer> {
    Arc::new(memory::BoundMemoryData::new(|message, cap| {
        let Ok(native) = messages::decode(message) else {
            return message.clone();
        };
        messages::encode(
            &crate::application::short_term_memory::truncate_checkpoint_message(
                native,
                cap.min(u32::MAX as usize) as u32,
            ),
        ).unwrap_or_else(|_| message.clone())
    }))
}

#[cfg(test)]
#[path = "context_reduction_tests.rs"]
mod reduction_tests;

/// Resolve the host codec before a generic reducer turns JSON into prose.
/// Reserved transport records must never become model-visible text.
struct HostDataBounding(memory::BoundMultimodalData);
#[async_trait]
impl fw::ContextReducer for HostDataBounding {
    async fn reduce(
        &self,
        input: &fw::ContextReductionInput<'_>,
    ) -> Result<Vec<fw::Message>, fw::AgentError> {
        // A bounded policy is not itself a request to rewrite data. Preserve
        // fitting host records, including their preview metadata, unless the
        // unified policy actually asks for compression. Media still requires
        // the admission checks below even when its estimated cost fits.
        if input.compression.is_none()
            && input.block.messages.len() <= input.budget.max_messages
            && !input.block.messages.iter().any(|message| message.content.has_media())
            && fw::context_tokens(&input.block.messages, input.model, input.estimator).await?
                <= input.budget.max_tokens.min(self.0.text_tokens)
        {
            return Ok(input.block.messages.clone());
        }
        let mut block = input.block.clone();
        for message in &mut block.messages {
            for content in &mut message.content.0 {
                if let fw::ContentBlock::Json { value } = content
                    && let Some(part) = value.get("tronMessagePart")
                {
                    let part: crate::domain::agent::AgentMessagePart =
                        serde_json::from_value(part.clone()).map_err(|error| {
                            fw::AgentError::InvalidConfiguration(error.to_string())
                        })?;
                    if let crate::domain::agent::AgentMessagePart::Context { kind, text, .. } = part {
                        *content = fw::ContentBlock::Text {
                            text: format!("[Runtime reference data: {kind}]\n{text}"),
                        };
                    } else {
                        return Err(fw::AgentError::InvalidConfiguration(
                            "runtime data bounding received a non-context host record".into(),
                        ));
                    }
                }
            }
        }
        self.0.reduce(&fw::ContextReductionInput { block: &block, ..*input }).await
    }
}

struct ScopedReduction {
    inner: Arc<dyn fw::ContextReducer>,
    extensions: fw::Extensions,
    limits: fw::RunLimits,
}
#[async_trait]
impl fw::ContextReducer for ScopedReduction {
    async fn project_history(&self, input: &fw::ContextReductionInput<'_>, summary_budget: fw::ContextBudget, recent_turns: usize, cached_only: bool) -> Result<Option<fw::HistoryProjection>, fw::AgentError> {
        let mut run = input.run.clone();
        if let Some(model) = self.extensions.get::<crate::application::agent::runtime::CompactionModelInput>() {
            run.extensions.insert((*model).clone());
        }
        self.inner.project_history(&fw::ContextReductionInput { run: &run, limits: &self.limits, ..*input }, summary_budget, recent_turns, cached_only).await
    }
    async fn cached_projection(&self, input: &fw::ContextReductionInput<'_>) -> Result<Option<Vec<fw::Message>>, fw::AgentError> {
        self.inner.cached_projection(input).await
    }
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
