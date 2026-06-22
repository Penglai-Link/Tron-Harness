use std::sync::Arc;

use serde::Deserialize;

use crate::{
    application::skill::SkillRegistryService,
    domain::{
        agent::{AgentRunContext, Tool, ToolError, ToolExecutionResult, ToolRequest},
        skill::WorkspaceSkillListQuery,
    },
};

use super::common::{
    json_observation, page_records, parse_tool_input, resolve_workspace_code, service_tool_error,
};

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
    workspace_code: Option<String>,
    current: Option<usize>,
    size: Option<usize>,
    search: Option<String>,
    include_disabled: Option<bool>,
}

impl Tool for SkillPageTool {
    fn name(&self) -> &str {
        "skill.page"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: SkillPageInput = parse_tool_input(request.input)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let mut records = self
            .skills
            .list_workspace_skills(
                workspace_code.as_str(),
                WorkspaceSkillListQuery {
                    search: input.search,
                },
            )
            .map_err(service_tool_error)?;
        if !input.include_disabled.unwrap_or(false) {
            records.retain(|item| item.enabled);
        }
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
    workspace_code: Option<String>,
    skill_name: String,
}

impl Tool for SkillViewTool {
    fn name(&self) -> &str {
        "skill.view"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: SkillViewInput = parse_tool_input(request.input)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let skill_name = input.skill_name.trim();
        if skill_name.is_empty() {
            return Err(ToolError::ExecutionFailed(
                "skillName is required".to_string(),
            ));
        }
        let output = self
            .skills
            .get_workspace_skill(workspace_code.as_str(), skill_name)
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}
