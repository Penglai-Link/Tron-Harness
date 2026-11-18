mod business;
mod coding;
mod knowledge_quality;
mod memory;
mod productivity;
mod rag;
mod rag_graph;
mod terminal;

use serde_json::Value;
use serde_json::json;

use crate::domain::agent::{ToolAvailabilityState, ToolOperationKind};

pub(super) fn file_reference_schema() -> Value {
    json!({
        "type": "object",
        "oneOf": [
            {
                "type": "object",
                "required": ["kind", "path"],
                "properties": {
                    "kind": {"type": "string", "enum": ["workspace_path"], "x-i18n-description-key": "agent.toolCatalog.productivity.params.fileRef.kind"},
                    "path": {"type": "string", "minLength": 1, "x-i18n-description-key": "agent.toolCatalog.productivity.params.fileRef.path"}
                },
                "additionalProperties": false
            },
            {
                "type": "object",
                "required": ["kind", "attachmentCode"],
                "properties": {
                    "kind": {"type": "string", "enum": ["session_attachment"], "x-i18n-description-key": "agent.toolCatalog.productivity.params.fileRef.kind"},
                    "attachmentCode": {"type": "string", "minLength": 1, "x-i18n-description-key": "agent.toolCatalog.productivity.params.fileRef.attachmentCode"}
                },
                "additionalProperties": false
            }
        ],
        "x-i18n-description-key": "agent.toolCatalog.productivity.params.fileRef"
    })
}

pub(super) fn file_source_selector_schema(path_alias: &str) -> Value {
    json!({
        "oneOf": [
            {"required": ["fileRef"], "not": {"required": [path_alias]}},
            {"required": [path_alias], "not": {"required": ["fileRef"]}}
        ]
    })
}

#[derive(Debug, Clone)]
pub(super) struct ToolDomainDefinition {
    pub domain_code: &'static str,
    pub name_i18n_key: &'static str,
    pub description_i18n_key: &'static str,
}

#[derive(Debug, Clone)]
pub(super) struct ToolCatalogDefinition {
    pub tool_code: &'static str,
    pub(super) tool_name: String,
    pub domain_code: &'static str,
    pub category_code: &'static str,
    pub name_i18n_key: &'static str,
    pub description_i18n_key: &'static str,
    pub operation_kind: ToolOperationKind,
    pub input_schema: Value,
    pub module_owner: &'static str,
    pub availability_state: ToolAvailabilityState,
    pub sample_result_summary_i18n_key: &'static str,
}

