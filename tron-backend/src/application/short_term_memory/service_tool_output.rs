use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::{
    domain::{
        agent::{AgentMessage, AgentMessagePart, AgentRunContext, ToolObservation},
        short_term_memory::ShortTermMemoryError,
    },
    shared::time::current_timestamp,
};

use super::{
    ShortTermMemoryService, ToolTranscriptStatus,
    service_parts::{expand_home, sanitize_name, tool_result_part},
};

pub(super) struct ExternalizedOutput {
    pub(super) externalized: bool,
    pub(super) metadata: Option<Value>,
}

pub(super) fn externalized_tool_output_notice(metadata: Option<&Value>) -> String {
    let Some(metadata) = metadata else {
        return "{\"externalized\":true}".into();
    };
    json!({
        "externalized": true,
        "rootConfigKey": metadata.get("rootConfigKey").cloned().unwrap_or(Value::Null),
        "rootPath": metadata.get("rootPath").cloned().unwrap_or(Value::Null),
        "relativePath": metadata.get("relativePath").cloned().unwrap_or(Value::Null),
        "sha256": metadata.get("sha256").cloned().unwrap_or(Value::Null),
        "bytes": metadata.get("bytes").cloned().unwrap_or(Value::Null),
        "chars": metadata.get("chars").cloned().unwrap_or(Value::Null),
        "lines": metadata.get("lines").cloned().unwrap_or(Value::Null),
    })
    .to_string()
}

impl ShortTermMemoryService {
    pub fn sweep_externalized_tool_output_orphans(&self) -> Result<usize, ShortTermMemoryError> {
        const STARTUP_ORPHAN_GRACE: Duration = Duration::from_secs(300);
        let root = expand_home(&self.config.tool_output.root);
        if !root.exists() {
            return Ok(0);
        }
        let root_path = root.display().to_string();
        let referenced = self
            .repository()
            .list_externalized_tool_output_paths(&root_path)?
            .into_iter()
            .collect::<HashSet<_>>();
        let mut deleted = 0usize;
        for file in collect_files(&root)? {
            let relative = match file.strip_prefix(&root) {
                Ok(value) => value.display().to_string(),
                Err(_) => continue,
            };
            if referenced.contains(&relative) || !file_is_older_than(&file, STARTUP_ORPHAN_GRACE) {
                continue;
            }
            fs::remove_file(&file).map_err(|_| ShortTermMemoryError::StorageFailure)?;
            deleted += 1;
        }
        Ok(deleted)
    }

    pub fn record_tool_result(
        &self,
        context: &AgentRunContext,
        observation: &ToolObservation,
        model_call_id: Option<String>,
        sequence: u32,
        status: ToolTranscriptStatus,
    ) -> Result<AgentMessage, ShortTermMemoryError> {
        let Some(memory) = context.short_term_memory.as_ref() else {
            return Ok(AgentMessage::tool_result(
                observation.tool_name.clone(),
                model_call_id,
                observation.output.clone(),
            ));
        };
        let externalized = self.externalize_tool_output(
            context,
            &observation.tool_name,
            sequence,
            &observation.output,
        )?;
        let output_for_context = if externalized.externalized {
            externalized_tool_output_notice(externalized.metadata.as_ref())
        } else {
            observation.output.clone()
        };
        let message = AgentMessage::new(
            "tool",
            vec![AgentMessagePart::ToolResult {
                call_id: model_call_id.clone(),
                tool_name: observation.tool_name.clone(),
                output: output_for_context.clone(),
                externalized: externalized.externalized,
                metadata: externalized.metadata.clone(),
            }],
        );
        let artifact_path = externalized
            .metadata
            .as_ref()
            .and_then(externalized_artifact_path);
        let event_model_call_id = model_call_id.clone();
        let mut part = tool_result_part(
            &memory.session_code,
            &context.workspace_code,
            1,
            observation,
            model_call_id,
            sequence,
            output_for_context,
            externalized.externalized,
            externalized.metadata,
            status,
        );
        part.part_code = self.next_code("ATP")?;
        let part_code = part.part_code.clone();
        let (message_code, transcript_message, transcript_parts) = self.build_transcript_message(
            &memory.session_code,
            &context.workspace_code,
            Some(&context.run_code),
            "tool",
            "tool_result",
            Some(context),
            vec![part],
            None,
        )?;
        let event = self.new_event(
            &memory.session_code,
            &context.workspace_code,
            Some(&context.run_code),
            Some(&memory.input_code),
            Some(&message_code),
            Some(&part_code),
            status.event_type(),
            "TOOL",
            json!({
                "toolName": observation.tool_name,
                "status": status.part_status(),
                "callIndex": sequence,
                "modelCallId": event_model_call_id.clone(),
                "messageCode": message_code.clone(),
                "partCode": part_code.clone(),
            }),
            None,
        )?;
        let created_event = self.new_event(
            &memory.session_code,
            &context.workspace_code,
            Some(&context.run_code),
            Some(&memory.input_code),
            Some(&message_code),
            Some(&part_code),
            "agent.message.tool_result_created",
            "RUNTIME",
            json!({
                "toolName": observation.tool_name,
                "externalized": externalized.externalized,
                "callIndex": sequence,
                "modelCallId": event_model_call_id,
                "messageCode": message_code.clone(),
                "partCode": part_code.clone(),
            }),
            None,
        )?;
        let events = [event, created_event]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        if let Err(error) = self.repository().create_message_with_events(
            &transcript_message,
            &transcript_parts,
            &events,
        ) {
            if let Some(path) = artifact_path {
                let _ = fs::remove_file(path);
            }
            return Err(error);
        }
        Ok(message)
    }

