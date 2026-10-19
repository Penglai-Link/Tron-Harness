use crate::{
    application::{model::ModelService, model_provider::ModelProviderService},
    domain::{
        agent::{AgentModelConfig, AutoModelCandidate, AutoModelRouteContext},
        model_provider::ModelSelection,
        reasoning::ReasoningEffort,
    },
};
use eliteforge_agent_capability_personai::{
    PERSONAI_PROVIDER, PERSONAI_QUALITY, PersonAiAutoRouter,
};
use eliteforge_agent_core as fw;
use std::sync::Arc;

#[derive(Debug, Clone)]
struct Candidate {
    code: String,
    selection: ModelSelection,
    config: AgentModelConfig,
    key: String,
    provider: bool,
}

#[derive(Clone)]
pub struct ChatAutoModelRouter {
    models: ModelService,
    providers: ModelProviderService,
    router: Arc<dyn fw::ChatAutoRouter>,
}

impl ChatAutoModelRouter {
    pub fn new(models: ModelService, providers: ModelProviderService) -> Self {
        Self {
            models,
            providers,
            router: Arc::new(PersonAiAutoRouter {
                prefer_responses: true,
                ..Default::default()
            }),
        }
    }

    pub fn with_router(mut self, router: Arc<dyn fw::ChatAutoRouter>) -> Self {
        self.router = router;
        self
    }

    pub fn resolve_initial(
        &self,
        workspace_code: &str,
        session_code: &str,
        agent_code: &str,
        reasoning_effort: Option<ReasoningEffort>,
    ) -> Result<(AgentModelConfig, AutoModelRouteContext), String> {
        self.resolve_initial_with_candidates(
            workspace_code,
            session_code,
            agent_code,
            reasoning_effort,
            &[],
        )
    }

    pub fn resolve_initial_with_candidates(
        &self,
        workspace_code: &str,
        session_code: &str,
        agent_code: &str,
        reasoning_effort: Option<ReasoningEffort>,
        allowed: &[String],
    ) -> Result<(AgentModelConfig, AutoModelRouteContext), String> {
        let initial_scope = self.providers.sdk_model_authorization_scope();
        let candidates = self.candidates(workspace_code, reasoning_effort, allowed)?;
        let authorization_scope = if candidates.iter().any(|candidate| candidate.provider) {
            let scope = initial_scope.map_err(|error| error.to_string())?;
            if self
                .providers
                .sdk_model_authorization_scope()
                .map_err(|error| error.to_string())?
                != scope
            {
                return Err("Auto authorization changed during initial selection".into());
            }
            Some(scope)
        } else {
            None
        };
        let assigned = self
            .providers
            .chat_auto_route_selection(workspace_code, session_code, agent_code)
            .map_err(|error| error.to_string())?;
        let available = candidates.iter().map(route_candidate).collect::<Vec<_>>();
        let previous = assigned
            .and_then(|selection| candidates.iter().find(|c| c.selection == selection))
            .map(|c| route_candidate(c).model);
        let affinity = serde_json::to_string(&(workspace_code, session_code, agent_code))
            .map_err(|error| error.to_string())?;
        let routers = fw::AutoRouters {
            source: Some(Arc::new(
                crate::application::model_router::TronAutoRouteDataSource(available.clone()),
            )),
            chat: Some(self.router.clone()),
            ..Default::default()
        };
        let selected = routers
            .select(
                &fw::AutoRouteContext {
                    operation: fw::ModelOperation::Chat,
                    stage: "",
                    affinity: &affinity,
                    call_key: "initial",
                    previous: previous.as_ref(),
                    transient_failures: 0,
                },
                &available,
            )
            .map_err(|error| error.to_string())?;
        let candidate = candidates
            .iter()
            .find(|candidate| route_candidate(candidate).model == selected)
            .cloned()
            .ok_or_else(|| "对话自动路由 selected an unauthorized model".to_string())?;
        self.providers
            .save_chat_auto_route_selection(
                workspace_code,
                session_code,
                agent_code,
                &candidate.selection,
                &candidate.config,
            )
            .map_err(|error| error.to_string())?;
        Ok((
            candidate.config.clone(),
            AutoModelRouteContext {
                candidate_model_codes: allowed.to_vec(),
                authorization_scope,
                authorized_candidates: candidates
                    .iter()
                    .map(|candidate| AutoModelCandidate {
                        selection: candidate.selection.clone(),
                        config: candidate.config.clone(),
                    })
                    .collect(),
                session_code: session_code.to_string(),
                agent_code: agent_code.to_string(),
                current_candidate_key: candidate.key,
                failed_candidate_keys: Vec::new(),
                fallback_attempts: 0,
            },
        ))
    }

