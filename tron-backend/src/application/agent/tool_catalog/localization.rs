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
        "agent.toolCatalog.domains.app.name" => "App Tools",
        "agent.toolCatalog.domains.app.description" => {
            "Browse workspace apps and execution targets."
        }
        "agent.toolCatalog.domains.model.name" => "Model Tools",
        "agent.toolCatalog.domains.model.description" => {
            "Inspect effective model catalog configuration."
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
        "agent.toolCatalog.tools.workspace.list.name" => "List workspaces",
        "agent.toolCatalog.tools.workspace.list.description" => {
            "List workspaces visible to the current actor."
        }
        "agent.toolCatalog.tools.workspace.list.sampleResultSummary" => {
            "Returns workspace tree nodes."
        }
        "agent.toolCatalog.tools.workspace.list.params.includeArchived" => {
            "Whether archived workspaces should be included. Defaults to false."
        }
        "agent.toolCatalog.tools.workspace.list.params.search" => {
            "Optional search text matched against workspace labels and metadata."
        }
        "agent.toolCatalog.tools.workspace.list.params.rootOnly" => {
            "Whether only root workspaces should be returned."
        }
        "agent.toolCatalog.tools.workspace.get.name" => "Get workspace",
        "agent.toolCatalog.tools.workspace.get.description" => {
            "Read workspace metadata by workspace code."
        }
        "agent.toolCatalog.tools.workspace.get.sampleResultSummary" => {
            "Returns one workspace detail record."
        }
        "agent.toolCatalog.tools.workspace.get.params.workspaceCode" => "Workspace code to load.",
        "agent.toolCatalog.tools.app.list.name" => "List apps",
        "agent.toolCatalog.tools.app.list.description" => {
            "List apps in the current or specified workspace."
        }
        "agent.toolCatalog.tools.app.list.sampleResultSummary" => "Returns paged app records.",
        "agent.toolCatalog.tools.app.list.params.workspaceCode" => {
            "Optional workspace code. Uses the current workspace when omitted."
        }
        "agent.toolCatalog.tools.app.list.params.search" => "Optional app search text.",
        "agent.toolCatalog.tools.app.list.params.includeArchived" => {
            "Whether archived apps should be included. Defaults to false."
        }
        "agent.toolCatalog.tools.task.list.name" => "List tasks",
        "agent.toolCatalog.tools.task.list.description" => {
            "List tasks in the current or specified workspace."
        }
        "agent.toolCatalog.tools.task.list.sampleResultSummary" => "Returns paged task records.",
        "agent.toolCatalog.tools.task.list.params.workspaceCode" => {
            "Optional workspace code. Uses the current workspace when omitted."
        }
        "agent.toolCatalog.tools.task.list.params.search" => "Optional task search text.",
        "agent.toolCatalog.tools.task.list.params.status" => "Optional task status filter.",
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
            "Task priority. Allowed values: low, medium, high."
        }
        "agent.toolCatalog.tools.task.create.params.taskType" => {
            "Task type. Allowed values: feature, bug."
        }
        "agent.toolCatalog.tools.task.create.params.sourceType" => {
            "Source type. Allowed values: internal, github, gitlab."
        }
        "agent.toolCatalog.tools.task.create.params.appCode" => "Optional related app code.",
        "agent.toolCatalog.tools.model.list.name" => "List models",
        "agent.toolCatalog.tools.model.list.description" => {
            "List effective model definitions for the current or specified workspace."
        }
        "agent.toolCatalog.tools.model.list.sampleResultSummary" => {
            "Returns the effective workspace model catalog."
        }
        "agent.toolCatalog.tools.model.list.params.workspaceCode" => {
            "Optional workspace code. Uses the current workspace when omitted."
        }
        "agent.toolCatalog.tools.skill.list.name" => "List skills",
        "agent.toolCatalog.tools.skill.list.description" => {
            "List workspace skill bindings and registry entries."
        }
        "agent.toolCatalog.tools.skill.list.sampleResultSummary" => {
            "Returns workspace skill records."
        }
        "agent.toolCatalog.tools.skill.list.params.workspaceCode" => {
            "Optional workspace code. Uses the current workspace when omitted."
        }
        "agent.toolCatalog.tools.skill.list.params.search" => "Optional skill search text.",
        "agent.toolCatalog.tools.skill.list.params.includeDisabled" => {
            "Whether disabled skills should be included. Defaults to false."
        }
        "agent.toolCatalog.tools.variable.list.name" => "List variables",
        "agent.toolCatalog.tools.variable.list.description" => {
            "List effective variables in the current or specified workspace."
        }
        "agent.toolCatalog.tools.variable.list.sampleResultSummary" => {
            "Returns effective workspace variables."
        }
        "agent.toolCatalog.tools.variable.list.params.workspaceCode" => {
            "Optional workspace code. Uses the current workspace when omitted."
        }
        "agent.toolCatalog.tools.variable.list.params.search" => "Optional variable search text.",
        "agent.toolCatalog.discovery.expand.description" => {
            "Expand one visible tool domain into concrete callable tools."
        }
        "agent.toolCatalog.discovery.expand.params.domainCode" => "Visible domain code to expand.",
        _ => key_leak_fallback(),
    }
}

