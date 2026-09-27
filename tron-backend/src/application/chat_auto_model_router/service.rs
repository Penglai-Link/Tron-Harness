use crate::{
    application::{model::ModelService, model_provider::ModelProviderService},
    domain::{
        agent::{AgentModelConfig, AutoModelRouteContext},
        model::ModelType,
        model_provider::{
            ModelProviderCode, ModelSelection, ModelSelectionKind, ProviderCatalogModelStatus,
        },
        reasoning::ReasoningEffort,
    },
};

#[derive(Debug, Clone)]
struct Candidate {
    selection: ModelSelection,
    config: AgentModelConfig,
    key: String,
    provider: bool,
    prefers_responses: bool,
}

#[derive(Clone)]
pub struct ChatAutoModelRouter {
    models: ModelService,
    providers: ModelProviderService,
}

impl ChatAutoModelRouter {
    pub fn new(models: ModelService, providers: ModelProviderService) -> Self {
        Self { models, providers }
    }

    pub fn resolve_initial(
        &self,
        workspace_code: &str,
        session_code: &str,
        agent_code: &str,
        reasoning_effort: Option<ReasoningEffort>,
    ) -> Result<(AgentModelConfig, AutoModelRouteContext), String> {
        let candidates = self.candidates(workspace_code, reasoning_effort)?;
        let assigned = self
            .providers
            .chat_auto_route_selection(workspace_code, session_code, agent_code)
            .map_err(|error| error.to_string())?;
        let candidate = assigned
            .and_then(|selection| {
                candidates
                    .iter()
                    .find(|candidate| candidate.selection == selection)
                    .cloned()
            })
            .or_else(|| {
                choose_initial_candidate(&candidates, workspace_code, session_code, agent_code)
            })
            .ok_or_else(|| "Auto has no available Text model with a context window".to_string())?;
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
                session_code: session_code.to_string(),
                agent_code: agent_code.to_string(),
                current_candidate_key: candidate.key,
                failed_candidate_keys: Vec::new(),
                fallback_attempts: 0,
            },
        ))
    }

    pub fn next_fallback(
        &self,
        workspace_code: &str,
        route: &mut AutoModelRouteContext,
        current_model: &AgentModelConfig,
        reasoning_effort: Option<ReasoningEffort>,
        person_ai_unavailable: bool,
    ) -> Result<Option<AgentModelConfig>, String> {
        if route.fallback_attempts >= 2 {
            return Ok(None);
        }
        let candidates = self.candidates(workspace_code, reasoning_effort)?;
        let mut eligible = candidates
            .into_iter()
            .filter(|candidate| candidate.key != route.current_candidate_key)
            .filter(|candidate| !route.failed_candidate_keys.contains(&candidate.key))
            .filter(|candidate| {
                fallback_candidate_is_allowed(candidate.provider, person_ai_unavailable)
            })
            .collect::<Vec<_>>();
        eligible.sort_by_key(|candidate| {
            (
                u8::from(!candidate.provider),
                context_window_rank(
                    current_model.context_window_tokens.unwrap_or_default(),
                    candidate.config.context_window_tokens.unwrap_or_default(),
                ),
                candidate.config.context_window_tokens.unwrap_or_default(),
                candidate.key.clone(),
            )
        });
        let Some(candidate) = eligible.into_iter().next() else {
            return Ok(None);
        };
        route
            .failed_candidate_keys
            .push(route.current_candidate_key.clone());
        route.fallback_attempts = route.fallback_attempts.saturating_add(1);
        route.current_candidate_key = candidate.key;
        self.providers
            .save_chat_auto_route_selection(
                workspace_code,
                &route.session_code,
                &route.agent_code,
                &candidate.selection,
                &candidate.config,
            )
            .map_err(|error| error.to_string())?;
        Ok(Some(candidate.config))
    }

    pub fn mark_success(
        &self,
        workspace_code: &str,
        route: &mut AutoModelRouteContext,
        model: &AgentModelConfig,
    ) -> Result<(), String> {
        let selection = self
            .candidates(workspace_code, model.reasoning_effort)?
            .into_iter()
            .find(|candidate| candidate.config.model_code == model.model_code)
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
    ) -> Result<Vec<Candidate>, String> {
        let mut candidates = Vec::new();
        // Provider catalog and credentials may be temporarily unavailable. In
        // that case Auto should still be able to route to enabled local models.
        for option in self
            .providers
            .list_text_select_options(workspace_code)
            .unwrap_or_default()
        {
            if option.selection.kind != ModelSelectionKind::Fixed
                || option.selection.provider_code != Some(ModelProviderCode::PersonAi)
                || !option.enabled
                || option.remote_status != ProviderCatalogModelStatus::Available
                || option.context_window_tokens.is_none_or(|value| value == 0)
            {
                continue;
            }
            if let Some(mut config) = self
                .providers
                .resolve_chat_selection(workspace_code, Some(&option.selection), None, None)
                .map_err(|error| error.to_string())?
            {
                config.reasoning_effort = reasoning_effort;
                let prefers_responses =
                    config.protocol == crate::domain::agent::AgentModelProtocol::Responses;
                candidates.push(Candidate {
                    key: candidate_key(&option.selection),
                    selection: option.selection,
                    config,
                    provider: true,
                    prefers_responses,
                });
            }
        }
        for option in self
            .models
            .list_workspace_select_options(workspace_code)
            .map_err(|error| error.to_string())?
        {
            if option.model_type != ModelType::Text
                || !option.enabled
                || option.context_window_tokens.is_none_or(|value| value == 0)
            {
                continue;
            }
            let Some(mut config) = self
                .models
                .text_agent_model_config_with_reasoning_effort(
                    workspace_code,
                    Some(&option.model_code),
                    None,
                )
                .map_err(|error| error.to_string())?
            else {
                continue;
            };
            config.reasoning_effort = reasoning_effort;
            let selection = ModelSelection::fixed(option.model_code);
            candidates.push(Candidate {
                key: candidate_key(&selection),
                selection,
                config,
                provider: false,
                prefers_responses: false,
            });
        }
        Ok(candidates)
    }
}

