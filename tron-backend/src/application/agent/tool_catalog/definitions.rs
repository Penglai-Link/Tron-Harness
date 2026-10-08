mod business;
mod coding;
mod memory;
mod productivity;
mod rag;
mod terminal;

use serde_json::Value;
use serde_json::json;

use crate::domain::agent::{ToolAvailabilityState, ToolOperationKind};

pub(super) fn file_reference_schema() -> Value {
    json!({
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
            domain_code: "rag",
            name_i18n_key: "agent.toolCatalog.domains.rag.name",
            description_i18n_key: "agent.toolCatalog.domains.rag.description",
        },
        ToolDomainDefinition {
            domain_code: "terminal",
            name_i18n_key: "agent.toolCatalog.terminal.name",
            description_i18n_key: "agent.toolCatalog.terminal.description",
        },
        ToolDomainDefinition {
            domain_code: "worktree",
            name_i18n_key: "agent.toolCatalog.domains.worktree.name",
            description_i18n_key: "agent.toolCatalog.domains.worktree.description",
        },
        ToolDomainDefinition {
            domain_code: "git",
            name_i18n_key: "agent.toolCatalog.domains.git.name",
            description_i18n_key: "agent.toolCatalog.domains.git.description",
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
            domain_code: "automation",
            name_i18n_key: "agent.toolCatalog.domains.automation.name",
            description_i18n_key: "agent.toolCatalog.domains.automation.description",
        },
        ToolDomainDefinition {
            domain_code: "app",
            name_i18n_key: "agent.toolCatalog.domains.app.name",
            description_i18n_key: "agent.toolCatalog.domains.app.description",
        },
        ToolDomainDefinition {
            domain_code: "bash",
            name_i18n_key: "agent.toolCatalog.domains.bash.name",
            description_i18n_key: "agent.toolCatalog.domains.bash.description",
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
            domain_code: "memory",
            name_i18n_key: "agent.toolCatalog.domains.memory.name",
            description_i18n_key: "agent.toolCatalog.domains.memory.description",
        },
        ToolDomainDefinition {
            domain_code: "model",
            name_i18n_key: "agent.toolCatalog.domains.model.name",
            description_i18n_key: "agent.toolCatalog.domains.model.description",
        },
        ToolDomainDefinition {
            domain_code: "resource",
            name_i18n_key: "agent.toolCatalog.domains.resource.name",
            description_i18n_key: "agent.toolCatalog.domains.resource.description",
        },
        ToolDomainDefinition {
            domain_code: "search",
            name_i18n_key: "agent.toolCatalog.domains.search.name",
            description_i18n_key: "agent.toolCatalog.domains.search.description",
        },
        ToolDomainDefinition {
            domain_code: "skill",
            name_i18n_key: "agent.toolCatalog.domains.skill.name",
            description_i18n_key: "agent.toolCatalog.domains.skill.description",
        },
        ToolDomainDefinition {
            domain_code: "task",
            name_i18n_key: "agent.toolCatalog.domains.task.name",
            description_i18n_key: "agent.toolCatalog.domains.task.description",
        },
        ToolDomainDefinition {
            domain_code: "variable",
            name_i18n_key: "agent.toolCatalog.domains.variable.name",
            description_i18n_key: "agent.toolCatalog.domains.variable.description",
        },
        ToolDomainDefinition {
            domain_code: "workspace",
            name_i18n_key: "agent.toolCatalog.domains.workspace.name",
            description_i18n_key: "agent.toolCatalog.domains.workspace.description",
        },
    ]
}

pub(super) fn tool_definitions() -> Vec<ToolCatalogDefinition> {
    let mut definitions = business::tool_definitions();
    definitions.extend(coding::tool_definitions());
    definitions.extend(terminal::tool_definitions());
    definitions.extend(memory::tool_definitions());
    definitions.extend(rag::tool_definitions());
    definitions.extend(productivity::tool_definitions());
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
