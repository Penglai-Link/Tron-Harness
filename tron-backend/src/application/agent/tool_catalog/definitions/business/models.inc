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
            json!([{ "current": 1, "size": 20, "search": "eliteforge" }]),
            json!({"type":"object","properties":{
                "current":{"type":"integer","minimum":1},"size":{"type":"integer","minimum":1,"maximum":100},
                "search":{"type":"string","x-i18n-description-key":"agent.toolCatalog.tools.skill.page.params.search"}
            },"additionalProperties":false}),
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
            json!([{ "skillName": "eliteforge-brainstorming" }]),
            json!({
                "type": "object",
                "required": ["skillName"],
                "properties": {
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
