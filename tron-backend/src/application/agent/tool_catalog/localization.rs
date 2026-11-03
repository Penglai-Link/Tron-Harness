mod catalog;
mod knowledge_quality;
mod rag_graph;
use catalog::{translate_en_us, translate_zh_cn};
mod business;
mod coding;
mod productivity;
mod terminal;

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
    let mut record = ToolCatalogRecord {
        display: None,
        tool_code: definition.tool_code.to_string(),
        tool_name: definition.tool_name,
        domain_code: definition.domain_code.to_string(),
        category: Some(super::builtin_category(definition.category_code)),
        source: eliteforge_agent_core::ToolSource {
            provider_id: Some(
                if matches!(definition.category_code, "tron.platform" | "tron.dude") {
                    super::HARNESS_TOOL_PROVIDER_ID
                } else {
                    definition.module_owner
                }
                .to_owned(),
            ),
            ..Default::default()
        },
        name: translate(locale, definition.name_i18n_key).to_string(),
        description: translate(locale, definition.description_i18n_key).to_string(),
        operation_kind: definition.operation_kind,
        input_schema: materialize_schema(locale, &definition.input_schema),
        module_owner: definition.module_owner.to_string(),
        availability_state: definition.availability_state,
        sample_result_summary: translate(locale, definition.sample_result_summary_i18n_key)
            .to_string(),
    };
    if record.domain_code == "dude_memory" {
        record.description = match locale {
            SupportedLocale::ZhCn => "管理当前用户或工作领域的记忆。".into(),
            _ => "Manage memories for the current user or work domain.".into(),
        };
        record.input_schema["description"] = record.description.clone().into();
        if let Some(scope) = record.input_schema.pointer_mut("/properties/scope") {
            scope["description"] = match locale {
                SupportedLocale::ZhCn => "选择用户或工作领域记忆。",
                _ => "Choose user or work-domain memory.",
            }.into();
        }
    } else if record.domain_code == "dude_task" {
        let scope = match locale {
            SupportedLocale::ZhCn => "管理当前 Dude 负责的平台任务。",
            _ => "Manage platform tasks owned by the current Dude.",
        };
        let description = match (record.tool_name.as_str(), locale) {
            ("dude_task_page", SupportedLocale::ZhCn) => "分页查询当前 Dude 负责的平台任务。",
            ("dude_task_page", _) => "Page platform tasks owned by the current Dude.",
            ("dude_task_create", SupportedLocale::ZhCn) => "创建由当前 Dude 负责的平台任务。",
            ("dude_task_create", _) => "Create a platform task owned by the current Dude.",
            _ => &record.description,
        };
        record.description = format!("{scope} {description}")
            .replace("task_get", "dude_task_get")
            .replace("task_update", "dude_task_update")
            .replace("task_restore", "dude_task_restore");
        record.input_schema["description"] = record.description.clone().into();
    }
    // These are explicit host aliases for official local bindings. Resources
    // come from that provider, while Tron keeps its stable ACL identities.
    if matches!(definition.module_owner, "coding" | "terminal") {
        static CATALOG: std::sync::OnceLock<Vec<eliteforge_agent_core::ToolDescriptor>> = std::sync::OnceLock::new();
        if let Some(descriptor)=CATALOG.get_or_init(eliteforge_agent_official_plugin_tools_local::local_tool_catalog).iter().find(|d|d.call_name==record.tool_name) {
            record.display=descriptor.display.clone();
            let resolved=descriptor.resolve_display(match locale {SupportedLocale::ZhCn=>"zh-CN",SupportedLocale::EnUs=>"en-US"});
            record.name=resolved.name;
            record.description=resolved.description;
            record.input_schema=resolved.input_schema;
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
            name: translate(locale, definition.name_i18n_key).to_string(),
            description: translate(locale, definition.description_i18n_key).to_string(),
            tool_count: items.len(),
            sample_tools,
            has_mutation_tools: items
                .iter()
                .any(|tool| tool.operation_kind == ToolOperationKind::Command),
        };
    }

    ToolDomainRecord {
        domain_code: domain_code.to_string(),
        name: match (locale, domain_code) {
            (SupportedLocale::ZhCn, "media") => "图片与文档".into(),
            (SupportedLocale::EnUs, "media") => "Images and documents".into(),
            _ => format!("{domain_code} Tools"),
        },
        description: match locale {
            SupportedLocale::ZhCn => format!("可用工具：{}。", sample_tools.join("、")),
            SupportedLocale::EnUs => format!("Available tools: {}.", sample_tools.join(", ")),
        },
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
    if let Some(value) = knowledge_quality::translate(locale, key) {
        return value;
    }
    if let Some(value) = rag_graph::translate(locale, key) {
        return value;
    }
    match locale {
        SupportedLocale::EnUs => translate_en_us(key),
        SupportedLocale::ZhCn => translate_zh_cn(key),
    }
}

fn key_leak_fallback() -> &'static str {
    "UNTRANSLATED_AGENT_CATALOG_KEY"
}
