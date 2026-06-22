use serde_json::{Value, json};

use crate::domain::agent::{ToolAvailabilityState, ToolOperationKind};

use super::super::{ToolCatalogDefinition, schema};

pub(super) fn definition(
    tool_code: &'static str,
    domain_code: &'static str,
    operation_kind: ToolOperationKind,
    input_schema: Value,
) -> ToolCatalogDefinition {
    ToolCatalogDefinition {
        tool_code,
        domain_code,
        name_i18n_key: tool_key(tool_code, "name"),
        description_i18n_key: tool_key(tool_code, "description"),
        operation_kind,
        input_schema,
        module_owner: domain_code,
        availability_state: ToolAvailabilityState::Available,
        sample_result_summary_i18n_key: tool_key(tool_code, "sampleResultSummary"),
    }
}

fn tool_key(tool_code: &'static str, suffix: &'static str) -> &'static str {
    match (tool_code, suffix) {
        ("workspace.page", "name") => "agent.toolCatalog.tools.workspace.page.name",
        ("workspace.page", "description") => "agent.toolCatalog.tools.workspace.page.description",
        ("workspace.page", "sampleResultSummary") => {
            "agent.toolCatalog.tools.workspace.page.sampleResultSummary"
        }
        ("workspace.get", "name") => "agent.toolCatalog.tools.workspace.get.name",
        ("workspace.get", "description") => "agent.toolCatalog.tools.workspace.get.description",
        ("workspace.get", "sampleResultSummary") => {
            "agent.toolCatalog.tools.workspace.get.sampleResultSummary"
        }
        ("app.page", "name") => "agent.toolCatalog.tools.app.page.name",
        ("app.page", "description") => "agent.toolCatalog.tools.app.page.description",
        ("app.page", "sampleResultSummary") => {
            "agent.toolCatalog.tools.app.page.sampleResultSummary"
        }
        ("app.create", "name") => "agent.toolCatalog.tools.app.create.name",
        ("app.create", "description") => "agent.toolCatalog.tools.app.create.description",
        ("app.create", "sampleResultSummary") => {
            "agent.toolCatalog.tools.app.create.sampleResultSummary"
        }
        ("app.update", "name") => "agent.toolCatalog.tools.app.update.name",
        ("app.update", "description") => "agent.toolCatalog.tools.app.update.description",
        ("app.update", "sampleResultSummary") => {
            "agent.toolCatalog.tools.app.update.sampleResultSummary"
        }
        ("app.delete", "name") => "agent.toolCatalog.tools.app.delete.name",
        ("app.delete", "description") => "agent.toolCatalog.tools.app.delete.description",
        ("app.delete", "sampleResultSummary") => {
            "agent.toolCatalog.tools.app.delete.sampleResultSummary"
        }
        ("app.restore", "name") => "agent.toolCatalog.tools.app.restore.name",
        ("app.restore", "description") => "agent.toolCatalog.tools.app.restore.description",
        ("app.restore", "sampleResultSummary") => {
            "agent.toolCatalog.tools.app.restore.sampleResultSummary"
        }
        ("task.page", "name") => "agent.toolCatalog.tools.task.page.name",
        ("task.page", "description") => "agent.toolCatalog.tools.task.page.description",
        ("task.page", "sampleResultSummary") => {
            "agent.toolCatalog.tools.task.page.sampleResultSummary"
        }
        ("task.create", "name") => "agent.toolCatalog.tools.task.create.name",
        ("task.create", "description") => "agent.toolCatalog.tools.task.create.description",
        ("task.create", "sampleResultSummary") => {
            "agent.toolCatalog.tools.task.create.sampleResultSummary"
        }
        ("task.update", "name") => "agent.toolCatalog.tools.task.update.name",
        ("task.update", "description") => "agent.toolCatalog.tools.task.update.description",
        ("task.update", "sampleResultSummary") => {
            "agent.toolCatalog.tools.task.update.sampleResultSummary"
        }
        ("task.delete", "name") => "agent.toolCatalog.tools.task.delete.name",
        ("task.delete", "description") => "agent.toolCatalog.tools.task.delete.description",
        ("task.delete", "sampleResultSummary") => {
            "agent.toolCatalog.tools.task.delete.sampleResultSummary"
        }
        ("task.restore", "name") => "agent.toolCatalog.tools.task.restore.name",
        ("task.restore", "description") => "agent.toolCatalog.tools.task.restore.description",
        ("task.restore", "sampleResultSummary") => {
            "agent.toolCatalog.tools.task.restore.sampleResultSummary"
        }
        ("model.page", "name") => "agent.toolCatalog.tools.model.page.name",
        ("model.page", "description") => "agent.toolCatalog.tools.model.page.description",
        ("model.page", "sampleResultSummary") => {
            "agent.toolCatalog.tools.model.page.sampleResultSummary"
        }
        ("model.create", "name") => "agent.toolCatalog.tools.model.create.name",
        ("model.create", "description") => "agent.toolCatalog.tools.model.create.description",
        ("model.create", "sampleResultSummary") => {
            "agent.toolCatalog.tools.model.create.sampleResultSummary"
        }
        ("model.update", "name") => "agent.toolCatalog.tools.model.update.name",
        ("model.update", "description") => "agent.toolCatalog.tools.model.update.description",
        ("model.update", "sampleResultSummary") => {
            "agent.toolCatalog.tools.model.update.sampleResultSummary"
        }
        ("model.enable", "name") => "agent.toolCatalog.tools.model.enable.name",
        ("model.enable", "description") => "agent.toolCatalog.tools.model.enable.description",
        ("model.enable", "sampleResultSummary") => {
            "agent.toolCatalog.tools.model.enable.sampleResultSummary"
        }
        ("model.disable", "name") => "agent.toolCatalog.tools.model.disable.name",
        ("model.disable", "description") => "agent.toolCatalog.tools.model.disable.description",
        ("model.disable", "sampleResultSummary") => {
            "agent.toolCatalog.tools.model.disable.sampleResultSummary"
        }
        ("model.delete", "name") => "agent.toolCatalog.tools.model.delete.name",
        ("model.delete", "description") => "agent.toolCatalog.tools.model.delete.description",
        ("model.delete", "sampleResultSummary") => {
            "agent.toolCatalog.tools.model.delete.sampleResultSummary"
        }
        ("skill.page", "name") => "agent.toolCatalog.tools.skill.page.name",
        ("skill.page", "description") => "agent.toolCatalog.tools.skill.page.description",
        ("skill.page", "sampleResultSummary") => {
            "agent.toolCatalog.tools.skill.page.sampleResultSummary"
        }
        ("skill.view", "name") => "agent.toolCatalog.tools.skill.view.name",
        ("skill.view", "description") => "agent.toolCatalog.tools.skill.view.description",
        ("skill.view", "sampleResultSummary") => {
            "agent.toolCatalog.tools.skill.view.sampleResultSummary"
        }
        ("variable.page", "name") => "agent.toolCatalog.tools.variable.page.name",
        ("variable.page", "description") => "agent.toolCatalog.tools.variable.page.description",
        ("variable.page", "sampleResultSummary") => {
            "agent.toolCatalog.tools.variable.page.sampleResultSummary"
        }
        ("variable.create", "name") => "agent.toolCatalog.tools.variable.create.name",
        ("variable.create", "description") => "agent.toolCatalog.tools.variable.create.description",
        ("variable.create", "sampleResultSummary") => {
            "agent.toolCatalog.tools.variable.create.sampleResultSummary"
        }
        ("variable.update", "name") => "agent.toolCatalog.tools.variable.update.name",
        ("variable.update", "description") => "agent.toolCatalog.tools.variable.update.description",
        ("variable.update", "sampleResultSummary") => {
            "agent.toolCatalog.tools.variable.update.sampleResultSummary"
        }
        ("variable.delete", "name") => "agent.toolCatalog.tools.variable.delete.name",
        ("variable.delete", "description") => "agent.toolCatalog.tools.variable.delete.description",
        ("variable.delete", "sampleResultSummary") => {
            "agent.toolCatalog.tools.variable.delete.sampleResultSummary"
        }
        _ => "agent.toolCatalog.tools.unknown.name",
    }
}