pub(super) fn domain_definitions() -> Vec<ToolDomainDefinition> {
    vec![
        ToolDomainDefinition {
            domain_code: "subagent",
            name_i18n_key: "agent.toolCatalog.domains.subagent.name",
            description_i18n_key: "agent.toolCatalog.domains.subagent.description",
        },
        ToolDomainDefinition {
            domain_code: "collaboration_thread",
            name_i18n_key: "agent.toolCatalog.domains.collaboration_thread.name",
            description_i18n_key: "agent.toolCatalog.domains.collaboration_thread.description",
        },
        ToolDomainDefinition {
            domain_code: "skill",
            name_i18n_key: "agent.toolCatalog.domains.skillLoading.name",
            description_i18n_key: "agent.toolCatalog.domains.skillLoading.description",
        },
        ToolDomainDefinition {
            domain_code: "tool",
            name_i18n_key: "agent.toolCatalog.domains.tool.name",
            description_i18n_key: "agent.toolCatalog.domains.tool.description",
        },
        ToolDomainDefinition {
            domain_code: "user_interaction",
            name_i18n_key: "agent.toolCatalog.domains.user_interaction.name",
            description_i18n_key: "agent.toolCatalog.domains.user_interaction.description",
        },
        ToolDomainDefinition {
            domain_code: "a2ui",
            name_i18n_key: "agent.toolCatalog.domains.a2ui.name",
            description_i18n_key: "agent.toolCatalog.domains.a2ui.description",
        },
        ToolDomainDefinition {
            domain_code: "harness_mcp",
            name_i18n_key: "agent.toolCatalog.domains.mcp.name",
            description_i18n_key: "agent.toolCatalog.domains.mcp.description",
        },
        ToolDomainDefinition {
            domain_code: "harness_prompt",
            name_i18n_key: "agent.toolCatalog.domains.prompt.name",
            description_i18n_key: "agent.toolCatalog.domains.prompt.description",
        },
        ToolDomainDefinition {
            domain_code: "media",
            name_i18n_key: "agent.toolCatalog.domains.media.name",
            description_i18n_key: "agent.toolCatalog.domains.media.description",
        },
        ToolDomainDefinition {
            domain_code: "assistant_identity",
            name_i18n_key: "agent.toolCatalog.domains.assistant.name",
            description_i18n_key: "agent.toolCatalog.domains.assistant.description",
        },
        ToolDomainDefinition {
            domain_code: "assistant_memory",
            name_i18n_key: "agent.toolCatalog.domains.assistant_memory.name",
            description_i18n_key: "agent.toolCatalog.domains.assistant_memory.description",
        },
        ToolDomainDefinition {
            domain_code: "assistant_task",
            name_i18n_key: "agent.toolCatalog.domains.assistant_task.name",
            description_i18n_key: "agent.toolCatalog.domains.assistant_task.description",
        },
        ToolDomainDefinition {
            domain_code: "assistant_work",
            name_i18n_key: "agent.toolCatalog.domains.assistant_work.name",
            description_i18n_key: "agent.toolCatalog.domains.assistant_work.description",
        },
        ToolDomainDefinition {
            domain_code: "assistant_notification",
            name_i18n_key: "agent.toolCatalog.domains.assistant_notification.name",
            description_i18n_key: "agent.toolCatalog.domains.assistant_notification.description",
        },
        ToolDomainDefinition {
            domain_code: "harness_rag",
            name_i18n_key: "agent.toolCatalog.domains.rag.name",
            description_i18n_key: "agent.toolCatalog.domains.rag.description",
        },
        ToolDomainDefinition {
            domain_code: "terminal",
            name_i18n_key: "agent.toolCatalog.terminal.name",
            description_i18n_key: "agent.toolCatalog.terminal.description",
        },
        ToolDomainDefinition {
            domain_code: "harness_worktree",
            name_i18n_key: "agent.toolCatalog.domains.worktree.name",
            description_i18n_key: "agent.toolCatalog.domains.worktree.description",
        },
        ToolDomainDefinition {
            domain_code: "word",
            name_i18n_key: "agent.toolCatalog.domains.word.name",
            description_i18n_key: "agent.toolCatalog.domains.word.description",
        },
        ToolDomainDefinition {
            domain_code: "excel",
            name_i18n_key: "agent.toolCatalog.domains.excel.name",
            description_i18n_key: "agent.toolCatalog.domains.excel.description",
        },
        ToolDomainDefinition {
            domain_code: "harness_automation",
            name_i18n_key: "agent.toolCatalog.domains.automation.name",
            description_i18n_key: "agent.toolCatalog.domains.automation.description",
        },
        ToolDomainDefinition {
            domain_code: "harness_app",
            name_i18n_key: "agent.toolCatalog.domains.app.name",
            description_i18n_key: "agent.toolCatalog.domains.app.description",
        },
        ToolDomainDefinition {
            domain_code: "command",
            name_i18n_key: "agent.toolCatalog.domains.command.name",
            description_i18n_key: "agent.toolCatalog.domains.command.description",
        },
        ToolDomainDefinition {
            domain_code: "chat_attachment",
            name_i18n_key: "agent.toolCatalog.domains.chat_attachment.name",
            description_i18n_key: "agent.toolCatalog.domains.chat_attachment.description",
        },
        ToolDomainDefinition {
            domain_code: "fs",
            name_i18n_key: "agent.toolCatalog.domains.fs.name",
            description_i18n_key: "agent.toolCatalog.domains.fs.description",
        },
        ToolDomainDefinition {
            domain_code: "harness_memory",
            name_i18n_key: "agent.toolCatalog.domains.memory.name",
            description_i18n_key: "agent.toolCatalog.domains.memory.description",
        },
        ToolDomainDefinition {
            domain_code: "harness_resource",
            name_i18n_key: "agent.toolCatalog.domains.resource.name",
            description_i18n_key: "agent.toolCatalog.domains.resource.description",
        },
        ToolDomainDefinition {
            domain_code: "search",
            name_i18n_key: "agent.toolCatalog.domains.search.name",
            description_i18n_key: "agent.toolCatalog.domains.search.description",
        },
        ToolDomainDefinition {
            domain_code: "web_search",
            name_i18n_key: "agent.toolCatalog.domains.web_search.name",
            description_i18n_key: "agent.toolCatalog.domains.web_search.description",
        },
        ToolDomainDefinition {
            domain_code: "harness_skill",
            name_i18n_key: "agent.toolCatalog.domains.skill.name",
            description_i18n_key: "agent.toolCatalog.domains.skill.description",
        },
        ToolDomainDefinition {
            domain_code: "harness_task",
            name_i18n_key: "agent.toolCatalog.domains.task.name",
            description_i18n_key: "agent.toolCatalog.domains.task.description",
        },
        ToolDomainDefinition {
            domain_code: "harness_variable",
            name_i18n_key: "agent.toolCatalog.domains.variable.name",
            description_i18n_key: "agent.toolCatalog.domains.variable.description",
        },
        ToolDomainDefinition {
            domain_code: "harness_workspace",
            name_i18n_key: "agent.toolCatalog.domains.workspace.name",
            description_i18n_key: "agent.toolCatalog.domains.workspace.description",
        },
        ToolDomainDefinition {
            domain_code: "artifacts",
            name_i18n_key: "agent.toolCatalog.domains.artifacts.name",
            description_i18n_key: "agent.toolCatalog.domains.artifacts.description",
        },
    ]
}

