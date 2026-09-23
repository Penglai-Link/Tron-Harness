use serde_json::json;

use crate::domain::agent::{ToolAvailabilityState, ToolOperationKind};

use super::{
    ToolCatalogDefinition, file_reference_schema, file_source_selector_schema, schema,
};

pub(super) fn tool_definitions() -> Vec<ToolCatalogDefinition> {
    vec![
        ToolCatalogDefinition {
            tool_code: "bash.exec",
            tool_name: "bash_exec".into(),
            domain_code: "bash",
            name_i18n_key: "agent.toolCatalog.tools.bash.exec.name",
            description_i18n_key: "agent.toolCatalog.tools.bash.exec.description",
            operation_kind: ToolOperationKind::Command,
            input_schema: schema(
                "agent.toolCatalog.tools.bash.exec.description",
                json!([{ "command": "sed -n '1,40p' tron-backend/Cargo.toml" }]),
                json!({
                    "type": "object",
                    "required": ["command"],
                    "properties": {
                        "command": { "type": "string", "minLength": 1, "x-i18n-description-key": "agent.toolCatalog.tools.bash.exec.params.command" },
                        "args": { "type": "array", "items": { "type": "string" }, "default": [], "x-i18n-description-key": "agent.toolCatalog.tools.bash.exec.params.args" },
                        "cwd": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.bash.exec.params.cwd" },
                        "stdin": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.bash.exec.params.stdin" },
                        "env": { "type": "object", "additionalProperties": { "type": "string" }, "x-i18n-description-key": "agent.toolCatalog.tools.bash.exec.params.env" }
                    },
                    "additionalProperties": false
                }),
            ),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.bash.exec.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "bash.task_read",
            tool_name: "bash_task_read".into(),
            domain_code: "bash",
            name_i18n_key: "agent.toolCatalog.tools.bash.task_read.name",
            description_i18n_key: "agent.toolCatalog.tools.bash.task_read.description",
            operation_kind: ToolOperationKind::Query,
            input_schema: schema(
                "agent.toolCatalog.tools.bash.task_read.description",
                json!([{ "taskId": "command-123-456-1", "stdoutCursor": 0, "stderrCursor": 0 }]),
                json!({
                    "type": "object",
                    "required": ["taskId"],
                    "properties": {
                        "taskId": { "type": "string", "minLength": 1, "x-i18n-description-key": "agent.toolCatalog.tools.bash.task_read.params.taskId" },
                        "stdoutCursor": { "type": "integer", "minimum": 0, "default": 0, "x-i18n-description-key": "agent.toolCatalog.tools.bash.task_read.params.stdoutCursor" },
                        "stderrCursor": { "type": "integer", "minimum": 0, "default": 0, "x-i18n-description-key": "agent.toolCatalog.tools.bash.task_read.params.stderrCursor" }
                    },
                    "additionalProperties": false
                }),
            ),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.bash.task_read.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "bash.task_stop",
            tool_name: "bash_task_stop".into(),
            domain_code: "bash",
            name_i18n_key: "agent.toolCatalog.tools.bash.task_stop.name",
            description_i18n_key: "agent.toolCatalog.tools.bash.task_stop.description",
            operation_kind: ToolOperationKind::Command,
            input_schema: schema(
                "agent.toolCatalog.tools.bash.task_stop.description",
                json!([{ "taskId": "command-123-456-1" }]),
                json!({
                    "type": "object",
                    "required": ["taskId"],
                    "properties": {
                        "taskId": { "type": "string", "minLength": 1, "x-i18n-description-key": "agent.toolCatalog.tools.bash.task_stop.params.taskId" }
                    },
                    "additionalProperties": false
                }),
            ),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.bash.task_stop.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "fs.read_text",
            tool_name: "fs_read_text".into(),
            domain_code: "fs",
            name_i18n_key: "agent.toolCatalog.tools.fs.read_text.name",
            description_i18n_key: "agent.toolCatalog.tools.fs.read_text.description",
            operation_kind: ToolOperationKind::Query,
            input_schema: schema(
                "agent.toolCatalog.tools.fs.read_text.description",
                json!([{ "fileRef": {"kind":"workspace_path", "path": "tron-backend/Cargo.toml"}, "startLine": 1, "endLine": 20 }]),
                json!({
                    "type": "object",
                    "properties": {
                        "fileRef": file_reference_schema(),
                        "path": { "type": "string", "minLength": 1, "x-i18n-description-key": "agent.toolCatalog.tools.fs.read_text.params.path" },
                        "startLine": { "type": "integer", "minimum": 1, "x-i18n-description-key": "agent.toolCatalog.tools.fs.read_text.params.startLine" },
                        "endLine": { "type": "integer", "minimum": 1, "x-i18n-description-key": "agent.toolCatalog.tools.fs.read_text.params.endLine" },
                        "maxBytes": { "type": "integer", "minimum": 1, "x-i18n-description-key": "agent.toolCatalog.tools.fs.read_text.params.maxBytes" }
                    },
                    "allOf": [file_source_selector_schema("path")],
                    "additionalProperties": false
                }),
            ),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.fs.read_text.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "chat_attachment.read_text",
            tool_name: "chat_attachment_read_text".into(),
            domain_code: "chat_attachment",
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
            name_i18n_key: "agent.toolCatalog.tools.fs.write_text.name",
            description_i18n_key: "agent.toolCatalog.tools.fs.write_text.description",
            operation_kind: ToolOperationKind::Command,
            input_schema: schema(
                "agent.toolCatalog.tools.fs.write_text.description",
                json!([{ "path": "notes/todo.txt", "content": "implement coding domain\n", "mode": "overwrite", "ensureParentDirs": true }]),
                json!({
                    "type": "object",
                    "required": ["path", "content"],
                    "properties": {
                        "path": { "type": "string", "minLength": 1, "x-i18n-description-key": "agent.toolCatalog.tools.fs.write_text.params.path" },
                        "content": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.fs.write_text.params.content" },
                        "mode": { "type": "string", "enum": ["create", "overwrite", "append"], "x-i18n-description-key": "agent.toolCatalog.tools.fs.write_text.params.mode" },
                        "ensureParentDirs": { "type": "boolean", "default": true, "x-i18n-description-key": "agent.toolCatalog.tools.fs.write_text.params.ensureParentDirs" }
                    },
                    "additionalProperties": false
                }),
            ),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.fs.write_text.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "fs.apply_patch",
            tool_name: "fs_apply_patch".into(),
            domain_code: "fs",
            name_i18n_key: "agent.toolCatalog.tools.fs.apply_patch.name",
            description_i18n_key: "agent.toolCatalog.tools.fs.apply_patch.description",
            operation_kind: ToolOperationKind::Command,
            input_schema: schema(
                "agent.toolCatalog.tools.fs.apply_patch.description",
                json!([{ "patch": "*** Begin Patch\n*** Update File: src/main.rs\n@@\n-fn old() {}\n+fn new() {}\n*** Add File: notes.txt\n+Updated entry point.\n*** End Patch", "dryRun": true }]),
                json!({
                    "type": "object",
                    "oneOf": [
                        {"required":["patch"], "not":{"anyOf":[{"required":["path"]},{"required":["action"]},{"required":["target"]},{"required":["content"]},{"required":["replaceAll"]},{"required":["expectedCount"]}]}},
                        {"required":["path","action","target"], "not":{"anyOf":[{"required":["patch"]},{"required":["cwd"]},{"required":["dryRun"]}]}}
                    ],
                    "properties": {
                        "patch": {"type":"string","minLength":1,"maxLength":4194304,"x-i18n-description-key":"agent.toolCatalog.tools.fs.apply_patch.params.patch"},
                        "cwd": {"type":"string","x-i18n-description-key":"agent.toolCatalog.tools.fs.apply_patch.params.cwd"},
                        "dryRun": {"type":"boolean","default":false,"x-i18n-description-key":"agent.toolCatalog.tools.fs.apply_patch.params.dryRun"},
                        "expectedCount": {"type":"integer","minimum":1,"x-i18n-description-key":"agent.toolCatalog.tools.fs.apply_patch.params.expectedCount"},
                        "path": { "type": "string", "minLength": 1, "x-i18n-description-key": "agent.toolCatalog.tools.fs.apply_patch.params.path" },
                        "action": { "type": "string", "enum": ["replace", "delete", "insert_before", "insert_after"], "x-i18n-description-key": "agent.toolCatalog.tools.fs.apply_patch.params.action" },
                        "target": { "type": "string", "minLength": 1, "x-i18n-description-key": "agent.toolCatalog.tools.fs.apply_patch.params.target" },
                        "content": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.fs.apply_patch.params.content" },
                        "replaceAll": { "type": "boolean", "default": false, "x-i18n-description-key": "agent.toolCatalog.tools.fs.apply_patch.params.replaceAll" }
                    },
                    "additionalProperties": false
                }),
            ),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.fs.apply_patch.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "fs.list",
            tool_name: "fs_list".into(),
            domain_code: "fs",
            name_i18n_key: "agent.toolCatalog.tools.fs.list.name",
            description_i18n_key: "agent.toolCatalog.tools.fs.list.description",
            operation_kind: ToolOperationKind::Query,
            input_schema: schema(
                "agent.toolCatalog.tools.fs.list.description",
                json!([{ "path": "tron-backend/src/application/agent", "recursive": false, "includeHidden": false }]),
                json!({
                    "type": "object",
                    "required": ["path"],
                    "properties": {
                        "path": { "type": "string", "minLength": 1, "x-i18n-description-key": "agent.toolCatalog.tools.fs.list.params.path" },
                        "recursive": { "type": "boolean", "default": false, "x-i18n-description-key": "agent.toolCatalog.tools.fs.list.params.recursive" },
                        "maxDepth": { "type": "integer", "minimum": 0, "default": 3, "x-i18n-description-key": "agent.toolCatalog.tools.fs.list.params.maxDepth" },
                        "includeHidden": { "type": "boolean", "default": false, "x-i18n-description-key": "agent.toolCatalog.tools.fs.list.params.includeHidden" },
                        "filesOnly": { "type": "boolean", "default": false, "x-i18n-description-key": "agent.toolCatalog.tools.fs.list.params.filesOnly" },
                        "directoriesOnly": { "type": "boolean", "default": false, "x-i18n-description-key": "agent.toolCatalog.tools.fs.list.params.directoriesOnly" }
                    },
                    "additionalProperties": false
                }),
            ),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.fs.list.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "fs.stat",
            tool_name: "fs_stat".into(),
            domain_code: "fs",
            name_i18n_key: "agent.toolCatalog.tools.fs.stat.name",
            description_i18n_key: "agent.toolCatalog.tools.fs.stat.description",
            operation_kind: ToolOperationKind::Query,
            input_schema: schema(
                "agent.toolCatalog.tools.fs.stat.description",
                json!([{ "path": "tron-backend/Cargo.toml", "followSymlink": false }]),
                json!({
                    "type": "object",
                    "required": ["path"],
                    "properties": {
                        "path": { "type": "string", "minLength": 1, "x-i18n-description-key": "agent.toolCatalog.tools.fs.stat.params.path" },
                        "followSymlink": { "type": "boolean", "default": false, "x-i18n-description-key": "agent.toolCatalog.tools.fs.stat.params.followSymlink" }
                    },
                    "additionalProperties": false
                }),
            ),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.fs.stat.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "search.grep",
            tool_name: "search_grep".into(),
            domain_code: "search",
            name_i18n_key: "agent.toolCatalog.tools.search.grep.name",
            description_i18n_key: "agent.toolCatalog.tools.search.grep.description",
            operation_kind: ToolOperationKind::Query,
            input_schema: schema(
                "agent.toolCatalog.tools.search.grep.description",
                json!([{ "pattern": "ToolRegistry", "root": "tron-backend/src", "glob": ["*.rs"], "caseSensitive": false }]),
                json!({
                    "type": "object",
                    "required": ["pattern"],
                    "properties": {
                        "pattern": { "type": "string", "minLength": 1, "x-i18n-description-key": "agent.toolCatalog.tools.search.grep.params.pattern" },
                        "root": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.search.grep.params.root" },
                        "glob": { "type": "array", "items": { "type": "string" }, "default": [], "x-i18n-description-key": "agent.toolCatalog.tools.search.grep.params.glob" },
                        "caseSensitive": { "type": "boolean", "default": false, "x-i18n-description-key": "agent.toolCatalog.tools.search.grep.params.caseSensitive" },
                        "word": { "type": "boolean", "default": false, "x-i18n-description-key": "agent.toolCatalog.tools.search.grep.params.word" },
                        "contextLines": { "type": "integer", "minimum": 0, "default": 0, "x-i18n-description-key": "agent.toolCatalog.tools.search.grep.params.contextLines" },
                        "maxResults": { "type": "integer", "minimum": 1, "default": 200, "x-i18n-description-key": "agent.toolCatalog.tools.search.grep.params.maxResults" }
                    },
                    "additionalProperties": false
                }),
            ),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.search.grep.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "search.glob",
            tool_name: "search_glob".into(),
            domain_code: "search",
            name_i18n_key: "agent.toolCatalog.tools.search.glob.name",
            description_i18n_key: "agent.toolCatalog.tools.search.glob.description",
            operation_kind: ToolOperationKind::Query,
            input_schema: schema(
                "agent.toolCatalog.tools.search.glob.description",
                json!([{ "patterns": ["*.rs", "*.toml"], "root": "tron-backend", "includeHidden": false }]),
                json!({
                    "type": "object",
                    "required": ["patterns"],
                    "properties": {
                        "patterns": { "type": "array", "minItems": 1, "items": { "type": "string" }, "x-i18n-description-key": "agent.toolCatalog.tools.search.glob.params.patterns" },
                        "root": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.search.glob.params.root" },
                        "includeHidden": { "type": "boolean", "default": false, "x-i18n-description-key": "agent.toolCatalog.tools.search.glob.params.includeHidden" },
                        "maxResults": { "type": "integer", "minimum": 1, "default": 500, "x-i18n-description-key": "agent.toolCatalog.tools.search.glob.params.maxResults" }
                    },
                    "additionalProperties": false
                }),
            ),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.search.glob.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "search.path",
            tool_name: "search_path".into(),
            domain_code: "search",
            name_i18n_key: "agent.toolCatalog.tools.search.path.name",
            description_i18n_key: "agent.toolCatalog.tools.search.path.description",
            operation_kind: ToolOperationKind::Query,
            input_schema: schema(
                "agent.toolCatalog.tools.search.path.description",
                json!([{ "patterns": ["*agent*"], "root": "tron-backend/src", "entryType": "directory", "includeHidden": false }]),
                json!({
                    "type": "object",
                    "required": ["patterns"],
                    "properties": {
                        "patterns": { "type": "array", "minItems": 1, "items": { "type": "string" }, "x-i18n-description-key": "agent.toolCatalog.tools.search.path.params.patterns" },
                        "root": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.search.path.params.root" },
                        "entryType": { "type": "string", "enum": ["any", "file", "directory"], "default": "any", "x-i18n-description-key": "agent.toolCatalog.tools.search.path.params.entryType" },
                        "includeHidden": { "type": "boolean", "default": false, "x-i18n-description-key": "agent.toolCatalog.tools.search.path.params.includeHidden" },
                        "maxDepth": { "type": "integer", "minimum": 0, "default": 8, "x-i18n-description-key": "agent.toolCatalog.tools.search.path.params.maxDepth" },
                        "maxResults": { "type": "integer", "minimum": 1, "default": 500, "x-i18n-description-key": "agent.toolCatalog.tools.search.path.params.maxResults" }
                    },
                    "additionalProperties": false
                }),
            ),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.search.path.sampleResultSummary",
        },
    ]
}
