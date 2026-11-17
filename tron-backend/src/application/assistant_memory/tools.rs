use super::AssistantMemoryService;
use crate::domain::{
    agent::{
        AgentRunContext, EffectiveTool, Tool, ToolError, ToolExecutionResult, ToolObservation,
        ToolRegistry, ToolRequest,
    },
    assistant_memory::*,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

pub fn register_assistant_memory_tools(
    registry: &ToolRegistry,
    service: AssistantMemoryService,
    auto_approve: Arc<dyn Fn(&str) -> bool + Send + Sync>,
) -> Result<(), ToolError> {
    registry.register(Arc::new(MemorySkillTool {
        service: service.clone(),
        auto_approve: auto_approve.clone(),
        submit: false,
    }))?;
    registry.register(Arc::new(MemorySkillTool {
        service,
        auto_approve,
        submit: true,
    }))
}
pub fn assistant_memory_tool_definitions() -> Vec<EffectiveTool> {
    vec![
        EffectiveTool {tool_name:"assistant_skill_begin".into(),domain_code:Some("assistant-memory".into()),description:"Load the installed enabled skill-creator instructions and create or resume an experience Skill candidate. You must use these instructions before submitting the artifact. Only the owning memory PersonAI can invoke this tool.".into(),input_schema:json!({"type":"object","required":["batchCode","skillName","sourceCodes"],"properties":{"batchCode":{"type":"string"},"skillName":{"type":"string"},"sourceCodes":{"type":"array","items":{"type":"string"}}},"additionalProperties":false}),skill_name:None,required_variable_keys:vec![]},
        EffectiveTool {tool_name:"assistant_skill_submit".into(),domain_code:Some("assistant-memory".into()),description:"Submit a complete Skill artifact from the author session that loaded skill-creator. All files are staged, executable behavioral checks run, and only a passing exact revision can enter approval/install.".into(),input_schema:json!({"type":"object","required":["candidateCode","files","behaviorChecks"],"properties":{"candidateCode":{"type":"string"},"files":{"type":"array","items":{"type":"object","required":["path","content","encoding"],"properties":{"path":{"type":"string"},"content":{"type":"string"},"encoding":{"enum":["Utf8","Base64"]}},"additionalProperties":false}},"behaviorChecks":{"type":"array","items":{"type":"object","required":["name","program","args"],"properties":{"name":{"type":"string"},"program":{"enum":["python3","node","bash","sh"]},"args":{"type":"array","items":{"type":"string"}}},"additionalProperties":false}}},"additionalProperties":false}),skill_name:None,required_variable_keys:vec![]},
    ]
}
struct MemorySkillTool {
    service: AssistantMemoryService,
    auto_approve: Arc<dyn Fn(&str) -> bool + Send + Sync>,
    submit: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BeginInput {
    batch_code: String,
    skill_name: String,
    source_codes: Vec<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SubmitInput {
    candidate_code: String,
    files: Vec<SkillArtifactFile>,
    behavior_checks: Vec<BehaviorCheck>,
}
impl Tool for MemorySkillTool {
    fn name(&self) -> &str {
        if self.submit {
            "assistant_skill_submit"
        } else {
            "assistant_skill_begin"
        }
    }
    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let owner = context
            .collaboration_owner
            .as_ref()
            .filter(|o| o.kind.eq_ignore_ascii_case("ASSISTANT"))
            .ok_or_else(|| ToolError::UnauthorizedTool(self.name().into()))?;
        let user = context
            .user_code
            .as_deref()
            .ok_or_else(|| ToolError::UnauthorizedTool(self.name().into()))?;
        let session = context
            .conversation_code
            .as_deref()
            .ok_or_else(|| ToolError::UnauthorizedTool(self.name().into()))?;
        let map = |e: AssistantMemoryError| ToolError::ExecutionFailed(e.to_string());
        let output: Value = if self.submit {
            let input: SubmitInput = serde_json::from_value(request.input)
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
            let candidate = self
                .service
                .get_candidate(user, &input.candidate_code)
                .map_err(map)?;
            if candidate.assistant_code != owner.code {
                return Err(ToolError::UnauthorizedTool(self.name().into()));
            }
            serde_json::to_value(
                self.service
                    .submit_candidate(
                        user,
                        &input.candidate_code,
                        session,
                        input.files,
                        input.behavior_checks,
                        (self.auto_approve)(&owner.code),
                    )
                    .map_err(map)?,
            )
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?
        } else {
            let input: BeginInput = serde_json::from_value(request.input)
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
            let batch = self
                .service
                .get_batch(user, &input.batch_code)
                .map_err(map)?;
            if batch.assistant_code != owner.code {
                return Err(ToolError::UnauthorizedTool(self.name().into()));
            }
            let result = self
                .service
                .begin_skill_authoring_for_sources(
                    user,
                    &input.batch_code,
                    &input.skill_name,
                    session,
                    &input.source_codes,
                )
                .map_err(map)?;
            json!({"candidate":result.candidate,"creatorMarkdown":result.creator_markdown,"creatorFiles":result.creator_files})
        };
        Ok(ToolExecutionResult::observation(ToolObservation {
            tool_name: self.name().into(),
            output: serde_json::to_string(&output)
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?,
        }))
    }
}
