use std::sync::Arc;

use serde::Deserialize;

use crate::{
    application::skill::SkillRegistryService,
    domain::agent::{AgentRunContext, Tool, ToolError, ToolExecutionResult, ToolRequest},
};

use super::common::{json_observation, page_records, parse_tool_input, service_tool_error};

pub(super) fn tools(skills: SkillRegistryService) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(SkillPageTool {
            skills: skills.clone(),
        }),
        Arc::new(SkillViewTool { skills }),
    ]
}

#[derive(Clone)]
struct SkillPageTool {
    skills: SkillRegistryService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SkillPageInput {
    current: Option<usize>,
    size: Option<usize>,
    search: Option<String>,
}

impl Tool for SkillPageTool {
    fn name(&self) -> &str {
        "skill_page"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: SkillPageInput = parse_tool_input(request.input)?;
        let agent = context.agent_code.as_deref().unwrap_or("main-agent");
        let mut records = self
            .skills
            .effective_agent_skills(agent)
            .map_err(service_tool_error)?;
        records.retain(|s| {
            s.available
                && s.assigned
                && input
                    .search
                    .as_ref()
                    .is_none_or(|q| s.skill_name.contains(q))
        });
        let output = page_records(records, input.current, input.size);
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct SkillViewTool {
    skills: SkillRegistryService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SkillViewInput {
    skill_name: String,
}

impl Tool for SkillViewTool {
    fn name(&self) -> &str {
        "skill_view"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: SkillViewInput = parse_tool_input(request.input)?;
        let agent = context.agent_code.as_deref().unwrap_or("main-agent");
        let skill_name = input.skill_name.trim();
        if skill_name.is_empty() {
            return Err(ToolError::ExecutionFailed(
                "skillName is required".to_string(),
            ));
        }
        if !self
            .skills
            .effective_agent_skill_names(agent)
            .map_err(service_tool_error)?
            .contains(skill_name)
        {
            return Err(service_tool_error(
                crate::domain::skill::SkillError::Unavailable,
            ));
        }
        let output = self
            .skills
            .get_registry_skill(skill_name)
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}
