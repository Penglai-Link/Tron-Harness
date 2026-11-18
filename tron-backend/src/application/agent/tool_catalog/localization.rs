use std::borrow::Cow;

use serde_json::Value;

use crate::{
    domain::agent::{ToolCatalogRecord, ToolDomainRecord, ToolOperationKind},
    shared::locale::SupportedLocale,
};

use super::definitions::{ToolCatalogDefinition, domain_definitions};

const DESCRIPTION_I18N_KEY: &str = "x-i18n-description-key";
/// Marks a JSON string value as a translation key that is resolved when the
/// input schema is materialized, so static schema examples stay out of source.
const INLINE_I18N_PREFIX: &str = "__i18n__";

pub(super) fn localize_tool_record(
    locale: SupportedLocale,
    definition: ToolCatalogDefinition,
) -> ToolCatalogRecord {
    let mut record = ToolCatalogRecord {
        display: None,
        tool_code: definition.tool_code.to_string(),
        tool_name: definition.tool_name,
        domain_code: definition.domain_code.to_string(),
        category: Some(super::builtin_category(definition.category_code)),
        source: eliteforge_agent_core::ToolSource {
            provider_id: Some(
                if matches!(definition.category_code, "tron.platform" | "tron.assistant") {
                    super::HARNESS_TOOL_PROVIDER_ID
                } else {
                    definition.module_owner
                }
                .to_owned(),
            ),
            ..Default::default()
        },
        name: translate(locale, definition.name_i18n_key).into_owned(),
        description: translate(locale, definition.description_i18n_key).into_owned(),
        operation_kind: definition.operation_kind,
        input_schema: materialize_schema(locale, &definition.input_schema),
        module_owner: definition.module_owner.to_string(),
        availability_state: definition.availability_state,
        sample_result_summary: translate(locale, definition.sample_result_summary_i18n_key)
            .into_owned(),
    };
    // Host aliases of official assistant tools keep the shared catalog identity
    // but present the host's own text, which lives in the shared resources too.
    if record.domain_code == "assistant_memory" {
        record.description = tron_i18n::t!(
            "agent.toolCatalog.records.assistant_memory.description",
            locale = locale
        )
        .into_owned();
        record.input_schema["description"] = record.description.clone().into();
        if let Some(scope) = record.input_schema.pointer_mut("/properties/scope") {
            scope["description"] = tron_i18n::t!(
                "agent.toolCatalog.records.assistant_memory.params.scope",
                locale = locale
            )
            .into_owned()
            .into();
        }
    } else if record.domain_code == "assistant_task" {
        let scope = tron_i18n::t!(
            "agent.toolCatalog.records.assistant_task.scope",
            locale = locale
        );
        let description = match record.tool_name.as_str() {
            "assistant_task_page" => tron_i18n::t!(
                "agent.toolCatalog.records.assistant_task.page.description",
                locale = locale
            )
            .into_owned(),
            "assistant_task_create" => tron_i18n::t!(
                "agent.toolCatalog.records.assistant_task.create.description",
                locale = locale
            )
            .into_owned(),
            _ => record.description.clone(),
        };
        record.description = format!("{scope} {description}")
            .replace("task_get", "assistant_task_get")
            .replace("task_update", "assistant_task_update")
            .replace("task_restore", "assistant_task_restore");
        record.input_schema["description"] = record.description.clone().into();
    }
    // These are explicit host aliases for official local bindings. Resources
    // come from that provider, while Tron keeps its stable ACL identities.
    if matches!(definition.module_owner, "coding" | "terminal") {
        static CATALOG: std::sync::OnceLock<Vec<eliteforge_agent_core::ToolDescriptor>> =
            std::sync::OnceLock::new();
        if let Some(descriptor) = CATALOG
            .get_or_init(eliteforge_agent_official_plugin_tools_local::local_tool_catalog)
            .iter()
            .find(|d| d.call_name == record.tool_name)
        {
            record.display = descriptor.display.clone();
            let resolved = descriptor.resolve_display(locale.as_str());
            record.name = resolved.name;
            record.description = resolved.description;
            record.input_schema = resolved.input_schema;
        }
    }
    record
}

pub(super) fn localize_domain_record(
    locale: SupportedLocale,
    domain_code: &str,
    items: &[&ToolCatalogRecord],
) -> ToolDomainRecord {
    // The directory is the model's only preview of a domain before expansion.
    // A wide window keeps run-leased dynamic tools visible without another
    // round trip; large domains still stay bounded.
    let sample_tools = items
        .iter()
        .take(SAMPLE_TOOL_WINDOW)
        .map(|tool| tool.tool_name.clone())
        .collect::<Vec<_>>();
    if let Some(definition) = domain_definitions()
        .into_iter()
        .find(|definition| definition.domain_code.eq_ignore_ascii_case(domain_code))
    {
        return ToolDomainRecord {
            domain_code: definition.domain_code.to_string(),
            name: translate(locale, definition.name_i18n_key).into_owned(),
            description: translate(locale, definition.description_i18n_key).into_owned(),
            tool_count: items.len(),
            sample_tools,
            has_mutation_tools: items
                .iter()
                .any(|tool| tool.operation_kind == ToolOperationKind::Command),
        };
    }

    ToolDomainRecord {
        domain_code: domain_code.to_string(),
        name: if domain_code == "media" {
            tron_i18n::t!("agent.toolCatalog.domains.media.name", locale = locale).into_owned()
        } else {
            tron_i18n::t!(
                "agent.toolCatalog.domains.fallback.name",
                locale = locale,
                code = domain_code
            )
        },
        description: tron_i18n::t!(
            "agent.toolCatalog.domains.fallback.description",
            locale = locale,
            tools = sample_tools.join(", ")
        ),
        tool_count: items.len(),
        sample_tools,
        has_mutation_tools: items
            .iter()
            .any(|tool| tool.operation_kind == ToolOperationKind::Command),
    }
}

const SAMPLE_TOOL_WINDOW: usize = 8;

pub(crate) fn materialize_schema(locale: SupportedLocale, schema: &Value) -> Value {
    let mut localized = schema.clone();
    materialize_schema_node(locale, &mut localized);
    localized
}

fn materialize_schema_node(locale: SupportedLocale, node: &mut Value) {
    // Inline translation sentinels reach here from static schema examples.
    if node.is_string() {
        let key = node
            .as_str()
            .and_then(|text| text.strip_prefix(INLINE_I18N_PREFIX))
            .map(str::to_owned);
        if let Some(key) = key {
            *node = Value::String(translate(locale, &key).into_owned());
        }
        return;
    }
    match node {
        Value::Object(map) => {
            if let Some(Value::String(key)) = map.remove(DESCRIPTION_I18N_KEY) {
                map.insert(
                    "description".to_string(),
                    Value::String(translate(locale, &key).into_owned()),
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

pub(crate) fn translate<'a>(locale: SupportedLocale, key: &'a str) -> Cow<'a, str> {
    tron_i18n::translate(locale, key)
}
