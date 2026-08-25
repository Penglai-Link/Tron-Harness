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
                    "category": { "type": "string", "enum": ["SYSTEM", "PERSONA", "TOOL", "TEMPLATE", "ACTION"] },
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
        "category": { "type": "string", "enum": ["SYSTEM", "PERSONA", "TOOL", "TEMPLATE", "ACTION"] },
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
