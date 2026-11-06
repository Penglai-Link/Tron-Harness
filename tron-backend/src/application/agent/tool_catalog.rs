mod definitions;
mod localization;

use std::collections::BTreeMap;

use crate::{
    domain::agent::{ToolCatalogRecord, ToolDomainRecord},
    shared::locale::SupportedLocale,
};

use self::{
    definitions::tool_definitions,
    localization::{localize_domain_record, localize_tool_record},
};

#[derive(Default)]
pub struct BusinessToolCatalog;

pub(crate) const HARNESS_TOOL_PROVIDER_ID: &str = "HarnessTools";

/// Legacy workspace tool assignments resolve to Dude-specific implementations.
pub(crate) fn dude_tool_replacement(code: &str) -> Option<&'static str> {
    match code {
        "memory.search" => Some("dude.memory_search"),
        "memory.save" => Some("dude.memory_save"),
        "memory.delete" => Some("dude.memory_delete"),
        "todo.update" => Some("dude.todo_update"),
        "task.get" => Some("dude.task_get"),
        "task.page" => Some("dude.task_page"),
        "task.create" => Some("dude.task_create"),
        "task.update" => Some("dude.task_update"),
        "task.delete" => Some("dude.task_delete"),
        "task.restore" => Some("dude.task_restore"),
        _ => None,
    }
}

pub(crate) fn dude_workspace_tool(tool: &ToolCatalogRecord) -> bool {
    matches!(tool.tool_code.as_str(), "memory.search" | "memory.save" | "memory.delete")
        || tool.input_schema.pointer("/properties/workspaceCode").is_some()
        || matches!(tool.domain_code.as_str(), "harness_rag" | "harness_app" | "harness_resource" | "harness_variable" | "harness_automation" | "harness_workspace" | "harness_prompt" | "harness_mcp" | "harness_worktree" | "chat_attachment" | "word" | "excel" | "fs" | "search" | "terminal" | "command")
}

pub(crate) fn is_fixed_code(code: &str) -> bool {
    static CODES: std::sync::OnceLock<std::collections::HashSet<String>> =
        std::sync::OnceLock::new();
    CODES
        .get_or_init(|| BusinessToolCatalog::tool_codes().into_iter().collect())
        .contains(code)
}

pub(crate) fn localize_plugin_display(record: &mut ToolCatalogRecord, locale: &str) {
    if let Some(display) = &record.display {
        let resolved = display.resolve(locale, &record.name, &record.description, &record.input_schema);
        record.name = resolved.name;
        record.description = resolved.description;
        record.input_schema = resolved.input_schema;
    }
}

pub(crate) fn builtin_category(code: &str) -> eliteforge_agent_core::ToolCategory {
    let (zh, en) = match code {
        "tron.platform" => ("平台工具", "Platform tools"),
        "tron.dude" => ("Dude 工具", "Dude tools"),
        "system.command" => ("系统命令工具", "System command tools"),
        "document.media" => ("文档与媒体工具", "Document and media tools"),
        "network" => ("网络检索工具", "Web retrieval tools"),
        "agent.collaboration" => ("协作工具", "Collaboration tools"),
        "interaction" => ("交互工具", "Interaction tools"),
        "execution" => ("执行工具", "Execution tools"),
        "tron.connector.tools.platform" => ("外部平台", "External platforms"),
        "tron.connector.tools.topic" => ("主题", "Topics"),
        _ => (code, code),
    };
    eliteforge_agent_core::ToolCategory::new(
        code,
        [("zh-CN".into(), zh.into()), ("en-US".into(), en.into())]
            .into_iter()
            .collect(),
    )
}

impl BusinessToolCatalog {
    pub fn domains(locale: &str) -> Vec<ToolDomainRecord> {
        Self::domains_for_records(&Self::list_tools(None, None, locale), locale)
    }

    pub fn domains_for_records(
        records: &[ToolCatalogRecord],
        locale: &str,
    ) -> Vec<ToolDomainRecord> {
        let locale = SupportedLocale::from_raw(locale);
        let mut grouped = BTreeMap::<String, Vec<&ToolCatalogRecord>>::new();
        for tool in records {
            grouped
                .entry(tool.domain_code.clone())
                .or_default()
                .push(tool);
        }

        grouped
            .into_iter()
            .map(|(domain_code, items)| localize_domain_record(locale, &domain_code, &items))
            .collect()
    }

