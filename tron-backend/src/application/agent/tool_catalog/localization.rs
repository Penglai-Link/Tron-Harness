mod business;
mod coding;

use serde_json::Value;

use crate::{
    domain::agent::{ToolCatalogRecord, ToolDomainRecord, ToolOperationKind},
    shared::locale::SupportedLocale,
};

use super::definitions::{ToolCatalogDefinition, domain_definitions};

const DESCRIPTION_I18N_KEY: &str = "x-i18n-description-key";

pub(super) fn localize_tool_record(
    locale: SupportedLocale,
    definition: ToolCatalogDefinition,
) -> ToolCatalogRecord {
    ToolCatalogRecord {
        tool_code: definition.tool_code.to_string(),
        domain_code: definition.domain_code.to_string(),
        name: translate(locale, definition.name_i18n_key).to_string(),
        description: translate(locale, definition.description_i18n_key).to_string(),
        operation_kind: definition.operation_kind,
        input_schema: materialize_schema(locale, &definition.input_schema),
        module_owner: definition.module_owner.to_string(),
        availability_state: definition.availability_state,
        sample_result_summary: translate(locale, definition.sample_result_summary_i18n_key)
            .to_string(),
    }
}

pub(super) fn localize_domain_record(
    locale: SupportedLocale,
    domain_code: &str,
    items: &[&ToolCatalogRecord],
) -> ToolDomainRecord {
    if let Some(definition) = domain_definitions()
        .into_iter()
        .find(|definition| definition.domain_code.eq_ignore_ascii_case(domain_code))
    {
        return ToolDomainRecord {
            domain_code: definition.domain_code.to_string(),
            name: translate(locale, definition.name_i18n_key).to_string(),
            description: translate(locale, definition.description_i18n_key).to_string(),
            tool_count: items.len(),
            sample_tools: items
                .iter()
                .take(3)
                .map(|tool| tool.tool_code.clone())
                .collect(),
            has_mutation_tools: items
                .iter()
                .any(|tool| tool.operation_kind == ToolOperationKind::Command),
        };
    }

    ToolDomainRecord {
        domain_code: domain_code.to_string(),
        name: format!("{domain_code} Tools"),
        description: match locale {
            SupportedLocale::ZhCn => format!("{domain_code} 领域下的业务工具。"),
            SupportedLocale::EnUs => format!("Business tools under the {domain_code} domain."),
        },
        tool_count: items.len(),
        sample_tools: items
            .iter()
            .take(3)
            .map(|tool| tool.tool_code.clone())
            .collect(),
        has_mutation_tools: items
            .iter()
            .any(|tool| tool.operation_kind == ToolOperationKind::Command),
    }
}

pub(crate) fn materialize_schema(locale: SupportedLocale, schema: &Value) -> Value {
    let mut localized = schema.clone();
    materialize_schema_node(locale, &mut localized);
    localized
}

fn materialize_schema_node(locale: SupportedLocale, node: &mut Value) {
    match node {
        Value::Object(map) => {
            if let Some(Value::String(key)) = map.remove(DESCRIPTION_I18N_KEY) {
                map.insert(
                    "description".to_string(),
                    Value::String(translate(locale, &key).to_string()),
                );
            }
            for value in map.values_mut() {
                materialize_schema_node(locale, value);
            }
        }
        Value::Array(items) => {
            for item in items {
                materialize_schema_node(locale, item);
            }
        }
        _ => {}
    }
}

pub(crate) fn translate(locale: SupportedLocale, key: &str) -> &'static str {
    match locale {
        SupportedLocale::EnUs => translate_en_us(key),
        SupportedLocale::ZhCn => translate_zh_cn(key),
    }
}

