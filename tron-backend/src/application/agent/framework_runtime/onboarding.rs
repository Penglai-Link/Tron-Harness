//! A bounded soul-establishment profile; ordinary Dude messages keep the full runtime.
use crate::{
    application::dude::DudeService,
    domain::{
        agent::AgentRunContext,
        dude::{DudeSoul, SoulState},
    },
};
use eliteforge_agent_core as fw;

/// Use the same admitted bindings and batch projection as gateway discovery.
/// A catalog record alone must never advertise an unavailable executor.
pub(super) fn tool_definitions(
    bindings: &[fw::ToolBinding],
    context: &AgentRunContext,
) -> Vec<fw::ToolDescriptor> {
    let mut tools = bindings
        .iter()
        .filter(|binding| {
            binding.exposed
                && matches!(
                    binding.descriptor.call_name.as_str(),
                    "dude_soul_read" | "dude_soul_update" | "a2ui_request_input"
                )
                && (binding.descriptor.call_name == "a2ui_request_input"
                    || context.is_tool_authorized(&binding.descriptor.call_name))
        })
        .map(|binding| fw::batch_tool_descriptor(&binding.descriptor))
        .collect::<Vec<_>>();
    tools.sort_by(|left, right| left.call_name.cmp(&right.call_name));
    tools
}

pub(super) fn tool_context(
    tools: &[fw::ToolDescriptor],
    run: &fw::RunContext,
) -> crate::domain::agent::AgentMessage {
    let tools = tools
        .iter()
        .filter(|tool| !run.is_tool_revoked(&tool.id))
        .map(|tool| {
            serde_json::json!({
                "call_name": tool.call_name,
                "description": tool.description,
                "input_schema": tool.input_schema,
            })
        })
        .collect::<Vec<_>>();
    crate::domain::agent::AgentMessage::new(
        "user",
        vec![crate::domain::agent::AgentMessagePart::Context {
            kind: "tool_domains".into(),
            text: serde_json::json!({
                "profile": "dude.initial-setup",
                "complete": true,
                "mode": "definition",
                "executionTool": "tool_execute",
                "skillsRequired": [],
                "tools": tools,
            })
            .to_string(),
            metadata: None,
        }],
    )
}

pub(super) fn next_action(context: &AgentRunContext, soul: &DudeSoul) -> &'static str {
    if soul.state == SoulState::Ready {
        return "complete";
    }
    let same_session =
        soul.session_code == context.conversation_code && soul.user_code == context.user_code;
    let answered = same_session
        && soul.pending_interaction_id.as_ref().is_some_and(|id| {
            context
                .short_term_memory
                .as_ref()
                .and_then(|memory| memory.input_metadata.as_ref())
                .and_then(|metadata| metadata["resume"].as_array())
                .is_some_and(|answers| {
                    answers
                        .iter()
                        .any(|answer| answer["interruptId"].as_str() == Some(id.as_str()))
                })
        });
    if answered {
        "save_confirmed_answers"
    } else if same_session && soul.pending_interaction_id.is_some() {
        "await_existing_form"
    } else {
        "present_form"
    }
}

pub(super) fn active(context: &AgentRunContext, dudes: Option<&DudeService>) -> bool {
    if !context.is_owning_dude() {
        return false;
    }
    let Some(memory) = &context.short_term_memory else {
        return false;
    };
    let metadata = memory.input_metadata.as_ref();
    let guide_input = metadata.is_some_and(|m| {
        m["a2uiClientCapabilities"]["version"] == eliteforge_agent_official_plugin_a2ui::VERSION
            && m["a2uiClientCapabilities"]["catalogIds"]
                .as_array()
                .is_some_and(|ids| {
                    ids.iter()
                        .any(|id| id == eliteforge_agent_official_plugin_a2ui::AGENT_CATALOG_ID)
                })
            && (m["onboarding"] == true
                || m["resume"]
                    .as_array()
                    .is_some_and(|answers| !answers.is_empty()))
    });
    if !guide_input {
        return false;
    }
    dudes
        .and_then(|d| d.soul(context.agent_code.as_deref()?).ok())
        .is_some_and(|s| {
            s.state != SoulState::Ready
                && !s.awakening_deferred
                && s.session_code == context.conversation_code
                && s.user_code == context.user_code
                && metadata.is_some_and(|m| {
                    m["onboarding"] == true
                        || m["resume"].as_array().is_some_and(|answers| {
                            answers.iter().any(|answer| {
                                answer["interruptId"].as_str()
                                    == s.pending_interaction_id.as_deref()
                            })
                        })
                })
        })
}

pub(super) fn narrow(context: &mut AgentRunContext) {
    let tools = context
        .authorized_tools()
        .iter()
        .filter(|t| matches!(t.tool_name.as_str(), "dude_soul_read" | "dude_soul_update"))
        .cloned()
        .collect::<Vec<_>>();
    context.tools = tools.clone();
    context.effective_tools = tools;
    context.tool_assignment = None;
    context.skills.clear();
    std::sync::Arc::make_mut(&mut context.messages).retain(|m| m.role != "system");
}

/// Use the original Framework executor, validation and durable interaction store.
pub(super) struct FormTools;
#[async_trait::async_trait]
impl fw::ToolProvider for FormTools {
    async fn lease(&self, run: &fw::RunContext) -> Result<fw::ToolLease, fw::AgentError> {
        let service = run
            .extensions
            .get::<eliteforge_agent_official_plugin_a2ui::A2uiService>();
        // The host supplies the profile's shared A2UI service in the run extension.
        let service = service
            .ok_or_else(|| fw::AgentError::ToolSetup("A2UI form service unavailable".into()))?;
        let mut lease = fw::ToolProvider::lease(&*service, run).await?;
        lease
            .tools
            .retain(|t| t.descriptor.call_name == "a2ui_request_input");
        for tool in &mut lease.tools {
            tool.exposed = true;
            // The onboarding prompt already embeds the negotiated form contract.
            // The general descriptor's skill lookup would send this bounded
            // profile searching for skill tools it deliberately does not lease.
            tool.descriptor.description = "Publish the first-awakening SubmitOnce form and wait for its response. The complete A2UI form contract is already in the Dude onboarding instructions; no skill lookup is required.".into();
        }
        Ok(lease)
    }
}
