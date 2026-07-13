use crate::{
    application::prompt::{
        PromptAnalyzeResult, PromptPreviewCommand, TestPreviewCommand, TestPreviewResult,
    },
    domain::prompt::{PromptError, PromptVariable, analyze_template, render_template},
};

use super::PromptService;

impl PromptService {
    pub fn analyze_prompt_template(
        &self,
        prompt_body: &str,
    ) -> Result<PromptAnalyzeResult, PromptError> {
        analyze_template(prompt_body)
            .map(|names| PromptAnalyzeResult {
                variables: names.into_iter().map(PromptVariable::string).collect(),
                invalid_variables: Vec::new(),
            })
            .map_err(Into::into)
    }

    pub async fn test_preview(
        &self,
        command: TestPreviewCommand,
    ) -> Result<TestPreviewResult, PromptError> {
        let stored = match (
            command.workspace_code.as_deref(),
            command.prompt_code.as_deref(),
        ) {
            (Some(workspace_code), Some(prompt_code)) => {
                Some(self.workspace_detail(workspace_code, prompt_code)?)
            }
            (None, Some(prompt_code)) => Some(self.platform_detail(prompt_code)?),
            _ => None,
        };
        if stored.as_ref().is_some_and(|prompt| !prompt.enabled) {
            return Err(PromptError::PreviewFailed("prompt is disabled".into()));
        }
        let prompt_body = command
            .prompt_body
            .as_deref()
            .or_else(|| stored.as_ref().map(|prompt| prompt.prompt_body.as_str()))
            .ok_or_else(|| {
                PromptError::InvalidInput("promptCode or promptBody is required".into())
            })?;
        let rendered_prompt = render_template(prompt_body, &command.variables)?;
        let model_code = command
            .model_code
            .or_else(|| stored.as_ref().and_then(|prompt| prompt.model_code.clone()));
        let generated = self
            .preview_runtime
            .generate(PromptPreviewCommand {
                workspace_code: command.workspace_code,
                model_code,
                rendered_prompt: rendered_prompt.clone(),
                temperature: command
                    .temperature
                    .or_else(|| stored.as_ref().and_then(|prompt| prompt.temperature)),
                max_output_tokens: command
                    .max_output_tokens
                    .or_else(|| stored.as_ref().and_then(|prompt| prompt.max_output_tokens)),
            })
            .await?;
        Ok(TestPreviewResult {
            rendered_prompt,
            model_code: Some(generated.model_code),
            output: generated.output,
        })
    }
}