pub(super) fn tool_definitions() -> Vec<ToolCatalogDefinition> {
    let mut definitions = business::tool_definitions();
    definitions.extend(coding::tool_definitions());
    definitions.extend(terminal::tool_definitions());
    definitions.extend(memory::tool_definitions());
    definitions.extend(rag::tool_definitions());
    definitions.extend(rag_graph::tool_definitions());
    definitions.extend(knowledge_quality::tool_definitions());
    definitions.extend(productivity::tool_definitions());
    for tool in &mut definitions {
        if tool.domain_code == "harness_rag" || tool.tool_name == "workspace_page" {
            if let Some(properties) = tool
                .input_schema
                .get_mut("properties")
                .and_then(Value::as_object_mut)
            {
                properties
                    .entry("workspaceCode")
                    .or_insert_with(|| json!({"type":"string", "minLength":1}));
            }
        }
        if tool
            .input_schema
            .pointer("/properties/workspaceCode")
            .is_some()
        {
            let object = tool
                .input_schema
                .as_object_mut()
                .expect("tool schema object");
            let required = object.entry("required").or_insert_with(|| json!([]));
            if let Some(required) = required.as_array_mut() {
                if !required.iter().any(|value| value == "workspaceCode") {
                    required.push(json!("workspaceCode"));
                }
            }
            if let Some(examples) = object.get_mut("examples").and_then(Value::as_array_mut) {
                for example in examples {
                    if let Some(example) = example.as_object_mut() {
                        example
                            .entry("workspaceCode")
                            .or_insert_with(|| json!("TRON_CORE"));
                    }
                }
            }
        }
    }
    let mut assistant_tools = Vec::new();
    for definition in &definitions {
        let (code, name, domain) = match definition.tool_code {
            "memory.get" => (
                "assistant.memory_get",
                "assistant_memory_get",
                "assistant_memory",
            ),
            "memory.search" => (
                "assistant.memory_search",
                "assistant_memory_search",
                "assistant_memory",
            ),
            "memory.save" => (
                "assistant.memory_save",
                "assistant_memory_save",
                "assistant_memory",
            ),
            "memory.delete" => (
                "assistant.memory_delete",
                "assistant_memory_delete",
                "assistant_memory",
            ),
            "todo.update" => (
                "assistant.todo_update",
                "assistant_todo_update",
                "assistant_task",
            ),
            "task.get" => ("assistant.task_get", "assistant_task_get", "assistant_task"),
            "task.page" => (
                "assistant.task_page",
                "assistant_task_page",
                "assistant_task",
            ),
            "task.create" => (
                "assistant.task_create",
                "assistant_task_create",
                "assistant_task",
            ),
            "task.update" => (
                "assistant.task_update",
                "assistant_task_update",
                "assistant_task",
            ),
            "task.delete" => (
                "assistant.task_delete",
                "assistant_task_delete",
                "assistant_task",
            ),
            "task.restore" => (
                "assistant.task_restore",
                "assistant_task_restore",
                "assistant_task",
            ),
            _ => continue,
        };
        let mut tool = definition.clone();
        tool.tool_code = code;
        tool.tool_name = name.into();
        tool.domain_code = domain;
        tool.category_code = "tron.assistant";
        tool.module_owner = "assistant";
        if let Some(properties) = tool
            .input_schema
            .get_mut("properties")
            .and_then(Value::as_object_mut)
        {
            properties.remove("workspaceCode");
            if !code.starts_with("assistant.memory_") {
                properties.remove("scopeCode");
            }
            if let Some(scope) = properties.get_mut("scope") {
                scope["enum"] = json!(["User", "WorkDomain"]);
            }
        }
        if let Some(required) = tool
            .input_schema
            .get_mut("required")
            .and_then(Value::as_array_mut)
        {
            required.retain(|value| value != "workspaceCode" && value != "scopeCode");
        }
        if let Some(examples) = tool
            .input_schema
            .get_mut("examples")
            .and_then(Value::as_array_mut)
        {
            for example in examples {
                if let Some(object) = example.as_object_mut() {
                    object.remove("workspaceCode");
                    object.remove("scopeCode");
                }
            }
        }
        assistant_tools.push(tool);
    }
    definitions.extend(assistant_tools);
    definitions
}

pub(super) fn schema(description_key: &'static str, examples: Value, mut schema: Value) -> Value {
    if let Value::Object(map) = &mut schema {
        map.insert(
            "x-i18n-description-key".to_string(),
            Value::String(description_key.to_string()),
        );
        map.insert("examples".to_string(), examples);
    }
    schema
}
