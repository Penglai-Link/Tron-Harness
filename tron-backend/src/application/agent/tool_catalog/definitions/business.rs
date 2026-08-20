mod helpers;

use serde_json::{Map, Value, json};

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
        automation_page_definition(),
        automation_get_definition(),
        automation_create_definition(),
        automation_update_definition(),
        automation_delete_definition(),
        automation_restore_definition(),
        automation_execute_definition(),
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
        platform_model_page_definition(),
        platform_model_get_definition(),
        platform_model_create_definition(),
        platform_model_update_definition(),
        platform_model_enable_definition(),
        platform_model_disable_definition(),
        platform_model_delete_definition(),
        skill_page_definition(),
        skill_view_definition(),
        variable_page_definition(),
        variable_create_definition(),
        variable_update_definition(),
        variable_delete_definition(),
        platform_variable_page_definition(),
        platform_variable_create_definition(),
        platform_variable_update_definition(),
        platform_variable_delete_definition(),
        platform_prompt_page_definition(),
        platform_prompt_get_definition(),
        platform_prompt_create_definition(),
        platform_prompt_update_definition(),
        platform_prompt_delete_definition(),
        platform_mcp_page_definition(),
        platform_mcp_get_definition(),
        platform_mcp_create_definition(),
        platform_mcp_update_definition(),
        platform_mcp_delete_definition(),
        platform_open_app_page_definition(),
        platform_open_app_get_definition(),
        platform_open_app_create_definition(),
        platform_open_app_update_definition(),
        platform_open_app_disable_definition(),
        platform_resource_query_definition(),
        platform_resource_get_definition(),
        platform_resource_create_definition(),
        platform_resource_update_definition(),
        platform_resource_delete_definition(),
        platform_resource_binary_definition(),
        resource_query_definition(),
        resource_get_definition(),
        resource_create_definition(),
        resource_update_definition(),
        resource_delete_definition(),
        resource_binary_definition(),
        resource_data_source_create_definition(),
        resource_data_source_update_definition(),
        resource_server_create_definition(),
        resource_server_account_create_definition(),
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

fn automation_page_definition() -> ToolCatalogDefinition {
    definition(
        "automation.page",
        "automation",
        ToolOperationKind::Query,
        schema(
            "agent.toolCatalog.tools.automation.page.description",
            json!([{ "workspaceCode": "TRON_CORE", "current": 1, "size": 20, "search": "digest", "status": "active", "includeArchived": false }]),
            workspace_scoped_page_schema(json!({
                "search": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.automation.page.params.search" },
                "status": { "type": "string", "enum": ["active", "paused", "archived"], "x-i18n-description-key": "agent.toolCatalog.tools.automation.common.params.status" },
                "includeArchived": { "type": "boolean", "default": false, "x-i18n-description-key": "agent.toolCatalog.common.params.includeArchived" }
            })),
        ),
    )
}

fn automation_get_definition() -> ToolCatalogDefinition {
    automation_code_definition("automation.get", ToolOperationKind::Query)
}

fn automation_create_definition() -> ToolCatalogDefinition {
    definition(
        "automation.create",
        "automation",
        ToolOperationKind::Command,
        schema(
            "agent.toolCatalog.tools.automation.create.description",
            json!([{ "workspaceCode": "TRON_CORE", "name": "Daily digest", "agentCode": "main-agent", "modelCode": "MODEL-1", "prompt": "Summarize workspace progress.", "cronExpression": "0 9 * * *", "status": "paused" }]),
            automation_mutation_schema(true),
        ),
    )
}

fn automation_update_definition() -> ToolCatalogDefinition {
    definition(
        "automation.update",
        "automation",
        ToolOperationKind::Command,
        schema(
            "agent.toolCatalog.tools.automation.update.description",
            json!([{ "workspaceCode": "TRON_CORE", "automationCode": "AUT-1", "name": "Daily release digest", "agentCode": "main-agent", "modelCode": "MODEL-1", "status": "active" }]),
            automation_mutation_schema(false),
        ),
    )
}

fn automation_delete_definition() -> ToolCatalogDefinition {
    automation_code_definition("automation.delete", ToolOperationKind::Command)
}

fn automation_restore_definition() -> ToolCatalogDefinition {
    automation_code_definition("automation.restore", ToolOperationKind::Command)
}

fn automation_execute_definition() -> ToolCatalogDefinition {
    automation_code_definition("automation.execute", ToolOperationKind::Command)
}

fn automation_code_definition(
    tool_code: &'static str,
    operation_kind: ToolOperationKind,
) -> ToolCatalogDefinition {
    definition(
        tool_code,
        "automation",
        operation_kind,
        schema(
            match tool_code {
                "automation.get" => "agent.toolCatalog.tools.automation.get.description",
                "automation.delete" => "agent.toolCatalog.tools.automation.delete.description",
                "automation.restore" => "agent.toolCatalog.tools.automation.restore.description",
                "automation.execute" => "agent.toolCatalog.tools.automation.execute.description",
                _ => "agent.toolCatalog.tools.automation.get.description",
            },
            json!([{ "workspaceCode": "TRON_CORE", "automationCode": "AUT-1" }]),
            json!({
                "type": "object",
                "required": ["automationCode"],
                "properties": {
                    "workspaceCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.common.params.workspaceCode" },
                    "automationCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.automation.common.params.automationCode" }
                },
                "additionalProperties": false
            }),
        ),
    )
}

