#![allow(dead_code)]
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};
use tron_backend::{
    adapters::outbound::persistence::sqlite::SqliteKnowledgeBaseRepository,
    app::AppState,
    application::{
        knowledge_base::KnowledgeBaseService,
        llm::{LlmRuntimeClient, LlmRuntimeModelResolver, LlmRuntimeRerankHit, LlmRuntimeService},
        model::RuntimeModelConfig,
        profile::ProfileRegistryService,
    },
    config::{AppConfig, ResourceConfig},
    domain::model::{ModelError, ModelProtocol, ModelType},
    shared::id::FixedSnowflakeIdProvider,
};
pub const USER: &str = "USR-AUTH-0001";
pub const WORKSPACE: &str = "TRON_CORE";
pub struct Fixture {
    pub directory: tempfile::TempDir,
    pub state: AppState,
    pub service: KnowledgeBaseService,
    pub client: FakeEmbedding,
    pub agent: String,
}
impl Fixture {
    pub fn new() -> Self {
        Self::with_resolver(Arc::new(EmbeddingResolver))
    }
    pub fn with_resolver(resolver: Arc<dyn LlmRuntimeModelResolver>) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let mut config = AppConfig::default();
        config.database_url = directory.path().join("kb.sqlite").display().to_string();
        config.graph_storage_path = directory.path().join("kb.graph").display().to_string();
        config.resource =
            ResourceConfig::with_default_root(directory.path().join("files").display().to_string());
        let config = crate::common::prepared_config(config);
        let profiles = ProfileRegistryService::new(":memory:").unwrap();
        profiles
            .register_or_update(USER, "profile.operator")
            .unwrap();
        profiles
            .register_or_update("OTHER-USER", "other.operator")
            .unwrap();
        let owner = profiles
            .list()
            .unwrap()
            .into_iter()
            .find(|p| p.person_ai_user_id == USER)
            .unwrap();
        profiles.activate(&owner.profile_id).unwrap();
        let state = AppState::new_with_profiles(
            config,
            Arc::new(FixedSnowflakeIdProvider::new(789000000000000000)),
            profiles,
        )
        .unwrap();
        state.knowledge_bases.stop_ingestion_workers();
        let agent = state
            .agent_management
            .list_workspace_internal_agents(WORKSPACE)
            .unwrap().into_iter().find(|agent| agent.agent_code == "main-agent").unwrap()
            .agent_code
            .clone();
        let client = FakeEmbedding::default();
        let llm = LlmRuntimeService::new(resolver, Arc::new(client.clone()));
        let service = KnowledgeBaseService::new(
            SqliteKnowledgeBaseRepository::connect(&state.config.database_url).unwrap(),
            state.rag.with_runtime(tron_backend::application::rag::DefaultRagRuntime::new(llm.clone(),FixtureGraphRouter)),
            state.resources.clone(),
            state.agent_management.clone(),
            llm,
        )
        .with_document_parser(Arc::new(FixtureDocumentParser));
        Self {
            directory,
            state,
            service,
            client,
            agent,
        }
    }
}
#[derive(Clone)]
pub struct FakeEmbedding {
    pub failing: Arc<AtomicBool>,
    pub blocking: Arc<AtomicBool>,
    pub block_after_calls: Arc<AtomicUsize>,
    pub calls: Arc<AtomicUsize>,
    pub provider_model: Arc<Mutex<Option<String>>>,
    pub dimensions: Arc<AtomicUsize>,
    pub dimension_sequence: Arc<Mutex<std::collections::VecDeque<usize>>>,
}
impl Default for FakeEmbedding {
    fn default() -> Self {
        Self {
            failing: Default::default(),
            blocking: Default::default(),
            block_after_calls: Arc::new(AtomicUsize::new(usize::MAX)),
            calls: Default::default(),
            provider_model: Default::default(),
            dimensions: Arc::new(AtomicUsize::new(1024)),
            dimension_sequence: Default::default(),
        }
    }
}
impl FakeEmbedding {
    fn vector(&self) -> Result<Vec<f32>, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.vector_value()
    }
    fn vector_value(&self) -> Result<Vec<f32>, String> {
        if self.failing.load(Ordering::SeqCst) {
            return Err("synthetic embedding outage".into());
        }
        let dimension = self
            .dimension_sequence
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| self.dimensions.load(Ordering::SeqCst));
        let mut v = vec![0.0; dimension];
        if !v.is_empty() {
            v[0] = 1.0;
        }
        Ok(v)
    }
}
impl LlmRuntimeClient for FakeEmbedding {
    fn model_authorization_scope(&self, _: &RuntimeModelConfig) -> Result<Option<String>, String> { Ok(Some("synthetic-fixture-account".into())) }
    fn generate_vision<'a>(&'a self, _: &'a RuntimeModelConfig, _: &'a str, mime: &'a str, bytes: &'a [u8]) -> Pin<Box<dyn Future<Output=Result<String,String>>+Send+'a>> {
        Box::pin(async move { assert!(mime.starts_with("image/")); assert!(!bytes.is_empty()); Ok(r#"{"description":"Synthetic diagram evidence","visibleText":[],"relations":[]}"#.into()) })
    }

    fn embed_batch_with_model<'a>(
        &'a self,
        _: &'a RuntimeModelConfig,
        inputs: &'a [String],
        _: usize,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        (
                            Vec<eliteforge_agent_framework::EmbeddingVector>,
                            Option<String>,
                        ),
                        String,
                    >,
                > + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            if self.blocking.load(Ordering::SeqCst)
                || self.calls.load(Ordering::SeqCst)
                    >= self.block_after_calls.load(Ordering::SeqCst)
            {
                std::future::pending::<()>().await;
            }
            self.calls.fetch_add(1, Ordering::SeqCst);
            let vectors = inputs
                .iter()
                .enumerate()
                .map(|(index, _)| {
                    self.vector_value()
                        .map(|values| eliteforge_agent_framework::EmbeddingVector { index, values })
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok((vectors, self.provider_model.lock().unwrap().clone()))
        })
    }
    fn embed_with_model<'a>(
        &'a self,
        _: &'a RuntimeModelConfig,
        _: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(Vec<f32>, Option<String>), String>> + Send + 'a>> {
        Box::pin(async move {
            if self.blocking.load(Ordering::SeqCst)
                || self.calls.load(Ordering::SeqCst)
                    >= self.block_after_calls.load(Ordering::SeqCst)
            {
                std::future::pending::<()>().await;
            }
            self.vector()
                .map(|vector| (vector, self.provider_model.lock().unwrap().clone()))
        })
    }
    fn embed_query_with_model_in_run<'a>(
        &'a self,
        _: &'a RuntimeModelConfig,
        _: &'a str,
        _: usize,
        _: &'a eliteforge_agent_core::RunContext,
    ) -> Pin<Box<dyn Future<Output = Result<(Vec<f32>, Option<String>), String>> + Send + 'a>> {
        Box::pin(async move {
            self.vector()
                .map(|vector| (vector, self.provider_model.lock().unwrap().clone()))
        })
    }
    fn generate<'a>(
        &'a self,
        _: &'a RuntimeModelConfig,
        prompt: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move {if prompt.contains("从下述正文抽取实体和关系") {
            if prompt.contains("Alpha calls Beta.") { Ok(r#"{"nodes":[{"ref":"alpha","name":"Alpha","type":"Service","evidence":"Alpha calls Beta."},{"ref":"beta","name":"Beta","type":"Service","evidence":"Alpha calls Beta."}],"edges":[{"from":"alpha","to":"beta","relationship":"CALLS","evidence":"Alpha calls Beta."}]}"#.into()) }
            else { Ok("{\"nodes\":[],\"edges\":[]}".into()) }
        } else {Err("unused".into())}})
    }
    fn embed<'a>(
        &'a self,
        _: &'a RuntimeModelConfig,
        _: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<f32>, String>> + Send + 'a>> {
        Box::pin(async { self.vector() })
    }
    fn embed_query_in_run<'a>(
        &'a self,
        _: &'a RuntimeModelConfig,
        _: &'a str,
        _: usize,
        _: &'a eliteforge_agent_core::RunContext,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<f32>, String>> + Send + 'a>> {
        Box::pin(async { self.vector() })
    }
    fn rerank<'a>(
        &'a self,
        _: &'a RuntimeModelConfig,
        _: &'a str,
        _: &'a [String],
        _: Option<usize>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<LlmRuntimeRerankHit>, String>> + Send + 'a>> {
        Box::pin(async { Ok(vec![]) })
    }
}
pub struct EmbeddingResolver;
impl LlmRuntimeModelResolver for EmbeddingResolver {
    fn auto_vision_runtime_model_config<'a>(&'a self, workspace: &'a str) -> Pin<Box<dyn Future<Output=Result<Option<RuntimeModelConfig>, ModelError>>+Send+'a>> {
        Box::pin(async move { self.vision_runtime_model_config(workspace, "PERSON_AI:FIXED:High:synthetic-vision") })
    }
    fn vision_runtime_model_config(&self, workspace: &str, code: &str) -> Result<Option<RuntimeModelConfig>, ModelError> {
        let mut config=self.embedding_runtime_model_config(workspace, Some(code))?.unwrap(); config.model_type=ModelType::Text; Ok(Some(config))
    }