    /// Authorization and ordering only. SDK selection/recovery owns attempts.
    pub fn authorized_models(
        &self,
        workspace: &str,
        current: &AgentModelConfig,
    ) -> Result<Vec<AgentModelConfig>, String> {
        self.authorized_models_with_candidates(workspace, current, &[])
    }

    pub fn authorized_models_with_candidates(
        &self,
        workspace: &str,
        current: &AgentModelConfig,
        allowed: &[String],
    ) -> Result<Vec<AgentModelConfig>, String> {
        let candidates = self
            .candidates(workspace, current.reasoning_effort, allowed)?
            .into_iter()
            .map(|candidate| AutoModelCandidate {
                selection: candidate.selection,
                config: candidate.config,
            })
            .collect();
        Ok(order_authorized_candidates(candidates, current))
    }

    pub fn snapshot_models(
        &self,
        route: &AutoModelRouteContext,
        current: &AgentModelConfig,
    ) -> Result<Vec<AgentModelConfig>, String> {
        if let Some(expected) = &route.authorization_scope {
            let actual = self
                .providers
                .sdk_model_authorization_scope()
                .map_err(|error| error.to_string())?;
            if &actual != expected {
                return Err("Auto authorization changed after initial selection".into());
            }
        }
        Ok(order_authorized_candidates(
            route.authorized_candidates.clone(),
            current,
        ))
    }

    pub fn mark_success(
        &self,
        workspace_code: &str,
        route: &mut AutoModelRouteContext,
        model: &AgentModelConfig,
    ) -> Result<(), String> {
        let candidates = if route.authorized_candidates.is_empty() {
            self.candidates(
                workspace_code,
                model.reasoning_effort,
                &route.candidate_model_codes,
            )?
            .into_iter()
            .map(|candidate| AutoModelCandidate {
                selection: candidate.selection,
                config: candidate.config,
            })
            .collect()
        } else {
            route.authorized_candidates.clone()
        };
        let selection = candidates
            .into_iter()
            .find(|candidate| candidate.config.identity() == model.identity())
            .map(|candidate| candidate.selection)
            .ok_or_else(|| "successful Auto model is no longer available".to_string())?;
        self.providers
            .save_chat_auto_route_selection(
                workspace_code,
                &route.session_code,
                &route.agent_code,
                &selection,
                model,
            )
            .map_err(|error| error.to_string())?;
        route.failed_candidate_keys.clear();
        route.fallback_attempts = 0;
        Ok(())
    }

    fn candidates(
        &self,
        workspace_code: &str,
        reasoning_effort: Option<ReasoningEffort>,
        allowed: &[String],
    ) -> Result<Vec<Candidate>, String> {
        let mut candidates = Vec::new();
        // An unavailable provider is represented by no enabled options.
        // A failed catalog read must retain its cause, not masquerade as revocation.
        for (code, selection, config) in self
            .providers
            .auto_chat_candidates(allowed, reasoning_effort)
            .map_err(|error| format!("Auto provider catalog lookup failed: {error}"))?
        {
            candidates.push(Candidate {
                code,
                key: candidate_key(&selection),
                selection,
                config,
                provider: true,
            });
        }
        for config in self
            .models
            .auto_chat_candidates(workspace_code, allowed, reasoning_effort)
            .map_err(|error| error.to_string())?
        {
            let selection = ModelSelection::fixed(config.model_code.clone());
            candidates.push(Candidate {
                code: config.model_code.clone(),
                key: candidate_key(&selection),
                selection,
                config,
                provider: false,
            });
        }
        candidates.retain(|candidate| allowed.is_empty() || allowed.contains(&candidate.code));
        let filtered = self
            .router
            .candidates(&candidates.iter().map(route_candidate).collect::<Vec<_>>());
        let filtered = filtered
            .into_iter()
            .map(|c| c.model)
            .collect::<std::collections::BTreeSet<_>>();
        candidates
            .retain(|candidate| filtered.contains(&fw::ModelRef::new("tron.chat", &candidate.key)));
        Ok(candidates)
    }
}

