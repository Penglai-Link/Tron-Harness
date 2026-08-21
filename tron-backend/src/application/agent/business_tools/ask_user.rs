use std::sync::Arc;

use serde::Deserialize;
use serde_json::json;

use crate::domain::agent::{
    AgentRunContext, EffectiveTool, Tool, ToolError, ToolExecutionResult, ToolObservation,
    ToolRequest,
};

pub const CHAT_ASK_USER_TOOL: &str = "chat.ask_user";

pub fn ask_user_definition() -> EffectiveTool {
    EffectiveTool {
        tool_name: CHAT_ASK_USER_TOOL.into(),
        domain_code: Some("chat".into()),
        description: "Ask the user a small set of structured questions when their answer materially changes the task. Stop all further work after calling this tool.".into(),
        input_schema: json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["intro", "questions"],
            "properties": {
                "intro": { "type": "string", "maxLength": 500 },
                "questions": {
                    "type": "array", "minItems": 1, "maxItems": 3,
                    "items": {
                        "type": "object", "additionalProperties": false,
                        "required": ["question", "options"],
                        "properties": {
                            "question": { "type": "string", "maxLength": 500 },
                            "options": {
                                "type": "array", "minItems": 2, "maxItems": 5,
                                "items": {
                                    "type": "object", "additionalProperties": false,
                                    "required": ["label"],
                                    "properties": {
                                        "label": { "type": "string", "maxLength": 120 },
                                        "description": { "type": ["string", "null"], "maxLength": 240 }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }),
        skill_name: None,
        required_variable_keys: Vec::new(),
    }
}

pub fn ask_user_tool() -> Arc<dyn Tool> {
    Arc::new(AskUserTool)
}

struct AskUserTool;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AskUserInput {
    intro: String,
    questions: Vec<AskUserQuestion>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AskUserQuestion {
    question: String,
    options: Vec<AskUserOption>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AskUserOption {
    label: String,
    description: Option<String>,
}

impl Tool for AskUserTool {
    fn name(&self) -> &str {
        CHAT_ASK_USER_TOOL
    }

    fn exec(
        &self,
        request: ToolRequest,
        _context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: AskUserInput = serde_json::from_value(request.input).map_err(|error| {
            ToolError::ExecutionFailed(format!("tool input is invalid: {error}"))
        })?;
        let intro = input.intro.trim().to_string();
        if intro.is_empty() || intro.chars().count() > 500 {
            return Err(ToolError::ExecutionFailed(
                "intro must contain 1 to 500 characters".into(),
            ));
        }
        if !(1..=3).contains(&input.questions.len()) {
            return Err(ToolError::ExecutionFailed(
                "questions must contain 1 to 3 items".into(),
            ));
        }
        for question in &input.questions {
            if question.question.trim().is_empty() || question.question.chars().count() > 500 {
                return Err(ToolError::ExecutionFailed(
                    "each question must contain 1 to 500 characters".into(),
                ));
            }
            if !(2..=5).contains(&question.options.len()) {
                return Err(ToolError::ExecutionFailed(
                    "each question must contain 2 to 5 options".into(),
                ));
            }
            for option in &question.options {
                if option.label.trim().is_empty() || option.label.chars().count() > 120 {
                    return Err(ToolError::ExecutionFailed(
                        "each option label must contain 1 to 120 characters".into(),
                    ));
                }
                if option
                    .description
                    .as_deref()
                    .is_some_and(|description| description.chars().count() > 240)
                {
                    return Err(ToolError::ExecutionFailed(
                        "option descriptions must contain at most 240 characters".into(),
                    ));
                }
            }
        }
        let output =
            serde_json::to_string(&json!({ "intro": intro, "questions": input.questions }))
                .map_err(|error| ToolError::ExecutionFailed(error.to_string()))?;
        Ok(ToolExecutionResult::observation(ToolObservation {
            tool_name: self.name().into(),
            output,
        }))
    }
}
