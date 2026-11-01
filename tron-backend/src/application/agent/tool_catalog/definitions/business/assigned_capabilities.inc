fn skill_page_definition() -> ToolCatalogDefinition {
    definition(
        "skill.page",
        "harness_skill",
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
        "harness_skill",
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
        "harness_variable",
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
        "harness_variable",
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
        "harness_variable",
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
        "harness_variable",
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