fn choose_initial_candidate(
    candidates: &[Candidate],
    workspace_code: &str,
    session_code: &str,
    agent_code: &str,
) -> Option<Candidate> {
    let pool = candidates
        .iter()
        .filter(|candidate| candidate.provider)
        .collect::<Vec<_>>();
    let pool = if pool.is_empty() {
        candidates.iter().collect::<Vec<_>>()
    } else {
        pool
    };
    pool.into_iter()
        .max_by_key(|candidate| {
            (
                u8::from(candidate.prefers_responses),
                rendezvous_score(workspace_code, session_code, agent_code, &candidate.key),
            )
        })
        .cloned()
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

fn rendezvous_score(workspace: &str, session: &str, agent: &str, candidate: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in [workspace, session, agent, candidate]
        .into_iter()
        .flat_map(|part| part.as_bytes().iter().copied().chain([0xff]))
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn context_window_rank(current: u32, candidate: u32) -> u8 {
    if candidate == current {
        0
    } else if candidate > current {
        1
    } else {
        2
    }
}

fn fallback_candidate_is_allowed(is_person_ai: bool, person_ai_unavailable: bool) -> bool {
    !is_person_ai || !person_ai_unavailable
}

#[cfg(test)]
mod tests {
    use super::{context_window_rank, fallback_candidate_is_allowed, rendezvous_score};

    #[test]
    fn rendezvous_routing_is_stable_per_session_agent_and_candidate() {
        assert_eq!(
            rendezvous_score("ws", "session", "agent", "model"),
            rendezvous_score("ws", "session", "agent", "model")
        );
        assert_ne!(
            rendezvous_score("ws", "session-a", "agent", "model"),
            rendezvous_score("ws", "session-b", "agent", "model")
        );
        assert_ne!(
            rendezvous_score("ws", "session", "agent-a", "model"),
            rendezvous_score("ws", "session", "agent-b", "model")
        );
    }

    #[test]
    fn fallback_window_order_preserves_then_grows_before_shrinking() {
        assert_eq!(context_window_rank(256, 256), 0);
        assert_eq!(context_window_rank(256, 512), 1);
        assert_eq!(context_window_rank(256, 128), 2);
    }

    #[test]
    fn provider_auth_failure_skips_remaining_person_ai_models() {
        assert!(!fallback_candidate_is_allowed(true, true));
        assert!(fallback_candidate_is_allowed(false, true));
        assert!(fallback_candidate_is_allowed(true, false));
    }
}
