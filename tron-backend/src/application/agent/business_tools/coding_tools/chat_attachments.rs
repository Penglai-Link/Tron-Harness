use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    application::resource::ResourceService,
    domain::agent::{AgentRunContext, Tool, ToolError, ToolExecutionResult, ToolRequest},
};

use super::{json_observation, parse_tool_input};

const DEFAULT_READ_BYTES: usize = 64 * 1024;
const MAX_READ_BYTES: usize = 256 * 1024;
const DEFAULT_JSON_VALUE_BYTES: usize = 64 * 1024;
const MAX_JSON_VALUE_BYTES: usize = 256 * 1024;
const MAX_JSON_OBJECT_KEYS: usize = 100;

pub(super) fn tools(resources: ResourceService) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(ChatAttachmentReadTextTool {
            resources: resources.clone(),
        }),
        Arc::new(ChatAttachmentParseJsonTool { resources }),
    ]
}

#[derive(Clone)]
struct ChatAttachmentReadTextTool {
    resources: ResourceService,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ChatAttachmentReadTextInput {
    attachment_code: String,
    start_byte: Option<usize>,
    max_bytes: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChatAttachmentDescriptor {
    attachment_code: String,
    file_name: String,
    content_type: String,
    byte_size: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChatAttachmentTextOutput {
    attachment: ChatAttachmentDescriptor,
    start_byte: usize,
    end_byte: usize,
    next_start_byte: Option<usize>,
    end_of_file: bool,
    truncated: bool,
    content: String,
}

impl Tool for ChatAttachmentReadTextTool {
    fn name(&self) -> &str {
        "chat_attachment_read_text"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: ChatAttachmentReadTextInput = parse_tool_input(request.input)?;
        let max_bytes = bounded_limit(input.max_bytes, DEFAULT_READ_BYTES, MAX_READ_BYTES)?;
        let requested_start = input.start_byte.unwrap_or(0) as u64;
        let range_start = requested_start.saturating_sub(3);
        conversation_code(context)?;
        let (record, range_start, bytes) =
            super::super::file_access::read_session_attachment_range(
                &self.resources,
                context,
                input.attachment_code.trim(),
                range_start,
                max_bytes.saturating_add(3) as u64,
            )
            .map_err(|error| {
                ToolError::ExecutionFailed(format!(
                    "read chat attachment `{}` failed: {error}",
                    input.attachment_code
                ))
            })?;
        if requested_start > record.byte_size {
            return Err(ToolError::ExecutionFailed(format!(
                "startByte {} exceeds attachment size {}",
                requested_start, record.byte_size
            )));
        }
        let text = std::str::from_utf8(&bytes).map_err(|_| {
            ToolError::ExecutionFailed(format!(
                "attachment `{}` is not valid UTF-8 text",
                input.attachment_code
            ))
        })?;
        let local_start = requested_start.saturating_sub(range_start) as usize;
        let start = previous_char_boundary(text, local_start);
        let requested_end = start.saturating_add(max_bytes).min(bytes.len());
        let end = next_char_boundary(text, requested_end);
        let content = text
            .get(start..end)
            .ok_or_else(|| ToolError::ExecutionFailed("text chunk boundary is invalid".into()))?;
        let global_start = range_start as usize + start;
        let global_end = range_start as usize + end;
        let end_of_file = global_end >= record.byte_size as usize;

        json_observation(
            self.name(),
            &ChatAttachmentTextOutput {
                attachment: descriptor(&record),
                start_byte: global_start,
                end_byte: global_end,
                next_start_byte: (!end_of_file).then_some(global_end),
                end_of_file,
                truncated: !end_of_file,
                content: content.to_string(),
            },
        )
    }
}

#[derive(Clone)]
struct ChatAttachmentParseJsonTool {
    resources: ResourceService,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ChatAttachmentParseJsonInput {
    attachment_code: Option<String>,
    file_ref: Option<Value>,
    json_path: Option<String>,
    max_value_bytes: Option<usize>,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonDocumentStats {
    nodes: usize,
    objects: usize,
    arrays: usize,
    strings: usize,
    numbers: usize,
    booleans: usize,
    nulls: usize,
    object_keys: usize,
    max_depth: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonSelection {
    path: String,
    value_type: &'static str,
    array_length: Option<usize>,
    string_length: Option<usize>,
    object_keys: Vec<String>,
    object_keys_truncated: bool,
    serialized_bytes: usize,
    value: Option<Value>,
    value_truncated: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChatAttachmentJsonOutput {
    attachment: Option<ChatAttachmentDescriptor>,
    file_name: String,
    file_ref: Value,
    valid_json: bool,
    root_type: &'static str,
    document: JsonDocumentStats,
    selection: JsonSelection,
}

impl Tool for ChatAttachmentParseJsonTool {
    fn name(&self) -> &str {
        "chat_attachment_parse_json"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: ChatAttachmentParseJsonInput = parse_tool_input(request.input)?;
        let max_value_bytes = bounded_limit(
            input.max_value_bytes,
            DEFAULT_JSON_VALUE_BYTES,
            MAX_JSON_VALUE_BYTES,
        )?;
        let source_input = match (input.file_ref, input.attachment_code) {
            (Some(file_ref), None) => serde_json::json!({"fileRef": file_ref}),
            (None, Some(attachment_code)) => serde_json::json!({
                "fileRef": {
                    "kind": "session_attachment",
                    "attachmentCode": attachment_code
                }
            }),
            (Some(_), Some(_)) => {
                return Err(ToolError::ExecutionFailed(
                    "provide fileRef or legacy attachmentCode, not both".into(),
                ));
            }
            (None, None) => {
                return Err(ToolError::ExecutionFailed(
                    "fileRef is required (legacy attachmentCode is also accepted)".into(),
                ));
            }
        };
        let source = super::super::file_access::resolve_input_file(
            &source_input,
            context,
            &self
                .resources
                .for_agent_in_workspace(context.agent_code.as_deref(), &context.workspace_code),
        )?;
        let record = source.attachment_record.as_ref();
        let document = serde_json::from_slice::<Value>(&source.bytes).map_err(|error| {
            ToolError::ExecutionFailed(format!(
                "file `{}` is not valid JSON: {error}",
                source.file_name
            ))
        })?;
        let mut stats = JsonDocumentStats::default();
        collect_json_stats(&document, 0, &mut stats);
        let path = input.json_path.unwrap_or_default();
        if !path.is_empty() && !path.starts_with('/') {
            return Err(ToolError::ExecutionFailed(
                "jsonPath must be an RFC 6901 JSON Pointer beginning with `/`".into(),
            ));
        }
        let selected = document.pointer(&path).ok_or_else(|| {
            ToolError::ExecutionFailed(format!("jsonPath `{path}` does not exist"))
        })?;
        let selection = select_json_value(&path, selected, max_value_bytes)?;
        json_observation(
            self.name(),
            &ChatAttachmentJsonOutput {
                attachment: record.map(descriptor),
                file_name: source.file_name,
                file_ref: source.reference,
                valid_json: true,
                root_type: json_type(&document),
                document: stats,
                selection,
            },
        )
    }
}

fn conversation_code(context: &AgentRunContext) -> Result<&str, ToolError> {
    context
        .conversation_code
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            ToolError::ExecutionFailed(
                "chat attachment tools require a conversation session".into(),
            )
        })
}

fn descriptor(
    record: &crate::domain::resource::ChatSessionAttachmentRecord,
) -> ChatAttachmentDescriptor {
    ChatAttachmentDescriptor {
        attachment_code: record.attachment_code.clone(),
        file_name: record.file_name.clone(),
        content_type: record.content_type.clone(),
        byte_size: record.byte_size,
    }
}

fn bounded_limit(
    requested: Option<usize>,
    default: usize,
    maximum: usize,
) -> Result<usize, ToolError> {
    let limit = requested.unwrap_or(default);
    if limit == 0 {
        return Err(ToolError::ExecutionFailed(
            "limit must be greater than zero".into(),
        ));
    }
    Ok(limit.min(maximum))
}

fn previous_char_boundary(text: &str, mut index: usize) -> usize {
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn next_char_boundary(text: &str, mut index: usize) -> usize {
    while index < text.len() && !text.is_char_boundary(index) {
        index += 1;
    }
    index
}

fn collect_json_stats(value: &Value, depth: usize, stats: &mut JsonDocumentStats) {
    stats.nodes += 1;
    stats.max_depth = stats.max_depth.max(depth);
    match value {
        Value::Null => stats.nulls += 1,
        Value::Bool(_) => stats.booleans += 1,
        Value::Number(_) => stats.numbers += 1,
        Value::String(_) => stats.strings += 1,
        Value::Array(items) => {
            stats.arrays += 1;
            for item in items {
                collect_json_stats(item, depth + 1, stats);
            }
        }
        Value::Object(object) => {
            stats.objects += 1;
            stats.object_keys += object.len();
            for item in object.values() {
                collect_json_stats(item, depth + 1, stats);
            }
        }
    }
}

fn select_json_value(
    path: &str,
    value: &Value,
    max_value_bytes: usize,
) -> Result<JsonSelection, ToolError> {
    let serialized = serde_json::to_vec(value).map_err(|error| {
        ToolError::ExecutionFailed(format!("serialize JSON selection failed: {error}"))
    })?;
    let (object_keys, object_keys_truncated) = match value {
        Value::Object(object) => {
            let keys = object
                .keys()
                .take(MAX_JSON_OBJECT_KEYS)
                .cloned()
                .collect::<Vec<_>>();
            (keys, object.len() > MAX_JSON_OBJECT_KEYS)
        }
        _ => (Vec::new(), false),
    };
    Ok(JsonSelection {
        path: path.to_string(),
        value_type: json_type(value),
        array_length: value.as_array().map(Vec::len),
        string_length: value.as_str().map(|text| text.chars().count()),
        object_keys,
        object_keys_truncated,
        serialized_bytes: serialized.len(),
        value: (serialized.len() <= max_value_bytes).then(|| value.clone()),
        value_truncated: serialized.len() > max_value_bytes,
    })
}

fn json_type(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::{JsonDocumentStats, collect_json_stats, select_json_value};
    use serde_json::json;

    #[test]
    fn text_chunk_boundaries_are_valid_utf8_boundaries() {
        let text = "你好\nworld";
        let start = super::previous_char_boundary(text, 1);
        let end = super::next_char_boundary(text, 4);

        assert_eq!(start, 0);
        assert_eq!(end, 6);
        assert_eq!(&text[start..end], "你好");
    }

    #[test]
    fn json_stats_cover_the_complete_document() {
        let document = json!({
            "topics": [{"name": "one"}, {"name": "two"}],
            "enabled": true,
            "missing": null
        });
        let mut stats = JsonDocumentStats::default();
        collect_json_stats(&document, 0, &mut stats);

        assert_eq!(stats.objects, 3);
        assert_eq!(stats.arrays, 1);
        assert_eq!(stats.strings, 2);
        assert_eq!(stats.booleans, 1);
        assert_eq!(stats.nulls, 1);
        assert_eq!(stats.object_keys, 5);
        assert_eq!(stats.nodes, 8);
    }

    #[test]
    fn json_selection_omits_large_values_but_keeps_shape() {
        let value = json!([{"topic": "one"}, {"topic": "two"}]);
        let selection = select_json_value("/topics", &value, 1).expect("selection should work");

        assert_eq!(selection.value_type, "array");
        assert_eq!(selection.array_length, Some(2));
        assert!(selection.value.is_none());
        assert!(selection.value_truncated);
    }
}
