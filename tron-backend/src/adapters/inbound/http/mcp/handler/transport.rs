use axum::{
    Json,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};

use crate::{
    adapters::inbound::http::auth::authenticate_headers,
    application::{
        mcp::McpService,
        user::UserService,
        variable::{CreateVariableCommand, VariableService},
    },
    shared::auth_holder::AuthHolder,
};

use super::super::error_mapper::mcp_error_response;

pub(super) const MCP_PROTOCOL_VERSION: &str = "2025-06-18";
const MCP_VARIABLE_CATALOG_URI_TEMPLATE: &str = "tron://variables/workspaces/{workspaceCode}";
const MCP_VARIABLE_DETAIL_URI_TEMPLATE: &str = "tron://variables/workspaces/{workspaceCode}/{key}";
const MCP_VARIABLE_UPSERT_TOOL: &str = "variable.upsert_workspace_local";
const MCP_VARIABLE_DELETE_TOOL: &str = "variable.delete_workspace_local";

pub async fn variables_transport(
    State(mcp): State<McpService>,
    State(users): State<UserService>,
    State(variables): State<VariableService>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> Response {
    let request_auth = match authenticate_headers(&users, &headers) {
        Ok(auth) => auth,
        Err(_) => {
            return mcp_bearer_error(
                payload.get("id").cloned(),
                "Bearer access token is invalid or expired",
            );
        }
    };

    match mcp.variables_transport_availability() {
        Ok(availability)
            if !availability.gateway_enabled || !availability.variables_endpoint_enabled =>
        {
            eprintln!("MCP variables transport request rejected because the endpoint is disabled");
            mcp_jsonrpc_error(
                StatusCode::SERVICE_UNAVAILABLE,
                payload.get("id").cloned(),
                -32002,
                "MCP variables endpoint is unavailable",
            )
        }
        Err(error) => mcp_error_response(error),
        Ok(_) => {
            let holder_context = request_auth.holder_context();
            AuthHolder::scope(holder_context, async move {
                handle_variables_transport_request(variables, payload)
            })
            .await
        }
    }
}

fn handle_variables_transport_request(variables: VariableService, payload: Value) -> Response {
    match payload
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default()
    {
        "ping" => mcp_jsonrpc_result(payload.get("id").cloned(), json!({})),
        "initialize" => handle_initialize(payload),
        "notifications/initialized" => StatusCode::ACCEPTED.into_response(),
        "resources/list" => mcp_jsonrpc_result(
            payload.get("id").cloned(),
            json!({
                "resources": []
            }),
        ),
        "resources/templates/list" => handle_resource_templates_list(payload),
        "resources/read" => handle_resource_read(variables, payload),
        "tools/list" => handle_tools_list(payload),
        "tools/call" => handle_tool_call(variables, payload),
        _ => mcp_jsonrpc_error(
            StatusCode::OK,
            payload.get("id").cloned(),
            -32601,
            "Method not supported by the variables transport",
        ),
    }
}

fn handle_initialize(payload: Value) -> Response {
    let Some(requested_version) = payload
        .pointer("/params/protocolVersion")
        .and_then(Value::as_str)
    else {
        return mcp_jsonrpc_error(
            StatusCode::OK,
            payload.get("id").cloned(),
            -32602,
            "protocolVersion is required",
        );
    };

    if requested_version != MCP_PROTOCOL_VERSION {
        return mcp_jsonrpc_error_with_data(
            StatusCode::OK,
            payload.get("id").cloned(),
            -32602,
            "Unsupported protocol version",
            json!({
                "supported": [MCP_PROTOCOL_VERSION],
                "requested": requested_version,
            }),
        );
    }

    mcp_jsonrpc_result(
        payload.get("id").cloned(),
        json!({
            "protocolVersion": MCP_PROTOCOL_VERSION,
            "serverInfo": {
                "name": "tron-mcp-variables",
                "version": env!("CARGO_PKG_VERSION")
            },
            "capabilities": {
                "resources": {},
                "tools": {}
            }
        }),
    )
}

fn handle_resource_templates_list(payload: Value) -> Response {
    mcp_jsonrpc_result(
        payload.get("id").cloned(),
        json!({
            "resourceTemplates": [
                {
                    "name": "workspace-variable-catalog",
                    "title": "Workspace Variable Catalog",
                    "uriTemplate": MCP_VARIABLE_CATALOG_URI_TEMPLATE,
                    "description": "Read the effective variable catalog for a workspace.",
                    "mimeType": "application/json"
                },
                {
                    "name": "workspace-variable-detail",
                    "title": "Workspace Variable Detail",
                    "uriTemplate": MCP_VARIABLE_DETAIL_URI_TEMPLATE,
                    "description": "Read one effective variable from a workspace.",
                    "mimeType": "application/json"
                }
            ]
        }),
    )
}

fn handle_resource_read(variables: VariableService, payload: Value) -> Response {
    let Some(uri) = payload.pointer("/params/uri").and_then(Value::as_str) else {
        return mcp_jsonrpc_error(
            StatusCode::OK,
            payload.get("id").cloned(),
            -32602,
            "uri is required",
        );
    };

    let target = match parse_variable_resource_uri(uri) {
        Ok(target) => target,
        Err(message) => {
            return mcp_jsonrpc_error(StatusCode::OK, payload.get("id").cloned(), -32602, message);
        }
    };

    let contents = match target {
        VariableResourceTarget::WorkspaceCatalog { workspace_code } => variables
            .list_variables(
                &workspace_code,
                crate::domain::variable::VariableCatalogQuery { search: None },
            )
            .and_then(|records| {
                serialize_resource_content(
                    uri,
                    serde_json::to_value(records)
                        .map_err(|_| crate::domain::variable::VariableError::StorageFailure),
                )
            }),
        VariableResourceTarget::WorkspaceVariable {
            workspace_code,
            key,
        } => variables
            .get_variable(&workspace_code, &key)
            .and_then(|record| {
                serialize_resource_content(
                    uri,
                    serde_json::to_value(record)
                        .map_err(|_| crate::domain::variable::VariableError::StorageFailure),
                )
            }),
    };

    match contents {
        Ok(content) => mcp_jsonrpc_result(
            payload.get("id").cloned(),
            json!({
                "contents": [content]
            }),
        ),
        Err(error) => mcp_jsonrpc_error(
            StatusCode::OK,
            payload.get("id").cloned(),
            variable_jsonrpc_error_code(&error),
            variable_jsonrpc_error_message(&error),
        ),
    }
}

fn handle_tools_list(payload: Value) -> Response {
    mcp_jsonrpc_result(
        payload.get("id").cloned(),
        json!({
            "tools": [
                {
                    "name": MCP_VARIABLE_UPSERT_TOOL,
                    "title": "Upsert Workspace Local Variable",
                    "description": "Create or update a workspace-local variable override.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "workspaceCode": { "type": "string" },
                            "key": { "type": "string" },
                            "value": { "type": "string" },
                            "description": { "type": ["string", "null"] }
                        },
                        "required": ["workspaceCode", "key", "value"]
                    },
                    "annotations": {
                        "title": "Upsert Workspace Local Variable",
                        "destructiveHint": false,
                        "idempotentHint": true
                    }
                },
                {
                    "name": MCP_VARIABLE_DELETE_TOOL,
                    "title": "Delete Workspace Local Variable",
                    "description": "Delete a workspace-local variable definition.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "workspaceCode": { "type": "string" },
                            "key": { "type": "string" }
                        },
                        "required": ["workspaceCode", "key"]
                    },
                    "annotations": {
                        "title": "Delete Workspace Local Variable",
                        "destructiveHint": true,
                        "idempotentHint": true
                    }
                }
            ]
        }),
    )
}

