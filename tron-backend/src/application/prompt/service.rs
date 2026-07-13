use std::sync::{Arc, Mutex};

use crate::{
    application::{
        prompt::{PromptPreviewRuntime, PromptRepository},
        workspace::WorkspaceService,
    },
    domain::prompt::{Prompt, PromptView},
    shared::id::IdGenerator,
};

mod catalog;
mod platform;
mod preview;
mod validation;
mod workspace;

#[derive(Clone)]
pub struct PromptService {
    repository: Arc<Mutex<Box<dyn PromptRepository>>>,
    workspaces: WorkspaceService,
    ids: Arc<dyn IdGenerator>,
    preview_runtime: Arc<dyn PromptPreviewRuntime>,
}

#[derive(Clone)]
struct ResolvedPrompt {
    view: PromptView,
    definition: Prompt,
    current_owned: Option<Prompt>,
    upstream_enabled: Option<bool>,
}

impl PromptService {
    pub fn new(
        repository: impl PromptRepository + 'static,
        workspaces: WorkspaceService,
        ids: Arc<dyn IdGenerator>,
        preview_runtime: Arc<dyn PromptPreviewRuntime>,
    ) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
            workspaces,
            ids,
            preview_runtime,
        }
    }
}