pub(super) fn page_properties(extra: Value) -> Value {
    let mut properties = json!({
        "current": { "type": "integer", "minimum": 1, "default": 1, "x-i18n-description-key": "agent.toolCatalog.common.params.current" },
        "size": { "type": "integer", "minimum": 1, "maximum": 100, "default": 50, "x-i18n-description-key": "agent.toolCatalog.common.params.size" }
    });
    if let (Value::Object(target), Value::Object(extra)) = (&mut properties, extra) {
        target.extend(extra);
    }
    properties
}

pub(super) fn workspace_scoped_page_schema(extra: Value) -> Value {
    json!({
        "type": "object",
        "properties": page_properties({
            let mut object = json!({
                "workspaceCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.common.params.workspaceCode" }
            });
            if let (Value::Object(target), Value::Object(extra)) = (&mut object, extra) {
                target.extend(extra);
            }
            object
        }),
        "additionalProperties": false
    })
}

pub(super) fn workspace_code_schema(description_key: &'static str, examples: Value) -> Value {
    schema(
        description_key,
        examples,
        json!({
            "type": "object",
            "required": ["workspaceCode"],
            "properties": {
                "workspaceCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.workspace.get.params.workspaceCode" }
            },
            "additionalProperties": false
        }),
    )
}

pub(super) fn app_code_definition(
    tool_code: &'static str,
    operation_kind: ToolOperationKind,
) -> ToolCatalogDefinition {
    definition(
        tool_code,
        "app",
        operation_kind,
        schema(
            tool_key(tool_code, "description"),
            json!([{ "workspaceCode": "TRON_CORE", "appCode": "APP-1" }]),
            json!({
                "type": "object",
                "required": ["appCode"],
                "properties": {
                    "workspaceCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.common.params.workspaceCode" },
                    "appCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.app.common.params.appCode" }
                },
                "additionalProperties": false
            }),
        ),
    )
}

