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
            "runtimeRole": { "type": "string", "enum": ["frontend", "backend", "mono"], "x-i18n-description-key": "agent.toolCatalog.tools.app.common.params.runtimeRole" },
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
            "priority": { "type": "string", "enum": ["low", "medium", "high", "extra_high"], "x-i18n-description-key": "agent.toolCatalog.tools.task.common.params.priority" },
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
            "supportedReasoningEfforts": { "type": "array", "items": { "type": "string", "enum": ["none", "minimal", "low", "medium", "high", "xhigh", "max", "ultra"] }, "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.supportedReasoningEfforts" },
            "defaultReasoningEffort": { "type": "string", "enum": ["none", "minimal", "low", "medium", "high", "xhigh", "max", "ultra"], "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.defaultReasoningEffort" },
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
