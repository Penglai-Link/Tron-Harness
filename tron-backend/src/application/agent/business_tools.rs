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
        memory::MemoryService,
        model::ModelService,
        rag::{
            RagDiagnosticsRepository, RagDocumentSourceRuntime, RagEmbeddingRuntime,
            RagIngestRepository, RagRetryIndexRepository, RagSearchRepository, RagSearchRuntime,
            RagStrategyRegistry,
        },
        short_term_memory::ShortTermMemoryService,
        skill::SkillRegistryService,
        task::TaskService,
        variable::VariableService,
        workspace::WorkspaceService,
    },
    domain::agent::ToolCatalogRecord,
};

pub use discovery::{
    DISCOVERY_EXPAND_TOOL_DOMAIN_TOOL, build_domain_discovery_guidance,
    discovery_expand_tool_definition,
};

pub fn tool_record_to_effective_tool(record: ToolCatalogRecord) -> EffectiveTool {
    EffectiveTool {
        tool_name: record.tool_code,
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
    );
    tools.extend(coding_tools::coding_domain_tools());
    tools.extend(memory_tools::memory_business_tools(memory));
    tools.extend(short_term_memory_tools::short_term_memory_tools(
        short_term_memory,
    ));
    tools.push(discovery::discovery_tool());

    for tool in tools {
        registry.register(tool)?;
    }
    Ok(())
}
