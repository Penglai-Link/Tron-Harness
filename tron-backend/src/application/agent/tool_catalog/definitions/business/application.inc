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
                "taskType": { "type": "string", "enum": ["general", "feature", "bug"], "x-i18n-description-key": "agent.toolCatalog.tools.task.common.params.taskType" },
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

fn dude_notify_definition() -> ToolCatalogDefinition {
    definition(
        "dude.notify",
        "dude",
        ToolOperationKind::Command,
        schema(
            "agent.toolCatalog.tools.dude.notify.description",
            json!([{ "title": "Need your decision", "body": "Choose the deployment window.", "sessionCode": "thread-id" }]),
            json!({
                "type": "object",
                "required": ["title", "body"],
                "properties": {
                    "title": {"type":"string","minLength":1,"maxLength":200},
                    "body": {"type":"string","minLength":1,"maxLength":8000},
                    "sessionCode": {"type":"string","minLength":1},
                    "taskCode": {"type":"string","minLength":1}
                },
                "additionalProperties": false
            }),
        ),
    )
}