fn automation_mutation_schema(create: bool) -> serde_json::Value {
    let required = if create {
        json!(["name", "prompt", "cronExpression"])
    } else {
        json!(["automationCode"])
    };
    let mut value = json!({
        "type": "object",
        "required": required,
        "properties": {
            "workspaceCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.common.params.workspaceCode" },
            "name": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.common.params.name" },
            "description": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.common.params.description" },
            "agentCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.automation.common.params.agentCode" },
            "modelCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.automation.common.params.modelCode" },
            "prompt": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.automation.common.params.prompt" },
            "cronExpression": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.automation.common.params.cronExpression" },
            "status": { "type": "string", "enum": ["active", "paused", "archived"], "x-i18n-description-key": "agent.toolCatalog.tools.automation.common.params.status" }
        },
        "additionalProperties": false
    });
    if !create {
        if let serde_json::Value::Object(map) = &mut value {
            if let Some(serde_json::Value::Object(properties)) = map.get_mut("properties") {
                properties.insert(
                    "automationCode".to_string(),
                    json!({ "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.automation.common.params.automationCode" }),
                );
            }
        }
    }
    value
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
                "priority": { "type": "string", "enum": ["low", "medium", "high", "extra_high"], "x-i18n-description-key": "agent.toolCatalog.tools.task.common.params.priority" },
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

fn platform_model_page_definition() -> ToolCatalogDefinition {
    definition(
        "platform.model.page",
        "model",
        ToolOperationKind::Query,
        schema(
            "agent.toolCatalog.tools.platform.model.page.description",
            json!([{ "current": 1, "size": 20, "search": "qwen", "enabled": true }]),
            json!({
                "type": "object",
                "properties": {
                    "current": { "type": "integer", "minimum": 1, "default": 1 },
                    "size": { "type": "integer", "minimum": 1, "maximum": 100, "default": 20 },
                    "search": { "type": "string" },
                    "enabled": { "type": "boolean" }
                },
                "additionalProperties": false
            }),
        ),
    )
}

fn platform_model_get_definition() -> ToolCatalogDefinition {
    platform_model_code_definition("platform.model.get", ToolOperationKind::Query)
}

fn platform_model_create_definition() -> ToolCatalogDefinition {
    definition(
        "platform.model.create",
        "model",
        ToolOperationKind::Command,
        schema(
            "agent.toolCatalog.tools.platform.model.create.description",
            json!([{ "displayName": "Qwen", "vendor": "Alibaba", "protocol": "responses", "modelType": "text", "baseUrl": "https://example.com/v1", "apiKeyRef": "OPENAI_API_KEY", "modelName": "qwen" }]),
            platform_model_mutation_schema(true),
        ),
    )
}

fn platform_model_update_definition() -> ToolCatalogDefinition {
    definition(
        "platform.model.update",
        "model",
        ToolOperationKind::Command,
        schema(
            "agent.toolCatalog.tools.platform.model.update.description",
            json!([{ "modelCode": "MODEL-1", "displayName": "Qwen updated", "vendor": "Alibaba", "protocol": "responses", "modelType": "text", "baseUrl": "https://example.com/v1", "apiKeyRef": "OPENAI_API_KEY", "modelName": "qwen" }]),
            platform_model_mutation_schema(false),
        ),
    )
}

fn platform_model_enable_definition() -> ToolCatalogDefinition {
    platform_model_code_definition("platform.model.enable", ToolOperationKind::Command)
}

fn platform_model_disable_definition() -> ToolCatalogDefinition {
    platform_model_code_definition("platform.model.disable", ToolOperationKind::Command)
}

fn platform_model_delete_definition() -> ToolCatalogDefinition {
    platform_model_code_definition("platform.model.delete", ToolOperationKind::Command)
}

fn platform_model_code_definition(
    tool_code: &'static str,
    operation_kind: ToolOperationKind,
) -> ToolCatalogDefinition {
    definition(
        tool_code,
        "model",
        operation_kind,
        schema(
            match tool_code {
                "platform.model.get" => "agent.toolCatalog.tools.platform.model.get.description",
                "platform.model.enable" => {
                    "agent.toolCatalog.tools.platform.model.enable.description"
                }
                "platform.model.disable" => {
                    "agent.toolCatalog.tools.platform.model.disable.description"
                }
                _ => "agent.toolCatalog.tools.platform.model.delete.description",
            },
            json!([{ "modelCode": "MODEL-1" }]),
            json!({
                "type": "object",
                "required": ["modelCode"],
                "properties": { "modelCode": { "type": "string", "minLength": 1 } },
                "additionalProperties": false
            }),
        ),
    )
}

