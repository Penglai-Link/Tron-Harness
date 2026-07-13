use crate::{
    application::prompt::{PromptAnalyzeResult, PromptAnalyzedVariable},
    domain::prompt::{PromptError, PromptVariableType, analyze_template},
};

use super::PromptService;

impl PromptService {
    pub fn analyze_prompt_template(
        &self,
        prompt_body: &str,
    ) -> Result<PromptAnalyzeResult, PromptError> {
        analyze_template(prompt_body)
            .map(|names| PromptAnalyzeResult {
                variables: names
                    .into_iter()
                    .map(|name| PromptAnalyzedVariable {
                        name,
                        variable_type: PromptVariableType::String,
                    })
                    .collect(),
                invalid_variables: Vec::new(),
            })
            .map_err(Into::into)
    }
}