    pub fn list_tools(
        domain_code: Option<&str>,
        search: Option<&str>,
        locale: &str,
    ) -> Vec<ToolCatalogRecord> {
        let locale = SupportedLocale::from_raw(locale);
        let normalized_domain = domain_code
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_ascii_lowercase());
        let normalized_search = search
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_ascii_lowercase());

        tool_definitions()
            .into_iter()
            .map(|definition| localize_tool_record(locale, definition))
            .chain(official_remote_records(locale))
            .filter(|tool| {
                normalized_domain
                    .as_ref()
                    .is_none_or(|target| tool.domain_code.eq_ignore_ascii_case(target))
            })
            .filter(|tool| {
                normalized_search.as_ref().is_none_or(|query| {
                    format!(
                        "{} {} {} {} {}",
                        tool.tool_code,
                        tool.domain_code,
                        tool.name,
                        tool.description,
                        tool.module_owner
                    )
                    .to_ascii_lowercase()
                    .contains(query)
                })
            })
            .collect()
    }

    pub fn tool(tool_code: &str, locale: &str) -> Option<ToolCatalogRecord> {
        if let Some(tool) = official_remote_records(SupportedLocale::from_raw(locale))
            .into_iter()
            .find(|t| t.tool_code.eq_ignore_ascii_case(tool_code))
        {
            return Some(tool);
        }
        let definition = Self::tool_definition(tool_code)?;
        Some(localize_tool_record(
            SupportedLocale::from_raw(locale),
            definition,
        ))
    }

    pub fn tool_codes() -> Vec<String> {
        tool_definitions()
            .into_iter()
            .map(|tool| tool.tool_code.to_string())
            .chain(
                official_remote_descriptors()
                    .into_iter()
                    .map(|tool| tool.id),
            )
            .collect()
    }

    fn tool_definition(tool_code: &str) -> Option<definitions::ToolCatalogDefinition> {
        let code = tool_code.trim();
        if code.is_empty() {
            return None;
        }
        tool_definitions()
            .into_iter()
            .find(|tool| tool.tool_code.eq_ignore_ascii_case(code))
    }
}

pub(crate) use localization::{materialize_schema, translate};



pub(crate) fn official_remote_descriptors() -> Vec<eliteforge_agent_core::ToolDescriptor> {
    let mut tools = eliteforge_agent_capability_personai::personai_tool_catalog();
    tools.extend(eliteforge_agent_official_plugin_tools_media::media_tool_catalog());
    tools.push(eliteforge_agent_official_plugin_tools_web::web_read_descriptor());
    tools.extend(eliteforge_agent_connector::connector_tool_catalog().into_iter().map(|entry| entry.descriptor));
    tools.push(crate::application::topic::topic_read_tool_descriptor());
    tools
}
fn official_remote_records(locale: SupportedLocale) -> Vec<ToolCatalogRecord> {
    use crate::domain::agent::{ToolAvailabilityState, ToolOperationKind};
    let source = |provider: &str, plugin: &str, pack: &str| eliteforge_agent_core::ToolSource {
        provider_id: Some(provider.into()),
        plugin_id: Some(plugin.into()),
        pack_id: Some(pack.into()),
        adapter_id: Some("tron.remote.tools".into()),
    };
    let mut entries = eliteforge_agent_capability_personai::personai_tool_catalog_with_categories()
        .into_iter()
        .map(|(tool, category)| {
            (
                tool,
                category,
                source(
                    "PersonAiToolProvider",
                    "capability.personai",
                    "framework.personai",
                ),
            )
        })
        .collect::<Vec<_>>();
    entries.extend(
        eliteforge_agent_official_plugin_tools_media::media_tool_catalog()
            .into_iter()
            .map(|tool| {
                (
                    tool,
                    eliteforge_agent_official_plugin_tools_media::media_tool_category(),
                    source(
                        "MediaToolsPlugin",
                        "framework.media.tools",
                        "framework.media",
                    ),
                )
            }),
    );
    entries.push((
        eliteforge_agent_official_plugin_tools_web::web_read_descriptor(),
        eliteforge_agent_official_plugin_tools_web::web_tool_category(),
        source("WebToolsPlugin", "framework.web.tools", "framework.web"),
    ));
    entries.extend(eliteforge_agent_connector::connector_tool_catalog().into_iter().map(|entry| (
        entry.descriptor,
        builtin_category("tron.connector.tools.platform"),
        eliteforge_agent_core::ToolSource {
            provider_id: Some("ConnectorTools".into()),
            plugin_id: Some("tron.connector.tools".into()),
            pack_id: Some("framework.connector".into()),
            adapter_id: Some("tron.connector.tools".into()),
        },
    )));
    entries.push((
        crate::application::topic::topic_read_tool_descriptor(),
        builtin_category("tron.connector.tools.topic"),
        eliteforge_agent_core::ToolSource { provider_id: Some("TopicTools".into()), plugin_id: Some("tron.connector.tools".into()), pack_id: None, adapter_id: Some("tron.connector.tools".into()) },
    ));
    entries
        .into_iter()
        .map(|(tool, category, source)| {
            let resolved = tool.resolve_display(match locale { SupportedLocale::ZhCn => "zh-CN", SupportedLocale::EnUs => "en-US" });
            let domain = if tool.call_name == "topic_read" {
                "topic"
            } else if tool.call_name.starts_with("connector_") {
                "connectors"
            } else if ["web_search", "web_read"].contains(&tool.call_name.as_str()) {
                "web_search"
            } else {
                "media"
            };
            ToolCatalogRecord {
                display: tool.display.clone(),
                tool_code: tool.id.clone(),
                tool_name: tool.call_name.clone(),
                domain_code: domain.into(),
                category: Some(category),
                source,
                name: resolved.name,
                description: resolved.description,
                operation_kind: match tool.operation_kind {
                    eliteforge_agent_core::ToolOperationKind::Query => ToolOperationKind::Query,
                    eliteforge_agent_core::ToolOperationKind::Command => ToolOperationKind::Command,
                    eliteforge_agent_core::ToolOperationKind::Unknown => ToolOperationKind::Unknown,
                },
                input_schema: resolved.input_schema,
                module_owner: if tool.call_name == "topic_read" { "Tron" } else { "Framework" }.into(),
                availability_state: ToolAvailabilityState::Available,
                sample_result_summary: String::new(),
            }
        })
        .collect()
}
