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

#[cfg(test)]
mod tests {
    use super::{BusinessToolCatalog, dude_workspace_tool};

    #[test]
    fn dude_tools_are_separate_from_workspace_tools() {
        for code in ["dude.memory_search", "dude.memory_save", "dude.memory_delete", "dude.todo_update", "dude.task_page", "dude.task_create", "dude.task_update", "dude.task_delete", "dude.task_restore"] {
            let tool = BusinessToolCatalog::tool(code, "zh-CN").expect("Dude tool exists");
            assert!(tool.tool_name.starts_with("dude_"));
            assert!(tool.domain_code.starts_with("dude_"));
            assert_eq!(tool.category.as_ref().unwrap().code, "tron.dude");
            assert!(tool.input_schema.pointer("/properties/workspaceCode").is_none());
            assert!(tool.input_schema.pointer("/properties/scopeCode").is_none());
        }
        let memory = BusinessToolCatalog::tool("dude.memory_search", "en-US").unwrap();
        assert_eq!(memory.input_schema["properties"]["scope"]["enum"], serde_json::json!(["USER", "WORK_DOMAIN"]));
        assert!(!dude_workspace_tool(&memory));
        for code in ["memory.search", "memory.save", "memory.delete", "todo.update", "task.page", "task.create", "task.update", "task.delete", "task.restore"] {
            let tool = BusinessToolCatalog::tool(code, "en-US").unwrap();
            assert!(dude_workspace_tool(&tool));
            assert!(tool.input_schema["required"].as_array().unwrap().iter().any(|field| field == "workspaceCode"));
        }
        for code in ["rag.search", "app.page", "variable.page", "resource.query"] {
            let tool = BusinessToolCatalog::tool(code, "en-US").unwrap();
            assert!(dude_workspace_tool(&tool));
        }
    }

    #[test]
    fn domain_samples_preserve_actual_call_names_including_aliases() {
        for locale in ["zh-CN", "en-US"] {
            let mut model = BusinessToolCatalog::tool("app.page", locale).unwrap();
            for (domain, call_name) in [
                ("harness_app", "app_page"),
                ("custom.remote", "remote.lookup-v2"),
            ] {
                model.domain_code = domain.into();
                model.tool_name = call_name.into();
                let domains = BusinessToolCatalog::domains_for_records(&[model.clone()], locale);
                assert_eq!(domains.len(), 1);
                assert_eq!(domains[0].sample_tools, vec![call_name.to_owned()]);
                assert!(!domains[0].sample_tools.contains(&model.tool_code));
            }
        }
    }

    #[test]
    fn framework_tools_share_catalog_identity_and_discovery_domains() {
        for descriptor in super::official_remote_descriptors() {
            let record = BusinessToolCatalog::tool(&descriptor.id, "en-US")
                .expect("Framework tool is selectable");
            assert_eq!(record.tool_name, descriptor.call_name);
            assert_eq!(record.input_schema, descriptor.resolve_display("en-US").input_schema);
            assert!(BusinessToolCatalog::tool_codes().contains(&descriptor.id));
            assert!(
                BusinessToolCatalog::list_tools(Some(&record.domain_code), None, "en-US")
                    .iter()
                    .any(|r| r.tool_code == descriptor.id)
            );
        }
        assert_eq!(super::official_remote_descriptors().len(), 8);
    }

    #[test]
    fn platform_variable_crud_tools_are_catalogued_without_workspace_scope() {
        let tools = BusinessToolCatalog::list_tools(Some("harness_variable"), None, "en-US");
        let codes = tools
            .iter()
            .map(|tool| tool.tool_code.as_str())
            .collect::<Vec<_>>();

        assert!(codes.contains(&"platform.variable.page"));
        assert!(codes.contains(&"platform.variable.create"));
        assert!(codes.contains(&"platform.variable.update"));
        assert!(codes.contains(&"platform.variable.delete"));
        let all_codes = BusinessToolCatalog::tool_codes();
        for code in [
            "platform.prompt.page",
            "platform.mcp.page",
            "platform.resource.query",
        ] {
            assert!(
                all_codes.iter().any(|candidate| candidate == code),
                "missing catalog tool {code}"
            );
        }

        let page = BusinessToolCatalog::tool("platform.variable.page", "en-US")
            .expect("platform variable page tool should be catalogued");
        assert!(
            !page
                .input_schema
                .get("properties")
                .and_then(|properties| properties.get("workspaceCode"))
                .is_some()
        );
    }

    #[test]
    fn catalog_keeps_business_code_separate_from_runtime_tool_name() {
        let page = BusinessToolCatalog::tool("app.page", "en-US")
            .expect("app page tool should be catalogued");
        assert_eq!(page.tool_code, "app.page");
        assert_eq!(page.tool_name, "app_page");
        assert!(
            page.tool_name
                .chars()
                .all(|character| character.is_ascii_alphanumeric()
                    || character == '_'
                    || character == '-')
        );

        let payload = serde_json::to_value(&page).expect("catalog record should serialize");
        assert!(payload.get("toolCode").is_some());
        assert_eq!(payload["toolName"], "app_page");
    }

    #[test]
    fn data_source_tools_describe_kind_specific_nested_attributes() {
        let create = BusinessToolCatalog::tool("resource.data_source.create", "en-US")
            .expect("data source create tool should be catalogued");
        let attributes = &create.input_schema["properties"]["attributes"];
        let variants = attributes["oneOf"]
            .as_array()
            .expect("attributes should expose connection variants");
        assert_eq!(variants.len(), 4);
        assert!(variants.iter().any(|variant| {
            variant["title"] == "MysqlConnection"
                && variant["properties"]["credentials"]["properties"]["password"]
                    .get("oneOf")
                    .is_some()
        }));
        assert!(
            create.input_schema["examples"][0]["attributes"]["endpoints"][0]["host"].is_string()
        );

        let update = BusinessToolCatalog::tool("resource.data_source.update", "en-US")
            .expect("data source update tool should be catalogued");
        assert_eq!(
            update.input_schema["properties"]["attributes"]["oneOf"]
                .as_array()
                .map(Vec::len),
            Some(4)
        );
    }
}

pub(crate) fn official_remote_descriptors() -> Vec<eliteforge_agent_core::ToolDescriptor> {
    let mut tools = eliteforge_agent_capability_personai::personai_tool_catalog();
    tools.extend(eliteforge_agent_official_plugin_tools_media::media_tool_catalog());
    tools.push(eliteforge_agent_official_plugin_tools_web::web_read_descriptor());
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
    entries
        .into_iter()
        .map(|(tool, category, source)| {
            let resolved = tool.resolve_display(match locale { SupportedLocale::ZhCn => "zh-CN", SupportedLocale::EnUs => "en-US" });
            let domain = if ["web_search", "web_read"].contains(&tool.call_name.as_str()) {
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
                module_owner: "Framework".into(),
                availability_state: ToolAvailabilityState::Available,
                sample_result_summary: String::new(),
            }
        })
        .collect()
}
