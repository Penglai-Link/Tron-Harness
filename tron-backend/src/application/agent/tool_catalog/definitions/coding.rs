use serde_json::json;

use crate::domain::agent::{ToolAvailabilityState, ToolOperationKind};

use super::{ToolCatalogDefinition, schema};

pub(super) fn tool_definitions() -> Vec<ToolCatalogDefinition> {
    vec![
        ToolCatalogDefinition {
            tool_code: "bash.exec",
            domain_code: "bash",
            name_i18n_key: "agent.toolCatalog.tools.bash.exec.name",
            description_i18n_key: "agent.toolCatalog.tools.bash.exec.description",
            operation_kind: ToolOperationKind::Command,
            input_schema: schema(
                "agent.toolCatalog.tools.bash.exec.description",
                json!([{ "command": "sed", "args": ["-n", "1,40p", "tron-backend/Cargo.toml"] }]),
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
            tool_code: "fs.read_text",
            domain_code: "fs",
            name_i18n_key: "agent.toolCatalog.tools.fs.read_text.name",
            description_i18n_key: "agent.toolCatalog.tools.fs.read_text.description",
            operation_kind: ToolOperationKind::Query,
            input_schema: schema(
                "agent.toolCatalog.tools.fs.read_text.description",
                json!([{ "path": "tron-backend/Cargo.toml", "startLine": 1, "endLine": 20 }]),
                json!({
                    "type": "object",
                    "required": ["path"],
                    "properties": {
                        "path": { "type": "string", "minLength": 1, "x-i18n-description-key": "agent.toolCatalog.tools.fs.read_text.params.path" },
                        "startLine": { "type": "integer", "minimum": 1, "x-i18n-description-key": "agent.toolCatalog.tools.fs.read_text.params.startLine" },
                        "endLine": { "type": "integer", "minimum": 1, "x-i18n-description-key": "agent.toolCatalog.tools.fs.read_text.params.endLine" },
                        "maxBytes": { "type": "integer", "minimum": 1, "x-i18n-description-key": "agent.toolCatalog.tools.fs.read_text.params.maxBytes" }
                    },
                    "additionalProperties": false
                }),
            ),
            module_owner: "coding",
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary_i18n_key: "agent.toolCatalog.tools.fs.read_text.sampleResultSummary",
        },
        ToolCatalogDefinition {
            tool_code: "fs.write_text",
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
            domain_code: "fs",
            name_i18n_key: "agent.toolCatalog.tools.fs.apply_patch.name",
            description_i18n_key: "agent.toolCatalog.tools.fs.apply_patch.description",
            operation_kind: ToolOperationKind::Command,
            input_schema: schema(
                "agent.toolCatalog.tools.fs.apply_patch.description",
                json!([{ "path": "notes/todo.txt", "action": "replace", "target": "todo", "content": "done", "replaceAll": false }]),
                json!({
                    "type": "object",
                    "required": ["path", "action", "target"],
                    "properties": {
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
    ]
}