    fn embedding_runtime_model_config(
        &self,
        _: &str,
        code: Option<&str>,
    ) -> Result<Option<RuntimeModelConfig>, ModelError> {
        Ok(Some(RuntimeModelConfig {
            supports_developer_role: false,
            model_code: code
                .filter(|code| !matches!(*code,"PERSON_AI:ROUTE:EmbeddingAuto"|"PERSON_AI:ROUTE:MultimodalEmbeddingAuto"))
                .unwrap_or("fixture-embedding")
                .into(),
            protocol: ModelProtocol::OpenAiCompatible,
            model_type: ModelType::Embedding,
            base_url: "http://fixture.invalid/v1".into(),
            api_path: None,
            api_key: "synthetic-key".into(),
            model_name: "fixture-embedding".into(),
            thinking_enabled: false,
            reasoning_effort: None,
            responses_stateful_enabled: false,
            temperature: None,
            max_tokens: None,
            context_window_tokens: None,
            top_p: None,
            max_retries: Some(0),
        }))
    }
    fn rerank_runtime_model_config(
        &self,
        _: &str,
        _: Option<&str>,
    ) -> Result<Option<RuntimeModelConfig>, ModelError> {
        Ok(None)
    }
}
pub struct FixtureGraphRouter;
impl tron_backend::application::model_router::TextModelRouter for FixtureGraphRouter {
    fn route_text(&self,workspace:&str,_:Option<&str>)->Result<RuntimeModelConfig,String>{
        let mut model=EmbeddingResolver.embedding_runtime_model_config(workspace,Some("fixture-graph")).map_err(|e|e.to_string())?.unwrap();
        model.model_type=ModelType::Text;Ok(model)
    }
}
pub fn office() -> Vec<u8> {
    use std::io::{Cursor, Write};
    use zip::{ZipWriter, write::SimpleFileOptions};
    let mut z = ZipWriter::new(Cursor::new(vec![]));
    z.start_file("word/document.xml", SimpleFileOptions::default())
        .unwrap();
    z.write_all(br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>orchid document evidence</w:t></w:r></w:p></w:body></w:document>"#).unwrap();
    z.finish().unwrap().into_inner()
}
pub fn spreadsheet() -> Vec<u8> {
    let mut book = umya_spreadsheet::new_file();
    book.sheet_by_name_mut("Sheet1")
        .unwrap()
        .cell_mut("A1")
        .set_value("orchid spreadsheet evidence");
    let mut out = vec![];
    umya_spreadsheet::writer::xlsx::write_writer(&book, &mut out).unwrap();
    out
}
pub fn pdf() -> Vec<u8> {
    use pdf_extract::{
        Document, Object, Stream,
        content::{Content, Operation},
        dictionary,
    };
    let mut d = Document::with_version("1.5");
    let pages = d.new_object_id();
    let font =
        d.add_object(dictionary! {"Type"=>"Font","Subtype"=>"Type1","BaseFont"=>"Helvetica"});
    let resources = d.add_object(dictionary! {"Font"=>dictionary!{"F1"=>font}});
    let content = Content {
        operations: vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec!["F1".into(), 12.into()]),
            Operation::new("Td", vec![10.into(), 100.into()]),
            Operation::new("Tj", vec![Object::string_literal("orchid PDF evidence")]),
            Operation::new("ET", vec![]),
        ],
    };
    let stream = d.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
    let page=d.add_object(dictionary!{"Type"=>"Page","Parent"=>pages,"Contents"=>stream,"Resources"=>resources,"MediaBox"=>vec![0.into(),0.into(),300.into(),300.into()]});
    d.objects.insert(
        pages,
        dictionary! {"Type"=>"Pages","Kids"=>vec![page.into()],"Count"=>1}.into(),
    );
    let catalog = d.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages});
    d.trailer.set("Root", catalog);
    let mut out = vec![];
    d.save_to(&mut out).unwrap();
    out
}

