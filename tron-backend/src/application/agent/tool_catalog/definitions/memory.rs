use super::{ToolCatalogDefinition, schema};
use crate::domain::agent::{ToolAvailabilityState, ToolOperationKind};
use serde_json::json;
pub(super) fn tool_definitions() -> Vec<ToolCatalogDefinition> {
    vec![
        ToolCatalogDefinition {
            tool_code: "memory.search",
            tool_name: "memory_search".into(),
            domain_code: "harness_memory",
            category_code: "tron.platform",
            name_i18n_key: "agent.toolCatalog.tools.memory.search.name",
            description_i18n_key: "agent.toolCatalog.tools.memory.search.description",
            operation_kind: ToolOperationKind::Query,
            input_schema: schema(
                "agent.toolCatalog.tools.memory.search.description",
                json!([]),
                json!({"type":"object","properties":{"query":{"type":"string","minLength":1},"scope":{"enum":["User","Space","WorkDomain"]},"scopeCode":{"type":"string"},"limit":{"type":"integer","minimum":1,"maximum":100,"default":8}},"required":["query"],"additionalProperties":false}),
            ),
            module_owner: "memory",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.memory.search.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "memory.get",
            tool_name: "memory_get".into(),
            domain_code: "harness_memory",
            category_code: "tron.platform",
            name_i18n_key: "agent.toolCatalog.tools.memory.get.name",
            description_i18n_key: "agent.toolCatalog.tools.memory.get.description",
            operation_kind: ToolOperationKind::Query,
            input_schema: schema(
                "agent.toolCatalog.tools.memory.get.description",
                json!([]),
                json!({"type":"object","properties":{"memoryCode":{"type":"string"}},"required":["memoryCode"],"additionalProperties":false}),
            ),
            module_owner: "memory",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.memory.get.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "memory.save",
            tool_name: "memory_save".into(),
            domain_code: "harness_memory",
            category_code: "tron.platform",
            name_i18n_key: "agent.toolCatalog.tools.memory.save.name",
            description_i18n_key: "agent.toolCatalog.tools.memory.save.description",
            operation_kind: ToolOperationKind::Command,
            input_schema: schema(
                "agent.toolCatalog.tools.memory.save.description",
                json!([]),
                json!({"type":"object","properties":{"memoryCode":{"type":"string"},"version":{"type":"integer","minimum":1},"scope":{"enum":["User","Space","WorkDomain"]},"scopeCode":{"type":"string"},"title":{"type":"string"},"text":{"type":"string"},"enabled":{"type":"boolean"}},"required":[],"additionalProperties":false}),
            ),
            module_owner: "memory",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.memory.save.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "memory.delete",
            tool_name: "memory_delete".into(),
            domain_code: "harness_memory",
            category_code: "tron.platform",
            name_i18n_key: "agent.toolCatalog.tools.memory.delete.name",
            description_i18n_key: "agent.toolCatalog.tools.memory.delete.description",
            operation_kind: ToolOperationKind::Command,
            input_schema: schema(
                "agent.toolCatalog.tools.memory.delete.description",
                json!([]),
                json!({"type":"object","properties":{"memoryCode":{"type":"string"},"version":{"type":"integer","minimum":1}},"required":["memoryCode","version"],"additionalProperties":false}),
            ),
            module_owner: "memory",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.memory.delete.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "todo.update",
            tool_name: "todo_update".into(),
            domain_code: "harness_memory",
            category_code: "tron.platform",
            name_i18n_key: "agent.toolCatalog.tools.todo.update.name",
            description_i18n_key: "agent.toolCatalog.tools.todo.update.description",
            operation_kind: ToolOperationKind::Command,
            input_schema: schema(
                "agent.toolCatalog.tools.todo.update.description",
                json!([{ "todos": [{ "position": 1, "content": "Inspect transcript storage", "status": "in_progress", "priority": "high" }] }]),
                json!({
                    "type": "object",
                    "required": ["workspaceCode", "todos"],
                    "properties": {
                        "workspaceCode": { "type": "string", "minLength": 1 },
                        "todos": {
                            "type": "array",
                            "maxItems": 50,
                            "items": {
                                "type": "object",
                                "required": ["position", "content", "status", "priority"],
                                "properties": {
                                    "todoCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.todo.update.params.todoCode" },
                                    "position": { "type": "integer", "minimum": 0, "x-i18n-description-key": "agent.toolCatalog.tools.todo.update.params.position" },
                                    "content": { "type": "string", "minLength": 1, "maxLength": 500, "x-i18n-description-key": "agent.toolCatalog.tools.todo.update.params.content" },
                                    "status": { "type": "string", "enum": ["pending", "in_progress", "completed", "cancelled"], "x-i18n-description-key": "agent.toolCatalog.tools.todo.update.params.status" },
                                    "priority": { "type": "string", "enum": ["high", "medium", "low"], "x-i18n-description-key": "agent.toolCatalog.tools.todo.update.params.priority" }
                                },
                                "additionalProperties": false
                            }
                        }
                    },
                    "additionalProperties": false
                }),
            ),
            module_owner: "short-term-memory",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.todo.update.sampleResultSummary",
        },
    ]
}