    pub fn record_tool_started(
        &self,
        context: &AgentRunContext,
        tool_name: &str,
        model_call_id: Option<&str>,
        sequence: u32,
    ) -> Result<(), ShortTermMemoryError> {
        let Some(memory) = context.short_term_memory.as_ref() else {
            return Ok(());
        };
        self.append_event(
            &memory.session_code,
            &context.workspace_code,
            Some(&context.run_code),
            Some(&memory.input_code),
            None,
            None,
            "agent.tool.started",
            "TOOL",
            json!({
                "toolName": tool_name,
                "modelCallId": model_call_id,
                "sequence": sequence,
            }),
            None,
        )
    }

    pub(super) fn externalize_tool_output(
        &self,
        context: &AgentRunContext,
        tool_name: &str,
        sequence: u32,
        output: &str,
    ) -> Result<ExternalizedOutput, ShortTermMemoryError> {
        let bytes = output.as_bytes();
        if bytes.len() <= self.config.tool_output.externalize_above_bytes {
            return Ok(ExternalizedOutput {
                externalized: false,
                metadata: None,
            });
        }
        let root = expand_home(&self.config.tool_output.root);
        let session = context.conversation_code.as_deref().unwrap_or("standalone");
        let relative = PathBuf::from(&context.workspace_code)
            .join(session)
            .join(&context.run_code)
            .join(format!("{sequence:04}-{}.txt", sanitize_name(tool_name)));
        let path = root.join(&relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|_| ShortTermMemoryError::StorageFailure)?;
        }
        fs::write(&path, bytes).map_err(|_| ShortTermMemoryError::StorageFailure)?;
        let sha256 = hex::encode(Sha256::digest(bytes));
        let line_count = output.lines().count();
        Ok(ExternalizedOutput {
            externalized: true,
            metadata: Some(json!({
                "bytes": bytes.len(),
                "chars": output.chars().count(),
                "lines": line_count,
                "sha256": sha256,
                "encoding": "utf-8",
                "toolName": tool_name,
                "externalizedAt": current_timestamp(),
                "externalizeAboveBytes": self.config.tool_output.externalize_above_bytes,
                "rootConfigKey": "shortTermMemory.toolOutput.root",
                "rootPath": root.display().to_string(),
                "relativePath": relative.display().to_string(),
            })),
        })
    }
}

fn collect_files(root: &Path) -> Result<Vec<PathBuf>, ShortTermMemoryError> {
    let mut files = Vec::new();
    collect_files_into(root, &mut files)?;
    Ok(files)
}

fn collect_files_into(root: &Path, files: &mut Vec<PathBuf>) -> Result<(), ShortTermMemoryError> {
    for entry in fs::read_dir(root).map_err(|_| ShortTermMemoryError::StorageFailure)? {
        let entry = entry.map_err(|_| ShortTermMemoryError::StorageFailure)?;
        let path = entry.path();
        let metadata = entry
            .metadata()
            .map_err(|_| ShortTermMemoryError::StorageFailure)?;
        if metadata.is_dir() {
            collect_files_into(&path, files)?;
        } else if metadata.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

fn file_is_older_than(path: &Path, grace: Duration) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    let Ok(modified) = metadata.modified() else {
        return false;
    };
    SystemTime::now()
        .duration_since(modified)
        .is_ok_and(|age| age >= grace)
}

fn externalized_artifact_path(metadata: &Value) -> Option<PathBuf> {
    let root = metadata.get("rootPath")?.as_str()?;
    let relative = metadata.get("relativePath")?.as_str()?;
    Some(PathBuf::from(root).join(relative))
}