fn translate_en_us(key: &str) -> &'static str {
    match key {
        "agent.toolCatalog.domains.automation.name" => "Automation Tools",
        "agent.toolCatalog.domains.automation.description" => {
            "Manage scheduled automations and manual runs."
        }
        "agent.toolCatalog.domains.app.name" => "App Tools",
        "agent.toolCatalog.domains.app.description" => {
            "Browse workspace apps and execution targets."
        }
        "agent.toolCatalog.domains.memory.name" => "Memory Tools",
        "agent.toolCatalog.domains.memory.description" => {
            "Search, save, and soft-delete scoped memories."
        }
        "agent.toolCatalog.domains.model.name" => "Model Tools",
        "agent.toolCatalog.domains.model.description" => {
            "Inspect effective model catalog configuration."
        }
        "agent.toolCatalog.domains.resource.name" => "Resource Tools",
        "agent.toolCatalog.domains.resource.description" => {
            "Manage file, data-source, server, and server-account resources."
        }
        "agent.toolCatalog.tools.resource.query.name" => "Query resources",
        "agent.toolCatalog.tools.resource.query.description" => {
            "List effective workspace resources by type."
        }
        "agent.toolCatalog.tools.resource.query.sampleResultSummary" => {
            "Returns matching resource records."
        }
        "agent.toolCatalog.tools.resource.get.name" => "Get resource",
        "agent.toolCatalog.tools.resource.get.description" => {
            "Read one effective resource with direct secret values redacted."
        }
        "agent.toolCatalog.tools.resource.get.sampleResultSummary" => {
            "Returns one resource detail record."
        }
        "agent.toolCatalog.tools.resource.create.name" => "Create file resource",
        "agent.toolCatalog.tools.resource.create.description" => {
            "Create a file, image, or PDF resource from an agent-local file."
        }
        "agent.toolCatalog.tools.resource.create.sampleResultSummary" => {
            "Returns the created resource code and kind."
        }
        "agent.toolCatalog.tools.resource.update.name" => "Update resource",
        "agent.toolCatalog.tools.resource.update.description" => {
            "Update local resource metadata and inheritance."
        }
        "agent.toolCatalog.tools.resource.update.sampleResultSummary" => {
            "Returns the updated resource code and kind."
        }
        "agent.toolCatalog.tools.resource.delete.name" => "Delete resource",
        "agent.toolCatalog.tools.resource.delete.description" => "Delete a local resource.",
        "agent.toolCatalog.tools.resource.delete.sampleResultSummary" => {
            "Returns whether the local resource was deleted."
        }
        "agent.toolCatalog.tools.resource.binary.name" => "Manage resource file",
        "agent.toolCatalog.tools.resource.binary.description" => {
            "Upload or download the file content of a file resource."
        }
        "agent.toolCatalog.tools.resource.binary.sampleResultSummary" => {
            "Returns file operation status."
        }
        "agent.toolCatalog.tools.resource.data_source.create.name" => "Create data source",
        "agent.toolCatalog.tools.resource.data_source.create.description" => {
            "Create and verify a data-source connection."
        }
        "agent.toolCatalog.tools.resource.data_source.create.sampleResultSummary" => {
            "Returns the created data-source code and kind."
        }
        "agent.toolCatalog.tools.resource.data_source.update.name" => "Update data source",
        "agent.toolCatalog.tools.resource.data_source.update.description" => {
            "Update and re-verify a data-source connection."
        }
        "agent.toolCatalog.tools.resource.data_source.update.sampleResultSummary" => {
            "Returns the updated data-source code and kind."
        }
        "agent.toolCatalog.schemas.resource.dataSource.attributes.description" => {
            "Connection attributes must match the selected resource kind."
        }
        "agent.toolCatalog.tools.resource.server.create.name" => "Create server",
        "agent.toolCatalog.tools.resource.server.create.description" => {
            "Create and verify an SSH server with its initial account."
        }
        "agent.toolCatalog.tools.resource.server.create.sampleResultSummary" => {
            "Returns the created server and initial-account records."
        }
        "agent.toolCatalog.tools.resource.server_account.create.name" => "Create server account",
        "agent.toolCatalog.tools.resource.server_account.create.description" => {
            "Create an account for an effective SSH server."
        }
        "agent.toolCatalog.tools.resource.server_account.create.sampleResultSummary" => {
            "Returns the created server-account code and kind."
        }
        "agent.toolCatalog.domains.skill.name" => "Skill Tools",
        "agent.toolCatalog.domains.skill.description" => {
            "Inspect workspace skill bindings and registry entries."
        }
        "agent.toolCatalog.domains.task.name" => "Task Tools",
        "agent.toolCatalog.domains.task.description" => "Read and create workspace tasks.",
        "agent.toolCatalog.domains.variable.name" => "Variable Tools",
        "agent.toolCatalog.domains.variable.description" => {
            "Inspect effective workspace variables."
        }
        "agent.toolCatalog.domains.workspace.name" => "Workspace Tools",
        "agent.toolCatalog.domains.workspace.description" => {
            "Inspect workspace hierarchy and metadata."
        }
        "agent.toolCatalog.tools.workspace.get.name" => "Get workspace",
        "agent.toolCatalog.tools.workspace.get.description" => {
            "Read workspace metadata by workspace code."
        }
        "agent.toolCatalog.tools.workspace.get.sampleResultSummary" => {
            "Returns one workspace detail record."
        }
        "agent.toolCatalog.tools.workspace.get.params.workspaceCode" => "Workspace code to load.",
        "agent.toolCatalog.tools.memory.search.name" => "Search memories",
        "agent.toolCatalog.tools.memory.search.description" => {
            "Search scoped memories or build a context pack for the current agent run."
        }
        "agent.toolCatalog.tools.memory.search.sampleResultSummary" => {
            "Returns matching memory items and an optional context pack."
        }
        "agent.toolCatalog.tools.memory.search.params.query" => {
            "Search text. Optional when memoryCode is set or when searching accessible scoped memories with an empty query."
        }
        "agent.toolCatalog.tools.memory.search.params.mode" => {
            "SEARCH returns results; CONTEXT_PACK returns baseline injectable memories plus query-related memories."
        }
        "agent.toolCatalog.tools.memory.search.params.limit" => {
            "Maximum number of memories to return. Defaults to 10."
        }
        "agent.toolCatalog.tools.memory.search.params.tokenBudget" => {
            "Maximum context-pack token budget. Defaults to 1200."
        }
        "agent.toolCatalog.tools.memory.search.params.trace" => {
            "Whether to persist a RAG search trace for diagnostics."
        }
        "agent.toolCatalog.tools.memory.save.name" => "Save memory",
        "agent.toolCatalog.tools.memory.save.description" => {
            "Create or update a scoped memory card."
        }
        "agent.toolCatalog.tools.memory.save.sampleResultSummary" => {
            "Returns the saved memory code and write status."
        }
        "agent.toolCatalog.tools.memory.save.params.text" => {
            "Memory text to store. Required for new memories."
        }
        "agent.toolCatalog.tools.memory.save.params.title" => "Optional memory title.",
        "agent.toolCatalog.tools.memory.save.params.semanticKey" => {
            "Stable semantic key used to derive the memory identity when memoryCode is omitted."
        }
        "agent.toolCatalog.tools.memory.save.params.injectable" => {
            "Whether the memory may be included in context packs."
        }
        "agent.toolCatalog.tools.memory.save.params.importanceScore" => {
            "Optional importance score."
        }
        "agent.toolCatalog.tools.memory.save.params.confidenceScore" => {
            "Optional confidence score."
        }
        "agent.toolCatalog.tools.memory.save.params.qualityScore" => "Optional quality score.",
        "agent.toolCatalog.tools.memory.save.params.expiresAt" => {
            "Optional expiration timestamp in milliseconds."
        }
        "agent.toolCatalog.tools.memory.save.params.derivedSourceType" => {
            "Optional upstream source type for derived memories."
        }
        "agent.toolCatalog.tools.memory.save.params.derivedSourceCode" => {
            "Optional upstream source code for derived memories."
        }
        "agent.toolCatalog.tools.memory.save.params.sourceRefs" => {
            "Optional array of source reference objects."
        }
        "agent.toolCatalog.tools.memory.delete.name" => "Delete memory",
        "agent.toolCatalog.tools.memory.delete.description" => {
            "Soft-delete a memory by archiving or disabling it."
        }
        "agent.toolCatalog.tools.memory.delete.sampleResultSummary" => {
            "Returns the memory code and delete action."
        }
        "agent.toolCatalog.tools.memory.delete.params.reason" => {
            "Human-readable reason for deleting the memory."
        }
        "agent.toolCatalog.tools.memory.delete.params.deleteMode" => {
            "ARCHIVE marks the memory archived; DISABLE marks it disabled."
        }
        "agent.toolCatalog.tools.todo.update.name" => "Update session todos",
        "agent.toolCatalog.tools.todo.update.description" => {
            "Replace the current chat session working-state todo list."
        }
        "agent.toolCatalog.tools.todo.update.sampleResultSummary" => {
            "Returns the normalized todo list stored for the session."
        }
        "agent.toolCatalog.tools.todo.update.params.todoCode" => {
            "Optional stable todo code. Omit it for new todos."
        }
        "agent.toolCatalog.tools.todo.update.params.position" => {
            "Zero-based todo order in the session list."
        }
        "agent.toolCatalog.tools.todo.update.params.content" => "Todo content.",
        "agent.toolCatalog.tools.todo.update.params.status" => {
            "Todo status. At most one item may be in_progress."
        }
        "agent.toolCatalog.tools.todo.update.params.priority" => "Todo priority.",
        "agent.toolCatalog.tools.memory.common.params.memoryCode" => {
            "Memory code in U/D/S:scopeCode:sourceCode:sourceVersion format."
        }
        "agent.toolCatalog.tools.memory.common.params.scope" => {
            "Memory scope. USER uses the current user, WORK_DOMAIN uses the current work domain, SPACE uses the current workspace."
        }
        "agent.toolCatalog.tools.memory.common.params.scopeCode" => {
            "Optional explicit scope code. It must belong to the current run context."
        }
        "agent.toolCatalog.tools.memory.common.params.memoryType" => {
            "Optional memory type filter or value."
        }
        "agent.toolCatalog.tools.memory.common.params.memoryStatus" => {
            "Memory status. Searches default to ACTIVE."
        }
        "agent.toolCatalog.tools.memory.common.params.reviewStatus" => {
            "Review status. Searches default to APPROVED."
        }
        "agent.toolCatalog.tools.memory.common.params.metadata" => {
            "Optional memory metadata object."
        }
        "agent.toolCatalog.tools.task.create.name" => "Create task",
        "agent.toolCatalog.tools.task.create.description" => {
            "Create a new task in the current or specified workspace."
        }
        "agent.toolCatalog.tools.task.create.sampleResultSummary" => {
            "Returns the created task detail."
        }
        "agent.toolCatalog.tools.task.create.params.workspaceCode" => {
            "Optional workspace code. Uses the current workspace when omitted."
        }
        "agent.toolCatalog.tools.task.create.params.title" => "Task title.",
        "agent.toolCatalog.tools.task.create.params.description" => "Optional task description.",
        "agent.toolCatalog.tools.task.create.params.priority" => {
            "Task priority. Allowed values: low, medium, high, extra_high."
        }
        "agent.toolCatalog.tools.task.create.params.taskType" => {
            "Task type. Allowed values: feature, bug."
        }
        "agent.toolCatalog.tools.task.create.params.sourceType" => {
            "Source type. Allowed values: internal, github, gitlab."
        }
        "agent.toolCatalog.tools.task.create.params.appCode" => "Optional related app code.",
        "agent.toolCatalog.tools.skill.view.name" => "View skill",
        "agent.toolCatalog.tools.skill.view.description" => {
            "View one workspace skill's progressive-disclosure details, including SKILL.md content and files."
        }
        "agent.toolCatalog.tools.skill.view.sampleResultSummary" => {
            "Returns one workspace skill detail with skillMarkdown and files."
        }
        "agent.toolCatalog.tools.skill.view.params.workspaceCode" => {
            "Optional workspace code. Uses the current workspace when omitted."
        }
        "agent.toolCatalog.tools.skill.view.params.skillName" => "Skill name to view.",
        "agent.toolCatalog.discovery.expand.description" => {
            "Expand one visible tool domain into concrete callable tools."
        }
        "agent.toolCatalog.discovery.expand.params.domainCode" => "Visible domain code to expand.",
        _ => business::translate_en_us(key)
            .or_else(|| coding::translate_en_us(key))
            .unwrap_or_else(key_leak_fallback),
    }
}

