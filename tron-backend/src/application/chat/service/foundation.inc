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
        model_router::LlmModelRouter,
        project::ProjectService,
        prompt::PromptService,
        resource::{ChatSessionAttachmentCreateCommand, ResourceService},
    },
    domain::agent::{AgentExecutionScope, AgentModelConfig, ModelTokenUsage},
    domain::chat::{
        ChatError, ChatMessage, ChatMessageAttachment, ChatMessageChannel, ChatMessageRole,
        ChatSession, ChatSessionGroup, ChatSessionSourceType, ChatSessionStatus,
        ChatUserInputOption, ChatUserInputQuestion, ChatUserInputRequest,
        ChatUserInputRequestStatus,
    },
    domain::model_provider::{ModelSelection, ProviderModelQuality},
    domain::resource::{ChatSessionAttachmentRecord, ResourceError},
    shared::{
        event_bus::EventBusHandle,
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
    pub(super) llm_router: Option<LlmModelRouter>,
    pub(super) event_bus: EventBusHandle,
    resources: Option<ResourceService>,
    projects: Option<ProjectService>,
    skills: Option<crate::application::skill::SkillRegistryService>,
    execution_memory: Option<crate::application::short_term_memory::ShortTermMemoryService>,
    id_provider: Arc<dyn SnowflakeIdProvider>,
    coordinator_namespace: u64,
    pub(super) active_runs: ActiveRunRegistry,
}

pub(super) struct GoalContinuationResult {
    pub(super) messages: Vec<ChatMessage>,
    pub(super) user_input_request: Option<ChatUserInputRequest>,
    // The actual last product child retains its native identity/status/schema.
    // None means that no continuation result replaced the initial invocation.
    pub(super) final_run: Option<AgentInvocationResult>,
}
impl GoalContinuationResult {
    pub(super) fn merge_into(
        self,
        run: &mut AgentInvocationResult,
        messages: &mut Vec<ChatMessage>,
        user_input_request: &mut Option<ChatUserInputRequest>,
    ) {
        messages.extend(self.messages);
        *user_input_request = user_input_request.take().or(self.user_input_request);
        if let Some(final_run) = self.final_run {
            *run = final_run;
        }
    }
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
            llm_router: None,
            event_bus: EventBusHandle::disabled(),
            resources: None,
            projects: None,
            execution_memory: None,
            skills: None,
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
            llm_router: self.llm_router.clone(),
            event_bus: self.event_bus.clone(),
            resources: self.resources.clone(),
            projects: self.projects.clone(),
            execution_memory: self.execution_memory.clone(),
            skills: self.skills.clone(),
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

    pub fn with_llm_router(mut self, router: LlmModelRouter) -> Self {
        self.llm_router = Some(router);
        self
    }

    pub fn with_event_bus(mut self, event_bus: EventBusHandle) -> Self {
        self.event_bus = event_bus;
        self
    }
}
