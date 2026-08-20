use std::collections::BTreeMap;

use serde::Serialize;

use crate::domain::open_app::{OPEN_API_CAPABILITY_AGENT_CHAT, OpenApiRouteGrant, OpenAppError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiCapabilityCatalogGroup {
    pub capability_code: String,
    pub capability_name: String,
    pub description: String,
    pub operations: Vec<OpenApiCapabilityCatalogOperation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiCapabilityCatalogOperation {
    pub operation_code: String,
    pub operation_name: String,
    pub description: String,
    pub routes: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
struct OpenApiRouteDefinition {
    method: &'static str,
    path_template: &'static str,
    capability_code: &'static str,
    capability_name: &'static str,
    capability_description: &'static str,
    operation_code: &'static str,
    operation_name: &'static str,
    operation_description: &'static str,
}

const OPEN_API_ROUTE_DEFINITIONS: &[OpenApiRouteDefinition] = &[
    OpenApiRouteDefinition {
        method: "GET",
        path_template: "/open-api/v1/chats/session-groups",
        capability_code: OPEN_API_CAPABILITY_AGENT_CHAT,
        capability_name: "Agent Chat OpenAPI",
        capability_description: "External session and message access for agent chat.",
        operation_code: "session.read",
        operation_name: "Read Sessions",
        operation_description: "Read session groups and AG-UI event feeds.",
    },
    OpenApiRouteDefinition {
        method: "POST",
        path_template: "/open-api/v1/chats/sessions",
        capability_code: OPEN_API_CAPABILITY_AGENT_CHAT,
        capability_name: "Agent Chat OpenAPI",
        capability_description: "External session and message access for agent chat.",
        operation_code: "session.write",
        operation_name: "Write Sessions",
        operation_description: "Create or update agent chat sessions.",
    },
    OpenApiRouteDefinition {
        method: "GET",
        path_template: "/open-api/v1/chats/sessions/get-active-run",
        capability_code: OPEN_API_CAPABILITY_AGENT_CHAT,
        capability_name: "Agent Chat OpenAPI",
        capability_description: "External session and message access for agent chat.",
        operation_code: "session.read",
        operation_name: "Read Sessions",
        operation_description: "Read the active run attached to an agent chat session.",
    },
    OpenApiRouteDefinition {
        method: "PATCH",
        path_template: "/open-api/v1/chats/sessions/{sessionCode}",
        capability_code: OPEN_API_CAPABILITY_AGENT_CHAT,
        capability_name: "Agent Chat OpenAPI",
        capability_description: "External session and message access for agent chat.",
        operation_code: "session.write",
        operation_name: "Write Sessions",
        operation_description: "Create or update agent chat sessions.",
    },
    OpenApiRouteDefinition {
        method: "GET",
        path_template: "/open-api/v1/chats/sessions/{sessionCode}/messages",
        capability_code: OPEN_API_CAPABILITY_AGENT_CHAT,
        capability_name: "Agent Chat OpenAPI",
        capability_description: "External session and message access for agent chat.",
        operation_code: "message.read",
        operation_name: "Read Messages",
        operation_description: "Read session message history.",
    },
    OpenApiRouteDefinition {
        method: "POST",
        path_template: "/open-api/v1/chats/sessions/steer-message",
        capability_code: OPEN_API_CAPABILITY_AGENT_CHAT,
        capability_name: "Agent Chat OpenAPI",
        capability_description: "External session and message access for agent chat.",
        operation_code: "message.write",
        operation_name: "Write Messages",
        operation_description: "Steer an active agent chat run with a user message.",
    },
    OpenApiRouteDefinition {
        method: "POST",
        path_template: "/open-api/v1/chats/sessions/stop-run",
        capability_code: OPEN_API_CAPABILITY_AGENT_CHAT,
        capability_name: "Agent Chat OpenAPI",
        capability_description: "External session and message access for agent chat.",
        operation_code: "message.write",
        operation_name: "Write Messages",
        operation_description: "Stop the active run attached to an agent chat session.",
    },
    OpenApiRouteDefinition {
        method: "POST",
        path_template: "/open-api/v1/chats/sessions/{sessionCode}/messages",
        capability_code: OPEN_API_CAPABILITY_AGENT_CHAT,
        capability_name: "Agent Chat OpenAPI",
        capability_description: "External session and message access for agent chat.",
        operation_code: "message.write",
        operation_name: "Write Messages",
        operation_description: "Create session messages and receive model replies.",
    },
    OpenApiRouteDefinition {
        method: "POST",
        path_template: "/open-api/v1/chats/sessions/{sessionCode}/messages/stream",
        capability_code: OPEN_API_CAPABILITY_AGENT_CHAT,
        capability_name: "Agent Chat OpenAPI",
        capability_description: "External session and message access for agent chat.",
        operation_code: "stream.consume",
        operation_name: "Consume Streams",
        operation_description: "Consume streaming chat responses.",
    },
    OpenApiRouteDefinition {
        method: "POST",
        path_template: "/open-api/v1/chats/sessions/{sessionCode}/attachments",
        capability_code: OPEN_API_CAPABILITY_AGENT_CHAT,
        capability_name: "Agent Chat OpenAPI",
        capability_description: "External session and message access for agent chat.",
        operation_code: "attachment.write",
        operation_name: "Write Attachments",
        operation_description: "Upload a private attachment to an agent chat session.",
    },
    OpenApiRouteDefinition {
        method: "GET",
        path_template: "/open-api/v1/chats/sessions/{sessionCode}/attachments/{attachmentCode}",
        capability_code: OPEN_API_CAPABILITY_AGENT_CHAT,
        capability_name: "Agent Chat OpenAPI",
        capability_description: "External session and message access for agent chat.",
        operation_code: "attachment.read",
        operation_name: "Read Attachments",
        operation_description: "Download a private attachment from an agent chat session.",
    },
    OpenApiRouteDefinition {
        method: "GET",
        path_template: "/open-api/v1/agents/agui/streams/{streamKey}/events",
        capability_code: OPEN_API_CAPABILITY_AGENT_CHAT,
        capability_name: "Agent Chat OpenAPI",
        capability_description: "External session and message access for agent chat.",
        operation_code: "session.read",
        operation_name: "Read Sessions",
        operation_description: "Read session groups and AG-UI event feeds.",
    },
    OpenApiRouteDefinition {
        method: "GET",
        path_template: "/open-api/v1/agents/agui/streams/{streamKey}/protocol-events",
        capability_code: OPEN_API_CAPABILITY_AGENT_CHAT,
        capability_name: "Agent Chat OpenAPI",
        capability_description: "External session and message access for agent chat.",
        operation_code: "session.read",
        operation_name: "Read Sessions",
        operation_description: "Read session groups and AG-UI event feeds.",
    },
];

pub fn resolve_open_api_route_grant(method: &str, path: &str) -> Option<OpenApiRouteGrant> {
    OPEN_API_ROUTE_DEFINITIONS
        .iter()
        .find(|definition| route_matches(definition, method, path))
        .map(|definition| OpenApiRouteGrant {
            capability_code: definition.capability_code,
            operation_code: definition.operation_code,
        })
}

pub fn list_open_api_capability_catalog() -> Vec<OpenApiCapabilityCatalogGroup> {
    let mut grouped = BTreeMap::<
        &'static str,
        (
            &'static str,
            &'static str,
            BTreeMap<&'static str, (&'static str, &'static str, Vec<String>)>,
        ),
    >::new();

    for definition in OPEN_API_ROUTE_DEFINITIONS {
        let capability = grouped
            .entry(definition.capability_code)
            .or_insert_with(|| {
                (
                    definition.capability_name,
                    definition.capability_description,
                    BTreeMap::new(),
                )
            });
        let operation = capability
            .2
            .entry(definition.operation_code)
            .or_insert_with(|| {
                (
                    definition.operation_name,
                    definition.operation_description,
                    Vec::new(),
                )
            });
        operation.2.push(format!(
            "{} {}",
            definition.method, definition.path_template
        ));
    }

    grouped
        .into_iter()
        .map(
            |(capability_code, (capability_name, description, operations))| {
                OpenApiCapabilityCatalogGroup {
                    capability_code: capability_code.to_owned(),
                    capability_name: capability_name.to_owned(),
                    description: description.to_owned(),
                    operations: operations
                        .into_iter()
                        .map(
                            |(operation_code, (operation_name, operation_description, routes))| {
                                OpenApiCapabilityCatalogOperation {
                                    operation_code: operation_code.to_owned(),
                                    operation_name: operation_name.to_owned(),
                                    description: operation_description.to_owned(),
                                    routes,
                                }
                            },
                        )
                        .collect(),
                }
            },
        )
        .collect()
}

pub fn validate_open_api_grant(
    capability_code: &str,
    operation_code: &str,
) -> Result<(), OpenAppError> {
    let capability_code = capability_code.trim();
    let operation_code = operation_code.trim();
    if capability_code.is_empty() {
        return Err(OpenAppError::InvalidCapability);
    }
    if operation_code.is_empty() {
        return Err(OpenAppError::InvalidOperation);
    }

    let capability_exists = OPEN_API_ROUTE_DEFINITIONS
        .iter()
        .any(|definition| definition.capability_code == capability_code);
    if !capability_exists {
        return Err(OpenAppError::InvalidCapability);
    }

    let operation_exists = OPEN_API_ROUTE_DEFINITIONS.iter().any(|definition| {
        definition.capability_code == capability_code && definition.operation_code == operation_code
    });
    if !operation_exists {
        return Err(OpenAppError::InvalidOperation);
    }

    Ok(())
}

fn route_matches(definition: &OpenApiRouteDefinition, method: &str, path: &str) -> bool {
    if definition.method != method {
        return false;
    }

    let actual_segments = normalized_segments(path);
    let template_segments = normalized_segments(definition.path_template);
    if actual_segments.len() != template_segments.len() {
        return false;
    }

    template_segments.iter().zip(actual_segments.iter()).all(
        |(template_segment, actual_segment)| {
            is_template_parameter(template_segment) || template_segment == actual_segment
        },
    )
}

fn normalized_segments(path: &str) -> Vec<&str> {
    path.trim_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect()
}

fn is_template_parameter(segment: &str) -> bool {
    segment.starts_with('{') && segment.ends_with('}')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_groups_operations_and_routes() {
        let catalog = list_open_api_capability_catalog();
        assert_eq!(catalog.len(), 1);
        let agent_chat = &catalog[0];
        assert_eq!(agent_chat.capability_code, OPEN_API_CAPABILITY_AGENT_CHAT);
        let session_read = agent_chat
            .operations
            .iter()
            .find(|operation| operation.operation_code == "session.read")
            .expect("session.read should be catalogued");
        assert!(
            session_read
                .routes
                .iter()
                .any(|route| route == "GET /open-api/v1/chats/session-groups")
        );
        assert!(
            session_read
                .routes
                .iter()
                .any(|route| route == "GET /open-api/v1/agents/agui/streams/{streamKey}/events")
        );
        assert!(
            session_read
                .routes
                .iter()
                .any(|route| { route == "GET /open-api/v1/chats/sessions/get-active-run" })
        );
        let attachment_read = agent_chat
            .operations
            .iter()
            .find(|operation| operation.operation_code == "attachment.read")
            .expect("attachment.read should be catalogued");
        assert!(attachment_read.routes.iter().any(|route| {
            route == "GET /open-api/v1/chats/sessions/{sessionCode}/attachments/{attachmentCode}"
        }));
        let attachment_write = agent_chat
            .operations
            .iter()
            .find(|operation| operation.operation_code == "attachment.write")
            .expect("attachment.write should be catalogued");
        assert!(attachment_write.routes.iter().any(|route| {
            route == "POST /open-api/v1/chats/sessions/{sessionCode}/attachments"
        }));
        let message_write = agent_chat
            .operations
            .iter()
            .find(|operation| operation.operation_code == "message.write")
            .expect("message.write should be catalogued");
        assert!(
            message_write
                .routes
                .iter()
                .any(|route| { route == "POST /open-api/v1/chats/sessions/stop-run" })
        );
    }

    #[test]
    fn route_grant_resolution_matches_dynamic_path_templates() {
        let route_grant = resolve_open_api_route_grant(
            "POST",
            "/open-api/v1/chats/sessions/SESSION-001/messages/stream",
        )
        .expect("stream route should resolve");
        assert_eq!(route_grant.capability_code, OPEN_API_CAPABILITY_AGENT_CHAT);
        assert_eq!(route_grant.operation_code, "stream.consume");

        let agui_events =
            resolve_open_api_route_grant("GET", "/open-api/v1/agents/agui/streams/STR-100/events")
                .expect("agui events route should resolve");
        assert_eq!(agui_events.operation_code, "session.read");
    }

    #[test]
    fn capability_validation_rejects_unknown_pairs() {
        assert!(validate_open_api_grant("agent-chat", "message.write").is_ok());
        assert_eq!(
            validate_open_api_grant("unknown", "message.write"),
            Err(OpenAppError::InvalidCapability)
        );
        assert_eq!(
            validate_open_api_grant("agent-chat", "unknown"),
            Err(OpenAppError::InvalidOperation)
        );
    }
}