// Baseline product tests isolate extraction behind the application port. Production
// uses Person-AI; provider result handling is covered separately.
pub struct FixtureDocumentParser;
#[async_trait::async_trait]
impl tron_backend::application::knowledge_base::parsing::KnowledgeDocumentParser
    for FixtureDocumentParser
{
    fn default_tier(&self) -> tron_backend::application::knowledge_base::parsing::ParsingTier {
        Default::default()
    }
    fn with_ocr_mode(
        &self,
        _: tron_backend::application::knowledge_base::parsing::OcrMode,
    ) -> Result<
        Arc<dyn tron_backend::application::knowledge_base::parsing::KnowledgeDocumentParser>,
        tron_backend::application::knowledge_base::KnowledgeBaseError,
    > {
        Ok(Arc::new(FixtureDocumentParser))
    }
    async fn capabilities(
        &self,
    ) -> Result<
        tron_backend::application::knowledge_base::parsing::ParsingCapabilities,
        tron_backend::application::knowledge_base::KnowledgeBaseError,
    > {
        use tron_backend::application::knowledge_base::parsing::*;
        Ok(ParsingCapabilities {
            default_ocr_mode: OcrMode::Auto,
            default_tier: ParsingTier::Standard,
            available_tiers: vec![
                ParsingTier::Flash,
                ParsingTier::Basic,
                ParsingTier::Standard,
            ],
            parser_version: "4.0.10-fixture".into(),
            office_tier: ParsingTier::Flash,
        })
    }
    async fn parse(
        &self,
        filename: &str,
        bytes: &[u8],
        tier: Option<tron_backend::application::knowledge_base::parsing::ParsingTier>,
    ) -> Result<
        tron_backend::application::knowledge_base::parsing::StructuredParse,
        tron_backend::application::knowledge_base::KnowledgeBaseError,
    > {
        use tron_backend::application::knowledge_base::{
            local_documents::parse_local_document, parsing::*,
        };
        let document = parse_local_document(filename, bytes).map_err(|e| {
            tron_backend::application::knowledge_base::KnowledgeBaseError::Invalid(e.to_string())
        })?;
        // The cache reloads the raw structured parse, so fixtures must obey the same
        // middle-v2 contract as the production parser rather than supply an empty object.
        let mut pages = std::collections::BTreeMap::<u32, Vec<serde_json::Value>>::new();
        for (index, section) in document.sections.iter().enumerate() {
            pages.entry(section.location.page.map(|p|p.saturating_sub(1)).unwrap_or(0)).or_default().push(serde_json::json!({"index":index,"type":if section.heading_level.is_some(){"paragraph_title"}else{"text"},"level":section.heading_level,"content":section.text}));
        }
        let middle = serde_json::json!({"metadata":{"file_suffix":document.format},"pages":pages.into_iter().map(|(page,blocks)|serde_json::json!({"page_idx":page,"blocks":blocks})).collect::<Vec<_>>()});
        Ok(StructuredParse {
            document,
            middle,
            structured: serde_json::json!({}),
            markdown: String::new(),
            assets: vec![],
            effective_tier: if filename.ends_with(".pdf") {
                tier.unwrap_or_default()
            } else {
                ParsingTier::Flash
            },
            parser_version: "4.0.10-fixture".into(),
            vendor_job_id: "fixture".into(),
        })
    }
}
