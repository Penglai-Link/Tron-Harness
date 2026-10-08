//! Synthetic fixture replays and mock models only. No provider credentials or network inference.
mod common;
#[path = "support/knowledge_base_fixture.rs"]
mod fixture;
use fixture::*;
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};
use tron_backend::{
    adapters::outbound::{
        mineru_documents::decode_archive, persistence::sqlite::SqliteKnowledgeBaseRepository,
    },
    application::{
        knowledge_base::{KnowledgeBaseError, KnowledgeBaseService, parsing::*},
        llm::{LlmRuntimeClient, LlmRuntimeModelResolver, LlmRuntimeRerankHit, LlmRuntimeService},
        model::RuntimeModelConfig,
        resource::ResourceBinaryDownloadCommand,
    },
    config::ImageDescriptionConfig,
    domain::model::{ModelError, ModelType},
};
const MODEL: &str = "PERSON_AI:FIXED:High:synthetic-vision";
const DOCX: &[u8] = include_bytes!("fixtures/mineru-v1/synthetic.docx.zip");
#[derive(Clone)]
struct Vision {
    embed: FakeEmbedding,
    calls: Arc<AtomicUsize>,
    fail: Arc<AtomicBool>,
    block_at: Arc<AtomicUsize>,
    scope: Arc<Mutex<String>>,
    flip_scope: Arc<AtomicBool>,
    bad_json: Arc<AtomicBool>,
    inputs: Arc<Mutex<Vec<String>>>,
    reached: Arc<tokio::sync::Notify>,
}
impl Vision {
    fn new(embed: FakeEmbedding) -> Self {
        Self {
            embed,
            calls: Default::default(),
            fail: Default::default(),
            block_at: Default::default(),
            scope: Arc::new(Mutex::new("synthetic-account-a".into())),
            flip_scope: Default::default(),
            bad_json: Default::default(),
            inputs: Default::default(),
            reached: Default::default(),
        }
    }
}
impl LlmRuntimeClient for Vision {
    fn model_authorization_scope(&self, _: &RuntimeModelConfig) -> Result<Option<String>, String> {
        Ok(Some(self.scope.lock().unwrap().clone()))
    }
    fn generate_vision<'a>(
        &'a self,
        config: &'a RuntimeModelConfig,
        prompt: &'a str,
        mime: &'a str,
        bytes: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move {
            assert_eq!(config.model_code, MODEL);
            assert_eq!(config.model_type, ModelType::Text);
            assert_eq!(mime, "image/jpeg");
            assert!(bytes.starts_with(&[0xff, 0xd8, 0xff]));
            assert!(prompt.contains("untrusted"));
            let call = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
            self.reached.notify_waiters();
            if self.block_at.load(Ordering::SeqCst) == call {
                std::future::pending::<()>().await;
            }
            if self.fail.load(Ordering::SeqCst) {
                return Err("synthetic outage".into());
            }
            if self.flip_scope.load(Ordering::SeqCst) {
                *self.scope.lock().unwrap() = "synthetic-account-b".into();
            }
            if self.bad_json.load(Ordering::SeqCst) {
                return Ok("truncated JSON".into());
            }
            Ok(r#"{"description":"Synthetic cobalt-orbit diagram shows UPLOAD leading to INDEX","visibleText":"UPLOAD INDEX","relations":["UPLOAD -> INDEX"]}"#.into())
        })
    }
    fn embed_batch_with_model<'a>(
        &'a self,
        c: &'a RuntimeModelConfig,
        inputs: &'a [String],
        dimensions: usize,
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
        self.inputs.lock().unwrap().extend(inputs.iter().cloned());
        self.embed.embed_batch_with_model(c, inputs, dimensions)
    }
    fn embed_with_model<'a>(
        &'a self,
        c: &'a RuntimeModelConfig,
        input: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(Vec<f32>, Option<String>), String>> + Send + 'a>> {
        self.inputs.lock().unwrap().push(input.into());
        self.embed.embed_with_model(c, input)
    }
    fn generate<'a>(
        &'a self,
        c: &'a RuntimeModelConfig,
        p: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        self.embed.generate(c, p)
    }
    fn embed<'a>(
        &'a self,
        c: &'a RuntimeModelConfig,
        p: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<f32>, String>> + Send + 'a>> {
        self.embed.embed(c, p)
    }
    fn rerank<'a>(
        &'a self,
        c: &'a RuntimeModelConfig,
        q: &'a str,
        d: &'a [String],
        n: Option<usize>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<LlmRuntimeRerankHit>, String>> + Send + 'a>> {
        self.embed.rerank(c, q, d, n)
    }
}
struct Resolver;
impl LlmRuntimeModelResolver for Resolver {
    fn vision_runtime_model_config(
        &self,
        w: &str,
        m: &str,
    ) -> Result<Option<RuntimeModelConfig>, ModelError> {
        let mut c = EmbeddingResolver
            .embedding_runtime_model_config(w, None)?
            .unwrap();
        c.model_code = m.into();
        c.model_name = "synthetic-vision".into();
        c.model_type = ModelType::Text;
        c.protocol = tron_backend::domain::model::ModelProtocol::OpenAiCompatible;
        Ok(Some(c))
    }
    fn embedding_runtime_model_config(
        &self,
        w: &str,
        m: Option<&str>,
    ) -> Result<Option<RuntimeModelConfig>, ModelError> {
        EmbeddingResolver.embedding_runtime_model_config(w, m)
    }
    fn rerank_runtime_model_config(
        &self,
        w: &str,
        m: Option<&str>,
    ) -> Result<Option<RuntimeModelConfig>, ModelError> {
        EmbeddingResolver.rerank_runtime_model_config(w, m)
    }
}
struct Replay {
    calls: Arc<AtomicUsize>,
    archive: Vec<u8>,
}
#[async_trait::async_trait]
impl KnowledgeDocumentParser for Replay {
    fn default_tier(&self) -> ParsingTier {
        ParsingTier::Standard
    }
    async fn capabilities(&self) -> Result<ParsingCapabilities, KnowledgeBaseError> {
        Ok(ParsingCapabilities {
            default_ocr_mode: tron_backend::application::knowledge_base::parsing::OcrMode::Auto,
            default_tier: ParsingTier::Standard,
            available_tiers: vec![ParsingTier::Flash, ParsingTier::Standard],
            parser_version: "4.0.10".into(),
            office_tier: ParsingTier::Flash,
        })
    }
    async fn parse(
        &self,
        _: &str,
        _: &[u8],
        _: Option<ParsingTier>,
    ) -> Result<StructuredParse, KnowledgeBaseError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        decode_archive(
            &self.archive,
            ParsingTier::Flash,
            "4.0.10",
            "synthetic-replay",
        )
    }
}
fn service(
    f: &Fixture,
    v: &Vision,
    enabled: bool,
    archive: Vec<u8>,
) -> (KnowledgeBaseService, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let s = KnowledgeBaseService::new(
        SqliteKnowledgeBaseRepository::connect(&f.state.config.database_url).unwrap(),
        tron_backend::application::rag::RagService::new(
            Arc::new(std::sync::Mutex::new(tron_backend::adapters::outbound::persistence::sqlite::SqliteRagRepository::connect(&f.state.config.database_url).unwrap())),
            tron_backend::application::rag::DefaultRagRuntime::new(LlmRuntimeService::new(Arc::new(Resolver), Arc::new(v.clone())), f.state.llm_router.clone()),
        ),
        f.state.resources.clone(),
        f.state.agent_management.clone(),
        LlmRuntimeService::new(Arc::new(Resolver), Arc::new(v.clone())),
    )
    .with_document_parser(Arc::new(Replay {
        calls: calls.clone(),
        archive,
    }))
    .with_image_descriptions(ImageDescriptionConfig {
        enabled,
        model_code: MODEL.into(),
        max_images_per_document: 64,
        timeout_seconds: 2,
    });
    (s, calls)
}
fn two_images() -> Vec<u8> {
    use std::io::{Cursor, Read, Write};
    let mut input = zip::ZipArchive::new(Cursor::new(DOCX)).unwrap();
    let mut files = Vec::new();
    for i in 0..input.len() {
        let mut f = input.by_index(i).unwrap();
        let name = f.name().to_owned();
        let mut b = vec![];
        f.read_to_end(&mut b).unwrap();
        files.push((name, b));
    }
    let image = files
        .iter()
        .find(|(n, _)| n.starts_with("images/"))
        .unwrap()
        .1
        .clone();
    let middle = files
        .iter_mut()
        .find(|(n, _)| n == "middle_json.json")
        .unwrap();
    let mut d: serde_json::Value = serde_json::from_slice(&middle.1).unwrap();
    let blocks = d["pages"][0]["blocks"].as_array_mut().unwrap();
    let mut second = blocks
        .iter()
        .find(|b| b["type"] == "image")
        .unwrap()
        .clone();
    second["index"] = serde_json::json!(10);
    second["content"][0]["index"] = serde_json::json!(10);
    second["content"][0]["image_path"] = serde_json::json!("images/second.jpg");
    blocks.push(second);
    middle.1 = serde_json::to_vec(&d).unwrap();
    files.push(("images/second.jpg".into(), image));
    let mut output = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (n, b) in files {
        output
            .start_file(n, zip::write::SimpleFileOptions::default())
            .unwrap();
        output.write_all(&b).unwrap();
    }
    output.finish().unwrap().into_inner()
}
#[tokio::test]
async fn default_off_preview_never_calls_vision_and_enabled_index_is_acl_scoped() {
    let f = Fixture::new();
    let v = Vision::new(f.client.clone());
    let (s, parses) = service(&f, &v, false, DOCX.to_vec());
    let kb = s.create(USER, "Images", "synthetic only").unwrap();
    let doc = s
        .upload(
            USER,
            &kb.id,
            "synthetic.docx",
            include_bytes!("fixtures/mineru-v1/synthetic.docx"),
        )
        .unwrap();
    let ready = s.index(USER, &kb.id, &doc.id).await.unwrap();
    assert_eq!(ready.status, "READY");
    assert_eq!(v.calls.load(Ordering::SeqCst), 0);
    let (s, _) = service(&f, &v, true, DOCX.to_vec());
    let before = s.preview(USER, &kb.id, &doc.id).await.unwrap();
    assert_eq!(
        before.image_analysis.unwrap().supplement_status.as_deref(),
        Some("pending")
    );
    assert_eq!(v.calls.load(Ordering::SeqCst), 0);
    let ready = s.index(USER, &kb.id, &doc.id).await.unwrap();
    assert_eq!(ready.status, "READY");
    assert_eq!(
        ready
            .image_analysis
            .as_ref()
            .unwrap()
            .with_generated_descriptions,
        1
    );
    assert_eq!(ready.image_description_artifacts.len(), 1);
    assert!(
        v.inputs
            .lock()
            .unwrap()
            .iter()
            .any(|s| s.contains("cobalt-orbit"))
    );
    let artifact = &ready.image_description_artifacts[0];
    let command = || ResourceBinaryDownloadCommand {
        workspace_code: WORKSPACE.into(),
        resource_code: artifact.resource_code.clone(),
    };
    assert!(
        f.state
            .resources
            .for_agent(Some(&f.agent))
            .download_binary_to_file(USER, command())
            .is_err()
    );
    assert!(
        s.search_for_agent(USER, WORKSPACE, &f.agent, "cobalt-orbit", 1200)
            .await
            .unwrap()
            .hits
            .is_empty()
    );
    s.set_grants(USER, &kb.id, vec![f.agent.clone()]).unwrap();
    let hits = s
        .search_for_agent(USER, WORKSPACE, &f.agent, "cobalt-orbit", 1200)
        .await
        .unwrap()
        .hits;
    assert!(!hits.is_empty());
    assert!(
        hits[0].location["imageAnnotations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["source"] == "framework.sdk.kb-image-description-v1"
                && a["modelCode"] == MODEL)
    );
    assert!(
        f.state
            .resources
            .for_agent(Some(&f.agent))
            .download_binary_to_file(USER, command())
            .is_ok()
    );
    assert!(
        f.state
            .resources
            .for_agent(Some(&f.agent))
            .download_binary_to_file("OTHER-USER", command())
            .is_err()
    );
    assert!(
        f.state
            .resources
            .for_agent(None)
            .download_binary_to_file(USER, command())
            .is_err()
    );
    let cached = s.preview(USER, &kb.id, &doc.id).await.unwrap();
    assert!(
        cached
            .chunks
            .iter()
            .any(|c| c.text.contains("cobalt-orbit"))
    );
    assert_eq!(v.calls.load(Ordering::SeqCst), 1);
    s.index(USER, &kb.id, &doc.id).await.unwrap();
    assert_eq!(v.calls.load(Ordering::SeqCst), 1);
    assert_eq!(parses.load(Ordering::SeqCst), 1);
    s.set_grants(USER, &kb.id, vec![]).unwrap();
    assert!(
        s.search_for_agent(USER, WORKSPACE, &f.agent, "cobalt-orbit", 1200)
            .await
            .unwrap()
            .hits
            .is_empty()
    );
    assert!(
        f.state
            .resources
            .for_agent(Some(&f.agent))
            .download_binary_to_file(USER, command())
            .is_err()
    );
}
#[tokio::test]
async fn provider_failure_and_invalid_json_are_retryable_without_reparsing() {
    let f = Fixture::new();
    let v = Vision::new(f.client.clone());
    let (s, parses) = service(&f, &v, true, DOCX.to_vec());
    let kb = s.create(USER, "Retry", "synthetic").unwrap();
    let d = s
        .upload(
            USER,
            &kb.id,
            "synthetic.docx",
            include_bytes!("fixtures/mineru-v1/synthetic.docx"),
        )
        .unwrap();
    v.fail.store(true, Ordering::SeqCst);
    let failed = s.index(USER, &kb.id, &d.id).await.unwrap();
    assert_eq!(failed.status, "FAILED");
    assert!(failed.active_source_code.is_none());
    assert!(failed.image_description_artifacts.is_empty());
    v.fail.store(false, Ordering::SeqCst);
    v.bad_json.store(true, Ordering::SeqCst);
    assert_eq!(s.index(USER, &kb.id, &d.id).await.unwrap().status, "FAILED");
    v.bad_json.store(false, Ordering::SeqCst);
    let ready = s.index(USER, &kb.id, &d.id).await.unwrap();
    assert_eq!(ready.status, "READY");
    assert_eq!(v.calls.load(Ordering::SeqCst), 3);
    assert_eq!(parses.load(Ordering::SeqCst), 1);
    f.client.failing.store(true, Ordering::SeqCst);
    let failed = s.index(USER, &kb.id, &d.id).await.unwrap();
    assert_eq!(failed.status, "FAILED");
    assert_eq!(failed.active_source_code, ready.active_source_code);
    assert_eq!(v.calls.load(Ordering::SeqCst), 3);
    f.client.failing.store(false, Ordering::SeqCst);
    assert_eq!(s.index(USER, &kb.id, &d.id).await.unwrap().status, "READY");
    assert_eq!(v.calls.load(Ordering::SeqCst), 3);
}
#[tokio::test]
async fn cancellation_keeps_successful_private_cache_and_retry_finishes_remaining_image() {
    let f = Fixture::new();
    let v = Vision::new(f.client.clone());
    v.block_at.store(2, Ordering::SeqCst);
    let (s, parses) = service(&f, &v, true, two_images());
    let kb = s.create(USER, "Cancel", "synthetic").unwrap();
    let d = s
        .upload(
            USER,
            &kb.id,
            "synthetic.docx",
            include_bytes!("fixtures/mineru-v1/synthetic.docx"),
        )
        .unwrap();
    {
        let pending = s.index(USER, &kb.id, &d.id);
        tokio::pin!(pending);
        tokio::select! {
            result = &mut pending => panic!("index unexpectedly completed: {result:?}"),
            result = tokio::time::timeout(std::time::Duration::from_secs(3), async {
                loop {
                    let reached = v.reached.notified();
                    if v.calls.load(Ordering::SeqCst) >= 2 { break; }
                    reached.await;
                }
            }) => result.unwrap(),
        }
    }
    let d = s.detail(USER, &kb.id).unwrap().documents.remove(0);
    assert_eq!(d.status, "FAILED");
    assert_eq!(d.image_description_artifacts.len(), 1);
    assert_eq!(v.calls.load(Ordering::SeqCst), 2);
    let p = s.preview(USER, &kb.id, &d.id).await.unwrap();
    assert_eq!(
        p.image_analysis.unwrap().supplement_status.as_deref(),
        Some("pending")
    );
    assert_eq!(v.calls.load(Ordering::SeqCst), 2);
    v.block_at.store(0, Ordering::SeqCst);
    let ready = s.index(USER, &kb.id, &d.id).await.unwrap();
    assert_eq!(ready.status, "READY");
    assert_eq!(ready.image_description_artifacts.len(), 2);
    assert_eq!(v.calls.load(Ordering::SeqCst), 3);
    assert_eq!(parses.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn account_change_during_response_is_rejected_and_cache_is_not_published() {
    let f = Fixture::new();
    let v = Vision::new(f.client.clone());
    v.flip_scope.store(true, Ordering::SeqCst);
    let (s, _) = service(&f, &v, true, DOCX.to_vec());
    let kb = s.create(USER, "Identity", "synthetic").unwrap();
    let d = s
        .upload(
            USER,
            &kb.id,
            "synthetic.docx",
            include_bytes!("fixtures/mineru-v1/synthetic.docx"),
        )
        .unwrap();
    let failed = s.index(USER, &kb.id, &d.id).await.unwrap();
    assert_eq!(failed.status, "FAILED");
    assert!(failed.image_description_artifacts.is_empty());
    assert!(failed.active_source_code.is_none());
    v.flip_scope.store(false, Ordering::SeqCst);
    let ready = s.index(USER, &kb.id, &d.id).await.unwrap();
    assert_eq!(ready.status, "READY");
    assert_eq!(v.calls.load(Ordering::SeqCst), 2);
    *v.scope.lock().unwrap() = "synthetic-account-c".into();
    s.index(USER, &kb.id, &d.id).await.unwrap();
    assert_eq!(v.calls.load(Ordering::SeqCst), 3);
}
