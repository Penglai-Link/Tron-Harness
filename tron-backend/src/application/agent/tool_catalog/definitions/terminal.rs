use super::{ToolCatalogDefinition, coding::official_schema, schema};
use crate::domain::agent::{ToolAvailabilityState, ToolOperationKind};
use serde_json::{Value, json};
pub(super) fn tool_definitions() -> Vec<ToolCatalogDefinition> {
    vec![
        ToolCatalogDefinition {
            tool_code: "terminal.start",
            tool_name: "terminal_start".into(),
            domain_code: "terminal",
            category_code: "system.command",
            name_i18n_key: "agent.toolCatalog.terminal.start.name",
            description_i18n_key: "agent.toolCatalog.terminal.start.description",
            operation_kind: ToolOperationKind::Command,
            input_schema: terminal_schema(
                "terminal_start",
                "agent.toolCatalog.terminal.start.description",
            ),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.terminal.result",
        },
        ToolCatalogDefinition {
            tool_code: "terminal.read",
            tool_name: "terminal_read".into(),
            domain_code: "terminal",
            category_code: "system.command",
            name_i18n_key: "agent.toolCatalog.terminal.read.name",
            description_i18n_key: "agent.toolCatalog.terminal.read.description",
            operation_kind: ToolOperationKind::Command,
            input_schema: terminal_schema(
                "terminal_read",
                "agent.toolCatalog.terminal.read.description",
            ),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.terminal.result",
        },
        ToolCatalogDefinition {
            tool_code: "terminal.write",
            tool_name: "terminal_write".into(),
            domain_code: "terminal",
            category_code: "system.command",
            name_i18n_key: "agent.toolCatalog.terminal.write.name",
            description_i18n_key: "agent.toolCatalog.terminal.write.description",
            operation_kind: ToolOperationKind::Command,
            input_schema: terminal_schema(
                "terminal_write",
                "agent.toolCatalog.terminal.write.description",
            ),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.terminal.result",
        },
        ToolCatalogDefinition {
            tool_code: "terminal.stop",
            tool_name: "terminal_stop".into(),
            domain_code: "terminal",
            category_code: "system.command",
            name_i18n_key: "agent.toolCatalog.terminal.stop.name",
            description_i18n_key: "agent.toolCatalog.terminal.stop.description",
            operation_kind: ToolOperationKind::Command,
            input_schema: terminal_schema(
                "terminal_stop",
                "agent.toolCatalog.terminal.stop.description",
            ),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.terminal.result",
        },
        ToolCatalogDefinition {
            tool_code: "terminal.list",
            tool_name: "terminal_list".into(),
            domain_code: "terminal",
            category_code: "system.command",
            name_i18n_key: "agent.toolCatalog.terminal.list.name",
            description_i18n_key: "agent.toolCatalog.terminal.list.description",
            operation_kind: ToolOperationKind::Command,
            input_schema: terminal_schema(
                "terminal_list",
                "agent.toolCatalog.terminal.list.description",
            ),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.terminal.result",
        },
    ]
}

// Display metadata belongs to Tron; all validation rules come from the official catalog.
fn terminal_schema(name: &str, description_key: &'static str) -> Value {
    let mut definition = official_schema(name);
    if let Some(properties) = definition
        .get_mut("properties")
        .and_then(Value::as_object_mut)
    {
        for (name, field) in properties {
            if let Some(field) = field.as_object_mut() {
                field.insert(
                    "x-i18n-description-key".into(),
                    Value::String(format!("agent.toolCatalog.terminal.params.{name}")),
                );
            }
        }
    }
    let examples = if name == "terminal_start" {
        json!([{"command":"npm run dev","cwd":"front"},{"interactive":true,"cwd":"front"}])
    } else {
        json!([])
    };
    schema(description_key, examples, definition)
}
