mod catalog;
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
    ToolCatalogRecord {
        tool_code: definition.tool_code.to_string(),
        tool_name: definition.tool_name,
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
        name: match (locale, domain_code) {
            (SupportedLocale::ZhCn, "media") => "图片与文档".into(),
            (SupportedLocale::EnUs, "media") => "Images and documents".into(),
            _ => format!("{domain_code} Tools"),
        },
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

fn key_leak_fallback() -> &'static str {
    "UNTRANSLATED_AGENT_CATALOG_KEY"
}