fn platform_model_mutation_schema(create: bool) -> serde_json::Value {
    let mut value = model_mutation_schema(create);
    if let Some(properties) = value
        .get_mut("properties")
        .and_then(|value| value.as_object_mut())
    {
        properties.remove("workspaceCode");
        properties.remove("allowInheritance");
    }
    value
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

fn platform_variable_page_definition() -> ToolCatalogDefinition {
    definition(
        "platform.variable.page",
        "variable",
        ToolOperationKind::Query,
        schema(
            "agent.toolCatalog.tools.platform.variable.page.description",
            json!([{ "current": 1, "size": 20, "search": "OPENAI" }]),
            json!({
                "type": "object",
                "properties": {
                    "current": { "type": "integer", "minimum": 1, "default": 1 },
                    "size": { "type": "integer", "minimum": 1, "maximum": 100, "default": 20 },
                    "search": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.platform.variable.page.params.search" }
                },
                "additionalProperties": false
            }),
        ),
    )
}

fn platform_variable_create_definition() -> ToolCatalogDefinition {
    definition(
        "platform.variable.create",
        "variable",
        ToolOperationKind::Command,
        schema(
            "agent.toolCatalog.tools.platform.variable.create.description",
            json!([{ "key": "OPENAI_API_KEY", "value": "sk-...", "isSensitive": true }]),
            platform_variable_mutation_schema(true),
        ),
    )
}

fn platform_variable_update_definition() -> ToolCatalogDefinition {
    definition(
        "platform.variable.update",
        "variable",
        ToolOperationKind::Command,
        schema(
            "agent.toolCatalog.tools.platform.variable.update.description",
            json!([{ "key": "OPENAI_API_KEY", "valueMode": "preserve", "description": "Updated description" }]),
            platform_variable_mutation_schema(false),
        ),
    )
}

fn platform_variable_delete_definition() -> ToolCatalogDefinition {
    definition(
        "platform.variable.delete",
        "variable",
        ToolOperationKind::Command,
        schema(
            "agent.toolCatalog.tools.platform.variable.delete.description",
            json!([{ "key": "OPENAI_API_KEY" }]),
            json!({
                "type": "object",
                "required": ["key"],
                "properties": {
                    "key": { "type": "string", "minLength": 1, "x-i18n-description-key": "agent.toolCatalog.tools.variable.common.params.key" }
                },
                "additionalProperties": false
            }),
        ),
    )
}

fn platform_variable_mutation_schema(create: bool) -> serde_json::Value {
    let required = if create {
        json!(["key", "value"])
    } else {
        json!(["key"])
    };
    json!({
        "type": "object",
        "required": required,
        "properties": {
            "key": { "type": "string", "minLength": 1, "x-i18n-description-key": "agent.toolCatalog.tools.variable.common.params.key" },
            "value": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.variable.common.params.value" },
            "description": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.common.params.description" },
            "isSensitive": { "type": "boolean", "x-i18n-description-key": "agent.toolCatalog.tools.variable.common.params.isSensitive" },
            "valueMode": { "type": "string", "enum": ["replace", "preserve"], "x-i18n-description-key": "agent.toolCatalog.tools.variable.common.params.valueMode" }
        },
        "additionalProperties": false
    })
}

fn platform_prompt_page_definition() -> ToolCatalogDefinition {
    definition(
        "platform.prompt.page",
        "prompt",
        ToolOperationKind::Query,
        schema(
            "agent.toolCatalog.tools.platform.prompt.page.description",
            json!([{ "current": 1, "size": 20, "search": "release", "enabled": true }]),
            json!({
                "type": "object",
                "properties": {
                    "current": { "type": "integer", "minimum": 1, "default": 1 },
                    "size": { "type": "integer", "minimum": 1, "maximum": 100, "default": 20 },
                    "search": { "type": "string" },
                    "category": { "type": "string", "enum": ["SYSTEM", "PERSONA", "TOOL", "TEMPLATE"] },
                    "enabled": { "type": "boolean" }
                },
                "additionalProperties": false
            }),
        ),
    )
}

fn platform_prompt_get_definition() -> ToolCatalogDefinition {
    platform_code_definition(
        "platform.prompt.get",
        "prompt",
        "promptCode",
        "PROMPT-1",
        ToolOperationKind::Query,
    )
}

fn platform_prompt_create_definition() -> ToolCatalogDefinition {
    definition(
        "platform.prompt.create",
        "prompt",
        ToolOperationKind::Command,
        schema(
            "agent.toolCatalog.tools.platform.prompt.create.description",
            json!([{ "displayName": "Release summary", "category": "TEMPLATE", "promptBody": "Summarize {{release}}", "enabled": true, "variables": [{ "name": "release", "type": "STRING", "description": "Release name" }] }]),
            platform_prompt_mutation_schema(true),
        ),
    )
}

fn platform_prompt_update_definition() -> ToolCatalogDefinition {
    definition(
        "platform.prompt.update",
        "prompt",
        ToolOperationKind::Command,
        schema(
            "agent.toolCatalog.tools.platform.prompt.update.description",
            json!([{ "promptCode": "PROMPT-1", "expectedVersion": 1, "displayName": "Release summary", "promptBody": "Summarize {{release}}", "enabled": true, "variables": [] }]),
            platform_prompt_mutation_schema(false),
        ),
    )
}

fn platform_prompt_delete_definition() -> ToolCatalogDefinition {
    platform_code_definition(
        "platform.prompt.delete",
        "prompt",
        "promptCode",
        "PROMPT-1",
        ToolOperationKind::Command,
    )
}

fn platform_prompt_mutation_schema(create: bool) -> serde_json::Value {
    let mut properties = json!({
        "displayName": { "type": "string", "minLength": 1 },
        "category": { "type": "string", "enum": ["SYSTEM", "PERSONA", "TOOL", "TEMPLATE"] },
        "promptBody": { "type": "string" },
        "description": { "type": "string" },
        "enabled": { "type": "boolean", "default": true },
        "variables": { "type": "array", "items": {
            "type": "object",
            "required": ["name", "description"],
            "properties": {
                "name": { "type": "string" },
                "type": { "type": "string", "enum": ["STRING", "NUMBER", "BOOLEAN", "JSON"], "default": "STRING" },
                "description": { "type": "string" }
            },
            "additionalProperties": false
        }}
    });
    let required = if create {
        json!(["displayName", "category", "promptBody", "variables"])
    } else {
        json!([
            "promptCode",
            "expectedVersion",
            "displayName",
            "promptBody",
            "variables"
        ])
    };
    if !create {
        properties
            .as_object_mut()
            .expect("prompt schema properties")
            .remove("category");
        properties["promptCode"] = json!({ "type": "string", "minLength": 1 });
        properties["expectedVersion"] = json!({ "type": "integer", "minimum": 1 });
    }
    json!({ "type": "object", "required": required, "properties": properties, "additionalProperties": false })
}

fn platform_mcp_page_definition() -> ToolCatalogDefinition {
    definition(
        "platform.mcp.page",
        "mcp",
        ToolOperationKind::Query,
        schema(
            "agent.toolCatalog.tools.platform.mcp.page.description",
            json!([{ "current": 1, "size": 20, "search": "github" }]),
            json!({ "type": "object", "properties": { "current": { "type": "integer", "minimum": 1 }, "size": { "type": "integer", "minimum": 1, "maximum": 100 }, "search": { "type": "string" } }, "additionalProperties": false }),
        ),
    )
}

fn platform_mcp_get_definition() -> ToolCatalogDefinition {
    platform_code_definition(
        "platform.mcp.get",
        "mcp",
        "serverCode",
        "MCP-1",
        ToolOperationKind::Query,
    )
}

fn platform_mcp_create_definition() -> ToolCatalogDefinition {
    definition(
        "platform.mcp.create",
        "mcp",
        ToolOperationKind::Command,
        schema(
            "agent.toolCatalog.tools.platform.mcp.create.description",
            json!([{ "serverCode": "github", "name": "GitHub MCP", "transportKind": "streamable_http", "url": "https://mcp.example.com", "authKind": "none" }]),
            platform_mcp_mutation_schema(true),
        ),
    )
}

fn platform_mcp_update_definition() -> ToolCatalogDefinition {
    definition(
        "platform.mcp.update",
        "mcp",
        ToolOperationKind::Command,
        schema(
            "agent.toolCatalog.tools.platform.mcp.update.description",
            json!([{ "serverCode": "github", "name": "GitHub MCP", "transportKind": "streamable_http", "url": "https://mcp.example.com", "authKind": "none" }]),
            platform_mcp_mutation_schema(false),
        ),
    )
}

fn platform_mcp_delete_definition() -> ToolCatalogDefinition {
    platform_code_definition(
        "platform.mcp.delete",
        "mcp",
        "serverCode",
        "github",
        ToolOperationKind::Command,
    )
}

fn platform_mcp_mutation_schema(create: bool) -> serde_json::Value {
    let required = if create {
        json!(["serverCode", "name", "transportKind"])
    } else {
        json!(["serverCode", "name", "transportKind"])
    };
    json!({
        "type": "object", "required": required,
        "properties": {
            "serverCode": { "type": "string", "minLength": 1 }, "name": { "type": "string", "minLength": 1 }, "description": { "type": "string" },
            "transportKind": { "type": "string", "enum": ["streamable_http", "stdio"] }, "url": { "type": "string" },
            "authKind": { "type": "string", "enum": ["none", "bearer_static"] }, "authPayload": { "type": "object" },
            "headers": { "type": "object", "additionalProperties": { "type": "string" } }, "headersVariableRefs": { "type": "object", "additionalProperties": { "type": "string" } },
            "command": { "type": "string" }, "args": { "type": "array", "items": { "type": "string" } },
            "envVariableRefs": { "type": "object", "additionalProperties": { "type": "string" } }, "envPassthroughRefs": { "type": "array", "items": { "type": "string" } }, "workingDirectory": { "type": "string" }
        }, "additionalProperties": false
    })
}

fn platform_open_app_page_definition() -> ToolCatalogDefinition {
    definition(
        "platform.open_app.page",
        "open_app",
        ToolOperationKind::Query,
        schema(
            "agent.toolCatalog.tools.platform.open_app.page.description",
            json!([{ "current": 1, "size": 20, "search": "client" }]),
            json!({ "type": "object", "properties": { "current": { "type": "integer", "minimum": 1 }, "size": { "type": "integer", "minimum": 1, "maximum": 100 }, "search": { "type": "string" }, "status": { "type": "string", "enum": ["Enabled", "Disabled", "Archived"] } }, "additionalProperties": false }),
        ),
    )
}

fn platform_open_app_get_definition() -> ToolCatalogDefinition {
    platform_code_definition(
        "platform.open_app.get",
        "open_app",
        "openAppCode",
        "OPENAPP-1",
        ToolOperationKind::Query,
    )
}

fn platform_open_app_create_definition() -> ToolCatalogDefinition {
    definition(
        "platform.open_app.create",
        "open_app",
        ToolOperationKind::Command,
        schema(
            "agent.toolCatalog.tools.platform.open_app.create.description",
            json!([{ "name": "External client", "description": "API client" }]),
            json!({ "type": "object", "required": ["name"], "properties": { "name": { "type": "string", "minLength": 1 }, "description": { "type": "string" } }, "additionalProperties": false }),
        ),
    )
}

fn platform_open_app_update_definition() -> ToolCatalogDefinition {
    definition(
        "platform.open_app.update",
        "open_app",
        ToolOperationKind::Command,
        schema(
            "agent.toolCatalog.tools.platform.open_app.update.description",
            json!([{ "openAppCode": "OPENAPP-1", "name": "External client", "status": "Enabled" }]),
            json!({ "type": "object", "required": ["openAppCode", "name", "status"], "properties": { "openAppCode": { "type": "string" }, "name": { "type": "string" }, "description": { "type": "string" }, "status": { "type": "string", "enum": ["Enabled", "Disabled", "Archived"] } }, "additionalProperties": false }),
        ),
    )
}

fn platform_open_app_disable_definition() -> ToolCatalogDefinition {
    platform_code_definition(
        "platform.open_app.disable",
        "open_app",
        "openAppCode",
        "OPENAPP-1",
        ToolOperationKind::Command,
    )
}

fn platform_resource_query_definition() -> ToolCatalogDefinition {
    definition(
        "platform.resource.query",
        "resource",
        ToolOperationKind::Query,
        schema(
            "agent.toolCatalog.tools.platform.resource.query.description",
            json!([{ "current": 1, "size": 20, "kind": "File" }]),
            json!({ "type": "object", "properties": { "current": { "type": "integer", "minimum": 1 }, "size": { "type": "integer", "minimum": 1, "maximum": 50 }, "search": { "type": "string" }, "kind": { "type": "string" } }, "additionalProperties": false }),
        ),
    )
}

fn platform_resource_get_definition() -> ToolCatalogDefinition {
    platform_code_definition(
        "platform.resource.get",
        "resource",
        "resourceCode",
        "release-notes",
        ToolOperationKind::Query,
    )
}

fn platform_resource_create_definition() -> ToolCatalogDefinition {
    definition(
        "platform.resource.create",
        "resource",
        ToolOperationKind::Command,
        schema(
            "agent.toolCatalog.tools.platform.resource.create.description",
            json!([{ "resourceCode": "release-notes", "kind": "File", "name": "Release notes", "sourcePath": "release-notes.md" }]),
            json!({ "type": "object", "required": ["resourceCode", "kind", "name", "sourcePath"], "properties": { "resourceCode": { "type": "string" }, "kind": resource_file_kind_schema(), "name": { "type": "string" }, "description": { "type": "string" }, "sourcePath": { "type": "string" } }, "additionalProperties": false }),
        ),
    )
}

fn platform_resource_update_definition() -> ToolCatalogDefinition {
    definition(
        "platform.resource.update",
        "resource",
        ToolOperationKind::Command,
        schema(
            "agent.toolCatalog.tools.platform.resource.update.description",
            json!([{ "resourceCode": "release-notes", "name": "Release notes v2" }]),
            json!({ "type": "object", "required": ["resourceCode"], "properties": { "resourceCode": { "type": "string" }, "name": { "type": "string" }, "description": { "type": "string" } }, "additionalProperties": false }),
        ),
    )
}

fn platform_resource_delete_definition() -> ToolCatalogDefinition {
    platform_code_definition(
        "platform.resource.delete",
        "resource",
        "resourceCode",
        "release-notes",
        ToolOperationKind::Command,
    )
}

fn platform_resource_binary_definition() -> ToolCatalogDefinition {
    definition(
        "platform.resource.binary",
        "resource",
        ToolOperationKind::Command,
        schema(
            "agent.toolCatalog.tools.platform.resource.binary.description",
            json!([{ "action": "upload", "resourceCode": "release-notes", "sourcePath": "notes.txt" }]),
            json!({ "type": "object", "required": ["action", "resourceCode"], "properties": { "action": { "type": "string", "enum": ["upload", "download"] }, "resourceCode": { "type": "string" }, "sourcePath": { "type": "string" }, "targetPath": { "type": "string" }, "overwrite": { "type": "boolean" } }, "additionalProperties": false }),
        ),
    )
}

fn platform_code_definition(
    tool_code: &'static str,
    domain: &'static str,
    field: &'static str,
    example: &'static str,
    operation_kind: ToolOperationKind,
) -> ToolCatalogDefinition {
    let examples = json!([{}]);
    let mut example_object = examples[0].as_object().cloned().unwrap_or_default();
    example_object.insert(field.to_string(), json!(example));
    let mut properties = serde_json::Map::new();
    properties.insert(
        field.to_string(),
        json!({ "type": "string", "minLength": 1 }),
    );
    definition(
        tool_code,
        domain,
        operation_kind,
        schema(
            tool_key_for_platform(tool_code, "description"),
            json!([example_object]),
            json!({
                "type": "object",
                "required": [field],
                "properties": properties,
                "additionalProperties": false
            }),
        ),
    )
}

fn tool_key_for_platform(tool_code: &'static str, suffix: &'static str) -> &'static str {
    match (tool_code, suffix) {
        ("platform.prompt.get", "description") => {
            "agent.toolCatalog.tools.platform.prompt.get.description"
        }
        ("platform.prompt.delete", "description") => {
            "agent.toolCatalog.tools.platform.prompt.delete.description"
        }
        ("platform.mcp.get", "description") => {
            "agent.toolCatalog.tools.platform.mcp.get.description"
        }
        ("platform.mcp.delete", "description") => {
            "agent.toolCatalog.tools.platform.mcp.delete.description"
        }
        ("platform.open_app.get", "description") => {
            "agent.toolCatalog.tools.platform.open_app.get.description"
        }
        ("platform.open_app.disable", "description") => {
            "agent.toolCatalog.tools.platform.open_app.disable.description"
        }
        ("platform.resource.get", "description") => {
            "agent.toolCatalog.tools.platform.resource.get.description"
        }
        ("platform.resource.delete", "description") => {
            "agent.toolCatalog.tools.platform.resource.delete.description"
        }
        _ => "agent.toolCatalog.tools.platform.resource.get.description",
    }
}

