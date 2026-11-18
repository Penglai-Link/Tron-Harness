//! Artifact Delivery: explicit file delivery for one run.
//!
//! The pack contributes a single tool (`artifacts.present`), the run options
//! object carrying the exact workspace root and the run-scoped collector, and
//! short model guidance. It never copies files, never persists records and
//! never executes file actions: the host owns persistence, projection and all
//! operating-system behaviour.
//!
//! The collector stays private to the run. Public custom events and tool
//! results carry only display fields, never the workspace root or relative
//! path used for authorization.

use std::{
    collections::HashMap,
    path::{Component, Path, PathBuf},
    sync::{Arc, Mutex},
    time::UNIX_EPOCH,
};

use async_trait::async_trait;
use eliteforge_agent_core::{
    AgentError, CapabilityPackBuilder, ContextBlock, ContextContributor, ContextContributorPlugin,
    ContextInput, ContextSourceKind, ManagedDeveloperPrompts, Message, OutputEvent, RunContext,
    Tool, ToolBinding, ToolDescriptor, ToolError, ToolLease, ToolOutput, ToolProvider,
    ToolProviderPlugin,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Stable capability pack identity.
pub const PACK_ID: &str = "tron.artifact-delivery";
/// Framework tool identity.
pub const TOOL_ID: &str = "artifacts.present";
/// Model-visible function name.
pub const TOOL_CALL_NAME: &str = "artifacts_present";
/// CUSTOM event carrying the public delivery notice.
pub const EVENT_NAME: &str = "agent.artifacts";
/// Managed developer prompt key the host may override.
pub const GUIDANCE_KEY: &str = "tron.artifact-delivery.guidance";
/// Protocol version of the public payload.
pub const VERSION: &str = "v1";

const DEFAULT_MAX_FILES_PER_CALL: usize = 8;
const DEFAULT_MAX_FILES_PER_RUN: usize = 32;
const DEFAULT_MAX_FILE_BYTES: u64 = 200 * 1024 * 1024;

const DEFAULT_GUIDANCE: &str = "\
When a task produces files the user should keep (reports, exports, images, documents, archives), call `artifacts_present` once they are final. \
Pass workspace-relative paths and a short description for each file. \
Do not list files you only read or edited; do not call the tool when the user did not receive a file. \
Files are referenced in place: nothing is copied, and a later move or deletion makes the delivery unreadable.";

/// Tunables applied when the pack is constructed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ArtifactDeliveryConfig {
    /// Maximum files accepted by one tool call.
    pub max_files_per_call: usize,
    /// Maximum distinct files a single run may deliver.
    pub max_files_per_run: usize,
    /// Maximum size of one delivered file.
    pub max_file_bytes: u64,
}

impl Default for ArtifactDeliveryConfig {
    fn default() -> Self {
        Self {
            max_files_per_call: DEFAULT_MAX_FILES_PER_CALL,
            max_files_per_run: DEFAULT_MAX_FILES_PER_RUN,
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
        }
    }
}

/// Trusted per-run delivery state inserted by the host into `RunRequest.extensions`.
#[derive(Clone)]
pub struct ArtifactsRunOptions {
    /// Exact execution root the host resolved for this run.
    pub workspace_root: PathBuf,
    /// Run-scoped collector the host reads after the run settled.
    pub collector: Arc<RunArtifactCollector>,
}

/// Public item shared by the custom event, the tool result and the message DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicArtifactItem {
    pub id: String,
    pub display: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub byte_size: Option<u64>,
}

/// Where a delivered file lives. Private: never part of a public payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactSource {
    pub kind: String,
    pub root: String,
    pub relative_path: String,
}

/// Cheap change detector; not a cryptographic integrity check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactFingerprint {
    pub size: u64,
    pub modified_at_ns: u64,
}

/// Private record persisted by the host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredArtifactRecord {
    pub id: String,
    pub display: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub byte_size: Option<u64>,
    pub source: ArtifactSource,
    pub fingerprint: ArtifactFingerprint,
}

impl StoredArtifactRecord {
    /// Public projection of this record. Actions are computed by the host.
    pub fn public(&self) -> PublicArtifactItem {
        PublicArtifactItem {
            id: self.id.clone(),
            display: self.display.clone(),
            description: self.description.clone(),
            media_type: self.media_type.clone(),
            byte_size: self.byte_size,
        }
    }
}

