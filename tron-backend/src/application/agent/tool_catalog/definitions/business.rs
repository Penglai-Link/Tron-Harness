use serde_json::json;

use crate::domain::agent::{ToolAvailabilityState, ToolOperationKind};

use super::{ToolCatalogDefinition, schema};

pub(super) fn tool_definitions() -> Vec<ToolCatalogDefinition> {
    vec![
        ToolCatalogDefinition {
            tool_code: "workspace.list",
            domain_code: "workspace",
            name_i18n_key: "agent.toolCatalog.tools.workspace.list.name",
            description_i18n_key: "agent.toolCatalog.tools.workspace.list.description",
            operation_kind: ToolOperationKind::Query,
            input_schema: schema(
                "agent.toolCatalog.tools.workspace.list.description",
                json!([{ "includeArchived": false, "search": "tron", "rootOnly": true }]),
                json!({
                    "type": "object",
                    "properties": {
                        "includeArchived": { "type": "boolean", "default": false, "x-i18n-description-key": "agent.toolCatalog.tools.workspace.list.params.includeArchived" },
                        "search": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.workspace.list.params.search" },
                        "rootOnly": { "type": "boolean", "x-i18n-description-key": "agent.toolCatalog.tools.workspace.list.params.rootOnly" }
                    },
                    "additionalProperties": false
                }),
            ),
            module_owner: "workspace",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.workspace.list.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "workspace.get",
            domain_code: "workspace",
            name_i18n_key: "agent.toolCatalog.tools.workspace.get.name",
            description_i18n_key: "agent.toolCatalog.tools.workspace.get.description",
            operation_kind: ToolOperationKind::Query,
            input_schema: schema(
                "agent.toolCatalog.tools.workspace.get.description",
                json!([{ "workspaceCode": "TRON_CORE" }]),
                json!({
                    "type": "object",
                    "required": ["workspaceCode"],
                    "properties": {
                        "workspaceCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.workspace.get.params.workspaceCode" }
                    },
                    "additionalProperties": false
                }),
            ),
            module_owner: "workspace",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.workspace.get.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "app.list",
            domain_code: "app",
            name_i18n_key: "agent.toolCatalog.tools.app.list.name",
            description_i18n_key: "agent.toolCatalog.tools.app.list.description",
            operation_kind: ToolOperationKind::Query,
            input_schema: schema(
                "agent.toolCatalog.tools.app.list.description",
                json!([{ "workspaceCode": "TRON_CORE", "search": "tron-web", "includeArchived": false }]),
                json!({
                    "type": "object",
                    "properties": {
                        "workspaceCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.app.list.params.workspaceCode" },
                        "search": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.app.list.params.search" },
                        "includeArchived": { "type": "boolean", "default": false, "x-i18n-description-key": "agent.toolCatalog.tools.app.list.params.includeArchived" }
                    },
                    "additionalProperties": false
                }),
            ),
            module_owner: "app",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.app.list.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "task.list",
            domain_code: "task",
            name_i18n_key: "agent.toolCatalog.tools.task.list.name",
            description_i18n_key: "agent.toolCatalog.tools.task.list.description",
            operation_kind: ToolOperationKind::Query,
            input_schema: schema(
                "agent.toolCatalog.tools.task.list.description",
                json!([{ "workspaceCode": "TRON_CORE", "search": "agent", "status": "open" }]),
                json!({
                    "type": "object",
                    "properties": {
                        "workspaceCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.task.list.params.workspaceCode" },
                        "search": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.task.list.params.search" },
                        "status": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.task.list.params.status" }
                    },
                    "additionalProperties": false
                }),
            ),
            module_owner: "task",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.task.list.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "task.create",
            domain_code: "task",
            name_i18n_key: "agent.toolCatalog.tools.task.create.name",
            description_i18n_key: "agent.toolCatalog.tools.task.create.description",
            operation_kind: ToolOperationKind::Command,
            input_schema: schema(
                "agent.toolCatalog.tools.task.create.description",
                json!([{ "workspaceCode": "TRON_CORE", "title": "Implement coding tools", "priority": "high", "taskType": "feature", "sourceType": "internal" }]),
                json!({
                    "type": "object",
                    "required": ["title"],
                    "properties": {
                        "workspaceCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.task.create.params.workspaceCode" },
                        "title": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.task.create.params.title" },
                        "description": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.task.create.params.description" },
                        "priority": { "type": "string", "enum": ["low", "medium", "high"], "x-i18n-description-key": "agent.toolCatalog.tools.task.create.params.priority" },
                        "taskType": { "type": "string", "enum": ["feature", "bug"], "x-i18n-description-key": "agent.toolCatalog.tools.task.create.params.taskType" },
                        "sourceType": { "type": "string", "enum": ["internal", "github", "gitlab"], "x-i18n-description-key": "agent.toolCatalog.tools.task.create.params.sourceType" },
                        "appCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.task.create.params.appCode" }
                    },
                    "additionalProperties": false
                }),
            ),
            module_owner: "task",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.task.create.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "model.list",
            domain_code: "model",
            name_i18n_key: "agent.toolCatalog.tools.model.list.name",
            description_i18n_key: "agent.toolCatalog.tools.model.list.description",
            operation_kind: ToolOperationKind::Query,
            input_schema: schema(
                "agent.toolCatalog.tools.model.list.description",
                json!([{ "workspaceCode": "TRON_CORE" }]),
                json!({
                    "type": "object",
                    "properties": {
                        "workspaceCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.model.list.params.workspaceCode" }
                    },
                    "additionalProperties": false
                }),
            ),
            module_owner: "model",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.model.list.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "skill.list",
            domain_code: "skill",
            name_i18n_key: "agent.toolCatalog.tools.skill.list.name",
            description_i18n_key: "agent.toolCatalog.tools.skill.list.description",
            operation_kind: ToolOperationKind::Query,
            input_schema: schema(
                "agent.toolCatalog.tools.skill.list.description",
                json!([{ "workspaceCode": "TRON_CORE", "search": "eliteforge", "includeDisabled": false }]),
                json!({
                    "type": "object",
                    "properties": {
                        "workspaceCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.skill.list.params.workspaceCode" },
                        "search": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.skill.list.params.search" },
                        "includeDisabled": { "type": "boolean", "default": false, "x-i18n-description-key": "agent.toolCatalog.tools.skill.list.params.includeDisabled" }
                    },
                    "additionalProperties": false
                }),
            ),
            module_owner: "skill",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.skill.list.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "skill.view",
            domain_code: "skill",
            name_i18n_key: "agent.toolCatalog.tools.skill.view.name",
            description_i18n_key: "agent.toolCatalog.tools.skill.view.description",
            operation_kind: ToolOperationKind::Query,
            input_schema: schema(
                "agent.toolCatalog.tools.skill.view.description",
                json!([{ "workspaceCode": "TRON_CORE", "skillName": "eliteforge-brainstorming" }]),
                json!({
                    "type": "object",
                    "required": ["skillName"],
                    "properties": {
                        "workspaceCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.skill.view.params.workspaceCode" },
                        "skillName": { "type": "string", "minLength": 1, "x-i18n-description-key": "agent.toolCatalog.tools.skill.view.params.skillName" }
                    },
                    "additionalProperties": false
                }),
            ),
            module_owner: "skill",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.skill.view.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "variable.list",
            domain_code: "variable",
            name_i18n_key: "agent.toolCatalog.tools.variable.list.name",
            description_i18n_key: "agent.toolCatalog.tools.variable.list.description",
            operation_kind: ToolOperationKind::Query,
            input_schema: schema(
                "agent.toolCatalog.tools.variable.list.description",
                json!([{ "workspaceCode": "TRON_CORE", "search": "OPENAI" }]),
                json!({
                    "type": "object",
                    "properties": {
                        "workspaceCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.variable.list.params.workspaceCode" },
                        "search": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.variable.list.params.search" }
                    },
                    "additionalProperties": false
                }),
            ),
            module_owner: "variable",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.variable.list.sampleResultSummary",
        },
    ]
}