fn handle_tool_call(variables: VariableService, payload: Value) -> Response {
    let Some(tool_name) = payload.pointer("/params/name").and_then(Value::as_str) else {
        return mcp_jsonrpc_error(
            StatusCode::OK,
            payload.get("id").cloned(),
            -32602,
            "tool name is required",
        );
    };

    let arguments = payload
        .pointer("/params/arguments")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let result = match tool_name {
        MCP_VARIABLE_UPSERT_TOOL => {
            let workspace_code = match required_argument(&arguments, "workspaceCode") {
                Ok(value) => value,
                Err(message) => {
                    return mcp_jsonrpc_error(
                        StatusCode::OK,
                        payload.get("id").cloned(),
                        -32602,
                        message,
                    );
                }
            };
            let key = match required_argument(&arguments, "key") {
                Ok(value) => value,
                Err(message) => {
                    return mcp_jsonrpc_error(
                        StatusCode::OK,
                        payload.get("id").cloned(),
                        -32602,
                        message,
                    );
                }
            };
            let value = match required_argument(&arguments, "value") {
                Ok(value) => value,
                Err(message) => {
                    return mcp_jsonrpc_error(
                        StatusCode::OK,
                        payload.get("id").cloned(),
                        -32602,
                        message,
                    );
                }
            };
            let description = optional_string_argument(&arguments, "description");
            match variables.upsert_variable(
                &workspace_code,
                CreateVariableCommand {
                    key,
                    value,
                    description,
                },
            ) {
                Ok(record) => tool_success_result(
                    serde_json::to_value(record)
                        .expect("effective variable should serialize to JSON"),
                ),
                Err(error) => match error {
                    crate::domain::variable::VariableError::StorageFailure => {
                        return mcp_jsonrpc_error(
                            StatusCode::OK,
                            payload.get("id").cloned(),
                            -32603,
                            "variable storage operation failed",
                        );
                    }
                    _ => tool_business_error_result(
                        variable_error_code(&error),
                        variable_jsonrpc_error_message(&error),
                    ),
                },
            }
        }
        MCP_VARIABLE_DELETE_TOOL => {
            let workspace_code = match required_argument(&arguments, "workspaceCode") {
                Ok(value) => value,
                Err(message) => {
                    return mcp_jsonrpc_error(
                        StatusCode::OK,
                        payload.get("id").cloned(),
                        -32602,
                        message,
                    );
                }
            };
            let key = match required_argument(&arguments, "key") {
                Ok(value) => value,
                Err(message) => {
                    return mcp_jsonrpc_error(
                        StatusCode::OK,
                        payload.get("id").cloned(),
                        -32602,
                        message,
                    );
                }
            };
            match variables.delete_variable(&workspace_code, &key) {
                Ok(result) => tool_success_result(
                    serde_json::to_value(result)
                        .expect("variable mutation result should serialize to JSON"),
                ),
                Err(error) => match error {
                    crate::domain::variable::VariableError::StorageFailure => {
                        return mcp_jsonrpc_error(
                            StatusCode::OK,
                            payload.get("id").cloned(),
                            -32603,
                            "variable storage operation failed",
                        );
                    }
                    _ => tool_business_error_result(
                        variable_error_code(&error),
                        variable_jsonrpc_error_message(&error),
                    ),
                },
            }
        }
        _ => {
            return mcp_jsonrpc_error(
                StatusCode::OK,
                payload.get("id").cloned(),
                -32602,
                &format!("Unknown tool: {tool_name}"),
            );
        }
    };

    mcp_jsonrpc_result(payload.get("id").cloned(), result)
}