fn resource_query_definition() -> ToolCatalogDefinition {
    definition(
        "resource.query",
        "resource",
        ToolOperationKind::Query,
        schema(
            "agent.toolCatalog.tools.resource.query.description",
            json!([{ "workspaceCode": "TRON_CORE", "kind": "File", "current": 1, "size": 20 }]),
            json!({
                "type": "object",
                "properties": {
                    "workspaceCode": resource_workspace_code_schema(),
                    "search": { "type": "string" },
                    "kind": resource_kind_schema(),
                    "current": { "type": "integer", "minimum": 1, "default": 1 },
                    "size": { "type": "integer", "minimum": 1, "maximum": 50, "default": 20 }
                },
                "additionalProperties": false
            }),
        ),
    )
}

fn resource_get_definition() -> ToolCatalogDefinition {
    definition(
        "resource.get",
        "resource",
        ToolOperationKind::Query,
        schema(
            "agent.toolCatalog.tools.resource.get.description",
            json!([{ "workspaceCode": "TRON_CORE", "resourceCode": "release-notes" }]),
            json!({
                "type": "object",
                "required": ["resourceCode"],
                "properties": {
                    "workspaceCode": resource_workspace_code_schema(),
                    "resourceCode": resource_code_schema()
                },
                "additionalProperties": false
            }),
        ),
    )
}

