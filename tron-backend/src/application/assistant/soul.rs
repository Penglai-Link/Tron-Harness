//! Platform identity writes, serialized with all other PersonAI mutations.
use super::*;
use crate::domain::agent::AgentRunContext;
use serde::Deserialize;

impl AssistantService {
    pub(crate) fn runtime_name(&self, code: &str, workspace: &str) -> Option<String> {
        if !workspace.is_empty() {
            if let Ok(policy) = self
                .management
                .workspace_internal_agent_policy(workspace, code)
            {
                return Some(policy.name);
            }
        }
        self.management
            .all_platform_internal_agents()
            .ok()?
            .into_iter()
            .find(|a| a.agent_code == code)
            .map(|a| a.name)
    }

    pub fn soul(&self, code: &str) -> Result<AssistantSoul, AssistantError> {
        // The repository checks the PersonAI and soul together; no full PersonAI listing.
        self.repo()?.soul(code)
    }
    pub fn save_soul(
        &self,
        code: &str,
        name: String,
        markdown: String,
    ) -> Result<AssistantSoul, AssistantError> {
        self.change_soul(code, |s| {
            s.name = name.trim().into();
            s.name_confirmed = true;
            s.markdown = markdown;
            Ok(())
        })
    }
    fn change_soul(
        &self,
        code: &str,
        change: impl FnOnce(&mut AssistantSoul) -> Result<(), AssistantError>,
    ) -> Result<AssistantSoul, AssistantError> {
        self.get(code)?;
        let mut repo = self.repo()?;
        let mut soul = repo.soul(code)?;
        let ready = soul.state == SoulState::Ready;
        change(&mut soul)?;
        if soul.name.trim().is_empty()
            || soul.name.chars().count() > 100
            || soul.markdown.len() > 16 * 1024
        {
            return Err(AssistantError::Invalid);
        }
        if ready && soul.markdown.trim().is_empty() {
            return Err(AssistantError::Invalid);
        }
        if soul.name_confirmed && !soul.markdown.trim().is_empty() {
            soul.state = SoulState::Ready;
            soul.pending_interaction_id = None;
        }
        repo.save_soul(&soul)?;
        Ok(soul)
    }
    pub fn begin_awakening(
        &self,
        code: &str,
        user: &str,
    ) -> Result<(AssistantSoul, bool), AssistantError> {
        self.get(code)?;
        let mut repo = self.repo()?;
        let mut s = repo.soul(code)?;
        if s.state == SoulState::Ready {
            return Ok((s, false));
        }
        if s.session_code.is_none() {
            let now = chrono::Utc::now().timestamp_millis();
            let session = repo.resolve_session(&AssistantSession {
                session_code: eliteforge_agent_core::SessionRecord::new("").id,
                assistant_code: code.into(),
                purpose: AssistantSessionPurpose::Interactive,
                correlation_key: "soul-awakening".into(),
                title: "首次唤醒".into(),
                workspace_code: None,
                created_at: now,
                updated_at: now,
            })?;
            s.session_code = Some(session.session_code);
            s.user_code = Some(user.into());
        }
        // Another account may edit the platform soul, but never take over private answers.
        if s.user_code.as_deref() != Some(user) {
            return Err(AssistantError::Conflict);
        }
        let first_entrance = !s.awakening_started;
        s.awakening_started = true;
        repo.save_soul(&s)?;
        Ok((s, first_entrance))
    }
    pub fn defer_awakening(&self, code: &str) -> Result<AssistantSoul, AssistantError> {
        self.change_soul(code, |s| {
            s.awakening_started = true;
            s.awakening_deferred = true;
            Ok(())
        })
    }
    pub fn record_soul_question(
        &self,
        session: &str,
        interaction: &str,
    ) -> Result<(), AssistantError> {
        let Some(d) = self.session(session)? else {
            return Ok(());
        };
        let s = self.soul(&d.assistant_code)?;
        if s.state == SoulState::Ready || s.session_code.as_deref() != Some(session) {
            return Ok(());
        }
        self.change_soul(&d.assistant_code, |s| {
            if s.state != SoulState::Ready {
                s.state = SoulState::AwaitingUser;
                s.pending_interaction_id = Some(interaction.into());
            }
            Ok(())
        })?;
        Ok(())
    }
    pub fn update_soul_from_user(
        &self,
        context: &AgentRunContext,
        input: SoulUpdate,
    ) -> Result<AssistantSoul, AssistantError> {
        input.validate()?;
        if !context.is_owning_assistant() || context.user_code.is_none() {
            return Err(AssistantError::Invalid);
        }
        let code = context
            .agent_code
            .as_deref()
            .ok_or(AssistantError::Invalid)?;
        let memory = context
            .short_term_memory
            .as_ref()
            .ok_or(AssistantError::Invalid)?;
        // Kind comes from the host, never from a model-supplied evidence string.
        let kind = memory.input_kind.to_ascii_uppercase();
        if kind != "USER_PROMPT" {
            return Err(AssistantError::Invalid);
        }
        let source = memory
            .current_input_message
            .as_ref()
            .filter(|m| m.role == "user")
            .ok_or(AssistantError::Invalid)?
            .text_content();
        if source.trim().is_empty() {
            return Err(AssistantError::Invalid);
        }
        // Intent is understood by the model using the trusted conversation and soul rules.
        // The host validates provenance and persistence invariants, not natural-language keywords.
        self.change_soul(code, |s| {
            let same_guide =
                s.session_code == context.conversation_code && s.user_code == context.user_code;
            let resumed = memory
                .input_metadata
                .as_ref()
                .and_then(|m| m.get("resume"))
                .and_then(|r| r.as_array())
                .filter(|r| !r.is_empty());
            // A late answer cannot replace a soul completed in the editor or an earlier run.
            if same_guide && s.state == SoulState::Ready && resumed.is_some() {
                return Err(AssistantError::Invalid);
            }
            let initializing = s.state != SoulState::Ready && same_guide;
            let answered = initializing
                && s.pending_interaction_id.is_some()
                && resumed.is_some_and(|entries| {
                    entries.iter().any(|entry| {
                        entry.get("interruptId").and_then(|id| id.as_str())
                            == s.pending_interaction_id.as_deref()
                    })
                });
            if initializing && resumed.is_some() && !answered {
                return Err(AssistantError::Invalid);
            }
            if let Some(name) = input.name {
                s.name = name.trim().into();
                s.name_confirmed = true;
            }
            if let Some(initial) = input.initial_soul {
                if !initializing {
                    return Err(AssistantError::Invalid);
                }
                s.markdown = initial;
            }
            for edit in input.edits {
                if edit.old.is_empty() {
                    if s.markdown != edit.new && !s.markdown.ends_with(&format!("\n{}", edit.new)) {
                        if !s.markdown.is_empty() {
                            s.markdown.push('\n');
                        }
                        s.markdown.push_str(&edit.new);
                    }
                } else if s.markdown.matches(&edit.old).count() == 1 {
                    s.markdown = s.markdown.replacen(&edit.old, &edit.new, 1);
                } else {
                    return Err(AssistantError::Invalid);
                }
            }
            Ok(())
        })
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SoulUpdate {
    pub name: Option<String>,
    pub initial_soul: Option<String>,
    #[serde(default)]
    pub edits: Vec<SoulEdit>,
}
impl SoulUpdate {
    fn validate(&self) -> Result<(), AssistantError> {
        if (self.name.is_none() && self.initial_soul.is_none() && self.edits.is_empty())
            || (self.initial_soul.is_some() && !self.edits.is_empty())
            || self
                .name
                .as_ref()
                .is_some_and(|v| v.trim().is_empty() || v.chars().count() > 100)
            || self
                .initial_soul
                .as_ref()
                .is_some_and(|v| v.trim().is_empty() || v.len() > 16 * 1024)
            || self
                .edits
                .iter()
                .any(|e| e.old == e.new || e.old.len() > 16 * 1024 || e.new.len() > 16 * 1024)
        {
            return Err(AssistantError::Invalid);
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoulEdit {
    pub old: String,
    pub new: String,
}

/// The initial wakeup is a real, visible user message, not a hidden prompt.
pub fn awakening_message(locale: &str) -> &'static str {
    match crate::shared::locale::SupportedLocale::from_raw(locale) {
        crate::shared::locale::SupportedLocale::EnUs => "Wake up",
        crate::shared::locale::SupportedLocale::ZhCn => "醒醒",
    }
}

pub fn runtime_input_kind(source: AssistantEventSource) -> String {
    if source == AssistantEventSource::UserMessage {
        "USER_PROMPT".into()
    } else {
        format!("ASSISTANT_{source:?}").to_uppercase()
    }
}
