use std::{collections::HashMap, future::Future, pin::Pin};

use crate::domain::prompt::{Prompt, PromptError};

pub trait PromptRepository: Send {
    fn list_prompts(&mut self) -> Result<Vec<Prompt>, PromptError>;
    fn create_prompt(&mut self, prompt: &Prompt) -> Result<(), PromptError>;
    fn update_prompt(&mut self, prompt: &Prompt, expected_version: u64) -> Result<(), PromptError>;
    fn delete_prompt(&mut self, prompt_code: &str) -> Result<bool, PromptError>;
}

#[derive(Debug, Clone)]
pub struct PromptPreviewCommand {
    pub workspace_code: Option<String>,
    pub model_code: Option<String>,
    pub rendered_prompt: String,
    pub temperature: Option<f64>,
    pub max_output_tokens: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct PromptPreviewOutput {
    pub model_code: String,
    pub output: String,
}

pub trait PromptPreviewRuntime: Send + Sync {
    fn generate<'a>(
        &'a self,
        command: PromptPreviewCommand,
    ) -> Pin<Box<dyn Future<Output = Result<PromptPreviewOutput, PromptError>> + Send + 'a>>;
}

#[derive(Debug, Clone)]
pub struct TestPreviewCommand {
    pub workspace_code: Option<String>,
    pub prompt_code: Option<String>,
    pub prompt_body: Option<String>,
    pub model_code: Option<String>,
    pub variables: HashMap<String, String>,
}
