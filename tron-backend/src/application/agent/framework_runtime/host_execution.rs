//! Native preparation and execution use one SDK owner through product completion.
use super::*;
impl FrameworkHost {
    pub(super) async fn run_inner(
        &self,
        harness: ReactAgent,
        mut context: AgentRunContext,
        sink: Option<ModelStreamSink>,
        steering: Option<ActiveRunControl>,
        registration: host_control::NativeRegistration,
    ) -> Result<AgentRunResult, AgentError> {
        collaboration::authorize_tools(self, &mut context);
        let admission = self.0.config_lock.lock().await;
        self.ensure_open()
            .map_err(AgentError::InvalidConfiguration)?;
        registration.ensure_active()?;
        crate::adapters::outbound::persistence::sqlite::SqliteChatRepository::ensure_run_session(
            self, &context,
        )
        .await
        .map_err(|e| AgentError::InvalidConfiguration(e.to_string()))?;
        let mut lifecycle = host_run::ProductLifecycle::start(&harness, &context)?;
        let agent = if let Some(code) = context.agent_code.as_deref() {
            self.internal_agent(&context.workspace_code, code)
                .await
                .map_err(map_error)?
        } else {
            self.agent().await.map_err(map_error)?.clone()
        };
        let session = context
            .conversation_code
            .clone()
            .unwrap_or_else(|| context.run_code.clone());
        let memory_input = context
            .short_term_memory
            .as_ref()
            .and_then(|memory| memory.current_input_message.as_ref());
        let (history, current) = if let Some(input) = memory_input {
            let history = if context.messages.last().is_some_and(|last| last == input) {
                &context.messages[..context.messages.len() - 1]
            } else {
                &context.messages[..]
            };
            (history, input)
        } else {
            let current = context
                .messages
                .iter()
                .rposition(|message| message.role == "user")
                .ok_or_else(|| {
                    AgentError::InvalidConfiguration("run requires a current user message".into())
                })?;
            if current + 1 != context.messages.len() {
                return Err(AgentError::InvalidConfiguration(
                    "current input must follow the prepared conversation history".into(),
                ));
            }
            (&context.messages[..current], &context.messages[current])
        };
        let prefix_len = context.short_term_memory.as_ref().map_or_else(
            || {
                history
                    .iter()
                    .take_while(|message| matches!(message.role.as_str(), "system" | "developer"))
                    .count()
            },
            |memory| memory.prompt_prefix.len().min(history.len()),
        );
        let history = super::super::messages::encode_history(&history[prefix_len..])
            .map_err(AgentError::InvalidConfiguration)?;
        let input = super::super::messages::encode(current)
            .map_err(AgentError::InvalidConfiguration)?
            .content;
        let answers = host_interactions::answers_from_parts(&current.parts)?;
        let a2ui_options = current
            .parts
            .iter()
            .find_map(|part| {
                if let crate::domain::agent::AgentMessagePart::Context {
                    kind,
                    metadata: Some(metadata),
                    ..
                } = part
                {
                    if kind == "tron_interaction_response" {
                        return metadata.get("a2uiClientCapabilities").and_then(|v| {
                            serde_json::from_value::<
                                eliteforge_agent_official_plugin_a2ui::A2uiClientCapabilities,
                            >(v.clone())
                            .ok()
                        });
                    }
                }
                None
            })
            .filter(|c| {
                c.version == eliteforge_agent_official_plugin_a2ui::VERSION
                    && c.catalog_ids.iter().any(|id| {
                        [
                            eliteforge_agent_official_plugin_a2ui::AGENT_CATALOG_ID,
                            eliteforge_agent_official_plugin_a2ui::MINIMAL_CATALOG_ID,
                        ]
                        .contains(&id.as_str())
                    })
            });
        let a2ui_options = a2ui_options.filter(|_| {
            self.0
                .tools_enabled
                .load(std::sync::atomic::Ordering::Acquire)
                && self
                    .capability_state()
                    .0
                    .get(eliteforge_agent_official_plugin_a2ui::A2UI_PACK_ID)
                    .copied()
                    .unwrap_or(true)
        });
        let key = fw::ConversationKey::new(&session);
        let snapshot = agent.conversation(&key).await.map_err(map_error)?;
        if snapshot.revision == 0 && !history.is_empty() {
            agent
                .append_conversation(&key, 0, history)
                .await
                .map_err(map_error)?;
        }
        let mut request = fw::RunRequest::new(&session, input);
        // Resolve the display language per run, including approval continuations.
        request.extensions.insert(
            match crate::shared::locale::SupportedLocale::from_raw(&context.locale) {
                crate::shared::locale::SupportedLocale::EnUs => {
                    tron_harness_capability::TronToolPolicyLocale::EnUs
                }
                crate::shared::locale::SupportedLocale::ZhCn => {
                    tron_harness_capability::TronToolPolicyLocale::ZhCn
                }
            },
        );
        if let Some(capabilities) = a2ui_options {
            request
                .extensions
                .insert(eliteforge_agent_official_plugin_a2ui::A2uiRunOptions {
                    capabilities,
                    owner_agent_id: context
                        .agent_code
                        .clone()
                        .unwrap_or_else(|| "main-agent".into()),
                });
        }
        request.run_id = Some(fw::RunId::parse_str(&context.run_code).map_err(|e| {
            AgentError::InvalidConfiguration(format!("invalid native run identity: {e}"))
        })?);
        let framework = self.framework().await.map_err(map_error)?;
        request.goal_id = framework
            .goals()
            .map_err(map_error)?
            .active(&session)
            .await
            .map_err(|e| map_error(e.into()))?
            .map(|g| g.id);
        self.shared_database()
            .await
            .map_err(|e| map_error(e.into()))?
            .transaction(async {
                host_interactions::apply_answers(&agent, &session, &answers, &mut request)
                    .await
                    .map_err(|e| fw::StateError::Backend(e.to_string()))
            })
            .await
            .map_err(|e| map_error(e.into()))?;
        if request.resume.is_none() {
            if let Some(management) = self.0.agent_management.get() {
                let policy = management
                    .workspace_internal_agent_policy(
                        &context.workspace_code,
                        context.agent_code.as_deref().unwrap_or("main-agent"),
                    )
                    .map_err(|e| AgentError::InvalidConfiguration(format!("{e:?}")))?;
                let design = management
                    .loop_design(policy.loop_code.as_deref().ok_or_else(|| {
                        AgentError::InvalidConfiguration("agent loop is not configured".into())
                    })?)
                    .map_err(|e| AgentError::InvalidConfiguration(format!("{e:?}")))?;
                request.extensions.insert(design);
            }
        }
        request.limits.max_steps = context.limits.max_steps as usize;
        request.limits.max_tool_calls = context.limits.max_tool_calls as usize;
        request.limits.tool_batch_budget_policy = fw::ToolBatchBudgetPolicy::ExecuteAvailable;
        // Domain discovery controls schema exposure; the effective catalog owns authority.
        request.limits.allow_unexposed_tools = true;
        request.limits.max_parallel_tools = context.limits.max_parallel_tool_calls.max(1) as usize;
        request.limits.max_context_messages = context
            .messages
            .len()
            .max(256)
            .saturating_add(request.limits.max_steps + request.limits.max_tool_calls * 2);
        request.limits.max_context_tokens =
            context.model.context_window_tokens.unwrap_or(128_000) as usize;
        request.limits.max_model_output_tokens = context
            .model
            .max_tokens
            .filter(|value| *value > 0)
            .unwrap_or_else(|| self.invocation_config().default_output_tokens);
        request.limits.max_total_tokens = (request.limits.max_context_tokens as u64
            + u64::from(request.limits.max_model_output_tokens))
        .saturating_mul(request.limits.max_steps as u64);
        let invocation = self.invocation_config();
        request.limits.model_timeout = Duration::from_secs(invocation.agent_model_timeout_seconds);
        request.limits.run_timeout = Duration::from_secs(
            invocation
                .agent_step_timeout_seconds
                .saturating_mul(context.limits.max_steps.max(1) as u64),
        );
        request.limits.max_model_retries = if context.auto_model_route.is_some() {
            2
        } else {
            context.model.max_retries.unwrap_or(0).min(3) as usize
        };
        request.limits.max_model_calls_per_step =
            Some((request.limits.max_model_retries + 1).max(3));
        request.limits.max_overflow_recoveries = harness.framework_overflow_limit();
        request.generation.temperature = context.model.temperature;
        request.generation.top_p = context.model.top_p;
        request.generation.max_output_tokens = context.model.max_tokens.filter(|value| *value > 0);
        let state = Arc::new(Mutex::new(HostRunState::new(context.clone())));
        request
            .extensions
            .insert(fw::ToolResultProcessorService(Arc::new(ToolAudit)));
        request.extensions.insert(
            eliteforge_agent_official_plugin_tools_local::CommandProgressSinkService(Arc::new(
                super::super::tools::CommandProgress {
                    harness: harness.clone(),
                    context: context.clone(),
                },
            )),
        );
        if let Some(skills) = self.0.skill_registry.get() {
            let agent = context.agent_code.as_deref().unwrap_or("main-agent");
            let names = skills
                .effective_agent_skills(agent)
                .map_err(|e| AgentError::InvalidConfiguration(e.to_string()))?
                .into_iter()
                .filter(|s| s.assigned && s.available)
                .map(|s| s.skill_name);
            request.extensions.insert(fw::SkillAccess {
                names: names.collect(),
            });
            request
                .extensions
                .insert(crate::application::skill::SkillAgentIdentity(agent.into()));
        }
        request.extensions.insert(HostExecution {
            harness: harness.clone(),
            state: state.clone(),
            summaries: self.summary_store().await.map_err(map_error)?,
            sink,
        });
        tron_harness_capability::TronRunScope {
            user_code: context.user_code.clone().unwrap_or_else(|| "system".into()),
            workspace_code: context.workspace_code.clone(),
            session_code: session,
            run_code: context.run_code.clone(),
            app_code: context.execution_scope.app_code.clone(),
            task_code: context.execution_scope.task_code.clone(),
            agent_code: context.agent_code.clone().or_else(|| {
                context
                    .short_term_memory
                    .as_ref()
                    .and_then(|memory| memory.agent_code.clone())
            }),
        }
        .attach(&mut request)
        .map_err(map_error)?;
        if let Some(resources) = &self.0.resources {
            request
                .extensions
                .insert(FileReferenceReaderService(Arc::new(AttachmentReader(
                    resources.clone(),
                ))));
        }
        if let Some(control) = steering {
            request
                .extensions
                .insert(fw::LiveInputSourceService(Arc::new(HostSteering {
                    control,
                })));
        }
        collaboration::attach(self, &context, &mut request);
        harness
            .framework_model_scope(&context)?
            .attach(&mut request);
        self.ensure_open()
            .map_err(AgentError::InvalidConfiguration)?;
        let trace_guard = self
            .telemetry()
            .await
            .map_err(|_| AgentError::InvalidConfiguration("telemetry unavailable".into()))?
            .begin(&context, &mut request)
            .map_err(|_| AgentError::InvalidConfiguration("trace registration failed".into()))?;
        let parent = eliteforge_agent_framework::current_continuation_parent();
        let mut result = if let Some(parent) = parent {
            let started = agent
                .start_child_with_output_owned::<eliteforge_agent_official_plugin_agui::AgUiStream>(
                    request, &parent,
                )
                .await;
            drop(admission);
            match started {
                Ok((handle, stream)) => {
                    self.wait_native_child(
                        agent.clone(),
                        handle,
                        stream,
                        &context,
                        registration.clone(),
                    )
                    .await
                }
                Err(error) => Err(error),
            }
        } else {
            let started = agent
                .start_with_output::<eliteforge_agent_official_plugin_agui::AgUiStream>(request);
            drop(admission);
            match started {
                Ok((handle, stream)) => {
                    self.wait_native(
                        agent.clone(),
                        handle,
                        stream,
                        &context,
                        registration.clone(),
                    )
                    .await
                }
                Err(error) => Err(error),
            }
        };
        trace_guard.finish(&result);
        drop(trace_guard);
        // Multiple terminal DAG nodes produce structured JSON rather than text.
        // Keep every result visible in Tron's text-oriented run/history contract.
        if let Ok(result) = &mut result {
            if result.status == fw::RunStatus::Completed && result.output.is_empty() {
                if let Some(value) = &result.structured {
                    result.output = serde_json::to_string_pretty(value)
                        .map_err(|error| AgentError::InvalidConfiguration(error.to_string()))?;
                }
            }
        }
        let state = state.lock().await;
        let failure = match &result {
            Err(error) => Some(match error.primary() {
                fw::AgentError::SessionBusy(_) | fw::AgentError::Cancelled => map_error_ref(error),
                _ if matches!(error, fw::AgentError::WithCleanup { .. }) => map_error_ref(error),
                _ => state.error.clone().unwrap_or_else(|| map_error_ref(error)),
            }),
            _ => None,
        };
        if let Some(error) = failure {
            lifecycle.finish(&state.context, &Err(fw::AgentError::Run(error.to_string())))?;
            return Err(error);
        }
        lifecycle.finish(&state.context, &result)?;
        let result = result.map_err(map_error)?;
        let output = result.output.clone();
        Ok(AgentRunResult {
            run_code: context.run_code,
            conversation_code: context.conversation_code,
            workspace_code: context.workspace_code,
            final_output: output,
            step_count: result.steps as u32,
            token_usage: state.usage,
            effective_model: Some(state.context.model.clone()),
            latest_request_estimate: state.estimate.clone(),
            latest_usage_audit: state.audit.clone(),
            system_agent_session_id: state.context.system_agent_session_id.clone(),
            observations: state.context.observations.clone(),
            native: Some(host_control::native_outcome(
                registration.run_id(),
                &result,
            )?),
            sdk_execution: Some(host_snapshot::product_snapshot(
                registration.snapshot(&result).map_err(map_error)?,
            )?),
        })
    }
}
