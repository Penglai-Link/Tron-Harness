mod helpers;

use serde_json::json;

use crate::domain::agent::ToolOperationKind;

use super::{ToolCatalogDefinition, schema};
use helpers::{
    app_code_definition, app_mutation_schema, app_update_schema, definition, model_code_definition,
    model_mutation_schema, page_properties, task_code_definition, task_mutation_schema,
    variable_mutation_schema, workspace_code_schema, workspace_scoped_page_schema,
};

pub(super) fn tool_definitions() -> Vec<ToolCatalogDefinition> {
    vec![
        workspace_page_definition(),
        workspace_get_definition(),
        app_page_definition(),
        app_create_definition(),
        app_update_definition(),
        app_delete_definition(),
        app_restore_definition(),
        task_page_definition(),
        task_create_definition(),
        task_update_definition(),
        task_delete_definition(),
        task_restore_definition(),
        model_page_definition(),
        model_create_definition(),
        model_update_definition(),
        model_enable_definition(),
        model_disable_definition(),
        model_delete_definition(),
        skill_page_definition(),
        skill_view_definition(),
        variable_page_definition(),
        variable_create_definition(),
        variable_update_definition(),
        variable_delete_definition(),
    ]
}

fn workspace_page_definition() -> ToolCatalogDefinition {
    definition(
        "workspace.page",
        "workspace",
        ToolOperationKind::Query,
        schema(
            "agent.toolCatalog.tools.workspace.page.description",
            json!([{ "current": 1, "size": 20, "includeArchived": false, "search": "tron", "rootOnly": true }]),
            json!({
                "type": "object",
                "properties": page_properties(json!({
                    "includeArchived": { "type": "boolean", "default": false, "x-i18n-description-key": "agent.toolCatalog.tools.workspace.page.params.includeArchived" },
                    "search": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.workspace.page.params.search" },
                    "rootOnly": { "type": "boolean", "default": true, "x-i18n-description-key": "agent.toolCatalog.tools.workspace.page.params.rootOnly" }
                })),
                "additionalProperties": false
            }),
        ),
    )
}

fn workspace_get_definition() -> ToolCatalogDefinition {
    definition(
        "workspace.get",
        "workspace",
        ToolOperationKind::Query,
        workspace_code_schema(
            "agent.toolCatalog.tools.workspace.get.description",
            json!([{ "workspaceCode": "TRON_CORE" }]),
        ),
    )
}

fn app_page_definition() -> ToolCatalogDefinition {
    definition(
        "app.page",
        "app",
        ToolOperationKind::Query,
        schema(
            "agent.toolCatalog.tools.app.page.description",
            json!([{ "workspaceCode": "TRON_CORE", "current": 1, "size": 20, "search": "web", "includeArchived": false }]),
            workspace_scoped_page_schema(json!({
                "search": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.app.page.params.search" },
                "includeArchived": { "type": "boolean", "default": false, "x-i18n-description-key": "agent.toolCatalog.tools.app.page.params.includeArchived" }
            })),
        ),
    )
}

fn app_create_definition() -> ToolCatalogDefinition {
    definition(
        "app.create",
        "app",
        ToolOperationKind::Command,
        schema(
            "agent.toolCatalog.tools.app.create.description",
            json!([{ "workspaceCode": "TRON_CORE", "name": "tron-web", "localDirectory": "/repo/tron-web", "runtimeRole": "frontend" }]),
            app_mutation_schema(true),
        ),
    )
}

fn app_update_definition() -> ToolCatalogDefinition {
    definition(
        "app.update",
        "app",
        ToolOperationKind::Command,
        schema(
            "agent.toolCatalog.tools.app.update.description",
            json!([{ "workspaceCode": "TRON_CORE", "appCode": "APP-1", "name": "tron-web", "defaultBranch": "main" }]),
            app_update_schema(),
        ),
    )
}

fn app_delete_definition() -> ToolCatalogDefinition {
    app_code_definition("app.delete", ToolOperationKind::Command)
}

