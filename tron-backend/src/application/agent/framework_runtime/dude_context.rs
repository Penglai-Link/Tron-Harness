//! Live platform identity: never captured in the run prefix or summary cache.
use crate::{
    application::dude::DudeService,
    domain::{agent::AgentRunContext, dude::SoulState},
};
use eliteforge_agent_core as fw;
use serde_json::json;

pub(super) struct DudeContributor {
    pub service: Option<DudeService>,
    pub context: AgentRunContext,
    pub id: &'static str,
}

#[async_trait::async_trait]
impl fw::ContextContributor for DudeContributor {
    fn block_id(&self) -> &str {
        self.id
    }

    fn kind(&self) -> fw::ContextSourceKind {
        if self.id == "runtime_identity" {
            fw::ContextSourceKind::Instruction
        } else {
            fw::ContextSourceKind::Data
        }
    }

    async fn contribute(
        &self,
        input: &fw::ContextInput<'_>,
    ) -> Result<fw::ContextBlock, fw::AgentError> {
        input.run.check_cancelled()?;
        let ctx = &self.context;
        let code = ctx.agent_code.as_deref().unwrap_or("agent");
        let own = ctx.is_owning_dude();
        let soul = if own && matches!(self.id, "runtime_identity" | "dude_onboarding") {
            Some(
                self.service
                    .as_ref()
                    .ok_or_else(|| fw::AgentError::Run("Dude identity service unavailable".into()))?
                    .soul(code)
                    .map_err(|error| fw::AgentError::Run(error.to_string()))?,
            )
        } else {
            None
        };

        if self.id == "runtime_identity" {
            let name = if let Some(soul) = &soul {
                soul.name.clone()
            } else {
                self.service
                    .as_ref()
                    .and_then(|service| service.runtime_name(code, &ctx.workspace_code))
                    .unwrap_or_else(|| code.into())
            };
            let mut guidance = managed_guidance(input, "dude.identity-guidance", &ctx.locale)?;
            if let Some(soul) = &soul {
                guidance.push('\n');
                guidance.push_str(&managed_guidance(input, "dude.soul-guidance", &ctx.locale)?);
                if soul.state != SoulState::Ready {
                    guidance.push('\n');
                    guidance.push_str(&managed_guidance(
                        input,
                        "dude.onboarding-language",
                        &ctx.locale,
                    )?);
                }
            }
            let data = json!({
                "identity": {
                    "agentCode": code,
                    "name": name,
                    "nameConfirmed": soul.as_ref().is_none_or(|value| value.name_confirmed),
                    "kind": if own { "Dude" } else { "Agent" },
                },
                "currentSoul": soul.as_ref().map(|value| value.markdown.as_str()),
                "interfaceLanguage": ctx.locale,
            });
            let messages = vec![
                fw::Message::system(guidance)
                    .with_instruction_origin(fw::InstructionOrigin::Plugin),
                fw::Message::user(format!("Current identity and soul data: {data}")),
            ];
            let mut tokens = 0usize;
            for message in &messages {
                tokens =
                    tokens.saturating_add(input.estimator.message(input.model, message).await?);
            }
            if tokens > 8192 {
                return Err(fw::ContextError::BudgetExceeded(
                    "identity and soul exceed their protected budget".into(),
                )
                .into());
            }
            return Ok(fw::ContextBlock::instructions_with_data(self.id, messages));
        }

        let text = match self.id {
            "dude_onboarding" => soul
                .filter(|value| value.state != SoulState::Ready)
                .map(|value| {
                    json!({
                        "state": value.state,
                        "nameConfirmed": value.name_confirmed,
                        "inOnboardingSession": value.session_code == ctx.conversation_code,
                        "pendingInteractionId": if value.user_code == ctx.user_code {
                            value.pending_interaction_id
                        } else {
                            None
                        },
                    })
                    .to_string()
                })
                .unwrap_or_default(),
            "dude_environment" if own => json!({
                "userCode": ctx.user_code,
                "workspaceCode": ctx.workspace_code,
                "scope": ctx.execution_scope,
                "toolCount": ctx.authorized_tools().len(),
                "availableTools": ctx.authorized_tools().iter().take(60).map(|tool| tool.tool_name.as_str()).collect::<Vec<_>>(),
                "domains": ctx.tool_exposure.visible_domains.iter().take(16).collect::<Vec<_>>(),
                "limits": ctx.limits,
            })
            .to_string(),
            "dude_wakeup" if own => {
                let memory = ctx.short_term_memory.as_ref();
                let work = self
                    .service
                    .as_ref()
                    .ok_or_else(|| fw::AgentError::Run("Dude service unavailable".into()))?
                    .work_execution_context(code, ctx.user_code.as_deref().unwrap_or(""))
                    .map_err(|error| fw::AgentError::Run(error.to_string()))?;
                json!({
                    "eventKind": memory.map(|value| &value.input_kind),
                    "event": memory.and_then(|value| value.input_metadata.as_ref()),
                    "relatedWork": work,
                })
                .to_string()
            }
            _ => String::new(),
        };
        Ok(fw::ContextBlock::data(self.id, text))
    }
}

fn managed_guidance(
    input: &fw::ContextInput<'_>,
    key: &str,
    locale: &str,
) -> Result<String, fw::AgentError> {
    let fallback = crate::domain::prompt::builtin_prompt_text(key, locale).ok_or_else(|| {
        fw::AgentError::InvalidConfiguration(format!("bundled developer prompt is missing: {key}"))
    })?;
    fw::ManagedDeveloperPrompts::resolve(input.run, key, fallback)
}
