fn platform_resource_query_definition() -> ToolCatalogDefinition {
    definition(
        "platform.resource.query",
        "harness_resource",
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
        "harness_resource",
        "resourceCode",
        "release-notes",
        ToolOperationKind::Query,
    )
}

fn platform_resource_create_definition() -> ToolCatalogDefinition {
    definition(
        "platform.resource.create",
        "harness_resource",
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
        "harness_resource",
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
        "harness_resource",
        "resourceCode",
        "release-notes",
        ToolOperationKind::Command,
    )
}

fn platform_resource_binary_definition() -> ToolCatalogDefinition {
    definition(
        "platform.resource.binary",
        "harness_resource",
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
        "harness_resource",
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
        "harness_resource",
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
        "harness_resource",
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
        "harness_resource",
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
        "harness_resource",
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
        "harness_resource",
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
        "harness_resource",
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
        "harness_resource",
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
        "harness_resource",
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
        "harness_resource",
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