fn resource_create_definition() -> ToolCatalogDefinition {
    definition(
        "resource.create",
        "resource",
        ToolOperationKind::Command,
        schema(
            "agent.toolCatalog.tools.resource.create.description",
            json!([{
                "workspaceCode": "TRON_CORE",
                "resourceCode": "release-notes",
                "kind": "File",
                "name": "Release notes",
                "description": "Current release notes",
                "allowInheritance": false,
                "sourcePath": "release-notes.md"
            }]),
            json!({
                "type": "object",
                "required": ["resourceCode", "kind", "name", "allowInheritance", "sourcePath"],
                "properties": {
                    "workspaceCode": resource_workspace_code_schema(),
                    "resourceCode": resource_code_schema(),
                    "kind": resource_file_kind_schema(),
                    "name": { "type": "string", "minLength": 1 },
                    "description": { "type": "string" },
                    "allowInheritance": { "type": "boolean" },
                    "sourcePath": { "type": "string", "minLength": 1 }
                },
                "additionalProperties": false
            }),
        ),
    )
}

fn resource_update_definition() -> ToolCatalogDefinition {
    definition(
        "resource.update",
        "resource",
        ToolOperationKind::Command,
        schema(
            "agent.toolCatalog.tools.resource.update.description",
            json!([{
                "workspaceCode": "TRON_CORE",
                "resourceCode": "release-notes",
                "name": "Release notes v2",
                "allowInheritance": true
            }]),
            json!({
                "type": "object",
                "required": ["resourceCode"],
                "properties": {
                    "workspaceCode": resource_workspace_code_schema(),
                    "resourceCode": resource_code_schema(),
                    "name": { "type": "string", "minLength": 1 },
                    "description": { "type": "string" },
                    "allowInheritance": { "type": "boolean" }
                },
                "additionalProperties": false
            }),
        ),
    )
}

