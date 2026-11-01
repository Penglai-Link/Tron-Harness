use serde_json::json;

use crate::domain::agent::{ToolAvailabilityState, ToolOperationKind};

use super::{ToolCatalogDefinition, schema};

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
                json!([{ "scope": "USER", "query": "repository workflow", "limit": 10 }]),
                json!({
                    "type": "object",
                    "properties": {
                        "workspaceCode": { "type": "string", "minLength": 1 },
                        "query": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.memory.search.params.query" },
                        "mode": { "type": "string", "enum": ["SEARCH", "CONTEXT_PACK"], "default": "SEARCH", "x-i18n-description-key": "agent.toolCatalog.tools.memory.search.params.mode" },
                        "memoryCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.memory.common.params.memoryCode" },
                        "scope": { "type": "string", "enum": ["USER", "WORK_DOMAIN", "SPACE"], "x-i18n-description-key": "agent.toolCatalog.tools.memory.common.params.scope" },
                        "scopeCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.memory.common.params.scopeCode" },
                        "memoryType": {
                            "oneOf": [
                                { "type": "string" },
                                { "type": "array", "items": { "type": "string" } }
                            ],
                            "x-i18n-description-key": "agent.toolCatalog.tools.memory.common.params.memoryType"
                        },
                        "memoryStatus": {
                            "oneOf": [
                                { "type": "string" },
                                { "type": "array", "items": { "type": "string" } }
                            ],
                            "default": "ACTIVE",
                            "x-i18n-description-key": "agent.toolCatalog.tools.memory.common.params.memoryStatus"
                        },
                        "reviewStatus": {
                            "oneOf": [
                                { "type": "string" },
                                { "type": "array", "items": { "type": "string" } }
                            ],
                            "default": "APPROVED",
                            "x-i18n-description-key": "agent.toolCatalog.tools.memory.common.params.reviewStatus"
                        },
                        "limit": { "type": "integer", "minimum": 1, "maximum": 100, "default": 10, "x-i18n-description-key": "agent.toolCatalog.tools.memory.search.params.limit" },
                        "tokenBudget": { "type": "integer", "minimum": 1, "default": 1200, "x-i18n-description-key": "agent.toolCatalog.tools.memory.search.params.tokenBudget" },
                        "trace": { "type": "boolean", "default": false, "x-i18n-description-key": "agent.toolCatalog.tools.memory.search.params.trace" }
                    },
                    "additionalProperties": false
                }),
            ),
            module_owner: "memory",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.memory.search.sampleResultSummary",
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
                json!([{ "scope": "USER", "memoryType": "USER_PREFERENCE", "text": "Prefers concise status updates.", "title": "Status update preference" }]),
                json!({
                    "type": "object",
                    "properties": {
                        "workspaceCode": { "type": "string", "minLength": 1 },
                        "memoryType": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.memory.common.params.memoryType" },
                        "text": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.memory.save.params.text" },
                        "memoryCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.memory.common.params.memoryCode" },
                        "scope": { "type": "string", "enum": ["USER", "WORK_DOMAIN", "SPACE"], "default": "USER", "x-i18n-description-key": "agent.toolCatalog.tools.memory.common.params.scope" },
                        "scopeCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.memory.common.params.scopeCode" },
                        "title": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.memory.save.params.title" },
                        "semanticKey": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.memory.save.params.semanticKey" },
                        "memoryStatus": { "type": "string", "default": "ACTIVE", "x-i18n-description-key": "agent.toolCatalog.tools.memory.common.params.memoryStatus" },
                        "reviewStatus": { "type": "string", "default": "APPROVED", "x-i18n-description-key": "agent.toolCatalog.tools.memory.common.params.reviewStatus" },
                        "injectable": { "type": "boolean", "default": true, "x-i18n-description-key": "agent.toolCatalog.tools.memory.save.params.injectable" },
                        "importanceScore": { "type": "number", "x-i18n-description-key": "agent.toolCatalog.tools.memory.save.params.importanceScore" },
                        "confidenceScore": { "type": "number", "x-i18n-description-key": "agent.toolCatalog.tools.memory.save.params.confidenceScore" },
                        "qualityScore": { "type": "number", "x-i18n-description-key": "agent.toolCatalog.tools.memory.save.params.qualityScore" },
                        "expiresAt": { "type": "integer", "x-i18n-description-key": "agent.toolCatalog.tools.memory.save.params.expiresAt" },
                        "derivedSourceType": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.memory.save.params.derivedSourceType" },
                        "derivedSourceCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.memory.save.params.derivedSourceCode" },
                        "sourceRefs": { "type": "array", "items": { "type": "object" }, "x-i18n-description-key": "agent.toolCatalog.tools.memory.save.params.sourceRefs" },
                        "metadata": { "type": "object", "x-i18n-description-key": "agent.toolCatalog.tools.memory.common.params.metadata" }
                    },
                    "additionalProperties": false
                }),
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
                json!([{ "memoryCode": "U:USR-ADMIN:source:v1", "reason": "Superseded by a newer memory.", "deleteMode": "ARCHIVE" }]),
                json!({
                    "type": "object",
                    "required": ["memoryCode", "reason"],
                    "properties": {
                        "workspaceCode": { "type": "string", "minLength": 1 },
                        "memoryCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.memory.common.params.memoryCode" },
                        "reason": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.memory.delete.params.reason" },
                        "deleteMode": { "type": "string", "enum": ["ARCHIVE", "DISABLE"], "default": "ARCHIVE", "x-i18n-description-key": "agent.toolCatalog.tools.memory.delete.params.deleteMode" },
                        "sourceRefs": { "type": "array", "items": { "type": "object" }, "x-i18n-description-key": "agent.toolCatalog.tools.memory.save.params.sourceRefs" },
                        "metadata": { "type": "object", "x-i18n-description-key": "agent.toolCatalog.tools.memory.common.params.metadata" }
                    },
                    "additionalProperties": false
                }),
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