#[derive(Default)]
struct CollectorState {
    records: Vec<StoredArtifactRecord>,
    index: HashMap<String, usize>,
}

/// Run-scoped, in-memory, ordered and de-duplicated delivery set.
pub struct RunArtifactCollector {
    max_files: usize,
    state: Mutex<CollectorState>,
}

impl RunArtifactCollector {
    pub fn new(max_files: usize) -> Self {
        Self {
            max_files: max_files.max(1),
            state: Mutex::new(CollectorState::default()),
        }
    }

    /// Merge validated files. Returns the public items of this call in order.
    pub fn record_many(
        &self,
        root: &Path,
        validated: Vec<ValidatedArtifact>,
    ) -> Result<Vec<PublicArtifactItem>, String> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut incoming = 0usize;
        for file in &validated {
            if !state.index.contains_key(&file.relative_path) {
                incoming += 1;
            }
        }
        if state.index.len() + incoming > self.max_files {
            return Err(format!(
                "a run can deliver at most {} distinct files",
                self.max_files
            ));
        }
        let root_text = root.to_string_lossy().into_owned();
        let mut items = Vec::with_capacity(validated.len());
        for file in validated {
            let fingerprint = ArtifactFingerprint {
                size: file.size,
                modified_at_ns: file.modified_at_ns,
            };
            if let Some(&position) = state.index.get(&file.relative_path) {
                let record = &mut state.records[position];
                record.description = file.description;
                record.media_type = file.media_type;
                record.byte_size = Some(file.size);
                record.fingerprint = fingerprint;
                items.push(record.public());
                continue;
            }
            let record = StoredArtifactRecord {
                id: format!("f_{}", eliteforge_agent_core::RunId::new_v4()),
                display: file.display,
                description: file.description,
                media_type: file.media_type,
                byte_size: Some(file.size),
                source: ArtifactSource {
                    kind: "workspace-file".into(),
                    root: root_text.clone(),
                    relative_path: file.relative_path.clone(),
                },
                fingerprint,
            };
            let position = state.records.len();
            items.push(record.public());
            state.records.push(record);
            state.index.insert(file.relative_path, position);
        }
        Ok(items)
    }

    /// Stable snapshot for persistence after the run settled.
    pub fn snapshot(&self) -> Vec<StoredArtifactRecord> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .records
            .clone()
    }
}

/// Result of validating one requested path.
pub struct ValidatedArtifact {
    pub display: String,
    pub description: Option<String>,
    pub media_type: Option<String>,
    pub relative_path: String,
    pub size: u64,
    pub modified_at_ns: u64,
}

#[derive(Debug, Deserialize)]
struct PresentArguments {
    #[serde(default)]
    files: Vec<PresentFile>,
}

#[derive(Debug, Deserialize)]
struct PresentFile {
    #[serde(default)]
    path: String,
    #[serde(default)]
    description: Option<String>,
}

fn media_type_for(name: &str) -> Option<String> {
    let extension = Path::new(name)
        .extension()
        .and_then(|value| value.to_str())?
        .to_ascii_lowercase();
    let media = match extension.as_str() {
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "txt" => "text/plain",
        "md" => "text/markdown",
        "json" => "application/json",
        "csv" => "text/csv",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "zip" => "application/zip",
        "dmg" => "application/x-apple-diskimage",
        "apk" => "application/vnd.android.package-archive",
        "mp4" => "video/mp4",
        "mp3" => "audio/mpeg",
        "html" => "text/html",
        "css" => "text/css",
        "js" => "text/javascript",
        "ts" => "text/typescript",
        "rs" => "text/rust",
        "py" => "text/x-python",
        _ => "application/octet-stream",
    };
    Some(media.to_owned())
}

