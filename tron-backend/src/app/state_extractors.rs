use axum::extract::FromRef;

use crate::{
    application::{
        agent::{AgentManagementService, AgentRunHistoryService},
        app::AppService,
        automation::AutomationService,
        chat::ChatService,
        graph::{GraphExplorerService, GraphService, OntologyService},
        llm::LlmRuntimeService,
        mcp::McpService,
        memory::MemoryWorkDomainService,
        model::ModelService,
        open_app::OpenAppService,
        prompt::PromptService,
        role::RoleService,
        short_term_memory::ShortTermMemoryService,
        skill::SkillRegistryService,
        task::TaskService,
        user::UserService,
        variable::VariableService,
        workspace::WorkspaceService,
    },
    config::AppConfig,
    shared::event_bus::EventBusHandle,
};

use super::{AppMemoryService, AppRagService, AppReadiness, AppState};

impl FromRef<AppState> for AppConfig {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}

impl FromRef<AppState> for AppReadiness {
    fn from_ref(state: &AppState) -> Self {
        state.readiness.clone()
    }
}

impl FromRef<AppState> for UserService {
    fn from_ref(state: &AppState) -> Self {
        state.users.clone()
    }
}

impl FromRef<AppState> for AppService {
    fn from_ref(state: &AppState) -> Self {
        state.apps.clone()
    }
}

impl FromRef<AppState> for EventBusHandle {
    fn from_ref(state: &AppState) -> Self {
        state.event_bus.clone()
    }
}

impl FromRef<AppState> for RoleService {
    fn from_ref(state: &AppState) -> Self {
        state.roles.clone()
    }
}

impl FromRef<AppState> for WorkspaceService {
    fn from_ref(state: &AppState) -> Self {
        state.workspaces.clone()
    }
}

impl FromRef<AppState> for GraphService {
    fn from_ref(state: &AppState) -> Self {
        state.graphs.clone()
    }
}

impl FromRef<AppState> for OntologyService {
    fn from_ref(state: &AppState) -> Self {
        state.ontologies.clone()
    }
}

impl FromRef<AppState> for GraphExplorerService {
    fn from_ref(state: &AppState) -> Self {
        state.graph_explorer.clone()
    }
}

impl FromRef<AppState> for TaskService {
    fn from_ref(state: &AppState) -> Self {
        state.tasks.clone()
    }
}

impl FromRef<AppState> for AutomationService {
    fn from_ref(state: &AppState) -> Self {
        state.automations.clone()
    }
}

impl FromRef<AppState> for ChatService {
    fn from_ref(state: &AppState) -> Self {
        state.chats.clone()
    }
}

impl FromRef<AppState> for ShortTermMemoryService {
    fn from_ref(state: &AppState) -> Self {
        state.short_term_memory.clone()
    }
}

impl FromRef<AppState> for VariableService {
    fn from_ref(state: &AppState) -> Self {
        state.variables.clone()
    }
}

impl FromRef<AppState> for SkillRegistryService {
    fn from_ref(state: &AppState) -> Self {
        state.skills.clone()
    }
}

impl FromRef<AppState> for ModelService {
    fn from_ref(state: &AppState) -> Self {
        state.models.clone()
    }
}

impl FromRef<AppState> for PromptService {
    fn from_ref(state: &AppState) -> Self {
        state.prompts.clone()
    }
}

impl FromRef<AppState> for OpenAppService {
    fn from_ref(state: &AppState) -> Self {
        state.open_apps.clone()
    }
}

impl FromRef<AppState> for LlmRuntimeService {
    fn from_ref(state: &AppState) -> Self {
        state.llm_runtime.clone()
    }
}

impl FromRef<AppState> for AppRagService {
    fn from_ref(state: &AppState) -> Self {
        state.rag.clone()
    }
}

impl FromRef<AppState> for AppMemoryService {
    fn from_ref(state: &AppState) -> Self {
        state.memory.clone()
    }
}

impl FromRef<AppState> for MemoryWorkDomainService {
    fn from_ref(state: &AppState) -> Self {
        state.memory_work_domains.clone()
    }
}

impl FromRef<AppState> for AgentRunHistoryService {
    fn from_ref(state: &AppState) -> Self {
        state.agent_runs.clone()
    }
}

impl FromRef<AppState> for AgentManagementService {
    fn from_ref(state: &AppState) -> Self {
        state.agent_management.clone()
    }
}

impl FromRef<AppState> for McpService {
    fn from_ref(state: &AppState) -> Self {
        state.mcp.clone()
    }
}