fn resource_delete_definition() -> ToolCatalogDefinition {
    definition(
        "resource.delete",
        "resource",
        ToolOperationKind::Command,
        schema(
            "agent.toolCatalog.tools.resource.delete.description",
            json!([{ "workspaceCode": "TRON_CORE", "resourceCode": "release-notes" }]),
            json!({
                "type": "object",
                "required": ["resourceCode"],
                "properties": {
                    "workspaceCode": resource_workspace_code_schema(),
                    "resourceCode": resource_code_schema()
                },
                "additionalProperties": false
            }),
        ),
    )
}

fn resource_binary_definition() -> ToolCatalogDefinition {
    definition(
        "resource.binary",
        "resource",
        ToolOperationKind::Command,
        schema(
            "agent.toolCatalog.tools.resource.binary.description",
            json!([{ "action": "upload", "resourceCode": "release-notes", "sourcePath": "notes.txt" }]),
            json!({
                "type": "object",
                "required": ["action", "resourceCode"],
                "properties": {
                    "workspaceCode": resource_workspace_code_schema(),
                    "action": { "type": "string", "enum": ["upload", "download"] },
                    "resourceCode": resource_code_schema(),
                    "sourcePath": { "type": "string", "minLength": 1 },
                    "targetPath": { "type": "string", "minLength": 1 },
                    "overwrite": { "type": "boolean", "default": false }
                },
                "allOf": [{ "oneOf": [
                    { "properties": { "action": { "const": "upload" } }, "required": ["sourcePath"], "not": { "required": ["targetPath"] } },
                    { "properties": { "action": { "const": "download" } }, "required": ["targetPath"], "not": { "required": ["sourcePath"] } }
                ] }],
                "additionalProperties": false
            }),
        ),
    )
}

fn resource_data_source_create_definition() -> ToolCatalogDefinition {
    definition(
        "resource.data_source.create",
        "resource",
        ToolOperationKind::Command,
        schema(
            "agent.toolCatalog.tools.resource.data_source.create.description",
            json!([{
                "workspaceCode": "TRON_CORE",
                "resourceCode": "mysql-primary",
                "kind": "MysqlConnection",
                "name": "Primary MySQL",
                "attributes": {
                    "connectionMode": "SINGLE",
                    "endpoints": [{ "host": "mysql.internal", "port": 3306 }],
                    "database": "tron",
                    "credentials": {
                        "username": "tron_user",
                        "password": { "source": "VARIABLE", "variableKey": "MYSQL_PASSWORD" }
                    },
                    "tls": { "mode": "DISABLED" }
                },
                "allowInheritance": false
            }]),
            json!({
                "type": "object",
                "required": ["resourceCode", "kind", "name", "attributes", "allowInheritance"],
                "properties": {
                    "workspaceCode": resource_workspace_code_schema(),
                    "resourceCode": resource_code_schema(),
                    "kind": resource_data_source_kind_schema(),
                    "name": { "type": "string", "minLength": 1 },
                    "description": { "type": "string" },
                    "attributes": resource_data_source_attributes_schema(),
                    "allowInheritance": { "type": "boolean" }
                },
                "allOf": [{ "oneOf": resource_data_source_kind_attribute_branches() }],
                "additionalProperties": false
            }),
        ),
    )
}

fn resource_data_source_update_definition() -> ToolCatalogDefinition {
    definition(
        "resource.data_source.update",
        "resource",
        ToolOperationKind::Command,
        schema(
            "agent.toolCatalog.tools.resource.data_source.update.description",
            json!([{
                "workspaceCode": "TRON_CORE",
                "resourceCode": "mysql-primary",
                "description": "Primary transactional database"
            }]),
            json!({
                "type": "object",
                "required": ["resourceCode"],
                "properties": {
                    "workspaceCode": resource_workspace_code_schema(),
                    "resourceCode": resource_code_schema(),
                    "name": { "type": "string", "minLength": 1 },
                    "description": { "type": "string" },
                    "attributes": resource_data_source_attributes_schema(),
                    "allowInheritance": { "type": "boolean" }
                },
                "additionalProperties": false
            }),
        ),
    )
}

fn resource_server_create_definition() -> ToolCatalogDefinition {
    definition(
        "resource.server.create",
        "resource",
        ToolOperationKind::Command,
        schema(
            "agent.toolCatalog.tools.resource.server.create.description",
            json!([{
                "workspaceCode": "TRON_CORE",
                "serverCode": "production-bastion",
                "name": "Production bastion",
                "host": "bastion.internal",
                "port": 22,
                "allowInheritance": false,
                "initialAccount": {
                    "accountCode": "bastion-ops",
                    "name": "Operations account",
                    "username": "ops",
                    "authenticationType": "Password",
                    "passwordVariableKey": "BASTION_PASSWORD",
                    "allowInheritance": false
                }
            }]),
            json!({
                "type": "object",
                "required": ["serverCode", "name", "host", "port", "allowInheritance", "initialAccount"],
                "properties": {
                    "workspaceCode": resource_workspace_code_schema(),
                    "serverCode": resource_code_schema(),
                    "name": { "type": "string", "minLength": 1 },
                    "description": { "type": "string" },
                    "host": { "type": "string", "minLength": 1 },
                    "port": { "type": "integer", "minimum": 1, "maximum": 65535 },
                    "allowInheritance": { "type": "boolean" },
                    "initialAccount": resource_server_account_schema()
                },
                "additionalProperties": false
            }),
        ),
    )
}