fn normalize_relative(path: &Path) -> Result<(String, PathBuf), String> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            // `./report.pdf` is a common model spelling of a workspace path.
            // CurDir carries no traversal; only ParentDir can leave the root.
            Component::CurDir => continue,
            Component::Normal(value) => {
                let text = value
                    .to_str()
                    .ok_or_else(|| "path is not valid UTF-8".to_owned())?;
                if text.is_empty() {
                    return Err("path contains an empty component".into());
                }
                normalized.push(value);
            }
            _ => return Err("path must stay inside the run workspace".into()),
        }
    }
    if normalized.as_os_str().is_empty() {
        return Err("path is empty".into());
    }
    let display = normalized
        .to_str()
        .ok_or_else(|| "path is not valid UTF-8".to_owned())?
        .replace('\\', "/");
    Ok((display, normalized))
}

/// Validate one requested path against the run workspace root.
pub fn validate_file(
    root: &Path,
    raw: &str,
    description: Option<String>,
    max_bytes: u64,
) -> Result<ValidatedArtifact, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("file path is empty".into());
    }
    if trimmed.contains('\0') || trimmed.contains('\\') {
        return Err(format!("invalid file path: {trimmed}"));
    }
    let requested = Path::new(trimmed);
    if requested.is_absolute() {
        return Err(format!(
            "file path must be relative to the run workspace: {trimmed}"
        ));
    }
    let (relative_path, _) = normalize_relative(requested)?;

    let canonical_root = root
        .canonicalize()
        .map_err(|error| format!("run workspace is unavailable: {error}"))?;

    let mut current = canonical_root.clone();
    let mut metadata = None;
    let components: Vec<_> = requested.components().collect();
    for (index, component) in components.iter().enumerate() {
        current.push(component.as_os_str());
        let entry = std::fs::symlink_metadata(&current)
            .map_err(|error| format!("cannot read {trimmed}: {error}"))?;
        if entry.file_type().is_symlink() {
            return Err(format!("symbolic links are not deliverable: {trimmed}"));
        }
        let last = index + 1 == components.len();
        if last {
            if !entry.file_type().is_file() {
                return Err(format!("not a regular file: {trimmed}"));
            }
            metadata = Some(entry);
        } else if !entry.file_type().is_dir() {
            return Err(format!("path is not a directory: {trimmed}"));
        }
    }
    let metadata = metadata.ok_or_else(|| format!("cannot read {trimmed}"))?;
    let resolved = current
        .canonicalize()
        .map_err(|error| format!("cannot resolve {trimmed}: {error}"))?;
    if !resolved.starts_with(&canonical_root) {
        return Err(format!(
            "file path leaves the run workspace: {trimmed}"
        ));
    }
    let size = metadata.len();
    if size > max_bytes {
        return Err(format!(
            "file exceeds the delivery size limit of {max_bytes} bytes: {trimmed}"
        ));
    }
    let modified_at_ns = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_nanos().min(u64::MAX as u128) as u64)
        .unwrap_or(0);
    let display = resolved
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "file name is not valid UTF-8".to_owned())?
        .to_owned();
    let media_type = media_type_for(&display);
    Ok(ValidatedArtifact {
        display,
        description,
        media_type,
        relative_path,
        size,
        modified_at_ns,
    })
}

fn descriptor(max_files: usize) -> ToolDescriptor {
    ToolDescriptor::new(
        TOOL_ID,
        TOOL_CALL_NAME,
        "Declare existing files as user-facing deliverables for this run. \
         Paths are relative to the run workspace. Call once the files are final; \
         nothing is copied, and a later move or deletion makes the delivery unreadable.",
        json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["files"],
            "properties": {
                "files": {
                    "type": "array",
                    "minItems": 1,
                    "maxItems": max_files,
                    "items": {
                        "type": "object",
                        "additionalProperties": false,
                        "required": ["path"],
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Workspace-relative path of an existing regular file."
                            },
                            "description": {
                                "type": "string",
                                "maxLength": 512,
                                "description": "Short description shown next to the file."
                            }
                        }
                    }
                }
            }
        }),
    )
}

/// Tool provider registered by the pack.
#[derive(Clone)]
pub struct ArtifactToolProvider {
    config: Arc<ArtifactDeliveryConfig>,
}

