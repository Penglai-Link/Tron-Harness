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
    use super::BusinessToolCatalog;

    #[test]
    fn platform_variable_crud_tools_are_catalogued_without_workspace_scope() {
        let tools = BusinessToolCatalog::list_tools(Some("variable"), None, "en-US");
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
            "platform.model.page",
            "platform.prompt.page",
            "platform.mcp.page",
            "platform.open_app.page",
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
        assert!(page
            .tool_name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_' || character == '-'));

        let payload = serde_json::to_value(&page).expect("catalog record should serialize");
        assert!(payload.get("toolCode").is_some());
        assert!(payload.get("toolName").is_none());
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