pub(super) fn task_code_definition(
    tool_code: &'static str,
    operation_kind: ToolOperationKind,
) -> ToolCatalogDefinition {
    definition(
        tool_code,
        "task",
        operation_kind,
        schema(
            tool_key(tool_code, "description"),
            json!([{ "workspaceCode": "TRON_CORE", "taskCode": "TSK-1" }]),
            json!({
                "type": "object",
                "required": ["taskCode"],
                "properties": {
                    "workspaceCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.common.params.workspaceCode" },
                    "taskCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.task.common.params.taskCode" }
                },
                "additionalProperties": false
            }),
        ),
    )
}

pub(super) fn model_code_definition(
    tool_code: &'static str,
    operation_kind: ToolOperationKind,
) -> ToolCatalogDefinition {
    definition(
        tool_code,
        "model",
        operation_kind,
        schema(
            tool_key(tool_code, "description"),
            json!([{ "workspaceCode": "TRON_CORE", "modelCode": "MDL-1" }]),
            json!({
                "type": "object",
                "required": ["modelCode"],
                "properties": {
                    "workspaceCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.common.params.workspaceCode" },
                    "modelCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.modelCode" }
                },
                "additionalProperties": false
            }),
        ),
    )
}

pub(super) fn app_mutation_schema(create: bool) -> Value {
    let required = if create {
        json!(["name", "localDirectory", "runtimeRole"])
    } else {
        json!([])
    };
    json!({
        "type": "object",
        "required": required,
        "properties": {
            "workspaceCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.common.params.workspaceCode" },
            "name": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.common.params.name" },
            "description": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.common.params.description" },
            "localDirectory": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.app.common.params.localDirectory" },
            "runtimeRole": { "type": "string", "enum": ["frontend", "backend"], "x-i18n-description-key": "agent.toolCatalog.tools.app.common.params.runtimeRole" },
            "repositoryUrl": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.app.common.params.repositoryUrl" },
            "defaultBranch": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.app.common.params.defaultBranch" },
            "tags": { "type": "array", "items": { "type": "string" }, "default": [], "x-i18n-description-key": "agent.toolCatalog.common.params.tags" }
        },
        "additionalProperties": false
    })
}

pub(super) fn app_update_schema() -> Value {
    let mut value = app_mutation_schema(false);
    if let Value::Object(map) = &mut value {
        map.insert("required".to_string(), json!(["appCode"]));
        if let Some(Value::Object(properties)) = map.get_mut("properties") {
            properties.insert(
                "appCode".to_string(),
                json!({ "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.app.common.params.appCode" }),
            );
        }
    }
    value
}