fn app_restore_definition() -> ToolCatalogDefinition {
    app_code_definition("app.restore", ToolOperationKind::Command)
}

fn task_page_definition() -> ToolCatalogDefinition {
    definition(
        "task.page",
        "task",
        ToolOperationKind::Query,
        schema(
            "agent.toolCatalog.tools.task.page.description",
            json!([{ "workspaceCode": "TRON_CORE", "current": 1, "size": 20, "search": "agent", "status": "open" }]),
            workspace_scoped_page_schema(json!({
                "search": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.task.page.params.search" },
                "status": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.task.page.params.status" },
                "priority": { "type": "string", "enum": ["low", "medium", "high"], "x-i18n-description-key": "agent.toolCatalog.tools.task.common.params.priority" },
                "taskType": { "type": "string", "enum": ["feature", "bug"], "x-i18n-description-key": "agent.toolCatalog.tools.task.common.params.taskType" },
                "sourceType": { "type": "string", "enum": ["internal", "github", "gitlab"], "x-i18n-description-key": "agent.toolCatalog.tools.task.common.params.sourceType" },
                "assigneeUserCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.task.page.params.assigneeUserCode" },
                "parentTaskCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.task.common.params.parentTaskCode" },
                "includeArchived": { "type": "boolean", "default": false, "x-i18n-description-key": "agent.toolCatalog.common.params.includeArchived" }
            })),
        ),
    )
}

fn task_create_definition() -> ToolCatalogDefinition {
    definition(
        "task.create",
        "task",
        ToolOperationKind::Command,
        schema(
            "agent.toolCatalog.tools.task.create.description",
            json!([{ "workspaceCode": "TRON_CORE", "title": "Implement platform tools", "priority": "high", "taskType": "feature", "sourceType": "internal" }]),
            task_mutation_schema(true),
        ),
    )
}

fn task_update_definition() -> ToolCatalogDefinition {
    definition(
        "task.update",
        "task",
        ToolOperationKind::Command,
        schema(
            "agent.toolCatalog.tools.task.update.description",
            json!([{ "workspaceCode": "TRON_CORE", "taskCode": "TSK-1", "title": "Updated title", "status": "in_progress", "progress": 40 }]),
            task_mutation_schema(false),
        ),
    )
}

fn task_delete_definition() -> ToolCatalogDefinition {
    task_code_definition("task.delete", ToolOperationKind::Command)
}

fn task_restore_definition() -> ToolCatalogDefinition {
    task_code_definition("task.restore", ToolOperationKind::Command)
}

fn model_page_definition() -> ToolCatalogDefinition {
    definition(
        "model.page",
        "model",
        ToolOperationKind::Query,
        schema(
            "agent.toolCatalog.tools.model.page.description",
            json!([{ "workspaceCode": "TRON_CORE", "current": 1, "size": 20, "search": "gpt", "modelType": "text", "enabled": true }]),
            workspace_scoped_page_schema(json!({
                "search": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.model.page.params.search" },
                "modelType": { "type": "string", "enum": ["text", "embedding", "rerank", "image"], "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.modelType" },
                "enabled": { "type": "boolean", "x-i18n-description-key": "agent.toolCatalog.common.params.enabled" }
            })),
        ),
    )
}

fn model_create_definition() -> ToolCatalogDefinition {
    definition(
        "model.create",
        "model",
        ToolOperationKind::Command,
        schema(
            "agent.toolCatalog.tools.model.create.description",
            json!([{ "workspaceCode": "TRON_CORE", "displayName": "Qwen", "vendor": "Alibaba", "protocol": "openai_compatible", "modelType": "text", "baseUrl": "https://dashscope.aliyuncs.com/compatible-mode/v1", "apiKeyRef": "DASHSCOPE_API_KEY", "modelName": "qwen-plus" }]),
            model_mutation_schema(true),
        ),
    )
}

