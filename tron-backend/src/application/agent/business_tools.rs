mod ask_user;
mod coding_tools;
mod discovery;
mod memory_tools;
mod platform_tools;
mod short_term_memory_tools;

use crate::domain::agent::{EffectiveTool, Tool, ToolError, ToolRegistry};
use crate::{
    application::{
        app::AppService,
        automation::AutomationService,
        mcp::McpService,
        memory::MemoryService,
        model::ModelService,
        prompt::PromptService,
        rag::{
            RagDiagnosticsRepository, RagDocumentSourceRuntime, RagEmbeddingRuntime,
            RagIngestRepository, RagRetryIndexRepository, RagSearchRepository, RagSearchRuntime,
            RagStrategyRegistry,
        },
        resource::ResourceService,
        short_term_memory::ShortTermMemoryService,
        skill::SkillRegistryService,
        task::TaskService,
        variable::VariableService,
        workspace::WorkspaceService,
    },
    domain::agent::ToolCatalogRecord,
};

pub use ask_user::{CHAT_ASK_USER_TOOL, ask_user_definition};
pub use discovery::{
    DISCOVERY_EXPAND_TOOL_DOMAIN_TOOL, build_domain_discovery_guidance,
    discovery_expand_tool_definition, domain_discovery_runtime_values,
};

pub fn tool_record_to_effective_tool(record: ToolCatalogRecord) -> EffectiveTool {
    EffectiveTool {
        tool_name: record.tool_name,
        domain_code: Some(record.domain_code),
        description: record.description,
        input_schema: record.input_schema,
        skill_name: None,
        required_variable_keys: Vec::new(),
    }
}

pub fn register_default_agent_tools<R, S, E>(
    registry: &ToolRegistry,
    workspaces: WorkspaceService,
    apps: AppService,
    automations: AutomationService,
    tasks: TaskService,
    models: ModelService,
    skills: SkillRegistryService,
    variables: VariableService,
    resources: ResourceService,
    prompts: PromptService,
    mcp: McpService,
    memory: MemoryService<R, S, E>,
    short_term_memory: ShortTermMemoryService,
) -> Result<(), ToolError>
where
    R: RagIngestRepository
        + RagSearchRepository
        + RagDiagnosticsRepository
        + RagRetryIndexRepository
        + Clone
        + Send
        + Sync
        + 'static,
    S: RagStrategyRegistry + Send + Sync + 'static,
    E: RagEmbeddingRuntime + RagSearchRuntime + RagDocumentSourceRuntime + Send + Sync + 'static,
{
    let mut tools: Vec<std::sync::Arc<dyn Tool>> = platform_tools::platform_business_tools(
        workspaces,
        apps,
        automations,
        tasks,
        models,
        skills,
        variables,
        resources.clone(),
        prompts,
        mcp,
    );
    tools.extend(coding_tools::coding_domain_tools(resources.clone()));
    tools.extend(memory_tools::memory_business_tools(memory));
    tools.extend(short_term_memory_tools::short_term_memory_tools(
        short_term_memory,
    ));
    tools.push(discovery::discovery_tool());
    tools.push(ask_user::ask_user_tool());

    for tool in tools {
        registry.register(tool)?;
    }
    Ok(())
}