fn translate_zh_cn(key: &str) -> &'static str {
    match key {
        "agent.toolCatalog.domains.app.name" => "应用工具",
        "agent.toolCatalog.domains.app.description" => "查看工作区应用和执行目标。",
        "agent.toolCatalog.domains.model.name" => "模型工具",
        "agent.toolCatalog.domains.model.description" => "查看生效中的模型目录配置。",
        "agent.toolCatalog.domains.skill.name" => "技能工具",
        "agent.toolCatalog.domains.skill.description" => "查看工作区技能绑定和注册项。",
        "agent.toolCatalog.domains.task.name" => "任务工具",
        "agent.toolCatalog.domains.task.description" => "读取并创建工作区任务。",
        "agent.toolCatalog.domains.variable.name" => "变量工具",
        "agent.toolCatalog.domains.variable.description" => "查看工作区生效变量。",
        "agent.toolCatalog.domains.workspace.name" => "工作区工具",
        "agent.toolCatalog.domains.workspace.description" => "查看工作区层级和元数据。",
        "agent.toolCatalog.tools.workspace.list.name" => "列出工作区",
        "agent.toolCatalog.tools.workspace.list.description" => "列出当前操作者可见的工作区。",
        "agent.toolCatalog.tools.workspace.list.sampleResultSummary" => "返回工作区树节点列表。",
        "agent.toolCatalog.tools.workspace.list.params.includeArchived" => {
            "是否包含已归档工作区，默认 false。"
        }
        "agent.toolCatalog.tools.workspace.list.params.search" => {
            "可选的搜索文本，用于匹配工作区名称和元数据。"
        }
        "agent.toolCatalog.tools.workspace.list.params.rootOnly" => "是否只返回根工作区。",
        "agent.toolCatalog.tools.workspace.get.name" => "获取工作区",
        "agent.toolCatalog.tools.workspace.get.description" => "根据工作区编码读取工作区元数据。",
        "agent.toolCatalog.tools.workspace.get.sampleResultSummary" => "返回单个工作区详情记录。",
        "agent.toolCatalog.tools.workspace.get.params.workspaceCode" => "要读取的工作区编码。",
        "agent.toolCatalog.tools.app.list.name" => "列出应用",
        "agent.toolCatalog.tools.app.list.description" => "列出当前或指定工作区中的应用。",
        "agent.toolCatalog.tools.app.list.sampleResultSummary" => "返回分页后的应用记录。",
        "agent.toolCatalog.tools.app.list.params.workspaceCode" => {
            "可选工作区编码；缺省时使用当前工作区。"
        }
        "agent.toolCatalog.tools.app.list.params.search" => "可选的应用搜索文本。",
        "agent.toolCatalog.tools.app.list.params.includeArchived" => {
            "是否包含已归档应用，默认 false。"
        }
        "agent.toolCatalog.tools.task.list.name" => "列出任务",
        "agent.toolCatalog.tools.task.list.description" => "列出当前或指定工作区中的任务。",
        "agent.toolCatalog.tools.task.list.sampleResultSummary" => "返回分页后的任务记录。",
        "agent.toolCatalog.tools.task.list.params.workspaceCode" => {
            "可选工作区编码；缺省时使用当前工作区。"
        }
        "agent.toolCatalog.tools.task.list.params.search" => "可选的任务搜索文本。",
        "agent.toolCatalog.tools.task.list.params.status" => "可选的任务状态筛选值。",
        "agent.toolCatalog.tools.task.create.name" => "创建任务",
        "agent.toolCatalog.tools.task.create.description" => "在当前或指定工作区中创建任务。",
        "agent.toolCatalog.tools.task.create.sampleResultSummary" => "返回新创建任务的详情。",
        "agent.toolCatalog.tools.task.create.params.workspaceCode" => {
            "可选工作区编码；缺省时使用当前工作区。"
        }
        "agent.toolCatalog.tools.task.create.params.title" => "任务标题。",
        "agent.toolCatalog.tools.task.create.params.description" => "可选的任务说明。",
        "agent.toolCatalog.tools.task.create.params.priority" => {
            "任务优先级，可选值：low、medium、high。"
        }
        "agent.toolCatalog.tools.task.create.params.taskType" => "任务类型，可选值：feature、bug。",
        "agent.toolCatalog.tools.task.create.params.sourceType" => {
            "来源类型，可选值：internal、github、gitlab。"
        }
        "agent.toolCatalog.tools.task.create.params.appCode" => "可选的关联应用编码。",
        "agent.toolCatalog.tools.model.list.name" => "列出模型",
        "agent.toolCatalog.tools.model.list.description" => {
            "列出当前或指定工作区中生效的模型定义。"
        }
        "agent.toolCatalog.tools.model.list.sampleResultSummary" => "返回工作区生效模型目录。",
        "agent.toolCatalog.tools.model.list.params.workspaceCode" => {
            "可选工作区编码；缺省时使用当前工作区。"
        }
        "agent.toolCatalog.tools.skill.list.name" => "列出技能",
        "agent.toolCatalog.tools.skill.list.description" => "列出工作区技能绑定和注册项。",
        "agent.toolCatalog.tools.skill.list.sampleResultSummary" => "返回工作区技能记录。",
        "agent.toolCatalog.tools.skill.list.params.workspaceCode" => {
            "可选工作区编码；缺省时使用当前工作区。"
        }
        "agent.toolCatalog.tools.skill.list.params.search" => "可选的技能搜索文本。",
        "agent.toolCatalog.tools.skill.list.params.includeDisabled" => {
            "是否包含已禁用技能，默认 false。"
        }
        "agent.toolCatalog.tools.variable.list.name" => "列出变量",
        "agent.toolCatalog.tools.variable.list.description" => "列出当前或指定工作区中的生效变量。",
        "agent.toolCatalog.tools.variable.list.sampleResultSummary" => "返回工作区生效变量列表。",
        "agent.toolCatalog.tools.variable.list.params.workspaceCode" => {
            "可选工作区编码；缺省时使用当前工作区。"
        }
        "agent.toolCatalog.tools.variable.list.params.search" => "可选的变量搜索文本。",
        "agent.toolCatalog.discovery.expand.description" => {
            "展开一个可见工具领域，加载其中可直接调用的具体工具。"
        }
        "agent.toolCatalog.discovery.expand.params.domainCode" => "需要展开的可见领域编码。",
        _ => key_leak_fallback(),
    }
}

fn key_leak_fallback() -> &'static str {
    "UNTRANSLATED_AGENT_CATALOG_KEY"
}
