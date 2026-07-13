use crate::{
    application::prompt::PromptAnalyzeResult,
    domain::prompt::{PromptError, PromptVariable, analyze_template},
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
}
