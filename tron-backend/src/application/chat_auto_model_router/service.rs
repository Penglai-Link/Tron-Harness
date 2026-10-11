use crate::{
    application::{model::ModelService, model_provider::ModelProviderService},
    domain::{
        agent::{AgentModelConfig, AutoModelCandidate, AutoModelRouteContext},
        model::ModelType,
        model_provider::{
            ModelProviderCode, ModelSelection, ModelSelectionKind, ProviderCatalogModelStatus,
        },
        reasoning::ReasoningEffort,
    },
};

#[derive(Debug, Clone)]
struct Candidate {
    code: String,
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
        for option in self
            .providers
            .list_text_select_options(workspace_code)
            .map_err(|error| format!("Auto provider catalog lookup failed: {error}"))?
        {
            if option.selection.kind != ModelSelectionKind::Fixed
                || option.selection.provider_code != Some(ModelProviderCode::PersonAi)
                || !option.enabled
                || option.remote_status != ProviderCatalogModelStatus::Available
                || (!allowed.is_empty() && !allowed.contains(&option.option_code))
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
                    code: option.option_code,
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
                || (!allowed.is_empty() && !allowed.contains(&option.option_code))
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
                code: option.option_code,
                key: candidate_key(&selection),
                selection,
                config,
                provider: false,
                prefers_responses: false,
            });
        }
        candidates.retain(|candidate| allowed.is_empty() || allowed.contains(&candidate.code));
        Ok(candidates)
    }
}

fn order_authorized_candidates(
    mut candidates: Vec<AutoModelCandidate>,
    current: &AgentModelConfig,
) -> Vec<AgentModelConfig> {
    candidates.sort_by_key(|candidate| {
        (
            u8::from(candidate.config.identity() != current.identity()),
            u8::from(candidate.selection.provider_code.is_none()),
            context_window_rank(
                current.context_window_tokens.unwrap_or_default(),
                candidate.config.context_window_tokens.unwrap_or_default(),
            ),
            candidate_key(&candidate.selection),
        )
    });
    candidates
        .into_iter()
        .map(|candidate| candidate.config)
        .collect()
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

#[cfg(test)]
mod tests {
    use super::{context_window_rank, rendezvous_score};

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

    #[test]
    fn fallback_window_order_preserves_then_grows_before_shrinking() {
        assert_eq!(context_window_rank(256, 256), 0);
        assert_eq!(context_window_rank(256, 512), 1);
        assert_eq!(context_window_rank(256, 128), 2);
    }
}
