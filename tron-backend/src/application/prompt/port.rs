use crate::domain::prompt::{Prompt, PromptError};

pub trait PromptRepository: Send {
    fn list_prompts(&mut self) -> Result<Vec<Prompt>, PromptError>;
    fn create_prompt(&mut self, prompt: &Prompt) -> Result<(), PromptError>;
    fn update_prompt(&mut self, prompt: &Prompt, expected_version: u64) -> Result<(), PromptError>;
    fn delete_prompt(&mut self, prompt_code: &str) -> Result<bool, PromptError>;
}