fn serialize_resource_content(
    uri: &str,
    value: Result<Value, crate::domain::variable::VariableError>,
) -> Result<Value, crate::domain::variable::VariableError> {
    let value = value?;
    Ok(json!({
        "uri": uri,
        "mimeType": "application/json",
        "text": serde_json::to_string(&value)
            .expect("resource payload should serialize to compact JSON")
    }))
}

fn required_argument(
    arguments: &serde_json::Map<String, Value>,
    key: &'static str,
) -> Result<String, &'static str> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or(match key {
            "workspaceCode" => "workspaceCode is required",
            "key" => "key is required",
            "value" => "value is required",
            _ => "required argument is missing",
        })
}

fn optional_string_argument(
    arguments: &serde_json::Map<String, Value>,
    key: &'static str,
) -> Option<String> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn tool_success_result(structured_content: Value) -> Value {
    json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string(&structured_content)
                    .expect("tool result should serialize to compact JSON")
            }
        ],
        "structuredContent": structured_content
    })
}

fn tool_business_error_result(code: &str, message: &str) -> Value {
    json!({
        "content": [
            {
                "type": "text",
                "text": message
            }
        ],
        "structuredContent": {
            "code": code,
            "message": message
        },
        "isError": true
    })
}

fn variable_error_code(error: &crate::domain::variable::VariableError) -> &'static str {
    match error {
        crate::domain::variable::VariableError::NotFound => "VARIABLE_NOT_FOUND",
        crate::domain::variable::VariableError::WorkspaceRequired => "VARIABLE_WORKSPACE_REQUIRED",
        crate::domain::variable::VariableError::InvalidWorkspace => "VARIABLE_WORKSPACE_INVALID",
        crate::domain::variable::VariableError::InvalidKey => "VARIABLE_KEY_INVALID",
        crate::domain::variable::VariableError::DuplicateKey => "VARIABLE_KEY_DUPLICATE",
        crate::domain::variable::VariableError::InheritedMutationDenied => {
            "VARIABLE_INHERITED_MUTATION_INVALID"
        }
        crate::domain::variable::VariableError::StorageFailure => "VARIABLE_STORAGE_FAILED",
    }
}

