use crate::{
    application::prompt::{PromptAnalyzeResult, RenderTemplateCommand, RenderTemplateResult},
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

    pub fn render_template(
        &self,
        command: RenderTemplateCommand,
    ) -> Result<RenderTemplateResult, PromptError> {
        Ok(RenderTemplateResult {
            rendered_prompt: render_template(&command.prompt_body, &command.variables)?,
        })
    }
}