impl ArtifactToolProvider {
    pub fn new(config: Arc<ArtifactDeliveryConfig>) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ToolProvider for ArtifactToolProvider {
    fn descriptors(&self) -> Option<Vec<ToolDescriptor>> {
        Some(vec![descriptor(self.config.max_files_per_call)])
    }

    async fn lease(&self, run: &RunContext) -> Result<ToolLease, eliteforge_agent_core::AgentError> {
        if run.extensions.get::<ArtifactsRunOptions>().is_none() {
            return Ok(ToolLease::new(vec![]));
        }
        Ok(ToolLease::new(vec![ToolBinding::new(
            descriptor(self.config.max_files_per_call),
            ArtifactPresentTool {
                config: self.config.clone(),
            },
        )
        .with_domain_code("artifacts")]))
    }
}

struct ArtifactPresentTool {
    config: Arc<ArtifactDeliveryConfig>,
}

#[async_trait]
impl Tool for ArtifactPresentTool {
    async fn call(
        &self,
        arguments: Value,
        run: &RunContext,
    ) -> Result<ToolOutput, ToolError> {
        let options = run.extensions.get::<ArtifactsRunOptions>().ok_or_else(|| {
            ToolError::Failed("artifact delivery is not enabled for this run".into())
        })?;
        let parsed: PresentArguments = serde_json::from_value(arguments)
            .map_err(|error| ToolError::Failed(format!("invalid arguments: {error}")))?;
        if parsed.files.is_empty() {
            return Err(ToolError::Failed(
                "at least one file must be delivered".into(),
            ));
        }
        if parsed.files.len() > self.config.max_files_per_call {
            return Err(ToolError::Failed(format!(
                "at most {} files can be delivered in one call",
                self.config.max_files_per_call
            )));
        }
        let mut validated = Vec::with_capacity(parsed.files.len());
        for file in parsed.files {
            match validate_file(
                &options.workspace_root,
                &file.path,
                file.description,
                self.config.max_file_bytes,
            ) {
                Ok(value) => validated.push(value),
                Err(error) => return Err(ToolError::Failed(error)),
            }
        }
        let items = options
            .collector
            .record_many(&options.workspace_root, validated)
            .map_err(ToolError::Failed)?;
        let payload = json!({
            "version": VERSION,
            "runId": run.run_id.to_string(),
            "items": items,
        });
        run.emit_output(OutputEvent::Custom {
            name: EVENT_NAME.into(),
            value: payload.clone(),
        });
        let mut output = ToolOutput::text(format!(
            "Delivered {} file(s) to the user.",
            items.len()
        ));
        output.structured = Some(payload);
        Ok(output)
    }
}

/// Model guidance, gated on the negotiated run options.
pub struct ArtifactGuidance {
    _config: Arc<ArtifactDeliveryConfig>,
}

impl ArtifactGuidance {
    pub fn new(config: Arc<ArtifactDeliveryConfig>) -> Self {
        Self { _config: config }
    }
}

#[async_trait]
impl ContextContributor for ArtifactGuidance {
    fn kind(&self) -> ContextSourceKind {
        ContextSourceKind::Instruction
    }

    fn block_id(&self) -> &str {
        GUIDANCE_KEY
    }

    async fn contribute(
        &self,
        input: &ContextInput<'_>,
    ) -> Result<ContextBlock, AgentError> {
        let messages = if input
            .run
            .extensions
            .get::<ArtifactsRunOptions>()
            .is_some()
        {
            let text = input
                .run
                .extensions
                .get::<ManagedDeveloperPrompts>()
                .and_then(|prompts| prompts.0.get(GUIDANCE_KEY).cloned())
                .unwrap_or_else(|| DEFAULT_GUIDANCE.to_owned());
            vec![Message::system(text)]
        } else {
            vec![]
        };
        Ok(ContextBlock::instructions(self.block_id(), messages))
    }
}

/// Build the artifact delivery capability pack.
pub fn artifact_delivery_pack(
    config: ArtifactDeliveryConfig,
) -> CapabilityPackBuilder<ArtifactDeliveryConfig> {
    let shared = Arc::new(config.clone());
    CapabilityPackBuilder::new(PACK_ID, config)
        .plugin(ToolProviderPlugin::new(
            "tron.artifact-delivery.tool",
            ArtifactToolProvider::new(shared.clone()),
        ))
        .plugin(ContextContributorPlugin::new(
            "tron.artifact-delivery.guidance",
            ArtifactGuidance::new(shared),
        ))
}