fn variable_jsonrpc_error_code(error: &crate::domain::variable::VariableError) -> i64 {
    match error {
        crate::domain::variable::VariableError::StorageFailure => -32603,
        _ => -32602,
    }
}

fn variable_jsonrpc_error_message(
    error: &crate::domain::variable::VariableError,
) -> &'static str {
    match error {
        crate::domain::variable::VariableError::NotFound => "variable was not found",
        crate::domain::variable::VariableError::WorkspaceRequired => "workspaceCode is required",
        crate::domain::variable::VariableError::InvalidWorkspace => {
            "variable workspace must reference an active workspace"
        }
        crate::domain::variable::VariableError::InvalidKey => {
            "variable key must use letters, digits, underscore, hyphen, or dot"
        }
        crate::domain::variable::VariableError::DuplicateKey => {
            "variable key already exists in the current workspace"
        }
        crate::domain::variable::VariableError::InheritedMutationDenied => {
            "override the inherited variable locally before modifying it"
        }
        crate::domain::variable::VariableError::StorageFailure => {
            "variable storage operation failed"
        }
    }
}

enum VariableResourceTarget {
    WorkspaceCatalog { workspace_code: String },
    WorkspaceVariable { workspace_code: String, key: String },
}

fn parse_variable_resource_uri(uri: &str) -> Result<VariableResourceTarget, &'static str> {
    let parsed = url::Url::parse(uri).map_err(|_| "uri must be a valid variable resource URI")?;
    if parsed.scheme() != "tron" || parsed.host_str() != Some("variables") {
        return Err("uri must use the tron://variables scheme");
    }

    let segments = parsed
        .path_segments()
        .map(|segments| segments.collect::<Vec<_>>())
        .unwrap_or_default();

    match segments.as_slice() {
        ["workspaces", workspace_code] if !workspace_code.trim().is_empty() => {
            Ok(VariableResourceTarget::WorkspaceCatalog {
                workspace_code: workspace_code.to_string(),
            })
        }
        ["workspaces", workspace_code, key]
            if !workspace_code.trim().is_empty() && !key.trim().is_empty() =>
        {
            Ok(VariableResourceTarget::WorkspaceVariable {
                workspace_code: workspace_code.to_string(),
                key: key.to_string(),
            })
        }
        _ => Err("uri must match a declared variable resource template"),
    }
}

fn mcp_bearer_error(id: Option<Value>, message: &str) -> Response {
    let mut response = mcp_jsonrpc_error(StatusCode::UNAUTHORIZED, id, -32001, message);
    response
        .headers_mut()
        .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
    response
}

fn mcp_jsonrpc_result(id: Option<Value>, result: Value) -> Response {
    Json(json!({
        "jsonrpc": "2.0",
        "id": id.unwrap_or(Value::Null),
        "result": result
    }))
    .into_response()
}

fn mcp_jsonrpc_error(status: StatusCode, id: Option<Value>, code: i64, message: &str) -> Response {
    mcp_jsonrpc_error_with_data(status, id, code, message, Value::Null)
}

fn mcp_jsonrpc_error_with_data(
    status: StatusCode,
    id: Option<Value>,
    code: i64,
    message: &str,
    data: Value,
) -> Response {
    (
        status,
        Json(json!({
            "jsonrpc": "2.0",
            "id": id.unwrap_or(Value::Null),
            "error": {
                "code": code,
                "message": message,
                "data": if data.is_null() { Value::Null } else { data }
            }
        })),
    )
        .into_response()
}
