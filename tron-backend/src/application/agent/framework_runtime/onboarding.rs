//! A bounded soul-establishment profile; ordinary Dude messages keep the full runtime.
use crate::{
    application::dude::DudeService,
    domain::{agent::AgentRunContext, dude::SoulState},
};
use eliteforge_agent_core as fw;

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
        }
        Ok(lease)
    }
}