fn order_authorized_candidates(
    candidates: Vec<AutoModelCandidate>,
    current: &AgentModelConfig,
) -> Vec<AgentModelConfig> {
    let data = candidates
        .iter()
        .map(|candidate| {
            route_candidate(&Candidate {
                code: candidate.config.model_code.clone(),
                key: candidate_key(&candidate.selection),
                provider: candidate.selection.provider_code.is_some(),
                selection: candidate.selection.clone(),
                config: candidate.config.clone(),
            })
        })
        .collect::<Vec<_>>();
    let current_ref = candidates
        .iter()
        .zip(&data)
        .find(|(candidate, _)| candidate.config.identity() == current.identity())
        .map(|(_, candidate)| candidate.model.clone())
        .unwrap_or_else(|| fw::ModelRef::new("tron.chat", ""));
    let ordered = PersonAiAutoRouter::recovery_candidates(
        &data,
        &current_ref,
        current.context_window_tokens.unwrap_or_default() as usize,
    );
    let mut configs = candidates
        .into_iter()
        .zip(data)
        .map(|(candidate, data)| (data.model, candidate.config))
        .collect::<std::collections::BTreeMap<_, _>>();
    ordered
        .into_iter()
        .filter_map(|entry| configs.remove(&entry.model))
        .collect()
}

fn route_candidate(candidate: &Candidate) -> fw::AutoRouteCandidate {
    let mut data = fw::AutoRouteCandidate::new(fw::ModelRef::new("tron.chat", &candidate.key));
    data.protocol = match candidate.config.protocol {
        crate::domain::agent::AgentModelProtocol::Responses => "openai.responses",
        _ => "openai.chat-completions",
    }
    .into();
    data.context_window_tokens = candidate.config.context_window_tokens.map(|v| v as usize);
    data.metadata
        .insert(PERSONAI_PROVIDER.into(), candidate.provider.into());
    if let Some(quality) = candidate.selection.quality_type {
        data.metadata
            .insert(PERSONAI_QUALITY.into(), quality.storage_value().into());
    }
    data
}

fn candidate_key(selection: &ModelSelection) -> String {
    serde_json::to_string(selection).unwrap_or_else(|_| {
        format!(
            "{}:{:?}",
            selection.model_code.as_deref().unwrap_or_default(),
            selection.kind
        )
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn snapshot_order_matches_full_identity_and_preserves_context_window_policy() {
        use super::*;
        let candidate = |vendor: &str, window| AutoModelCandidate {
            selection: ModelSelection::person_ai_fixed(
                format!("{vendor}/model"),
                crate::domain::model_provider::ProviderModelQuality::High,
            ),
            config: serde_json::from_value(serde_json::json!({
                "modelCode": format!("PERSON_AI:FIXED:High:{vendor}/model"),
                "protocol": "CHAT_COMPLETIONS", "baseUrl": "https://fixture.test",
                "apiKey": "fixture", "modelName": "Model", "contextWindowTokens": window,
            }))
            .unwrap(),
        };
        let small = candidate("small", 128);
        let current = candidate("current", 256);
        let large = candidate("large", 512);
        let ordered = order_authorized_candidates(
            vec![small.clone(), large.clone(), current.clone()],
            &current.config,
        );
        assert_eq!(ordered, vec![current.config, large.config, small.config]);
    }
}