fn translate_zh_cn(key: &str) -> &'static str {
    match key {
        "agent.toolCatalog.domains.automation.name" => "自动化工具",
        "agent.toolCatalog.domains.automation.description" => "管理定时自动化和手动执行。",
        "agent.toolCatalog.domains.app.name" => "应用工具",
        "agent.toolCatalog.domains.app.description" => "查看工作区应用和执行目标。",
        "agent.toolCatalog.domains.memory.name" => "记忆工具",
        "agent.toolCatalog.domains.memory.description" => {
            "检索、保存和软删除有作用域的 Agent 记忆。"
        }
        "agent.toolCatalog.domains.model.name" => "模型工具",
        "agent.toolCatalog.domains.model.description" => "查看生效中的模型目录配置。",
        "agent.toolCatalog.domains.resource.name" => "资源工具",
        "agent.toolCatalog.domains.resource.description" => {
            "管理文件、数据源、服务器和服务器账号资源。"
        }
        "agent.toolCatalog.tools.resource.query.name" => "查询资源",
        "agent.toolCatalog.tools.resource.query.description" => "按类型查询工作区生效资源。",
        "agent.toolCatalog.tools.resource.query.sampleResultSummary" => "返回匹配的资源记录。",
        "agent.toolCatalog.tools.resource.get.name" => "获取资源",
        "agent.toolCatalog.tools.resource.get.description" => "读取单个生效资源，并脱敏直接密钥。",
        "agent.toolCatalog.tools.resource.get.sampleResultSummary" => "返回单个资源详情记录。",
        "agent.toolCatalog.tools.resource.create.name" => "创建文件资源",
        "agent.toolCatalog.tools.resource.create.description" => {
            "从 Agent 本地文件创建文件、图片或 PDF 资源。"
        }
        "agent.toolCatalog.tools.resource.create.sampleResultSummary" => {
            "返回已创建资源的编码和类型。"
        }
        "agent.toolCatalog.tools.resource.update.name" => "更新资源",
        "agent.toolCatalog.tools.resource.update.description" => "更新本地资源的元数据和继承设置。",
        "agent.toolCatalog.tools.resource.update.sampleResultSummary" => {
            "返回已更新资源的编码和类型。"
        }
        "agent.toolCatalog.tools.resource.delete.name" => "删除资源",
        "agent.toolCatalog.tools.resource.delete.description" => "删除本地资源。",
        "agent.toolCatalog.tools.resource.delete.sampleResultSummary" => "返回本地资源是否已删除。",
        "agent.toolCatalog.tools.resource.binary.name" => "管理资源文件",
        "agent.toolCatalog.tools.resource.binary.description" => "上传或下载文件资源的文件内容。",
        "agent.toolCatalog.tools.resource.binary.sampleResultSummary" => "返回文件操作状态。",
        "agent.toolCatalog.tools.resource.data_source.create.name" => "创建数据源",
        "agent.toolCatalog.tools.resource.data_source.create.description" => {
            "创建并验证数据源连接。"
        }
        "agent.toolCatalog.tools.resource.data_source.create.sampleResultSummary" => {
            "返回已创建数据源的编码和类型。"
        }
        "agent.toolCatalog.tools.resource.data_source.update.name" => "更新数据源",
        "agent.toolCatalog.tools.resource.data_source.update.description" => {
            "更新并重新验证数据源连接。"
        }
        "agent.toolCatalog.tools.resource.data_source.update.sampleResultSummary" => {
            "返回已更新数据源的编码和类型。"
        }
        "agent.toolCatalog.schemas.resource.dataSource.attributes.description" => {
            "连接属性必须与所选资源类型匹配。"
        }
        "agent.toolCatalog.tools.resource.server.create.name" => "创建服务器",
        "agent.toolCatalog.tools.resource.server.create.description" => {
            "创建并验证带初始账号的 SSH 服务器。"
        }
        "agent.toolCatalog.tools.resource.server.create.sampleResultSummary" => {
            "返回已创建的服务器和初始账号记录。"
        }
        "agent.toolCatalog.tools.resource.server_account.create.name" => "创建服务器账号",
        "agent.toolCatalog.tools.resource.server_account.create.description" => {
            "为生效的 SSH 服务器创建账号。"
        }
        "agent.toolCatalog.tools.resource.server_account.create.sampleResultSummary" => {
            "返回已创建服务器账号的编码和类型。"
        }
        "agent.toolCatalog.domains.skill.name" => "技能工具",
        "agent.toolCatalog.domains.skill.description" => "查看工作区技能绑定和注册项。",
        "agent.toolCatalog.domains.task.name" => "任务工具",
        "agent.toolCatalog.domains.task.description" => "读取并创建工作区任务。",
        "agent.toolCatalog.domains.variable.name" => "变量工具",
        "agent.toolCatalog.domains.variable.description" => "查看工作区生效变量。",
        "agent.toolCatalog.domains.workspace.name" => "工作区工具",
        "agent.toolCatalog.domains.workspace.description" => "查看工作区层级和元数据。",
        "agent.toolCatalog.tools.workspace.get.name" => "获取工作区",
        "agent.toolCatalog.tools.workspace.get.description" => "根据工作区编码读取工作区元数据。",
        "agent.toolCatalog.tools.workspace.get.sampleResultSummary" => "返回单个工作区详情记录。",
        "agent.toolCatalog.tools.workspace.get.params.workspaceCode" => "要读取的工作区编码。",
        "agent.toolCatalog.tools.memory.search.name" => "检索记忆",
        "agent.toolCatalog.tools.memory.search.description" => {
            "检索当前运行上下文可访问的记忆，或生成可注入上下文包。"
        }
        "agent.toolCatalog.tools.memory.search.sampleResultSummary" => {
            "返回匹配的记忆条目和可选上下文包。"
        }
        "agent.toolCatalog.tools.memory.search.params.query" => {
            "检索文本；提供 memoryCode 或使用空查询检索可访问记忆时可省略。"
        }
        "agent.toolCatalog.tools.memory.search.params.mode" => {
            "SEARCH 返回结果列表；CONTEXT_PACK 返回默认可注入记忆和当前 query 相关记忆。"
        }
        "agent.toolCatalog.tools.memory.search.params.limit" => "最多返回的记忆数量，默认 10。",
        "agent.toolCatalog.tools.memory.search.params.tokenBudget" => {
            "上下文包 token 预算上限，默认 1200。"
        }
        "agent.toolCatalog.tools.memory.search.params.trace" => "是否记录 RAG 检索诊断轨迹。",
        "agent.toolCatalog.tools.memory.save.name" => "保存记忆",
        "agent.toolCatalog.tools.memory.save.description" => "创建或更新有作用域的记忆卡片。",
        "agent.toolCatalog.tools.memory.save.sampleResultSummary" => {
            "返回保存后的 memoryCode 和写入状态。"
        }
        "agent.toolCatalog.tools.memory.save.params.text" => "要保存的记忆正文；新建记忆时必填。",
        "agent.toolCatalog.tools.memory.save.params.title" => "可选的记忆标题。",
        "agent.toolCatalog.tools.memory.save.params.semanticKey" => {
            "稳定语义键；未传 memoryCode 时用于派生记忆身份。"
        }
        "agent.toolCatalog.tools.memory.save.params.injectable" => "该记忆是否可进入上下文包。",
        "agent.toolCatalog.tools.memory.save.params.importanceScore" => "可选的重要性评分。",
        "agent.toolCatalog.tools.memory.save.params.confidenceScore" => "可选的置信度评分。",
        "agent.toolCatalog.tools.memory.save.params.qualityScore" => "可选的质量评分。",
        "agent.toolCatalog.tools.memory.save.params.expiresAt" => "可选的过期时间戳，单位毫秒。",
        "agent.toolCatalog.tools.memory.save.params.derivedSourceType" => "可选的派生来源类型。",
        "agent.toolCatalog.tools.memory.save.params.derivedSourceCode" => "可选的派生来源编码。",
        "agent.toolCatalog.tools.memory.save.params.sourceRefs" => "可选的来源引用对象数组。",
        "agent.toolCatalog.tools.memory.delete.name" => "删除记忆",
        "agent.toolCatalog.tools.memory.delete.description" => "通过归档或禁用来软删除记忆。",
        "agent.toolCatalog.tools.memory.delete.sampleResultSummary" => {
            "返回 memoryCode 和删除动作。"
        }
        "agent.toolCatalog.tools.memory.delete.params.reason" => "删除记忆的人类可读原因。",
        "agent.toolCatalog.tools.memory.delete.params.deleteMode" => {
            "ARCHIVE 表示归档；DISABLE 表示禁用。"
        }
        "agent.toolCatalog.tools.todo.update.name" => "更新会话待办",
        "agent.toolCatalog.tools.todo.update.description" => "替换当前聊天会话的工作状态待办列表。",
        "agent.toolCatalog.tools.todo.update.sampleResultSummary" => {
            "返回写入会话后的规范化待办列表。"
        }
        "agent.toolCatalog.tools.todo.update.params.todoCode" => {
            "可选的稳定待办编码；新待办可省略。"
        }
        "agent.toolCatalog.tools.todo.update.params.position" => "会话待办列表中的从 0 开始排序。",
        "agent.toolCatalog.tools.todo.update.params.content" => "待办内容。",
        "agent.toolCatalog.tools.todo.update.params.status" => {
            "待办状态；最多只能有一个 in_progress。"
        }
        "agent.toolCatalog.tools.todo.update.params.priority" => "待办优先级。",
        "agent.toolCatalog.tools.memory.common.params.memoryCode" => {
            "记忆编码，格式为 U/D/S:scopeCode:sourceCode:sourceVersion。"
        }
        "agent.toolCatalog.tools.memory.common.params.scope" => {
            "记忆作用域：USER 使用当前用户，WORK_DOMAIN 使用当前工作域，SPACE 使用当前工作区。"
        }
        "agent.toolCatalog.tools.memory.common.params.scopeCode" => {
            "可选的显式作用域编码，必须属于当前运行上下文。"
        }
        "agent.toolCatalog.tools.memory.common.params.memoryType" => "可选的记忆类型筛选或取值。",
        "agent.toolCatalog.tools.memory.common.params.memoryStatus" => {
            "记忆状态；检索默认使用 ACTIVE。"
        }
        "agent.toolCatalog.tools.memory.common.params.reviewStatus" => {
            "审核状态；检索默认使用 APPROVED。"
        }
        "agent.toolCatalog.tools.memory.common.params.metadata" => "可选的记忆元数据对象。",
        "agent.toolCatalog.tools.task.create.name" => "创建任务",
        "agent.toolCatalog.tools.task.create.description" => "在当前或指定工作区中创建任务。",
        "agent.toolCatalog.tools.task.create.sampleResultSummary" => "返回新创建任务的详情。",
        "agent.toolCatalog.tools.task.create.params.workspaceCode" => {
            "可选工作区编码；缺省时使用当前工作区。"
        }
        "agent.toolCatalog.tools.task.create.params.title" => "任务标题。",
        "agent.toolCatalog.tools.task.create.params.description" => "可选的任务说明。",
        "agent.toolCatalog.tools.task.create.params.priority" => {
            "任务优先级，可选值：low、medium、high、extra_high。"
        }
        "agent.toolCatalog.tools.task.create.params.taskType" => "任务类型，可选值：feature、bug。",
        "agent.toolCatalog.tools.task.create.params.sourceType" => {
            "来源类型，可选值：internal、github、gitlab。"
        }
        "agent.toolCatalog.tools.task.create.params.appCode" => "可选的关联应用编码。",
        "agent.toolCatalog.tools.skill.view.name" => "查看技能",
        "agent.toolCatalog.tools.skill.view.description" => {
            "查看一个工作区技能的渐进式披露详情，包括 SKILL.md 内容和文件列表。"
        }
        "agent.toolCatalog.tools.skill.view.sampleResultSummary" => {
            "返回单个工作区技能详情，包含 skillMarkdown 和 files。"
        }
        "agent.toolCatalog.tools.skill.view.params.workspaceCode" => {
            "可选工作区编码；缺省时使用当前工作区。"
        }
        "agent.toolCatalog.tools.skill.view.params.skillName" => "要查看的技能名称。",
        "agent.toolCatalog.discovery.expand.description" => {
            "展开一个可见工具领域，加载其中可直接调用的具体工具。"
        }
        "agent.toolCatalog.discovery.expand.params.domainCode" => "需要展开的可见领域编码。",
        _ => business::translate_zh_cn(key)
            .or_else(|| coding::translate_zh_cn(key))
            .unwrap_or_else(key_leak_fallback),
    }
}

fn key_leak_fallback() -> &'static str {
    "UNTRANSLATED_AGENT_CATALOG_KEY"
}
