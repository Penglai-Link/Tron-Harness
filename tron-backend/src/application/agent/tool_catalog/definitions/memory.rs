use serde_json::json;

use crate::domain::agent::{ToolAvailabilityState, ToolOperationKind};

use super::{ToolCatalogDefinition, schema};

pub(super) fn tool_definitions() -> Vec<ToolCatalogDefinition> {
    vec![
        ToolCatalogDefinition {
            tool_code: "memory.search",
            domain_code: "memory",
            name_i18n_key: "agent.toolCatalog.tools.memory.search.name",
            description_i18n_key: "agent.toolCatalog.tools.memory.search.description",
            operation_kind: ToolOperationKind::Query,
            input_schema: schema(
                "agent.toolCatalog.tools.memory.search.description",
                json!([{ "scope": "USER", "query": "repository workflow", "limit": 10 }]),
                json!({
                    "type": "object",
                    "properties": {
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
            domain_code: "memory",
            name_i18n_key: "agent.toolCatalog.tools.memory.save.name",
            description_i18n_key: "agent.toolCatalog.tools.memory.save.description",
            operation_kind: ToolOperationKind::Command,
            input_schema: schema(
                "agent.toolCatalog.tools.memory.save.description",
                json!([{ "scope": "USER", "memoryType": "USER_PREFERENCE", "text": "Prefers concise status updates.", "title": "Status update preference" }]),
                json!({
                    "type": "object",
                    "properties": {
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
            domain_code: "memory",
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
    ]
}
