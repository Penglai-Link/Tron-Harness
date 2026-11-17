//! Host-owned context descriptions. Plugin text comes from ContextContributor metadata.
use std::collections::BTreeMap;

use super::{ContextProvidersConfig, ContextReductionStrategy};
use eliteforge_agent_core::{
    ContextBlockMetadata, ContextBlockUpdateMode, ContextCompressionMetadata,
};

use crate::application::agent::framework_runtime::ContextLayoutContributorInfo;

pub(crate) struct ContextBlockCatalogEntry {
    pub metadata: ContextBlockMetadata,
    pub provider_id: Option<String>,
}

/// The catalog is shared by layout controls and live/historical assembly presentation.
pub(crate) fn context_block_catalog(
    contributors: Vec<ContextLayoutContributorInfo>,
    providers: &ContextProvidersConfig,
) -> BTreeMap<String, ContextBlockCatalogEntry> {
    let mut catalog = HOST_BLOCKS
        .iter()
        .map(|(id, ..)| {
            (
                (*id).to_owned(),
                ContextBlockCatalogEntry {
                    metadata: configured_context_block_metadata(id, providers)
                        .expect("declared host context block"),
                    provider_id: None,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    // Plugin metadata wins, just as plugin reducers win during assembly.
    for contributor in contributors {
        catalog.insert(
            contributor.block_id,
            ContextBlockCatalogEntry {
                metadata: contributor.metadata,
                provider_id: Some(contributor.provider_id),
            },
        );
    }
    for (alias, id) in [
        ("framework.instructions", "seed_prompt_prefix"),
        ("framework.history", "current_session_history"),
        ("current_session_checkpoint", "conversation_history_summary"),
    ] {
        if let Some(entry) = catalog.get(id) {
            catalog.insert(
                alias.into(),
                ContextBlockCatalogEntry {
                    metadata: entry.metadata.clone(),
                    provider_id: entry.provider_id.clone(),
                },
            );
        }
    }
    catalog
}

/// Metadata used both by native contributors and the presentation catalog.
pub(crate) fn native_context_block_metadata(id: &str) -> Option<ContextBlockMetadata> {
    configured_context_block_metadata(id, &ContextProvidersConfig::default())
}

fn configured_context_block_metadata(
    id: &str,
    providers: &ContextProvidersConfig,
) -> Option<ContextBlockMetadata> {
    let (_, name, description, zh_name, zh_description, compression) =
        HOST_BLOCKS.iter().find(|(block, ..)| *block == id)?;
    let mode = match id {
        "current_session_history" => ContextBlockUpdateMode::Append,
        "seed_prompt_prefix" | "agent.directory-guidance" | "assistant.guidance" => {
            ContextBlockUpdateMode::Fixed
        }
        _ => ContextBlockUpdateMode::Replace,
    };
    Some(
        ContextBlockMetadata::new(*name, *description)
            .with_locale("zh-CN", *zh_name, *zh_description)
            .with_update_mode(mode)
            .with_compression(compression.resolve(id, providers)),
    )
}

// Each native source declares its semantics with its other presentation metadata.
// Data sources resolve the policy already used by assembly; plugins supply their own metadata.
enum CompressionSource {
    Keep,
    DataPolicy,
    History,
    HistorySummary,
    Working,
    Unspecified,
}

impl CompressionSource {
    fn resolve(&self, id: &str, providers: &ContextProvidersConfig) -> ContextCompressionMetadata {
        match self {
            Self::Keep => ContextCompressionMetadata::keep(),
            Self::DataPolicy => providers
                .data_blocks
                .get(id)
                .map(|block| block.policy.compression_metadata())
                .unwrap_or_default(),
            Self::History => providers.history.compression_metadata(),
            Self::HistorySummary
                if providers.history.strategy == ContextReductionStrategy::Checkpoint =>
            {
                ContextCompressionMetadata::coordinated_summary()
            }
            Self::HistorySummary => ContextCompressionMetadata::keep(),
            Self::Working => providers.working.compression_metadata(),
            Self::Unspecified => ContextCompressionMetadata::default(),
        }
    }
}

const HOST_BLOCKS: &[(&str, &str, &str, &str, &str, CompressionSource)] = &[
    (
        "seed_prompt_prefix",
        "System prompt",
        "Base system and developer instructions, including the usage rules supplied by enabled capabilities.",
        "系统提示词",
        "基础系统提示、开发者指令，以及已启用能力提供的使用规则。",
        CompressionSource::Keep,
    ),
    (
        "conversation_history_summary",
        "Conversation summary",
        "A compressed summary of earlier conversation turns, retaining decisions, progress and information needed to continue.",
        "对话历史摘要",
        "较早对话的压缩摘要，保留决策、进展及继续任务所需的信息。",
        CompressionSource::HistorySummary,
    ),
    (
        "current_session_history",
        "Conversation history",
        "Recent user and assistant messages, model reasoning retained by the provider, tool calls and their results.",
        "对话历史",
        "近期用户与助手消息、供应商保留的推理内容，以及工具调用和执行结果。",
        CompressionSource::History,
    ),
    (
        "current_session_working_state",
        "Working state",
        "The current session's goal, plan, tasks, progress and other mutable state used to resume work.",
        "工作状态",
        "当前会话的目标、计划、任务、进度及恢复工作所需的动态状态。",
        CompressionSource::Working,
    ),
    (
        "runtime_identity",
        "Runtime identity",
        "The current agent or PersonAI's name, identity confirmation state, soul settings and interface language.",
        "运行身份",
        "当前智能体或 PersonAI 的名称、身份确认状态、灵魂设定及界面语言。",
        CompressionSource::DataPolicy,
    ),
    (
        "assistant_onboarding",
        "PersonAI onboarding",
        "The PersonAI's initialization state, name confirmation and pending interaction; empty once initialization is complete.",
        "PersonAI 唤醒配置",
        "PersonAI 的初始化状态、名称确认情况及待处理交互；初始化完成后为空。",
        CompressionSource::DataPolicy,
    ),
    (
        "tool_domains",
        "Tool domains",
        "The authorized tool-domain catalog and instructions for discovering the tools needed for a task. Full tool schemas are counted separately.",
        "工具领域",
        "已授权工具领域的目录，以及按任务发现工具的说明；完整工具参数定义单独统计。",
        CompressionSource::DataPolicy,
    ),
    (
        "agent_directory",
        "Agent directory",
        "Authorized agents that can receive delegated tasks, with their codes, names and responsibilities.",
        "智能体目录",
        "可委派任务的已授权智能体，包括标识、名称和职责。",
        CompressionSource::DataPolicy,
    ),
    (
        "page_context",
        "Page context",
        "The latest user input's page location snapshot: title, path, workspace and selected object. An input without a page snapshot clears this block.",
        "页面上下文",
        "最新一条用户消息发送时的页面位置快照，包括标题、路径、工作区和选中对象；该条消息没有页面快照时清空本块。",
        CompressionSource::DataPolicy,
    ),
    (
        "application_context",
        "Application execution context",
        "Selected applications, working directories, Git branches, execution environment and loaded project instructions such as AGENTS.md.",
        "应用执行上下文",
        "选中的应用、工作目录、Git 分支、执行环境，以及已加载的 AGENTS.md 等项目指令。",
        CompressionSource::DataPolicy,
    ),
    (
        "topic_memo",
        "Topic notes and references",
        "The current topic's notes and resource references, refreshed for each model request.",
        "主题说明与引用",
        "当前主题的说明和资源引用，在每次模型请求时刷新。",
        CompressionSource::Keep,
    ),
    (
        "long_term_memory",
        "Long-term memory",
        "Relevant enabled memories, with original sources and retrieval diagnostics.",
        "长期记忆",
        "按相关性召回的启用记忆，展示原始来源、实际用量和检索异常。",
        CompressionSource::DataPolicy,
    ),
    (
        "knowledge_base",
        "Knowledge-base information",
        "Knowledge-base information and retrieved reference material available within the current agent's authorization scope.",
        "知识库信息",
        "当前智能体权限范围内可用的知识库信息，以及检索得到的参考资料。",
        CompressionSource::DataPolicy,
    ),
    (
        "assistant_environment",
        "PersonAI environment",
        "The PersonAI's user and workspace, selected applications, working directory and execution scope.",
        "PersonAI 运行环境",
        "PersonAI 所属用户和工作区、选中的应用、工作目录及执行范围。",
        CompressionSource::DataPolicy,
    ),
    (
        "assistant_wakeup",
        "Current trigger",
        "The event, notification or scheduled task that triggered this PersonAI run, and its related input.",
        "本轮触发信息",
        "触发本轮 PersonAI 运行的事件、通知或定时任务，以及相关输入。",
        CompressionSource::DataPolicy,
    ),
    (
        "agent.directory-guidance",
        "Agent directory instructions",
        "Rules for selecting authorized agents and delegating tasks through the agent directory.",
        "智能体目录指令",
        "通过智能体目录选择已授权智能体并委派任务的规则。",
        CompressionSource::Keep,
    ),
    (
        "assistant.guidance",
        "PersonAI instructions",
        "Instructions governing PersonAI initialization, background work, progress and notifications.",
        "PersonAI 指令",
        "管理 PersonAI 初始化、后台工作、进展汇报和通知的规则。",
        CompressionSource::Keep,
    ),
    (
        "protocol",
        "Tool definitions",
        "Names, descriptions and parameter schemas of the tools sent to the model. These protocol tokens are deducted before allocating context-block shares.",
        "工具定义",
        "实际发送给模型的工具名称、描述和参数结构。其协议开销先从输入窗口扣除，再按比例分配各上下文块预算。",
        CompressionSource::Unspecified,
    ),
];
