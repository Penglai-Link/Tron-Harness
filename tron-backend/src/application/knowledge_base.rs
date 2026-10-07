pub mod ingestion;
pub mod deletion;
mod ingestion_worker;
pub mod local_documents;
pub mod parsing;
mod parsing_cache;
pub mod fidelity; // Retained diagnostic evidence; never invoked by normal ingestion.
pub mod pdf_visual_source;
pub mod office_visuals;
mod source_locations;
mod visual_evidence;
pub use visual_evidence::{KnowledgeBaseVisualEvidence, KnowledgeBaseVisualContext};
pub mod image_descriptions;
use crate::{
    adapters::outbound::persistence::sqlite::SqliteKnowledgeBaseRepository,
    app::AppRagService,
    application::{
        agent::AgentManagementService,
        llm::{LlmEmbeddingCallCommand, LlmRuntimeService},
        rag::*,
        resource::{
            ResourceBinaryDownloadCommand, ResourceCommitBinary, ResourceCreateCommand,
            ResourceService,
        },
        workspace::WorkspaceService,
    },
    domain::{rag::*, resource::ResourceKind},
    shared::sqlite_extensions::TRON_VECTOR_INDEX_DIMENSION,
};
use local_documents::{ChunkConfig, ImageAnalysisSummary, LocatedChunk, parse_local_document, split_document};
use parsing::{KnowledgeDocumentParser, ParsingArtifact, ParsingCapabilities, ParsingTier};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, io::Write};
#[derive(Debug, Clone)]
pub enum KnowledgeBaseError {
    NotFound,
    Forbidden,
    Conflict,
    Invalid(String),
    Storage(String),
}
impl std::fmt::Display for KnowledgeBaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => f.write_str("知识库或文档不存在"),
            Self::Forbidden => f.write_str("无知识库权限"),
            Self::Conflict => f.write_str("文档正在索引或状态已改变，请刷新后重试"),
            Self::Invalid(s) | Self::Storage(s) => f.write_str(s),
        }
    }
}
impl std::error::Error for KnowledgeBaseError {}
impl From<RagError> for KnowledgeBaseError {
    fn from(e: RagError) -> Self {
        Self::Storage(e.to_string())
    }
}
impl From<crate::domain::resource::ResourceError> for KnowledgeBaseError {
    fn from(e: crate::domain::resource::ResourceError) -> Self {
        use crate::domain::resource::ResourceError;
        match e {
            ResourceError::NotFound => Self::NotFound,
            ResourceError::Forbidden => Self::Forbidden,
            ResourceError::Conflict => Self::Conflict,
            ResourceError::InvalidInput
            | ResourceError::InvalidCode
            | ResourceError::InvalidWorkspace
            | ResourceError::WorkspaceRequired => Self::Invalid(e.to_string()),
            _ => Self::Storage(e.to_string()),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeBase {
    pub id: String,
    pub name: String,
    pub description: String,
    pub workspace_code: String,
    pub owner_user_code: String,
    pub default_chunk_config: ChunkConfig,
    #[serde(default)]
    pub documents: Vec<KnowledgeBaseDocument>,
    #[serde(default)]
    pub document_count: usize,
    #[serde(default)]
    pub agent_codes: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeBaseDocument {
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub stage: Option<ingestion::IngestionStage>,
    #[serde(default)]
    pub wait_reason: Option<ingestion::IngestionWaitReason>,
    pub id: String,
    pub name: String,
    pub file_name: String,
    pub resource_code: String,
    pub resource_revision: u64,
    pub chunk_config: Option<ChunkConfig>,
    pub status: String,
    pub error: Option<String>,
    pub chunk_count: usize,
    pub index_revision: u64,
    pub revision: u64,
    pub active_source_code: Option<String>,
    pub embedding_model_code: Option<String>,
    pub embedding_dimension: Option<usize>,
    #[serde(default)]
    pub embedding_provider_model: Option<String>,
    #[serde(default)]
    pub embedding_space_fingerprint: Option<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_analysis: Option<ImageAnalysisSummary>,
    #[serde(default)]
    pub parsing_tier: Option<ParsingTier>,
    #[serde(default)]
    pub parsing_artifact: Option<ParsingArtifact>,
    #[serde(default)]
    pub derived_resource_codes: Vec<String>,
    #[serde(default)]
    pub visual_source: Option<pdf_visual_source::PdfVisualSource>,
    #[serde(default)]
    pub image_description_artifacts: Vec<image_descriptions::ImageDescriptionArtifact>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeBasePreview {
    pub fidelity_issues: Vec<fidelity::FidelityIssue>,
    pub chunk_count: usize,
    pub chunks: Vec<LocatedChunk>,
    pub warnings: Vec<String>,
    pub image_analysis: Option<ImageAnalysisSummary>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeBaseHit {
    pub knowledge_base_id: String,
    pub document_id: String,
    pub resource_code: String,
    pub resource_revision: u64,
    pub source_code: String,
    pub chunk_code: String,
    pub title: String,
    pub text: String,
    pub location: Value,
    pub score: f64,
}
#[derive(Debug, Clone)]
pub struct KnowledgeBaseContext {
    pub text: String,
    pub source_version: String,
    pub hits: Vec<KnowledgeBaseHit>,
}
#[derive(Clone)]
pub struct KnowledgeBaseService {
    cleanup_gate: Arc<std::sync::Mutex<()>>,
    queue: Arc<dyn ingestion::KnowledgeIngestionQueue>,
    ingestion_config: crate::config::KnowledgeIngestionConfig,
    workers: Arc<ingestion_worker::IngestionWorkers>,
    parse_checkpoint: Option<Arc<dyn parsing::ParseCheckpoint>>,
    repository: SqliteKnowledgeBaseRepository,
    rag: AppRagService,
    resources: ResourceService,
    workspaces: WorkspaceService,
    agents: AgentManagementService,
    llm: LlmRuntimeService,
    parser: Option<Arc<dyn KnowledgeDocumentParser>>,
    office_renderer: Option<Arc<dyn parsing::KnowledgeOfficeRenderer>>,
    image_description: crate::config::ImageDescriptionConfig,
    visual_context_enabled: bool,
}
impl KnowledgeBaseService {
    pub fn new(
        repository: SqliteKnowledgeBaseRepository,
        rag: AppRagService,
        resources: ResourceService,
        workspaces: WorkspaceService,
        agents: AgentManagementService,
        llm: LlmRuntimeService,
    ) -> Self {
        let queue = Arc::new(repository.ingestion_queue().expect("knowledge ingestion schema"));
        Self {
            cleanup_gate: Default::default(),
            queue,
            ingestion_config: Default::default(),
            workers: Default::default(),
            parse_checkpoint: None,
            repository,
            rag,
            resources,
            workspaces,
            agents,
            llm,
            parser: None,
            office_renderer: None,
            image_description: Default::default(),
            visual_context_enabled: false,
        }
    }
    /// Disabled by default; enabling allows authorized KB image bytes to reach the answer model.
    pub fn with_visual_context(mut self, enabled: bool) -> Self { self.visual_context_enabled=enabled; self }
    pub fn visual_context_enabled(&self) -> bool { self.visual_context_enabled }
    pub fn with_image_descriptions(mut self, config: crate::config::ImageDescriptionConfig) -> Self { self.image_description = config; self }
    pub fn with_office_renderer(mut self, renderer: Arc<dyn parsing::KnowledgeOfficeRenderer>) -> Self { self.office_renderer=Some(renderer); self }
    pub fn with_document_parser(mut self, parser: Arc<dyn KnowledgeDocumentParser>) -> Self { self.parser = Some(parser); self }
    pub async fn parsing_capabilities(&self, user: &str, workspace: &str) -> Result<ParsingCapabilities, KnowledgeBaseError> {
        self.workspace(user, workspace)?;
        self.parser.as_ref().ok_or_else(||KnowledgeBaseError::Invalid("文档解析服务未配置".into()))?.capabilities().await
    }
    fn workspace(&self, user: &str, workspace: &str) -> Result<(), KnowledgeBaseError> {
        if user.trim().is_empty()
            || workspace.trim().is_empty()
            || !self
                .workspaces
                .workspace_is_active(workspace)
                .unwrap_or(false)
        {
            return Err(KnowledgeBaseError::Forbidden);
        }
        Ok(())
    }
    fn owned(
        &self,
        user: &str,
        workspace: &str,
        id: &str,
    ) -> Result<KnowledgeBase, KnowledgeBaseError> {
        self.workspace(user, workspace)?;
        if !self.repository.owns(user, workspace, id) {
            return Err(KnowledgeBaseError::NotFound);
        }
        self.repository.get(id)
    }
    pub fn list(
        &self,
        user: &str,
        workspace: &str,
    ) -> Result<Vec<KnowledgeBase>, KnowledgeBaseError> {
        self.workspace(user, workspace)?;
        let mut list = self.repository.list(user, workspace)?;
        for kb in &mut list {
            kb.document_count = self.repository.documents(&kb.id)?.len();
        }
        Ok(list)
    }
    pub fn detail(
        &self,
        user: &str,
        workspace: &str,
        id: &str,
    ) -> Result<KnowledgeBase, KnowledgeBaseError> {
        let mut kb = self.owned(user, workspace, id)?;
        kb.documents = self.repository.documents(id)?;
        for doc in &mut kb.documents {
            if let Some(task) = self.queue.for_document(&doc.id).map_err(ingestion_worker::queue_error)?.first() {
                doc.task_id = Some(task.id.clone()); doc.stage = Some(task.stage);
                doc.wait_reason = task.wait_reason.clone();
            }
        }
        kb.document_count = kb.documents.len();
        kb.agent_codes = self.repository.grants(id)?;
        Ok(kb)
    }
    pub fn published_chunks(
        &self,
        user: &str,
        workspace: &str,
        id: &str,
        document: &str,
        page: usize,
        page_size: usize,
    ) -> Result<RagSourceUnitsPage, KnowledgeBaseError> {
        let _guard = self.cleanup_gate.lock().map_err(invalid_storage)?;
        let kb = self.owned(user, workspace, id)?;
        let doc = self.repository.document(&kb.id, document)?;
        if page == 0 || page_size == 0 || page_size > 100 || page > 1_000_000 {
            return Err(KnowledgeBaseError::Invalid(
                "Invalid chunk pagination".into(),
            ));
        }
        let Some(source) = doc.active_source_code else {
            return Ok(RagSourceUnitsPage {
                items: vec![],
                total: 0,
                page,
                page_size,
            });
        };
        self.rag
            .repository_handle()
            .source_units_page(
                RagDebugSourceRequest {
                    workspace_code: workspace.into(),
                    corpus_id: kb.id,
                    source_code: source,
                    source_version: "v1".into(),
                    scope_type: "SPACE".into(),
                    scope_code: workspace.into(),
                    include_text_units: true,
                    include_warnings: false,
                },
                page,
                page_size,
            )
            .map_err(Into::into)
    }

    pub fn create(
        &self,
        user: &str,
        workspace: &str,
        name: &str,
        description: &str,
    ) -> Result<KnowledgeBase, KnowledgeBaseError> {
        self.workspace(user, workspace)?;
        if name.trim().is_empty() || name.chars().count() > 120 {
            return Err(KnowledgeBaseError::Invalid(
                "知识库名称需为1–120字符".into(),
            ));
        }
        let id = format!("kb_{}", new_id());
        let kb = KnowledgeBase {
            id: id.clone(),
            name: name.trim().into(),
            description: description.trim().into(),
            workspace_code: workspace.into(),
            owner_user_code: user.into(),
            default_chunk_config: ChunkConfig::default(),
            documents: vec![],
            document_count: 0,
            agent_codes: vec![],
        };
        // Ownership exists before corpus publication, so generic RAG never observes an unprotected KB.
        self.repository.insert(&kb, user)?;
        let table = format!("rag_{}_units", id);
        let target = RagStorageSegmentTarget {
            id: "documents".into(),
            backend: RagStorageBackend::Sqlite,
            layout: "EMBEDDED".into(),
            table: Some(table),
            namespace: None,
            node_label: None,
            code_strategy: Some("CORPUS_SOURCE_VERSION_SCOPE_HASH".into()),
        };
        let mapping: BTreeMap<String, String> = [
            ("sourceCode", "source_code"),
            ("sourceVersion", "source_version"),
            ("scopeType", "scope_type"),
            ("scopeCode", "scope_code"),
            ("status", "source_status"),
            ("title", "title"),
            ("metadata", "source_metadata"),
            ("text", "text"),
            ("textUnitStatus", "status"),
        ]
        .into_iter()
        .map(|(a, b)| (a.into(), b.into()))
        .collect();
        let corpus = RagCorpusRecord {
            corpus_id: id.clone(),
            display_name: kb.name.clone(),
            description: Some(kb.description.clone()),
            status: RagStatus::Enabled,
            capabilities: json!({"fulltext":true,"vector":true,"tree":false,"graph":false}),
            filter_schema: json!({"fields":{"sourceCode":{"type":"string","column":"source_code","operators":["eq","in"]}}}),
            field_mapping: mapping.clone(),
            source_field_mapping: mapping
                .iter()
                .filter(|(k, _)| k.as_str() != "text" && k.as_str() != "textUnitStatus")
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            text_unit_field_mapping: mapping
                .iter()
                .filter(|(k, _)| {
                    matches!(
                        k.as_str(),
                        "sourceCode"
                            | "sourceVersion"
                            | "scopeType"
                            | "scopeCode"
                            | "text"
                            | "textUnitStatus"
                    )
                })
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            storage_binding: RagStorageBinding {
                segment_bindings: ["source", "textUnit", "fulltext", "vector"]
                    .into_iter()
                    .map(|s| (s.into(), "documents".into()))
                    .collect(),
                segment_targets: BTreeMap::from([("documents".into(), target)]),
            },
        };
        self.rag.repository_handle().save_managed_corpus(&corpus)?;
        Ok(kb)
    }
    pub fn upload(
        &self,
        user: &str,
        workspace: &str,
        kb_id: &str,
        filename: &str,
        bytes: &[u8],
    ) -> Result<KnowledgeBaseDocument, KnowledgeBaseError> {
        self.upload_original(user, workspace, kb_id, filename, bytes, false)
    }
    pub(super) fn upload_original(
        &self,
        user: &str,
        workspace: &str,
        kb_id: &str,
        filename: &str,
        bytes: &[u8],
        request_ingestion: bool,
    ) -> Result<KnowledgeBaseDocument, KnowledgeBaseError> {
        self.owned(user, workspace, kb_id)?;
        // Serialize original creation across service clones; stable content IDs also
        // prevent duplicate persisted documents after request replay or restart.
        static UPLOAD_GATE: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _upload_guard = UPLOAD_GATE.lock().map_err(invalid_storage)?;
        crate::domain::resource::validate_binary_name(filename, 255)?;
        if bytes.len() > 20 * 1024 * 1024 {
            return Err(KnowledgeBaseError::Invalid("资料超过20 MiB上限".into()));
        }
        let ext = std::path::Path::new(filename)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !matches!(ext.as_str(), "md" | "docx" | "xlsx" | "pdf") {
            return Err(KnowledgeBaseError::Invalid(
                "支持 .md/.docx/.xlsx/.pdf；旧 .doc/.xls 请先另存为新格式".into(),
            ));
        }
        let id = format!("doc_{}", hash(&serde_json::to_string(&(user, workspace, kb_id, filename, format!("{:x}", Sha256::digest(bytes)))).map_err(invalid_storage)?));
        if let Ok(existing) = self.repository.document(kb_id, &id) {
            if request_ingestion { self.repository.request_ingestion(kb_id, &existing)?; }
            return self.repository.document(kb_id, &id);
        }
        let resource = format!("KB_{}", new_id());
        self.repository.mark_private(user, workspace, &resource)?;
        let mut temporary = tempfile::NamedTempFile::new().map_err(invalid_storage)?;
        temporary.write_all(bytes).map_err(invalid_storage)?;
        self.resources.create_file_resource(
            user,
            ResourceCreateCommand {
                workspace_code: workspace.into(),
                resource_code: resource.clone(),
                kind: if ext == "pdf" {
                    ResourceKind::Pdf
                } else {
                    ResourceKind::File
                },
                name: filename.into(),
                description: "知识库原件".into(),
                allow_inheritance: false,
            },
            Some(ResourceCommitBinary {
                source: temporary.reopen().map_err(invalid_storage)?,
                file_name: filename.into(),
            }),
        )?;
        let (_, stored) = self.resources.download_binary_to_file(
            user,
            ResourceBinaryDownloadCommand {
                workspace_code: workspace.into(),
                resource_code: resource.clone(),
            },
        )?;
        let doc = KnowledgeBaseDocument {
            task_id: None,
            stage: None,
            wait_reason: None,
            id,
            name: filename.into(),
            file_name: filename.into(),
            resource_code: resource,
            resource_revision: stored.revision,
            chunk_config: None,
            status: if request_ingestion { "QUEUED" } else { "UPLOADED" }.into(),
            error: None,
            chunk_count: 0,
            index_revision: 0,
            revision: 0,
            active_source_code: None,
            embedding_model_code: None,
            embedding_dimension: None,
            embedding_provider_model: None,
            embedding_space_fingerprint: None,
            warnings: vec![],
            image_analysis: None,
            parsing_tier: None,
            parsing_artifact: None,
            derived_resource_codes: vec![],
            visual_source: None,
            image_description_artifacts: vec![],
        };
        self.repository.insert_document_with_intent(kb_id, &doc, request_ingestion)?;
        Ok(doc)
    }
    /// Product upload uses the existing per-document index lifecycle; no extra job layer.
    pub async fn upload_and_index(&self, user:&str, workspace:&str, kb:&str, filename:&str, bytes:&[u8]) -> Result<KnowledgeBaseDocument,KnowledgeBaseError> {
        let doc=self.upload_original(user,workspace,kb,filename,bytes,true)?;
        self.index(user,workspace,kb,&doc.id).await
    }
    /// Saving document settings automatically rebuilds only that document.
    pub async fn configure_and_index(&self,user:&str,workspace:&str,kb:&str,doc:&str,config:Option<ChunkConfig>,tier:Option<Option<ParsingTier>>) -> Result<KnowledgeBaseDocument,KnowledgeBaseError> {
        self.configure_parsing(user,workspace,kb,doc,config,tier).await?;
        self.index(user,workspace,kb,doc).await
    }
    async fn chunks(
        &self,
        user: &str,
        workspace: &str,
        kb: &KnowledgeBase,
        doc: &KnowledgeBaseDocument,
        generate_descriptions: bool,
    ) -> Result<(Vec<LocatedChunk>, Vec<String>, KnowledgeBaseDocument), KnowledgeBaseError> {
        let (_, record) = self.resources.download_binary_to_file(
            user,
            ResourceBinaryDownloadCommand {
                workspace_code: workspace.into(),
                resource_code: doc.resource_code.clone(),
            },
        )?;
        if record.revision != doc.resource_revision {
            return Err(KnowledgeBaseError::Conflict);
        }
        let mut bytes = Vec::new();
        self.resources
            .download_binary_storage(&record, &mut bytes)?;
        if let Some(checkpoint) = &self.parse_checkpoint {
            let task = checkpoint.task()?;
            if task.configuration["originalSha256"].as_str() != Some(format!("{:x}", Sha256::digest(&bytes)).as_str()) { return Err(KnowledgeBaseError::Conflict); }
        }
        let (parsed, parsed_doc) = self.parsed_document(user, workspace, kb, doc, &bytes).await?;
        if generate_descriptions && fidelity::blocked(&parsed_doc) { return Err(KnowledgeBaseError::Invalid("现有解析产物已记录失败，未发布索引；原件已保留".into())); }
        let (parsed, mut parsed_doc) = self.supplement_image_descriptions(user, workspace, kb, &parsed_doc, parsed, generate_descriptions).await?;
        parsed_doc.image_analysis = parsed.image_analysis.clone();
        if let Some(checkpoint) = &self.parse_checkpoint { checkpoint.chunking()?; }
        let chunks = split_document(
            &parsed,
            doc.chunk_config
                .as_ref()
                .unwrap_or(&kb.default_chunk_config),
        )
        .map_err(|e| KnowledgeBaseError::Invalid(e.to_string()))?;
        Ok((chunks, parsed.warnings, parsed_doc))
    }
    pub async fn preview(
        &self,
        user: &str,
        workspace: &str,
        kb: &str,
        doc: &str,
    ) -> Result<KnowledgeBasePreview, KnowledgeBaseError> {
        let kb = self.owned(user, workspace, kb)?;
        let doc = self.repository.document(&kb.id, doc)?;
        let (chunks, warnings, parsed_doc) = self.chunks(user, workspace, &kb, &doc, false).await?;
        Ok(KnowledgeBasePreview {
            fidelity_issues: vec![], // Legacy response field; no interactive audit workflow.
            chunk_count: chunks.len(),
            chunks: chunks.into_iter().take(20).collect(),
            warnings,
            image_analysis: parsed_doc.image_analysis,
        })
    }
    pub async fn configure_parsing(&self, user: &str, workspace: &str, kb: &str, doc: &str, config: Option<ChunkConfig>, tier: Option<Option<ParsingTier>>) -> Result<KnowledgeBaseDocument, KnowledgeBaseError> {
        self.owned(user,workspace,kb)?;
        if let Some(Some(selected)) = tier {
            let filename = self.repository.document(kb,doc)?.file_name.to_ascii_lowercase();
            if (filename.ends_with(".docx") || filename.ends_with(".xlsx")) && selected != ParsingTier::Flash { return Err(KnowledgeBaseError::Invalid("Word/Excel仅支持原生解析".into())); }
            if filename.ends_with(".md") {return Err(KnowledgeBaseError::Invalid("Markdown使用直接读取，无需解析质量覆盖".into()));}
            let capabilities=self.parsing_capabilities(user,workspace).await?;
            if !capabilities.available_tiers.contains(&selected) {return Err(KnowledgeBaseError::Invalid("选定解析质量不可用".into()));}
        }
        self.configure_document(user,workspace,kb,doc,config,tier)
    }
    pub fn configure(
        &self,
        user: &str,
        workspace: &str,
        kb: &str,
        doc: &str,
        config: Option<ChunkConfig>,
    ) -> Result<KnowledgeBaseDocument, KnowledgeBaseError> {
        self.configure_document(user,workspace,kb,doc,config,None)
    }
    fn configure_document(&self,user:&str,workspace:&str,kb:&str,doc:&str,config:Option<ChunkConfig>,tier:Option<Option<ParsingTier>>) -> Result<KnowledgeBaseDocument,KnowledgeBaseError> {
        self.owned(user, workspace, kb)?;
        if let Some(c) = &config {
            c.validate()
                .map_err(|e| KnowledgeBaseError::Invalid(e.to_string()))?;
        }
        let mut doc = self.repository.document(kb, doc)?;
        if ingestion_worker::in_progress(&doc.status) {
            return Err(KnowledgeBaseError::Conflict);
        }
        let revision = doc.revision;
        doc.revision += 1;
        doc.chunk_config = config;
        if let Some(tier)=tier { doc.parsing_tier=tier; }
        doc.status = if fidelity::blocked(&doc) {
            "REVIEW_REQUIRED"
        } else if doc.active_source_code.is_some() {
            "NEEDS_REBUILD"
        } else {
            "UPLOADED"
        }
        .into();
        doc.error = fidelity::blocked(&doc).then(|| "数据保真核验发现未解决字形或符号，禁止发布索引；请核对原件及图片".into());
        self.repository.save_document(kb, &doc, revision)?;
        Ok(doc)
    }
    pub fn set_grants(
        &self,
        user: &str,
        workspace: &str,
        kb: &str,
        agents: Vec<String>,
    ) -> Result<KnowledgeBase, KnowledgeBaseError> {
        self.owned(user, workspace, kb)?;
        let valid = self
            .agents
            .list_workspace_internal_agents(workspace)
            .map_err(|e| KnowledgeBaseError::Storage(format!("{e:?}")))?;
        if agents
            .iter()
            .any(|a| !valid.iter().any(|p| p.agent_code == *a))
        {
            return Err(KnowledgeBaseError::Invalid(
                "只支持当前空间的内部 Agent".into(),
            ));
        }
        self.repository.set_grants(kb, &agents)?;
        self.detail(user, workspace, kb)
    }
    pub fn remove_document(&self, user: &str, workspace: &str, kb: &str, doc: &str) -> Result<deletion::DocumentDeletion, KnowledgeBaseError> {
        self.start_ingestion_workers()?;
        self.request_document_deletion(user, workspace, kb, doc)
    }
    pub fn start_index(&self, user: &str, workspace: &str, kb: &str, doc: &str) -> Result<KnowledgeBaseDocument, KnowledgeBaseError> {
        self.enqueue_index(user, workspace, kb, doc)?;
        self.start_ingestion_workers()?;
        self.document_with_task(user, workspace, kb, doc)
    }
    pub fn upload_and_start_index(&self, user: &str, workspace: &str, kb: &str, name: &str, bytes: &[u8]) -> Result<KnowledgeBaseDocument, KnowledgeBaseError> {
        let doc = self.upload_original(user, workspace, kb, name, bytes, true)?;
        let tasks = self.queue.for_document(&doc.id).map_err(ingestion_worker::queue_error)?;
        if tasks.first().is_none() { return self.start_index(user, workspace, kb, &doc.id); }
        self.start_ingestion_workers()?;
        self.document_with_task(user, workspace, kb, &doc.id)
    }
    pub async fn configure_and_start_index(&self, user: &str, workspace: &str, kb: &str, doc: &str, config: Option<ChunkConfig>, tier: Option<Option<ParsingTier>>) -> Result<KnowledgeBaseDocument, KnowledgeBaseError> {
        self.configure_parsing(user, workspace, kb, doc, config, tier).await?;
        self.start_index(user, workspace, kb, doc)
    }
    /// Awaited entrypoint uses exactly the same durable stages as background work.
    pub async fn index(&self, user: &str, workspace: &str, kb: &str, doc: &str) -> Result<KnowledgeBaseDocument, KnowledgeBaseError> {
        let task = self.enqueue_index(user, workspace, kb, doc)?;
        self.run_task_inline(&task.id).await?;
        self.document_with_task(user, workspace, kb, doc)
    }
    fn deactivate(
        &self,
        workspace: &str,
        kb: &str,
        source: &str,
    ) -> Result<(), KnowledgeBaseError> {
        self.rag
            .repository_handle()
            .deactivate_source(RagDeactivateSourceCommand {
                corpus_id: kb.into(),
                source_code: source.into(),
                source_version: "v1".into(),
                workspace_code: workspace.into(),
                scope: RagIngestScope {
                    scope_type: "SPACE".into(),
                    scope_code: workspace.into(),
                },
            })?;
        Ok(())
    }
    fn require_rebuild(
        &self, kb: &str, mut document: KnowledgeBaseDocument, message: &str,
    ) -> Result<(), KnowledgeBaseError> {
        if ingestion_worker::in_progress(&document.status) || document.status == "NEEDS_REBUILD" {
            return Ok(());
        }
        let revision = document.revision;
        document.revision += 1;
        document.status = "NEEDS_REBUILD".into();
        document.error = Some(message.into());
        // Preserve the published pointer and avoid overwriting concurrent document changes.
        match self.repository.save_document(kb, &document, revision) {
            Ok(()) | Err(KnowledgeBaseError::Conflict) => Ok(()),
            Err(error) => Err(error),
        }
    }
    pub async fn search_for_agent(
        &self,
        user: &str,
        workspace: &str,
        agent: &str,
        query: &str,
        token_budget: usize,
    ) -> Result<KnowledgeBaseContext, KnowledgeBaseError> {
        self.search_allowed(user, workspace, agent, query, token_budget, None)
            .await
    }
    pub async fn search_for_agent_in_run(
        &self,
        user: &str,
        workspace: &str,
        agent: &str,
        query: &str,
        token_budget: usize,
        run: &eliteforge_agent_framework::RunContext,
    ) -> Result<KnowledgeBaseContext, KnowledgeBaseError> {
        if token_budget == 0 || query.trim().is_empty() {
            return Ok(KnowledgeBaseContext {
                text: String::new(),
                source_version: String::new(),
                hits: vec![],
            });
        }
        run.check_cancelled().map_err(invalid_storage)?;
        let allowed = self.allowed(user, workspace, agent)?;
        let mut candidates = Vec::new();
        for kb in &allowed {
            for document in kb.documents.iter().filter(|d| d.active_source_code.is_some() && !fidelity::blocked(d)) {
                let (Some(model),Some(dim),Some(stored)) = (&document.embedding_model_code,document.embedding_dimension,&document.embedding_space_fingerprint) else {continue;};
                if dim != TRON_VECTOR_INDEX_DIMENSION {
                    self.require_rebuild(&kb.id, document.clone(), "已发布索引不是1024维，请重建索引")?;
                    continue;
                }
                if let Ok(current) = self.llm.embedding_space_fingerprint(workspace,Some(model),dim) {
                    if &current != stored {
                        self.require_rebuild(&kb.id, document.clone(), "Embedding模型配置与已发布索引不同，请重建索引")?;
                        continue;
                    }
                    if document.status != "NEEDS_REBUILD" {
                        candidates.push((kb.id.clone(),document.clone()));
                    }
                }
            }
        }
        let observed_models: std::collections::BTreeSet<_> = candidates.iter()
            .filter_map(|(_,d)|d.embedding_provider_model.as_ref()).collect();
        let incompatible = observed_models.len() > 1 || candidates.first().is_some_and(|(_,first)| candidates.iter().any(|(_,other)| {
            first.embedding_model_code != other.embedding_model_code
                || first.embedding_dimension != other.embedding_dimension
                || first.embedding_space_fingerprint != other.embedding_space_fingerprint
                || match (&first.embedding_provider_model,&other.embedding_provider_model) {
                    (Some(a),Some(b)) => a != b,
                    _ => false,
                }
        }));
        let space = if incompatible {
            // One query cannot mix spaces. Require rebuild; keep published generations.
            for (kb,document) in candidates {
                self.require_rebuild(&kb, document, "已授权文档的Embedding模型或维度不一致，请重建索引后检索")?;
            }
            None
        } else {
            candidates.first().map(|(_,d)|(d.embedding_model_code.clone().unwrap(),d.embedding_dimension.unwrap()))
        };
        let embedding = if let Some((model, dimensions)) = space {
            match self
                .llm
                .embedding_query_in_run(
                    LlmEmbeddingCallCommand {
                        workspace_code: workspace.into(),
                        model_code: Some(model.clone()),
                        input: query.into(),
                    },
                    dimensions,
                    run,
                )
                .await
            {
                Ok(result) => {
                    if let Some(actual) = &result.provider_model {
                        for kb in self.allowed(user, workspace, agent)? {
                            for document in kb.documents {
                                if document.embedding_model_code.as_ref() == Some(&model)
                                    && document.embedding_provider_model.as_ref().is_some_and(|expected| expected != actual)
                                {
                                    self.require_rebuild(&kb.id, document, "实际Embedding模型与已发布索引不同，请重建索引")?;
                                }
                            }
                        }
                    }
                    Some((model, result.embedding, result.provider_model))
                }
                Err(error) => {
                    let message = error.to_string();
                    if message.contains("does not match index space")
                        || message.contains("invalid embedding index, dimension or value")
                    {
                        for kb in self.allowed(user, workspace, agent)? {
                            for document in kb.documents {
                                if document.embedding_model_code.as_ref() == Some(&model) {
                                    self.require_rebuild(&kb.id, document, "查询Embedding响应与索引维度/数值不一致，请重建索引后重试")?;
                                }
                            }
                        }
                    }
                    None
                },
            }
        } else {
            None
        };
        run.check_cancelled().map_err(invalid_storage)?;
        self.search_allowed(user, workspace, agent, query, token_budget, embedding)
            .await
    }
    fn allowed(
        &self,
        user: &str,
        workspace: &str,
        agent: &str,
    ) -> Result<Vec<KnowledgeBase>, KnowledgeBaseError> {
        self.workspace(user, workspace)?;
        if self
            .agents
            .workspace_internal_agent_policy(workspace, agent)
            .is_err()
        {
            return Ok(vec![]);
        }
        let mut allowed = vec![];
        for kb in self.repository.list(user, workspace)? {
            if self.repository.grants(&kb.id)?.iter().any(|a| a == agent) {
                let detail=self.detail(user,workspace,&kb.id)?;

                allowed.push(detail);
            }
        }
        Ok(allowed)
    }
    async fn search_allowed(
        &self,
        user: &str,
        workspace: &str,
        agent: &str,
        query: &str,
        token_budget: usize,
        embedding: Option<(String, Vec<f32>, Option<String>)>,
    ) -> Result<KnowledgeBaseContext, KnowledgeBaseError> {
        if token_budget == 0 || query.trim().is_empty() {
            return Ok(KnowledgeBaseContext {
                text: String::new(),
                source_version: String::new(),
                hits: vec![],
            });
        }
        let mut hits = vec![];
        let mut versions = vec![];
        for kb in self.allowed(user, workspace, agent)? {
            let sources: Vec<String> = kb
                .documents
                .iter()
                .filter(|d|!fidelity::blocked(d)).filter_map(|d| d.active_source_code.clone())
                .collect();
            if sources.is_empty() {
                continue;
            }
            let mut identities = kb
                .documents
                .iter()
                .map(|d| {
                    format!(
                        "{}:{}:{}:{}:{}:{}",
                        d.id,
                        d.resource_revision,
                        d.active_source_code.as_deref().unwrap_or(""),
                        d.index_revision,
                        d.revision,
                        d.status
                    )
                })
                .collect::<Vec<_>>();
            identities.sort();
            versions.push(format!(
                "{}:{}:{}",
                kb.id,
                self.repository.acl_revision(&kb.id)?,
                identities.join("|")
            ));
            let vector = embedding.as_ref().filter(|(model, values, provider_model)| {
                if values.len() != TRON_VECTOR_INDEX_DIMENSION {
                    return false;
                }
                kb.documents
                    .iter()
                    .filter(|d| d.active_source_code.is_some() && !fidelity::blocked(d))
                    .all(|d| {
                        d.status != "NEEDS_REBUILD"
                            && d.embedding_model_code.as_ref() == Some(model)
                            && d.embedding_dimension == Some(values.len())
                            && match (&d.embedding_provider_model, provider_model) {
                                (Some(expected), Some(actual)) => expected == actual,
                                _ => true, // Unknown identity stays unknown under the confirmed single-model deployment.
                            }
                            && d.embedding_space_fingerprint
                                .as_ref()
                                .is_some_and(|stored| {
                                    self.llm
                                        .embedding_space_fingerprint(
                                            workspace,
                                            Some(model),
                                            values.len(),
                                        )
                                        .is_ok_and(|current| current == *stored)
                                })
                    })
            });
            let routes = if vector.is_some() {
                vec!["VECTOR_SQLITE_COSINE_V1", "FULLTEXT_SQLITE_LIKE_V1"]
            } else {
                vec!["FULLTEXT_SQLITE_LIKE_V1"]
            };
            for retriever in routes {
                let route_result = self
                    .rag
                    .repository_handle()
                    .search_route(RagRouteSearchQuery {
                        corpus_id: kb.id.clone(),
                        route_id: "knowledge_base".into(),
                        retriever: retriever.into(),
                        workspace_code: workspace.into(),
                        query: query.into(),
                        query_embedding: vector.map(|(_, v, _)| v.clone()),
                        query_embedding_model_code: vector.map(|(m, _, _)| m.clone()),
                        source_version: "v1".into(),
                        scopes: vec![RagSearchScope {
                            scope_type: "SPACE".into(),
                            scope_code: workspace.into(),
                        }],
                        filters: json!({"sourceCode":{"op":"in","value":sources}}),
                        policy: json!({}),
                        return_fields: vec![],
                        limit: 8,
                    });
                let rows = match route_result {
                    Ok(rows) => rows,
                    Err(_) if retriever.contains("VECTOR") => continue,
                    Err(error) => return Err(error.into()),
                };
                for row in rows {
                    let Some(doc) = kb
                        .documents
                        .iter()
                        .find(|d| !fidelity::blocked(d) && d.active_source_code.as_ref() == Some(&row.source_code))
                    else {
                        continue;
                    };
                    if hits
                        .iter()
                        .any(|h: &KnowledgeBaseHit| h.chunk_code == row.target_code)
                    {
                        continue;
                    }
                    hits.push(KnowledgeBaseHit {
                        knowledge_base_id: kb.id.clone(),
                        document_id: doc.id.clone(),
                        resource_code: doc.resource_code.clone(),
                        resource_revision: doc.resource_revision,
                        source_code: row.source_code,
                        chunk_code: row.target_code,
                        title: doc.name.clone(),
                        text: row.text,
                        location: {
                            let mut location=row.text_metadata.get("location").cloned().unwrap_or(Value::Null);
                            let name=doc.file_name.to_ascii_lowercase();
                            if (name.ends_with(".docx") || name.ends_with(".xlsx")) && let Some(object)=location.as_object_mut() { object.remove("page"); }
                            location
                        },
                        score: row.storage_score.unwrap_or(0.0),
                    });
                }
            }
        }
        let currently_allowed = self.allowed(user, workspace, agent)?;
        hits.retain(|hit| {
            currently_allowed.iter().any(|kb| {
                kb.id == hit.knowledge_base_id
                    && kb
                        .documents
                        .iter()
                        .any(|d| !fidelity::blocked(d) && d.active_source_code.as_ref() == Some(&hit.source_code))
            })
        });
        let mut remaining = token_budget.saturating_mul(4);
        let mut selected = vec![];
        let mut text = String::new();
        for mut hit in hits.into_iter().take(8) {
            if remaining < 32 {
                break;
            }
            let location = compact_location(&hit.location);
            let title: String = hit.title.chars().take(32).collect();
            let reference = format!(
                "[文档:{title} ID:{}{}]",
                hit.document_id,
                if location.is_empty() {
                    String::new()
                } else {
                    format!(" 位置:{location}")
                }
            );
            // Keep evidence first: the SDK may apply a stricter token budget than
            // our conservative character estimate. Prefix clipping must retain
            // document content, while full citation metadata remains in hits.
            let overhead = reference.chars().count() + 3;
            let body_budget = if remaining > overhead + 32 {
                remaining - overhead
            } else {
                remaining.saturating_sub(2)
            };
            hit.text = hit.text.chars().take(body_budget).collect();
            if hit.text.is_empty() {
                continue;
            }
            let body_len = hit.text.chars().count();
            text.push_str(&hit.text);
            remaining = remaining.saturating_sub(body_len);
            if remaining >= overhead {
                text.push('\n');
                text.push_str(&reference);
                remaining = remaining.saturating_sub(overhead - 2);
            }
            text.push_str("\n\n");
            remaining = remaining.saturating_sub(2);
            selected.push(hit);
        }
        Ok(KnowledgeBaseContext {
            text,
            source_version: {
                versions.sort();
                hash(&format!(
                    "{user}:{workspace}:{agent}:{}",
                    versions.join("|")
                ))
            },
            hits: selected,
        })
    }
}
fn compact_location(location: &Value) -> String {
    let mut parts = Vec::new();
    if let Some(page) = location.get("page").and_then(Value::as_u64) {
        parts.push(format!("页{page}"));
    }
    if let Some(page) = location.get("renderedPage").and_then(Value::as_u64) { parts.push(format!("转换预览{page}（非原文页）")); }
    if let Some(sheet) = location.get("sheet").and_then(Value::as_str) {
        parts.push(sheet.chars().take(24).collect::<String>());
    }
    for (start_key, end_key, label) in [
        ("rowStart", "rowEnd", "行"),
        ("paragraph", "paragraphEnd", "段"),
        ("lineStart", "lineEnd", "行"),
    ] {
        if let Some(start) = location.get(start_key).and_then(Value::as_u64) {
            let end = location.get(end_key).and_then(Value::as_u64);
            parts.push(match end {
                Some(end) if end != start => format!("{label}{start}-{end}"),
                _ => format!("{label}{start}"),
            });
        }
    }
    if let Some(cells) = location.get("cellRanges").and_then(Value::as_array) {
        let values: Vec<_> = cells.iter().filter_map(Value::as_str).collect();
        if values.len() <= 4 {
            parts.extend(values.into_iter().map(str::to_owned));
        } else {
            // Do not imply the first few cells cover all evidence in a merged chunk.
            parts.push(format!("{}个单元格；完整地址见来源", values.len()));
        }
    }
    if parts.is_empty() {
        if let Some(blocks) = location.get("blocks").and_then(Value::as_array) { parts.extend(blocks.iter().filter_map(Value::as_str).take(2).map(|b|format!("块{b}"))); }
    }
    if let Some(heading) = location.get("heading").and_then(Value::as_str)
        && !heading.is_empty()
    {
        parts.push(heading.chars().take(24).collect());
    }
    parts.join(" ").chars().take(64).collect()
}
fn hash(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}
fn invalid_storage(error: impl std::fmt::Display) -> KnowledgeBaseError {
    KnowledgeBaseError::Storage(error.to_string())
}

fn new_id() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
