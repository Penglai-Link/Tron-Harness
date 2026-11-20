use serde_json::json;

use crate::domain::agent::{ToolAvailabilityState, ToolOperationKind};

use super::{ToolCatalogDefinition, file_reference_schema, file_source_selector_schema, schema};

pub(super) fn tool_definitions() -> Vec<ToolCatalogDefinition> {
    vec![
        ToolCatalogDefinition {
            tool_code: "bash.exec",
            tool_name: "command_exec".into(),
            domain_code: "command",
            category_code: "system.command",
            name_i18n_key: "agent.toolCatalog.tools.bash.exec.name",
            description_i18n_key: "agent.toolCatalog.tools.bash.exec.description",
            operation_kind: ToolOperationKind::Command,
            input_schema: official_schema("command_exec"),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.bash.exec.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "bash.task_read",
            tool_name: "command_exec_read".into(),
            domain_code: "command",
            category_code: "system.command",
            name_i18n_key: "agent.toolCatalog.tools.bash.task_read.name",
            description_i18n_key: "agent.toolCatalog.tools.bash.task_read.description",
            operation_kind: ToolOperationKind::Query,
            input_schema: official_schema("command_exec_read"),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.bash.task_read.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "bash.task_stop",
            tool_name: "command_exec_stop".into(),
            domain_code: "command",
            category_code: "system.command",
            name_i18n_key: "agent.toolCatalog.tools.bash.task_stop.name",
            description_i18n_key: "agent.toolCatalog.tools.bash.task_stop.description",
            operation_kind: ToolOperationKind::Command,
            input_schema: official_schema("command_exec_stop"),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.bash.task_stop.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "fs.read_text",
            tool_name: "fs_read_text".into(),
            domain_code: "fs",
            category_code: "system.command",
            name_i18n_key: "agent.toolCatalog.tools.fs.read_text.name",
            description_i18n_key: "agent.toolCatalog.tools.fs.read_text.description",
            operation_kind: ToolOperationKind::Query,
            input_schema: official_schema("fs_read_text"),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.fs.read_text.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "chat_attachment.read_text",
            tool_name: "chat_attachment_read_text".into(),
            domain_code: "chat_attachment",
            category_code: "document.media",
            name_i18n_key: "agent.toolCatalog.tools.chat_attachment.read_text.name",
            description_i18n_key: "agent.toolCatalog.tools.chat_attachment.read_text.description",
            operation_kind: ToolOperationKind::Query,
            input_schema: schema(
                "agent.toolCatalog.tools.chat_attachment.read_text.description",
                json!([{ "attachmentCode": "ATTACHMENT-1", "startByte": 0, "maxBytes": 65536 }]),
                json!({
                    "type": "object",
                    "required": ["attachmentCode"],
                    "properties": {
                        "attachmentCode": { "type": "string", "minLength": 1, "x-i18n-description-key": "agent.toolCatalog.tools.chat_attachment.read_text.params.attachmentCode" },
                        "startByte": { "type": "integer", "minimum": 0, "x-i18n-description-key": "agent.toolCatalog.tools.chat_attachment.read_text.params.startByte" },
                        "maxBytes": { "type": "integer", "minimum": 1, "maximum": 262144, "default": 65536, "x-i18n-description-key": "agent.toolCatalog.tools.chat_attachment.read_text.params.maxBytes" }
                    },
                    "additionalProperties": false
                }),
            ),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.chat_attachment.read_text.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "chat_attachment.parse_json",
            tool_name: "chat_attachment_parse_json".into(),
            domain_code: "chat_attachment",
            category_code: "document.media",
            name_i18n_key: "agent.toolCatalog.tools.chat_attachment.parse_json.name",
            description_i18n_key: "agent.toolCatalog.tools.chat_attachment.parse_json.description",
            operation_kind: ToolOperationKind::Query,
            input_schema: schema(
                "agent.toolCatalog.tools.chat_attachment.parse_json.description",
                json!([{ "fileRef": {"kind":"session_attachment", "attachmentCode": "ATTACHMENT-1"}, "jsonPath": "/topics" }]),
                json!({
                    "type": "object",
                    "properties": {
                        "fileRef": file_reference_schema(),
                        "attachmentCode": { "type": "string", "minLength": 1, "x-i18n-description-key": "agent.toolCatalog.tools.chat_attachment.parse_json.params.attachmentCode" },
                        "jsonPath": { "type": "string", "default": "", "x-i18n-description-key": "agent.toolCatalog.tools.chat_attachment.parse_json.params.jsonPath" },
                        "maxValueBytes": { "type": "integer", "minimum": 1, "maximum": 262144, "default": 65536, "x-i18n-description-key": "agent.toolCatalog.tools.chat_attachment.parse_json.params.maxValueBytes" }
                    },
                    "allOf": [file_source_selector_schema("attachmentCode")],
                    "additionalProperties": false
                }),
            ),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.chat_attachment.parse_json.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "fs.write_text",
            tool_name: "fs_write_text".into(),
            domain_code: "fs",
            category_code: "system.command",
            name_i18n_key: "agent.toolCatalog.tools.fs.write_text.name",
            description_i18n_key: "agent.toolCatalog.tools.fs.write_text.description",
            operation_kind: ToolOperationKind::Command,
            input_schema: official_schema("fs_write_text"),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.fs.write_text.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "fs.apply_patch",
            tool_name: "fs_apply_patch".into(),
            domain_code: "fs",
            category_code: "system.command",
            name_i18n_key: "agent.toolCatalog.tools.fs.apply_patch.name",
            description_i18n_key: "agent.toolCatalog.tools.fs.apply_patch.description",
            operation_kind: ToolOperationKind::Command,
            input_schema: official_schema("fs_apply_patch"),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.fs.apply_patch.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "fs.list",
            tool_name: "fs_list".into(),
            domain_code: "fs",
            category_code: "system.command",
            name_i18n_key: "agent.toolCatalog.tools.fs.list.name",
            description_i18n_key: "agent.toolCatalog.tools.fs.list.description",
            operation_kind: ToolOperationKind::Query,
            input_schema: official_schema("fs_list"),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.fs.list.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "fs.stat",
            tool_name: "fs_stat".into(),
            domain_code: "fs",
            category_code: "system.command",
            name_i18n_key: "agent.toolCatalog.tools.fs.stat.name",
            description_i18n_key: "agent.toolCatalog.tools.fs.stat.description",
            operation_kind: ToolOperationKind::Query,
            input_schema: official_schema("fs_stat"),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.fs.stat.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "search.grep",
            tool_name: "search_grep".into(),
            domain_code: "search",
            category_code: "system.command",
            name_i18n_key: "agent.toolCatalog.tools.search.grep.name",
            description_i18n_key: "agent.toolCatalog.tools.search.grep.description",
            operation_kind: ToolOperationKind::Query,
            input_schema: official_schema("search_grep"),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.search.grep.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "search.glob",
            tool_name: "search_glob".into(),
            domain_code: "search",
            category_code: "system.command",
            name_i18n_key: "agent.toolCatalog.tools.search.glob.name",
            description_i18n_key: "agent.toolCatalog.tools.search.glob.description",
            operation_kind: ToolOperationKind::Query,
            input_schema: official_schema("search_glob"),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.search.glob.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "search.path",
            tool_name: "search_path".into(),
            domain_code: "search",
            category_code: "system.command",
            name_i18n_key: "agent.toolCatalog.tools.search.path.name",
            description_i18n_key: "agent.toolCatalog.tools.search.path.description",
            operation_kind: ToolOperationKind::Query,
            input_schema: official_schema("search_path"),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.search.path.sampleResultSummary",
        },
    ]
}

pub(super) fn official_schema(name: &str) -> serde_json::Value {
    let tool = eliteforge_agent_official_plugin_tools_local::local_tool_catalog()
        .into_iter()
        .find(|tool| tool.call_name == name)
        .expect("official local tool catalog entry");
    // Catalog records are model-facing; use the narrower projection when the
    // tool keeps legacy fields only for executor-side rejection.
    tool.model_input_schema.unwrap_or(tool.input_schema)
}
