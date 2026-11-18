//! Live platform data and managed instructions, assembled through the shared layout.
use crate::{
    application::assistant::AssistantService,
    domain::{agent::AgentRunContext, assistant::SoulState},
};
use eliteforge_agent_core as fw;
use serde_json::json;
use tron_i18n::t;

pub(super) struct AssistantContributor {
    pub service: Option<AssistantService>,
    pub context: AgentRunContext,
    pub id: &'static str,
    pub onboarding: bool,
}

#[async_trait::async_trait]
impl fw::ContextContributor for AssistantContributor {
    fn layout_metadata(&self) -> fw::ContextBlockMetadata {
        crate::application::short_term_memory::native_context_block_metadata(self.block_id())
            .expect("native PersonAI context metadata")
    }
    fn block_id(&self) -> &str {
        self.id
    }

    async fn contribute(
        &self,
        input: &fw::ContextInput<'_>,
    ) -> Result<fw::ContextBlock, fw::AgentError> {
        input.run.check_cancelled()?;
        let ctx = &self.context;
        let code = ctx.agent_code.as_deref().unwrap_or("agent");
        let own = ctx.is_owning_assistant();
        let soul = if own && matches!(self.id, "runtime_identity" | "assistant_onboarding") {
            Some(
                self.service
                    .as_ref()
                    .ok_or_else(|| {
                        fw::AgentError::Run(
                            t!(
                                "agent_runtime.assistant.identity_service_unavailable",
                                locale = tron_harness_capability::run_locale(input.run)
                            )
                            .into_owned(),
                        )
                    })?
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
            let data = json!({
                "identity": {
                    "agentCode": code,
                    "name": name,
                    "nameConfirmed": soul.as_ref().is_none_or(|value| value.name_confirmed),
                    "soulState": soul.as_ref().map(|value| value.state),
                    "kind": if own { "Assistant" } else { "Agent" },
                },
                "currentSoul": soul.as_ref().map(|value| value.markdown.as_str()),
                "interfaceLanguage": ctx.locale,
            });
            let messages = vec![fw::Message::user(format!(
                "Current identity and soul data: {data}"
            ))];
            return Ok(fw::ContextBlock {
                id: self.id.into(),
                messages,
                trust: fw::ContextTrust::Data,
                source_version: None,
            });
        }

        let text = match self.id {
            "assistant_onboarding" => soul
                .filter(|value| self.onboarding || value.state != SoulState::Ready)
                .map(|value| {
                    let next_action = self
                        .onboarding
                        .then(|| super::onboarding::next_action(ctx, &value));
                    let mut data = json!({
                        "state": value.state,
                        "nameConfirmed": value.name_confirmed,
                        "inOnboardingSession": value.session_code == ctx.conversation_code,
                        "pendingInteractionId": if value.user_code == ctx.user_code {
                            value.pending_interaction_id
                        } else {
                            None
                        },
                    });
                    if let Some(action) = next_action {
                        data["nextAction"] = action.into();
                    }
                    data.to_string()
                })
                .unwrap_or_default(),
            "assistant_environment" if own => json!({
                "userCode": ctx.user_code,
                "workspaceCode": ctx.execution_workspace_code(),
                "workspaceName": ctx.execution_scope.workspace_name,
                "scope": ctx.execution_scope,
            })
            .to_string(),
            "assistant_wakeup" if own => {
                let memory = ctx.short_term_memory.as_ref();
                let work = self
                    .service
                    .as_ref()
                    .ok_or_else(|| {
                        fw::AgentError::Run(
                            t!(
                                "agent_runtime.assistant.service_unavailable",
                                locale = tron_harness_capability::run_locale(input.run)
                            )
                            .into_owned(),
                        )
                    })?
                    .work_execution_context(code, ctx.user_code.as_deref().unwrap_or(""))
                    .map_err(|error| fw::AgentError::Run(error.to_string()))?;
                let event = trigger_event(memory.and_then(|value| value.input_metadata.as_ref()));
                let mut data = serde_json::Map::new();
                if let Some(event) = event {
                    data.insert("event".into(), event);
                }
                if !work.is_empty() {
                    data.insert("relatedWork".into(), json!(work));
                }
                if data.is_empty() {
                    String::new()
                } else {
                    serde_json::Value::Object(data).to_string()
                }
            }
            _ => String::new(),
        };
        Ok(fw::ContextBlock {
            id: self.id.into(),
            messages: if text.is_empty() {
                Vec::new()
            } else {
                vec![fw::Message::user(text)]
            },
            trust: fw::ContextTrust::Data,
            source_version: None,
        })
    }
}

// Ordinary user text already belongs to history. Transport controls (resume,
// annotations and client capabilities) are not model-visible trigger data.
fn trigger_event(metadata: Option<&serde_json::Value>) -> Option<serde_json::Value> {
    let metadata = metadata?.as_object()?;
    let source = metadata.get("source")?.as_str()?;
    if source.is_empty() || source == "UserMessage" {
        return None;
    }
    let event = ["source", "eventId", "taskCode", "payload"]
        .into_iter()
        .filter_map(|key| {
            metadata
                .get(key)
                .filter(|value| !value.is_null())
                .map(|value| (key.into(), value.clone()))
        })
        .collect::<serde_json::Map<_, _>>();
    Some(serde_json::Value::Object(event))
}

pub(super) struct AssistantGuidance {
    pub locale: String,
    pub onboarding: bool,
}
impl AssistantGuidance {
    pub(super) fn prompt_keys(onboarding: bool) -> &'static [&'static str] {
        if onboarding {
            &["assistant.initial-setup"]
        } else {
            &[
                "assistant.identity-guidance",
                "assistant.ui-page-context",
                "assistant.event-guidance",
                "assistant.soul-guidance",
                "assistant.onboarding-language",
            ]
        }
    }
}
#[async_trait::async_trait]
impl fw::ContextContributor for AssistantGuidance {
    fn layout_metadata(&self) -> fw::ContextBlockMetadata {
        crate::application::short_term_memory::native_context_block_metadata(self.block_id())
            .expect("native PersonAI guidance metadata")
    }
    fn kind(&self) -> fw::ContextSourceKind {
        fw::ContextSourceKind::Instruction
    }
    fn block_id(&self) -> &str {
        "assistant.guidance"
    }
    async fn contribute(
        &self,
        input: &fw::ContextInput<'_>,
    ) -> Result<fw::ContextBlock, fw::AgentError> {
        let keys = Self::prompt_keys(self.onboarding);
        let mut messages = keys
            .iter()
            .map(|key| {
                managed_guidance(input, key, &self.locale).map(|text| {
                    let text = if *key == "assistant.soul-guidance" {
                        text.replace(" task_update ", " assistant_task_update ")
                    } else {
                        text
                    };
                    fw::Message::system(text).with_instruction_origin(fw::InstructionOrigin::Plugin)
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        if !self.onboarding {
            messages.push(
                fw::Message::system(
                    t!(
                        "agent_runtime.assistant.guidance.lockstep",
                        locale = self.locale.as_str()
                    )
                    .into_owned(),
                )
                .with_instruction_origin(fw::InstructionOrigin::Plugin),
            );
        }
        Ok(fw::ContextBlock::instructions(self.block_id(), messages))
    }
}

fn managed_guidance(
    input: &fw::ContextInput<'_>,
    key: &str,
    locale: &str,
) -> Result<String, fw::AgentError> {
    let fallback = crate::domain::prompt::builtin_prompt_text(key, locale).ok_or_else(|| {
        fw::AgentError::InvalidConfiguration(t!(
            "agent_runtime.assistant.prompt_missing",
            locale = locale,
            key = key
        ))
    })?;
    fw::ManagedDeveloperPrompts::resolve(input.run, key, fallback)
}