fn resource_server_account_create_definition() -> ToolCatalogDefinition {
    definition(
        "resource.server_account.create",
        "resource",
        ToolOperationKind::Command,
        schema(
            "agent.toolCatalog.tools.resource.server_account.create.description",
            json!([{
                "workspaceCode": "TRON_CORE",
                "serverCode": "production-bastion",
                "account": {
                    "accountCode": "bastion-deploy",
                    "name": "Deploy account",
                    "username": "deploy",
                    "authenticationType": "PrivateKey",
                    "privateKeyVariableKey": "DEPLOY_PRIVATE_KEY",
                    "allowInheritance": false
                }
            }]),
            json!({
                "type": "object",
                "required": ["serverCode", "account"],
                "properties": {
                    "workspaceCode": resource_workspace_code_schema(),
                    "serverCode": resource_code_schema(),
                    "account": resource_server_account_schema()
                },
                "additionalProperties": false
            }),
        ),
    )
}

fn resource_kind_schema() -> serde_json::Value {
    json!({ "type": "string", "enum": ["File", "Image", "Pdf", "MysqlConnection", "PostgresqlConnection", "RedisConnection", "KafkaConnection", "ServerConnection", "ServerAccount"] })
}

fn resource_file_kind_schema() -> serde_json::Value {
    json!({ "type": "string", "enum": ["File", "Image", "Pdf"] })
}

fn resource_data_source_kind_schema() -> serde_json::Value {
    json!({ "type": "string", "enum": ["MysqlConnection", "PostgresqlConnection", "RedisConnection", "KafkaConnection"] })
}

fn resource_data_source_kind_attribute_branches() -> Vec<Value> {
    [
        ("MysqlConnection", mysql_connection_attributes_schema()),
        (
            "PostgresqlConnection",
            postgresql_connection_attributes_schema(),
        ),
        ("RedisConnection", redis_connection_attributes_schema()),
        ("KafkaConnection", kafka_connection_attributes_schema()),
    ]
    .into_iter()
    .map(|(kind, attributes)| {
        json!({
            "properties": {
                "kind": { "const": kind },
                "attributes": attributes,
            },
            "required": ["kind", "attributes"]
        })
    })
    .collect()
}

fn resource_data_source_attributes_schema() -> Value {
    json!({
        "type": "object",
        "oneOf": [
            mysql_connection_attributes_schema(),
            postgresql_connection_attributes_schema(),
            redis_connection_attributes_schema(),
            kafka_connection_attributes_schema()
        ],
        "x-i18n-description-key": "agent.toolCatalog.schemas.resource.dataSource.attributes.description"
    })
}

fn mysql_connection_attributes_schema() -> Value {
    json!({
        "title": "MysqlConnection",
        "type": "object",
        "required": ["connectionMode", "endpoints", "database", "credentials", "tls"],
        "properties": {
            "connectionMode": { "type": "string", "enum": ["SINGLE", "FAILOVER", "LOAD_BALANCE", "REPLICATION"] },
            "endpoints": {
                "type": "array", "minItems": 1, "maxItems": 128,
                "items": connection_endpoint_schema(true)
            },
            "database": { "type": "string", "minLength": 1, "maxLength": 64 },
            "credentials": credentials_schema(true),
            "tls": tls_schema(&["DISABLED", "PREFERRED", "REQUIRED", "VERIFY_CA", "VERIFY_IDENTITY"], false),
            "properties": scalar_properties_schema()
        },
        "additionalProperties": false
    })
}

fn postgresql_connection_attributes_schema() -> Value {
    json!({
        "title": "PostgresqlConnection",
        "type": "object",
        "required": ["endpoints", "database", "credentials", "targetSession", "loadBalance", "tls"],
        "properties": {
            "endpoints": {
                "type": "array", "minItems": 1, "maxItems": 128,
                "items": connection_endpoint_schema(false)
            },
            "database": { "type": "string", "minLength": 1, "maxLength": 63 },
            "credentials": credentials_schema(true),
            "targetSession": { "type": "string", "enum": ["ANY", "READ_WRITE", "READ_ONLY", "PRIMARY", "STANDBY", "PREFER_STANDBY"] },
            "loadBalance": { "type": "string", "enum": ["DISABLED", "RANDOM"] },
            "tls": tls_schema(&["DISABLED", "ALLOW", "PREFERRED", "REQUIRED", "VERIFY_CA", "VERIFY_FULL"], true),
            "properties": scalar_properties_schema()
        },
        "additionalProperties": false
    })
}

fn redis_connection_attributes_schema() -> Value {
    json!({
        "title": "RedisConnection",
        "type": "object",
        "required": ["topology", "endpoints", "database", "tls"],
        "properties": {
            "topology": { "type": "string", "enum": ["STANDALONE", "SENTINEL", "CLUSTER"] },
            "endpoints": {
                "type": "array", "minItems": 1, "maxItems": 128,
                "items": connection_endpoint_schema(false)
            },
            "database": { "type": "integer", "minimum": 0 },
            "credentials": credentials_schema(false),
            "tls": tls_schema(&["DISABLED", "REQUIRED", "VERIFY_CA", "VERIFY_FULL"], true),
            "sentinel": {
                "type": "object", "required": ["masterName", "tls"],
                "properties": {
                    "masterName": { "type": "string", "minLength": 1, "maxLength": 128, "pattern": "^\\S+$" },
                    "credentials": credentials_schema(false),
                    "tls": tls_schema(&["DISABLED", "REQUIRED", "VERIFY_CA", "VERIFY_FULL"], true)
                },
                "additionalProperties": false
            },
            "properties": scalar_properties_schema()
        },
        "allOf": [{ "oneOf": [
            { "properties": { "topology": { "const": "SENTINEL" } }, "required": ["sentinel"] },
            { "properties": { "topology": { "enum": ["STANDALONE", "CLUSTER"] } }, "not": { "required": ["sentinel"] } }
        ] }],
        "additionalProperties": false
    })
}

