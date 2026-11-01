fn platform_variable_page_definition() -> ToolCatalogDefinition {
    definition(
        "platform.variable.page",
        "harness_variable",
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
        "harness_variable",
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
        "harness_variable",
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
        "harness_variable",
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