fn model_update_definition() -> ToolCatalogDefinition {
    definition(
        "model.update",
        "model",
        ToolOperationKind::Command,
        schema(
            "agent.toolCatalog.tools.model.update.description",
            json!([{ "workspaceCode": "TRON_CORE", "modelCode": "MDL-1", "displayName": "Qwen Plus", "enabled": true }]),
            model_mutation_schema(false),
        ),
    )
}

fn model_enable_definition() -> ToolCatalogDefinition {
    model_code_definition("model.enable", ToolOperationKind::Command)
}

fn model_disable_definition() -> ToolCatalogDefinition {
    model_code_definition("model.disable", ToolOperationKind::Command)
}

fn model_delete_definition() -> ToolCatalogDefinition {
    model_code_definition("model.delete", ToolOperationKind::Command)
}

fn skill_page_definition() -> ToolCatalogDefinition {
    definition(
        "skill.page",
        "skill",
        ToolOperationKind::Query,
        schema(
            "agent.toolCatalog.tools.skill.page.description",
            json!([{ "workspaceCode": "TRON_CORE", "current": 1, "size": 20, "search": "eliteforge", "includeDisabled": false }]),
            workspace_scoped_page_schema(json!({
                "search": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.skill.page.params.search" },
                "includeDisabled": { "type": "boolean", "default": false, "x-i18n-description-key": "agent.toolCatalog.tools.skill.page.params.includeDisabled" }
            })),
        ),
    )
}

fn skill_view_definition() -> ToolCatalogDefinition {
    definition(
        "skill.view",
        "skill",
        ToolOperationKind::Query,
        schema(
            "agent.toolCatalog.tools.skill.view.description",
            json!([{ "workspaceCode": "TRON_CORE", "skillName": "eliteforge-brainstorming" }]),
            json!({
                "type": "object",
                "required": ["skillName"],
                "properties": {
                    "workspaceCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.common.params.workspaceCode" },
                    "skillName": { "type": "string", "minLength": 1, "x-i18n-description-key": "agent.toolCatalog.tools.skill.view.params.skillName" }
                },
                "additionalProperties": false
            }),
        ),
    )
}

fn variable_page_definition() -> ToolCatalogDefinition {
    definition(
        "variable.page",
        "variable",
        ToolOperationKind::Query,
        schema(
            "agent.toolCatalog.tools.variable.page.description",
            json!([{ "workspaceCode": "TRON_CORE", "current": 1, "size": 20, "search": "OPENAI" }]),
            workspace_scoped_page_schema(json!({
                "search": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.variable.page.params.search" }
            })),
        ),
    )
}

fn variable_create_definition() -> ToolCatalogDefinition {
    definition(
        "variable.create",
        "variable",
        ToolOperationKind::Command,
        schema(
            "agent.toolCatalog.tools.variable.create.description",
            json!([{ "workspaceCode": "TRON_CORE", "key": "OPENAI_API_KEY", "value": "sk-...", "isSensitive": true }]),
            variable_mutation_schema(true),
        ),
    )
}

fn variable_update_definition() -> ToolCatalogDefinition {
    definition(
        "variable.update",
        "variable",
        ToolOperationKind::Command,
        schema(
            "agent.toolCatalog.tools.variable.update.description",
            json!([{ "workspaceCode": "TRON_CORE", "key": "OPENAI_API_KEY", "valueMode": "preserve", "description": "Updated description" }]),
            variable_mutation_schema(false),
        ),
    )
}

fn variable_delete_definition() -> ToolCatalogDefinition {
    definition(
        "variable.delete",
        "variable",
        ToolOperationKind::Command,
        schema(
            "agent.toolCatalog.tools.variable.delete.description",
            json!([{ "workspaceCode": "TRON_CORE", "key": "OPENAI_API_KEY" }]),
            json!({
                "type": "object",
                "required": ["key"],
                "properties": {
                    "workspaceCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.common.params.workspaceCode" },
                    "key": { "type": "string", "minLength": 1, "x-i18n-description-key": "agent.toolCatalog.tools.variable.common.params.key" }
                },
                "additionalProperties": false
            }),
        ),
    )
}
