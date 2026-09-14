use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde_json::{Value, json};

use crate::{
    application::{
        agent::{
            ActiveRunRegistry, AgentInvocationCommand, AgentInvocationError, AgentInvocationResult,
            AgentInvocationService, AgentSteerInput, ModelStreamSink,
        },
        chat::{
            ChatRepository, ChatResumeEntry, ChatSendResult, ChatSessionAttachmentUploadCommand,
            ChatSessionQuery, ChatSteerError, ChatSteerResult, CreateChatSessionCommand,
            SendChatMessageCommand, SteerChatMessageCommand, UpdateChatSessionCommand,
        },
        llm::LlmRuntimeService,
        project::ProjectService,
        prompt::PromptService,
        resource::{ChatSessionAttachmentCreateCommand, ResourceService},
    },
    domain::agent::{AgentExecutionScope, AgentModelConfig, ModelTokenUsage},
    domain::chat::{
        ChatError, ChatMessage, ChatMessageAttachment, ChatMessageChannel, ChatMessageRole,
        ChatSession, ChatSessionGroup, ChatSessionSourceType, ChatSessionStatus,
        ChatUserInputAnswer, ChatUserInputAnswerKind, ChatUserInputOption, ChatUserInputQuestion,
        ChatUserInputRequest, ChatUserInputRequestStatus,
    },
    domain::resource::{ChatSessionAttachmentRecord, ResourceError},
    shared::{
        id::{SnowflakeIdProvider, SnowflakedSnowflakeIdProvider},
        time::current_timestamp,
    },
};

use super::{
    attachments::normalize_chat_attachments,
    title::{DEFAULT_MANUAL_SESSION_TITLE, should_generate_manual_session_title},
};

const MAIN_AGENT_CODE: &str = "main-agent";
const MAX_AGENT_ATTACHMENT_IMAGE_BYTES: usize = 5 * 1024 * 1024;
const MAX_AGENT_ATTACHMENT_TEXT_CHARS: usize = 256 * 1024;
static SESSION_RUN_COORDINATOR: OnceLock<Mutex<BTreeSet<String>>> = OnceLock::new();
static CHAT_SERVICE_COORDINATOR_NAMESPACE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
pub struct ChatService {
    repository: Arc<Mutex<Box<dyn ChatRepository>>>,
    pub(super) prompt_service: Option<PromptService>,
    resources: Option<ResourceService>,
    projects: Option<ProjectService>,
    id_provider: Arc<dyn SnowflakeIdProvider>,
    coordinator_namespace: u64,
    pub(super) active_runs: ActiveRunRegistry,
}

pub(super) struct GoalContinuationResult {
    pub(super) messages: Vec<ChatMessage>,
    pub(super) user_input_request: Option<ChatUserInputRequest>,
}

impl ChatService {
    pub fn new(repository: impl ChatRepository + 'static) -> Self {
        Self::with_id_provider(
            repository,
            Arc::new(SnowflakedSnowflakeIdProvider::default()),
        )
    }

    pub fn with_id_provider(
        repository: impl ChatRepository + 'static,
        id_provider: Arc<dyn SnowflakeIdProvider>,
    ) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
            prompt_service: None,
            resources: None,
            projects: None,
            id_provider,
            coordinator_namespace: CHAT_SERVICE_COORDINATOR_NAMESPACE
                .fetch_add(1, Ordering::Relaxed),
            active_runs: ActiveRunRegistry::new(),
        }
    }

    pub fn clone_with_id_provider(&self, id_provider: Arc<dyn SnowflakeIdProvider>) -> Self {
        Self {
            repository: self.repository.clone(),
            prompt_service: self.prompt_service.clone(),
            resources: self.resources.clone(),
            projects: self.projects.clone(),
            id_provider,
            coordinator_namespace: self.coordinator_namespace,
            active_runs: self.active_runs.clone(),
        }
    }

    pub fn with_active_runs(mut self, active_runs: ActiveRunRegistry) -> Self {
        self.active_runs = active_runs;
        self
    }

    pub fn with_resources(mut self, resources: ResourceService) -> Self {
        self.resources = Some(resources);
        self
    }

    pub fn with_projects(mut self, projects: ProjectService) -> Self {
        self.projects = Some(projects);
        self
    }

    pub fn with_prompt_service(mut self, prompt_service: PromptService) -> Self {
        self.prompt_service = Some(prompt_service);
        self
    }
}
