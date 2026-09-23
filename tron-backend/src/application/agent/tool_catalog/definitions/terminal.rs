use super::{ToolCatalogDefinition, schema};
use crate::domain::agent::{ToolAvailabilityState, ToolOperationKind};
use serde_json::json;
pub(super) fn tool_definitions() -> Vec<ToolCatalogDefinition> {
    vec![
        ToolCatalogDefinition {
            tool_code: "terminal.start",
            tool_name: "terminal_start".into(),
            domain_code: "terminal",
            name_i18n_key: "agent.toolCatalog.terminal.start.name",
            description_i18n_key: "agent.toolCatalog.terminal.start.description",
            operation_kind: ToolOperationKind::Command,
            input_schema: schema(
                "agent.toolCatalog.terminal.start.description",
                json!([{"command":"npm run dev","cwd":"front"},{"interactive":true,"cwd":"front"}]),
                json!({"type": "object", "properties": {"command": {"type": "string", "minLength": 1, "x-i18n-description-key": "agent.toolCatalog.terminal.params.command"}, "args": {"type": "array", "items": {"type": "string"}, "x-i18n-description-key": "agent.toolCatalog.terminal.params.args"}, "interactive": {"type": "boolean", "x-i18n-description-key": "agent.toolCatalog.terminal.params.interactive"}, "cwd": {"type": "string", "x-i18n-description-key": "agent.toolCatalog.terminal.params.cwd"}, "env": {"type": "object", "additionalProperties": {"type": "string"}, "x-i18n-description-key": "agent.toolCatalog.terminal.params.env"}}, "oneOf": [{"required": ["command"], "not": {"properties": {"interactive": {"const": true}}}}, {"required": ["interactive"], "properties": {"interactive": {"const": true}}, "not": {"anyOf": [{"required": ["command"]}, {"required": ["args"]}]}}], "additionalProperties": false}),
            ),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.terminal.result",
        },
        ToolCatalogDefinition {
            tool_code: "terminal.read",
            tool_name: "terminal_read".into(),
            domain_code: "terminal",
            name_i18n_key: "agent.toolCatalog.terminal.read.name",
            description_i18n_key: "agent.toolCatalog.terminal.read.description",
            operation_kind: ToolOperationKind::Command,
            input_schema: schema(
                "agent.toolCatalog.terminal.read.description",
                json!([]),
                json!({"type": "object", "required": ["terminalId"], "properties": {"terminalId": {"type": "string", "x-i18n-description-key": "agent.toolCatalog.terminal.params.terminalId"}, "cursor": {"type": "integer", "minimum": 0, "x-i18n-description-key": "agent.toolCatalog.terminal.params.cursor"}}, "additionalProperties": false}),
            ),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.terminal.result",
        },
        ToolCatalogDefinition {
            tool_code: "terminal.write",
            tool_name: "terminal_write".into(),
            domain_code: "terminal",
            name_i18n_key: "agent.toolCatalog.terminal.write.name",
            description_i18n_key: "agent.toolCatalog.terminal.write.description",
            operation_kind: ToolOperationKind::Command,
            input_schema: schema(
                "agent.toolCatalog.terminal.write.description",
                json!([]),
                json!({"type": "object", "required": ["terminalId", "data"], "properties": {"terminalId": {"type": "string", "x-i18n-description-key": "agent.toolCatalog.terminal.params.terminalId"}, "data": {"type": "string", "maxLength": 65536, "x-i18n-description-key": "agent.toolCatalog.terminal.params.data"}}, "additionalProperties": false}),
            ),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.terminal.result",
        },
        ToolCatalogDefinition {
            tool_code: "terminal.stop",
            tool_name: "terminal_stop".into(),
            domain_code: "terminal",
            name_i18n_key: "agent.toolCatalog.terminal.stop.name",
            description_i18n_key: "agent.toolCatalog.terminal.stop.description",
            operation_kind: ToolOperationKind::Command,
            input_schema: schema(
                "agent.toolCatalog.terminal.stop.description",
                json!([]),
                json!({"type": "object", "required": ["terminalId"], "properties": {"terminalId": {"type": "string", "x-i18n-description-key": "agent.toolCatalog.terminal.params.terminalId"}}, "additionalProperties": false}),
            ),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.terminal.result",
        },
        ToolCatalogDefinition {
            tool_code: "terminal.list",
            tool_name: "terminal_list".into(),
            domain_code: "terminal",
            name_i18n_key: "agent.toolCatalog.terminal.list.name",
            description_i18n_key: "agent.toolCatalog.terminal.list.description",
            operation_kind: ToolOperationKind::Command,
            input_schema: schema(
                "agent.toolCatalog.terminal.list.description",
                json!([]),
                json!({"type": "object", "required": [], "properties": {}, "additionalProperties": false}),
            ),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.terminal.result",
        },
    ]
}