pub(super) fn task_mutation_schema(create: bool) -> Value {
    let required = if create {
        json!(["title"])
    } else {
        json!(["taskCode"])
    };
    json!({
        "type": "object",
        "required": required,
        "properties": {
            "workspaceCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.common.params.workspaceCode" },
            "taskCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.task.common.params.taskCode" },
            "parentTaskCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.task.common.params.parentTaskCode" },
            "title": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.task.common.params.title" },
            "description": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.common.params.description" },
            "priority": { "type": "string", "enum": ["low", "medium", "high"], "x-i18n-description-key": "agent.toolCatalog.tools.task.common.params.priority" },
            "taskType": { "type": "string", "enum": ["feature", "bug"], "x-i18n-description-key": "agent.toolCatalog.tools.task.common.params.taskType" },
            "sourceType": { "type": "string", "enum": ["internal", "github", "gitlab"], "x-i18n-description-key": "agent.toolCatalog.tools.task.common.params.sourceType" },
            "sourceUrl": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.task.common.params.sourceUrl" },
            "assigneeUserCodes": { "type": "array", "items": { "type": "string" }, "x-i18n-description-key": "agent.toolCatalog.tools.task.common.params.assigneeUserCodes" },
            "agentCodes": { "type": "array", "items": { "type": "string" }, "x-i18n-description-key": "agent.toolCatalog.tools.task.common.params.agentCodes" },
            "appCodes": { "type": "array", "items": { "type": "string" }, "x-i18n-description-key": "agent.toolCatalog.tools.task.common.params.appCodes" },
            "appCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.task.common.params.appCode" },
            "status": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.task.page.params.status" },
            "progress": { "type": "integer", "minimum": 0, "maximum": 100, "x-i18n-description-key": "agent.toolCatalog.tools.task.common.params.progress" }
        },
        "additionalProperties": false
    })
}

pub(super) fn model_mutation_schema(create: bool) -> Value {
    let required = if create {
        json!([
            "displayName",
            "vendor",
            "protocol",
            "modelType",
            "baseUrl",
            "apiKeyRef",
            "modelName"
        ])
    } else {
        json!(["modelCode"])
    };
    json!({
        "type": "object",
        "required": required,
        "properties": {
            "workspaceCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.common.params.workspaceCode" },
            "modelCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.modelCode" },
            "displayName": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.displayName" },
            "vendor": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.vendor" },
            "protocol": { "type": "string", "enum": ["openai_compatible", "responses", "rerank"], "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.protocol" },
            "modelType": { "type": "string", "enum": ["text", "embedding", "rerank", "image"], "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.modelType" },
            "modelCapabilities": { "type": "array", "items": { "type": "string" }, "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.modelCapabilities" },
            "baseUrl": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.baseUrl" },
            "apiPath": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.apiPath" },
            "apiKeyRef": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.apiKeyRef" },
            "modelName": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.modelName" },
            "enabled": { "type": "boolean", "default": true, "x-i18n-description-key": "agent.toolCatalog.common.params.enabled" },
            "description": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.common.params.description" },
            "thinkingEnabled": { "type": "boolean", "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.thinkingEnabled" },
            "structuredOutputEnabled": { "type": "boolean", "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.structuredOutputEnabled" },
            "responsesStatefulEnabled": { "type": "boolean", "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.responsesStatefulEnabled" },
            "temperature": { "type": "number", "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.temperature" },
            "maxTokens": { "type": "integer", "minimum": 1, "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.maxTokens" },
            "contextWindowTokens": { "type": "integer", "minimum": 1, "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.contextWindowTokens" },
            "topP": { "type": "number", "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.topP" },
            "maxRetries": { "type": "integer", "minimum": 0, "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.maxRetries" },
            "allowInheritance": { "type": "boolean", "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.allowInheritance" }
        },
        "additionalProperties": false
    })
}

pub(super) fn variable_mutation_schema(create: bool) -> Value {
    let required = if create {
        json!(["key", "value"])
    } else {
        json!(["key"])
    };
    json!({
        "type": "object",
        "required": required,
        "properties": {
            "workspaceCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.common.params.workspaceCode" },
            "key": { "type": "string", "minLength": 1, "x-i18n-description-key": "agent.toolCatalog.tools.variable.common.params.key" },
            "value": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.variable.common.params.value" },
            "description": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.common.params.description" },
            "isSensitive": { "type": "boolean", "x-i18n-description-key": "agent.toolCatalog.tools.variable.common.params.isSensitive" },
            "valueMode": { "type": "string", "enum": ["replace", "preserve"], "x-i18n-description-key": "agent.toolCatalog.tools.variable.common.params.valueMode" }
        },
        "additionalProperties": false
    })
}