fn kafka_connection_attributes_schema() -> Value {
    json!({
        "title": "KafkaConnection",
        "type": "object",
        "required": ["brokers", "securityProtocol"],
        "properties": {
            "brokers": {
                "type": "array", "minItems": 1, "maxItems": 128,
                "items": connection_endpoint_schema(false)
            },
            "securityProtocol": { "type": "string", "enum": ["PLAINTEXT", "SSL", "SASL_PLAINTEXT", "SASL_SSL"] },
            "sasl": {
                "type": "object", "required": ["mechanism", "username", "password"],
                "properties": {
                    "mechanism": { "type": "string", "enum": ["PLAIN", "SCRAM_SHA_256", "SCRAM_SHA_512"] },
                    "username": { "type": "string", "minLength": 1, "maxLength": 128 },
                    "password": secret_reference_schema()
                },
                "additionalProperties": false
            },
            "tls": tls_schema(&["REQUIRED", "VERIFY_CA", "VERIFY_FULL"], true),
            "properties": scalar_properties_schema()
        },
        "allOf": [
            { "oneOf": [
                { "properties": { "securityProtocol": { "enum": ["SASL_PLAINTEXT", "SASL_SSL"] } }, "required": ["sasl"] },
                { "properties": { "securityProtocol": { "enum": ["PLAINTEXT", "SSL"] } }, "not": { "required": ["sasl"] } }
            ] },
            { "oneOf": [
                { "properties": { "securityProtocol": { "enum": ["SSL", "SASL_SSL"] } }, "required": ["tls"] },
                { "properties": { "securityProtocol": { "enum": ["PLAINTEXT", "SASL_PLAINTEXT"] } }, "not": { "required": ["tls"] } }
            ] }
        ],
        "additionalProperties": false
    })
}

fn connection_endpoint_schema(include_role: bool) -> Value {
    let mut properties = Map::new();
    properties.insert(
        "host".to_string(),
        json!({ "type": "string", "minLength": 1, "maxLength": 255 }),
    );
    properties.insert(
        "port".to_string(),
        json!({ "type": "integer", "minimum": 1, "maximum": 65535 }),
    );
    if include_role {
        properties.insert(
            "role".to_string(),
            json!({ "type": "string", "enum": ["PRIMARY", "REPLICA"] }),
        );
    }
    json!({
        "type": "object",
        "required": ["host", "port"],
        "properties": properties,
        "additionalProperties": false
    })
}

fn credentials_schema(username_required: bool) -> Value {
    let mut required = vec![Value::String("password".to_string())];
    if username_required {
        required.push(Value::String("username".to_string()));
    }
    json!({
        "type": "object",
        "required": required,
        "properties": {
            "username": { "type": "string", "minLength": 1, "maxLength": 128 },
            "password": secret_reference_schema()
        },
        "additionalProperties": false
    })
}

fn secret_reference_schema() -> Value {
    json!({
        "oneOf": [
            { "type": "string", "minLength": 1, "maxLength": 128, "description": "Legacy variable key" },
            {
                "type": "object",
                "oneOf": [
                    {
                        "properties": {
                            "source": { "const": "DIRECT" },
                            "value": { "type": "string", "minLength": 1, "maxLength": 4096 }
                        },
                        "required": ["source", "value"],
                        "additionalProperties": false
                    },
                    {
                        "properties": {
                            "source": { "const": "VARIABLE" },
                            "variableKey": { "type": "string", "minLength": 1, "maxLength": 128 }
                        },
                        "required": ["source", "variableKey"],
                        "additionalProperties": false
                    }
                ]
            }
        ]
    })
}

fn tls_schema(modes: &[&str], enforce_client_key_dependencies: bool) -> Value {
    let mut schema = json!({
        "type": "object",
        "required": ["mode"],
        "properties": {
            "mode": { "type": "string", "enum": modes },
            "caCertificate": { "type": "string", "minLength": 1, "maxLength": 128 },
            "clientCertificate": { "type": "string", "minLength": 1, "maxLength": 128 },
            "clientKey": { "type": "string", "minLength": 1, "maxLength": 128 },
            "clientKeyPassword": { "type": "string", "minLength": 1, "maxLength": 128 }
        },
        "additionalProperties": false
    });
    if enforce_client_key_dependencies {
        schema["allOf"] = json!([
            {
                "oneOf": [
                    { "required": ["clientCertificate", "clientKey"] },
                    { "not": { "anyOf": [{ "required": ["clientCertificate"] }, { "required": ["clientKey"] }] } }
                ]
            },
            { "oneOf": [
                { "required": ["clientKeyPassword", "clientKey"] },
                { "not": { "required": ["clientKeyPassword"] } }
            ] }
        ]);
    }
    schema
}

fn scalar_properties_schema() -> Value {
    json!({
        "type": "object",
        "maxProperties": 128,
        "additionalProperties": {
            "oneOf": [
                { "type": "string" },
                { "type": "number" },
                { "type": "boolean" }
            ]
        }
    })
}

fn resource_server_account_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "required": ["accountCode", "name", "username", "authenticationType", "allowInheritance"],
        "properties": {
            "accountCode": resource_code_schema(),
            "name": { "type": "string", "minLength": 1 },
            "description": { "type": "string" },
            "username": { "type": "string", "minLength": 1 },
            "authenticationType": { "type": "string", "enum": ["Password", "PrivateKey"] },
            "passwordVariableKey": { "type": "string", "minLength": 1 },
            "privateKeyVariableKey": { "type": "string", "minLength": 1 },
            "privateKeyPassphraseVariableKey": { "type": "string", "minLength": 1 },
            "allowInheritance": { "type": "boolean" }
        },
        "allOf": [{ "oneOf": [
            { "properties": { "authenticationType": { "const": "Password" } }, "required": ["passwordVariableKey"] },
            { "properties": { "authenticationType": { "const": "PrivateKey" } }, "required": ["privateKeyVariableKey"] }
        ] }],
        "additionalProperties": false
    })
}

fn resource_workspace_code_schema() -> serde_json::Value {
    json!({ "type": "string", "minLength": 1 })
}

fn resource_code_schema() -> serde_json::Value {
    json!({ "type": "string", "minLength": 1, "maxLength": 64 })
}
