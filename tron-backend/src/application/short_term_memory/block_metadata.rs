//! Host-owned context descriptions. Plugin text comes from ContextContributor metadata.
use std::collections::BTreeMap;

use eliteforge_agent_core::ContextBlockMetadata;

use crate::application::agent::framework_runtime::ContextLayoutContributorInfo;

pub(crate) struct ContextBlockCatalogEntry {
    pub metadata: ContextBlockMetadata,
    pub provider_id: Option<String>,
}

/// The catalog is shared by layout controls and live/historical assembly presentation.
pub(crate) fn context_block_catalog(
    contributors: Vec<ContextLayoutContributorInfo>,
) -> BTreeMap<String, ContextBlockCatalogEntry> {
    let mut catalog = HOST_BLOCKS
        .iter()
        .map(|(id, name, description, zh_name, zh_description)| {
            (
                (*id).to_owned(),
                ContextBlockCatalogEntry {
                    metadata: ContextBlockMetadata::new(*name, *description).with_locale(
                        "zh-CN",
                        *zh_name,
                        *zh_description,
                    ),
                    provider_id: None,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
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

const HOST_BLOCKS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "seed_prompt_prefix",
        "System prompt",
        "Base system and developer instructions, including the usage rules supplied by enabled capabilities.",
        "系统提示词",
        "基础系统提示、开发者指令，以及已启用能力提供的使用规则。",
    ),
    (
        "conversation_history_summary",
        "Conversation summary",
        "A compressed summary of earlier conversation turns, retaining decisions, progress and information needed to continue.",
        "对话历史摘要",
        "较早对话的压缩摘要，保留决策、进展及继续任务所需的信息。",
    ),
    (
        "current_session_history",
        "Conversation history",
        "Recent user and assistant messages, model reasoning retained by the provider, tool calls and their results.",
        "对话历史",
        "近期用户与助手消息、供应商保留的推理内容，以及工具调用和执行结果。",
    ),
    (
        "current_session_working_state",
        "Working state",
        "The current session's goal, plan, tasks, progress and other mutable state used to resume work.",
        "工作状态",
        "当前会话的目标、计划、任务、进度及恢复工作所需的动态状态。",
    ),
    (
        "runtime_identity",
        "Runtime identity",
        "The current agent or Dude's name, identity confirmation state, soul settings and interface language.",
        "运行身份",
        "当前智能体或 Dude 的名称、身份确认状态、灵魂设定及界面语言。",
    ),
    (
        "dude_onboarding",
        "Dude onboarding",
        "The Dude's initialization state, name confirmation and pending interaction; empty once initialization is complete.",
        "Dude 唤醒配置",
        "Dude 的初始化状态、名称确认情况及待处理交互；初始化完成后为空。",
    ),
    (
        "tool_domains",
        "Tool domains",
        "The authorized tool-domain catalog and instructions for discovering the tools needed for a task. Full tool schemas are counted separately.",
        "工具领域",
        "已授权工具领域的目录，以及按任务发现工具的说明；完整工具参数定义单独统计。",
    ),
    (
        "agent_directory",
        "Agent directory",
        "Authorized agents that can receive delegated tasks, with their codes, names and responsibilities.",
        "智能体目录",
        "可委派任务的已授权智能体，包括标识、名称和职责。",
    ),
    (
        "page_context",
        "Page context",
        "Page content, selected text and attached page annotations explicitly supplied for this conversation.",
        "页面上下文",
        "本次对话明确提供的页面内容、选中文本及页面批注。",
    ),
    (
        "application_context",
        "Application execution context",
        "Selected applications, working directories, Git branches, execution environment and loaded project instructions such as AGENTS.md.",
        "应用执行上下文",
        "选中的应用、工作目录、Git 分支、执行环境，以及已加载的 AGENTS.md 等项目指令。",
    ),
    (
        "knowledge_base",
        "Knowledge-base information",
        "Knowledge-base information and retrieved reference material available within the current agent's authorization scope.",
        "知识库信息",
        "当前智能体权限范围内可用的知识库信息，以及检索得到的参考资料。",
    ),
    (
        "dude_environment",
        "Dude environment",
        "The Dude's user and workspace, selected applications, working directory and execution scope.",
        "Dude 运行环境",
        "Dude 所属用户和工作区、选中的应用、工作目录及执行范围。",
    ),
    (
        "dude_wakeup",
        "Current trigger",
        "The event, notification or scheduled task that triggered this Dude run, and its related input.",
        "本轮触发信息",
        "触发本轮 Dude 运行的事件、通知或定时任务，以及相关输入。",
    ),
    (
        "agent.directory-guidance",
        "Agent directory instructions",
        "Rules for selecting authorized agents and delegating tasks through the agent directory.",
        "智能体目录指令",
        "通过智能体目录选择已授权智能体并委派任务的规则。",
    ),
    (
        "dude.guidance",
        "Dude instructions",
        "Instructions governing Dude initialization, background work, progress and notifications.",
        "Dude 指令",
        "管理 Dude 初始化、后台工作、进展汇报和通知的规则。",
    ),
    (
        "protocol",
        "Tool definitions",
        "Names, descriptions and parameter schemas of the tools sent to the model. These protocol tokens are deducted before allocating context-block shares.",
        "工具定义",
        "实际发送给模型的工具名称、描述和参数结构。其协议开销先从输入窗口扣除，再按比例分配各上下文块预算。",
    ),
];
