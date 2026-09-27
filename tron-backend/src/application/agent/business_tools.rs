mod ask_user;
mod coding_tools;
mod discovery;
mod file_access;
mod memory_tools;
mod platform_tools;
mod productivity_tools;
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
    tools.extend(productivity_tools::tools(resources.clone()));
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

pub(crate) fn framework_local_scope(
    context: &crate::domain::agent::AgentRunContext,
) -> Result<
    eliteforge_agent_official_plugin_tools_local::FileSystemToolsConfig,
    crate::domain::agent::ToolError,
> {
    let cwd = coding_tools::default_cwd_for_context(context);
    // An unlinked legacy host session explicitly had machine-wide path authority.
    // Linked sessions retain exactly their resolved application/worktree grants.
    let roots = if context.execution_scope.allowed_local_directories.is_empty() {
        vec![
            cwd.ancestors()
                .last()
                .ok_or_else(|| {
                    crate::domain::agent::ToolError::ExecutionFailed(
                        "working directory is not absolute".into(),
                    )
                })?
                .to_path_buf(),
        ]
    } else {
        context
            .execution_scope
            .allowed_local_directories
            .iter()
            .map(|root| coding_tools::expand_tilde(root))
            .collect()
    };
    Ok(
        eliteforge_agent_official_plugin_tools_local::FileSystemToolsConfig::new(cwd)
            .with_authorized_roots(roots)
            .with_authorized_symlinks(true)
            .with_max_edit_bytes(32 * 1024 * 1024),
    )
}

pub(crate) fn framework_file_reference(
    reference: serde_json::Value,
    context: &crate::domain::agent::AgentRunContext,
    resources: &crate::application::resource::ResourceService,
    max_bytes: usize,
) -> Result<
    eliteforge_agent_official_plugin_tools_local::ReferencedFile,
    crate::domain::agent::ToolError,
> {
    let source = file_access::resolve_input_file_bounded(
        &serde_json::json!({"fileRef":reference}),
        context,
        resources,
        max_bytes as u64,
    )?;
    if source.bytes.len() > max_bytes {
        return Err(crate::domain::agent::ToolError::ExecutionFailed(
            "attachment exceeds input limit".into(),
        ));
    }
    Ok(
        eliteforge_agent_official_plugin_tools_local::ReferencedFile {
            name: source.file_name,
            bytes: source.bytes,
            reference: source.reference,
        },
    )
}
