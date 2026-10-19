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
// Build the replay archive from checked-in input, rather than an ignored generated ZIP.
fn docx_archive() -> Vec<u8> {
    use std::io::{Cursor, Read, Write};
    let mut original = zip::ZipArchive::new(Cursor::new(include_bytes!(
        "fixtures/mineru-v1/synthetic.docx"
    )))
    .unwrap();
    let mut image = Vec::new();
    original
        .by_name("docProps/thumbnail.jpeg")
        .unwrap()
        .read_to_end(&mut image)
        .unwrap();
    let middle = serde_json::json!({"schema":"docvortex.middle","schema_version":"2.0",
    "metadata":{"file_suffix":"docx"},"extensions":{"mineru":{"tier":"flash"}},
    "pages":[{"page_idx":0,"blocks":[
        {"type":"paragraph","index":0,"content":"Synthetic source text for image indexing"},
        {"type":"image","index":1,"content":[{"type":"image_body","index":1,"image_path":"images/figure.jpg","content":""}]}
    ]}]});
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in [
        ("middle_json.json", serde_json::to_vec(&middle).unwrap()),
        ("structured_content.json", b"{}".to_vec()),
        ("markdown.md", b"Synthetic image document".to_vec()),
        ("images/figure.jpg", image),
    ] {
        archive
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        archive.write_all(&data).unwrap();
    }
    archive.finish().unwrap().into_inner()
}
#[derive(Clone)]
struct Vision {
    embed: FakeEmbedding,
    image_inputs: Arc<AtomicUsize>,
    graph_calls: Arc<AtomicUsize>,
    calls: Arc<AtomicUsize>,
    fail: Arc<AtomicBool>,
    block_at: Arc<AtomicUsize>,
    scope: Arc<Mutex<String>>,
    flip_scope: Arc<AtomicBool>,
    bad_json: Arc<AtomicBool>,
    inputs: Arc<Mutex<Vec<String>>>,
    rerank_inputs: Arc<Mutex<Vec<String>>>,
    reached: Arc<tokio::sync::Notify>,
}
impl Vision {
    fn new(embed: FakeEmbedding) -> Self {
        Self {
            embed,
            image_inputs: Default::default(),
            graph_calls: Default::default(),
            calls: Default::default(),
            fail: Default::default(),
            block_at: Default::default(),
            scope: Arc::new(Mutex::new("synthetic-account-a".into())),
            flip_scope: Default::default(),
            bad_json: Default::default(),
            inputs: Default::default(),
            rerank_inputs: Default::default(),
            reached: Default::default(),
        }
    }
}
impl LlmRuntimeClient for Vision {
    fn embed_content_batch_with_model<'a>(
        &'a self,
        c: &'a RuntimeModelConfig,
        inputs: &'a [eliteforge_agent_framework::Content],
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
        Box::pin(async move {
            for input in inputs {
                for block in &input.0 {
                    if let eliteforge_agent_framework::ContentBlock::Media { media } = block {
                        assert_eq!(c.model_code, "fixture-multimodal");
                        let eliteforge_agent_framework::MediaSource::Base64(bytes) = &media.source
                        else {
                            panic!("expected authorized inline image")
                        };
                        use base64::Engine;
                        assert!(
                            base64::engine::general_purpose::STANDARD
                                .decode(bytes)
                                .unwrap()
                                .starts_with(&[0xff, 0xd8, 0xff])
                        );
                        self.image_inputs.fetch_add(1, Ordering::SeqCst);
                    }
                }
            }
            let texts = inputs
                .iter()
                .map(eliteforge_agent_framework::Content::text)
                .collect::<Vec<_>>();
            self.embed_batch_with_model(c, &texts, dimensions).await
        })
    }
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
            Ok(r#"{"description":"Synthetic cobalt-orbit diagram shows UPLOAD leading to INDEX","visibleText":["UPLOAD","INDEX"],"relations":["UPLOAD -> INDEX"]}"#.into())
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
        Box::pin(async move {
            if c.model_code == "synthetic-graph" {
                self.graph_calls.fetch_add(1, Ordering::SeqCst);
                // The mock recognizes its fixture facts and selected model,
                // independently of the extraction prompt's wording/delimiters.
                let quote = if p.contains("Synthetic source text for image indexing") {
                    "Synthetic".to_owned()
                } else if let Some(topic) = (0..8).find(|n| p.contains(&format!("orchid{n}"))) {
                    format!("orchid{topic}")
                } else if p.contains("Topic") {
                    "Topic".to_owned()
                } else if p.contains("Synthetic") {
                    "Synthetic".to_owned()
                } else {
                    panic!("graph mock received a source outside its synthetic fixture")
                };
                return Ok(serde_json::json!({"nodes":[{"ref":"synthetic-entity","name":quote,"type":"Concept","evidence":quote}],"edges":[]}).to_string());
            }
            self.embed.generate(c, p).await
        })
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
        if c.model_code == "synthetic-rerank" {
            return Box::pin(async move {
                *self.rerank_inputs.lock().unwrap() = d.to_vec();
                let mut ranked = d
                    .iter()
                    .enumerate()
                    .map(|(index, text)| LlmRuntimeRerankHit {
                        document_index: index,
                        score: if text.contains("See Figure 1")
                            && text.contains("Figure 1 Synthetic cobalt rating")
                        {
                            10.
                        } else {
                            0.
                        },
                    })
                    .collect::<Vec<_>>();
                ranked.sort_by(|a, b| b.score.total_cmp(&a.score));
                ranked.truncate(n.unwrap_or(ranked.len()));
                Ok(ranked)
            });
        }
        self.embed.rerank(c, q, d, n)
    }
}
struct Resolver;
impl LlmRuntimeModelResolver for Resolver {
    fn embedding_accepts_images(&self, _: &str, model: &str) -> Result<bool, ModelError> {
        Ok(model == "fixture-multimodal")
    }
    fn auto_vision_runtime_model_config<'a>(
        &'a self,
        w: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<RuntimeModelConfig>, ModelError>> + Send + 'a>>
    {
        Box::pin(async move { self.vision_runtime_model_config(w, MODEL) })
    }
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
        if m == Some("synthetic-rerank") {
            let mut config = EmbeddingResolver
                .embedding_runtime_model_config(w, m)?
                .unwrap();
            config.model_type = ModelType::Rerank;
            return Ok(Some(config));
        }
        EmbeddingResolver.rerank_runtime_model_config(w, m)
    }
}
struct FixtureTextRouter;
struct RevokePixelMember {
    owner: KnowledgeBaseService,
    checks: AtomicUsize,
}
impl tron_backend::application::rag::RagAccessResolver for RevokePixelMember {
    fn resolve(
        &self,
        identity: &tron_backend::application::rag::RagIdentity,
    ) -> Result<
        Vec<tron_backend::application::rag::AuthorizedRagRange>,
        tron_backend::domain::rag::RagError,
    > {
        tron_backend::application::rag::RagAccessResolver::resolve(&self.owner, identity)
    }
    fn recheck(
        &self,
        identity: &tron_backend::application::rag::RagIdentity,
        result: &mut tron_backend::application::rag::RagEvidenceResult,
    ) -> Result<(), tron_backend::domain::rag::RagError> {
        self.recheck_sources(identity, result)
    }
    fn recheck_sources(
        &self,
        identity: &tron_backend::application::rag::RagIdentity,
        result: &mut tron_backend::application::rag::RagEvidenceResult,
    ) -> Result<(), tron_backend::domain::rag::RagError> {
        tron_backend::application::rag::RagAccessResolver::recheck_sources(
            &self.owner,
            identity,
            result,
        )?;
        if self.checks.fetch_add(1, Ordering::SeqCst) > 0 {
            result
                .hits
                .retain(|hit| hit.row.text_metadata["location"]["blockKind"] != "image_vector");
        }
        Ok(())
    }
}
impl tron_backend::application::model_router::TextModelRouter for FixtureTextRouter {
    fn route_text(&self, w: &str, m: Option<&str>) -> Result<RuntimeModelConfig, String> {
        Resolver
            .vision_runtime_model_config(w, m.unwrap_or("synthetic-graph"))
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "fixture model unavailable".into())
    }
}
#[derive(Clone)]
struct Replay {
    calls: Arc<AtomicUsize>,
    archive: Vec<u8>,
}
#[async_trait::async_trait]
impl KnowledgeDocumentParser for Replay {
    fn with_ocr_mode(
        &self,
        _: OcrMode,
    ) -> Result<Arc<dyn KnowledgeDocumentParser>, KnowledgeBaseError> {
        Ok(Arc::new(self.clone()))
    }
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
        f.state
            .rag
            .with_runtime(tron_backend::application::rag::DefaultRagRuntime::new(
                LlmRuntimeService::new(Arc::new(Resolver), Arc::new(v.clone())),
                FixtureTextRouter,
            )),
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
    let mut input = zip::ZipArchive::new(Cursor::new(docx_archive())).unwrap();
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

fn referenced_image_and_repeated_logo() -> Vec<u8> {
    use std::io::{Cursor, Read, Write};
    let mut input = zip::ZipArchive::new(Cursor::new(docx_archive())).unwrap();
    let mut output = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for index in 0..input.len() {
        let mut file = input.by_index(index).unwrap();
        let name = file.name().to_owned();
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        if name == "middle_json.json" {
            let mut middle: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            middle["pages"][0]["blocks"] = serde_json::json!([
                {"type":"paragraph","index":0,"content":"Synthetic source text for image indexing. See Figure 1 for the cobalt rating."},
                {"type":"image","index":1,"content":[{"type":"image_caption","index":2,"content":"Figure 1 Synthetic cobalt rating"},{"type":"image_body","index":3,"image_path":"images/figure.jpg","content":""}]},
                {"type":"paragraph","index":4,"content":"Synthetic unrelated bulletin."},
                {"type":"image","index":5,"content":[{"type":"image_caption","index":6,"content":"Figure 2 Synthetic bulletin logo"},{"type":"image_body","index":7,"image_path":"images/figure.jpg","content":""}]}
            ]);
            bytes = serde_json::to_vec(&middle).unwrap();
        }
        output
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        output.write_all(&bytes).unwrap();
    }
    output.finish().unwrap().into_inner()
}

#[tokio::test]
async fn source_group_reranks_body_image_description_and_original_together_without_following_a_shared_asset()
 {
    use tron_backend::application::rag::RagQueryInput;
    let f = Fixture::new();
    let vision = Vision::new(f.client.clone());
    let (service, parses) = service(&f, &vision, true, referenced_image_and_repeated_logo());
    let kb = service
        .create(USER, "Source association", "synthetic")
        .unwrap();
    let mut settings = f.state.rag.settings_for_corpus(&kb.id).unwrap();
    settings.embedding_model_code = Some("fixture-multimodal".into());
    settings.graph = true;
    settings.graph_model_code = Some("synthetic-graph".into());
    settings.rerank_model_code = Some("synthetic-rerank".into());
    f.state.rag.configure_corpus(&kb.id, settings).unwrap();
    let doc = service
        .upload(
            USER,
            &kb.id,
            "synthetic.docx",
            include_bytes!("fixtures/mineru-v1/synthetic.docx"),
        )
        .unwrap();
    let indexed = service.index(USER, &kb.id, &doc.id).await.unwrap();
    assert_eq!(indexed.status, "READY", "{:?}", indexed.error);
    let result = service
        .recall_for_user(
            USER,
            RagQueryInput {
                query: "cobalt rating".into(),
                corpus_ids: vec![kb.id.clone()],
                filters: serde_json::json!({}),
                limit: 1,
            },
        )
        .await
        .unwrap();
    assert!(
        result.stages.iter().all(|stage| stage.status != "FAILED"),
        "{:?}",
        result.stages
    );
    assert_eq!(result.evidence.len(), 1, "{:?}", result.warnings);
    let hit = &result.evidence[0];
    assert!(
        hit.parts
            .iter()
            .any(|part| part.chunk_code == hit.chunk_code && !part.image_vector),
        "the authorization test below must revoke a related member while keeping the anchor allowed"
    );
    assert!(hit.text.contains("See Figure 1"), "{}", hit.text);
    assert!(hit.text.contains("Figure 1 Synthetic cobalt rating"));
    assert!(hit.text.contains("UPLOAD leading to INDEX"));
    assert!(!hit.text.contains("unrelated bulletin"));
    assert!(!hit.text.contains("Figure 2"));
    assert_eq!(
        hit.images.len(),
        1,
        "shared pixels must not merge two original occurrences"
    );
    assert!(
        hit.images[0]
            .annotations
            .iter()
            .any(|annotation| annotation.text.contains("UPLOAD leading to INDEX"))
    );
    assert!(
        hit.associations
            .iter()
            .any(|link| link.relationship == "REFERENCES_FIGURE" && link.quote == "Figure 1")
    );
    assert!(
        hit.parts
            .iter()
            .all(|part| part.file_name.as_deref() == Some("synthetic.docx"))
    );
    let wire = serde_json::to_value(&result).unwrap();
    assert!(
        wire["graphContext"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|context| context["nodes"].as_array().unwrap())
            .all(|node| node.get("properties").is_none() && node.get("embedding").is_none())
    );
    let reranked = vision.rerank_inputs.lock().unwrap();
    assert!(
        reranked.len() >= 2,
        "Top1 is applied after source grouping and rerank"
    );
    assert!(reranked.iter().any(|text| text.contains("See Figure 1")
        && text.contains("Figure 1 Synthetic cobalt rating")
        && text.contains("UPLOAD leading to INDEX")));
    drop(reranked);
    service
        .set_grants(USER, &kb.id, vec![f.agent.clone()])
        .unwrap();
    vision.rerank_inputs.lock().unwrap().clear();
    let rag = f
        .state
        .rag
        .with_runtime(tron_backend::application::rag::DefaultRagRuntime::new(
            LlmRuntimeService::new(Arc::new(Resolver), Arc::new(vision.clone())),
            FixtureTextRouter,
        ));
    let revoked = rag
        .search(
            &RevokePixelMember {
                owner: service.clone(),
                checks: AtomicUsize::new(0),
            },
            &tron_backend::application::rag::RagIdentity {
                user_code: USER.into(),
                workspace_code: WORKSPACE.into(),
                agent_code: f.agent.clone(),
            },
            RagQueryInput {
                query: "cobalt rating".into(),
                corpus_ids: vec![kb.id.clone()],
                filters: serde_json::json!({}),
                limit: 1,
            },
            None,
        )
        .await
        .unwrap();
    assert!(
        revoked
            .evidence
            .iter()
            .all(|hit| !hit.text.contains("See Figure 1")),
        "revoking a related pixel row must remove its whole source group"
    );
    assert!(
        vision
            .rerank_inputs
            .lock()
            .unwrap()
            .iter()
            .all(|text| !text.contains("See Figure 1")),
        "unauthorized members must not reach the external reranker"
    );
    assert!(
        revoked
            .stages
            .iter()
            .filter(
                |stage| stage.channel.starts_with("graph") || stage.channel == "source.structure"
            )
            .all(|stage| stage.hit_count == 0)
    );
    assert_eq!(parses.load(Ordering::SeqCst), 1);
    assert_eq!(vision.image_inputs.load(Ordering::SeqCst), 1);
    assert!(
        service
            .recall_for_user(
                "other-user",
                RagQueryInput {
                    query: "cobalt".into(),
                    corpus_ids: vec![kb.id],
                    filters: serde_json::json!({}),
                    limit: 1
                }
            )
            .await
            .is_err()
    );
}
#[tokio::test]
async fn preview_never_calls_vision_and_default_description_index_is_acl_scoped() {
    let f = Fixture::new();
    let v = Vision::new(f.client.clone());
    let (s, parses) = service(&f, &v, false, docx_archive());
    let kb = s.create(USER, "Images", "synthetic only").unwrap();
    let doc = s
        .upload(
            USER,
            &kb.id,
            "synthetic.docx",
            include_bytes!("fixtures/mineru-v1/synthetic.docx"),
        )
        .unwrap();
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
        workspace_code: "PLATFORM".into(),
        resource_code: artifact.resource_code.clone(),
    };
    assert!(
        f.state
            .resources
            .for_agent_in_workspace(Some(&f.agent), WORKSPACE)
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
    // This fixture returns identical vectors for all content. The ACL and
    // provenance contract applies to the matching image evidence regardless of
    // its position among the newly added graph and body candidates.
    assert!(
        hits.iter().any(|hit| hit.text.contains("cobalt-orbit")
            && hit.location["imageAnnotations"]
                .as_array()
                .is_some_and(|annotations| annotations
                    .iter()
                    .any(|a| a["source"] == "framework.sdk.kb-image-description-v1"
                        && a["modelCode"] == MODEL))),
        "retrieved image descriptions must preserve their model and source annotations"
    );
    assert!(
        f.state
            .resources
            .for_agent_in_workspace(Some(&f.agent), WORKSPACE)
            .download_binary_to_file(USER, command())
            .is_ok()
    );
    assert!(
        f.state
            .resources
            .for_agent_in_workspace(Some(&f.agent), WORKSPACE)
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
            .for_agent_in_workspace(Some(&f.agent), WORKSPACE)
            .download_binary_to_file(USER, command())
            .is_err()
    );
}
#[tokio::test]
async fn provider_failure_and_invalid_json_are_retryable_without_reparsing() {
    let f = Fixture::new();
    let v = Vision::new(f.client.clone());
    let (s, parses) = service(&f, &v, true, docx_archive());
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
    let reused = s.index(USER, &kb.id, &d.id).await.unwrap();
    assert_eq!(
        reused.status, "READY",
        "unchanged vectors must bypass unavailable provider"
    );
    assert_ne!(reused.active_source_code, ready.active_source_code);
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
    let (s, _) = service(&f, &v, true, docx_archive());
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

#[tokio::test]
async fn model_switch_supplements_original_images_without_parsing_or_describing_again() {
    let f = Fixture::new();
    let v = Vision::new(f.client.clone());
    let (s, parses) = service(&f, &v, false, docx_archive());
    let kb = s.create(USER, "Image mode switch", "synthetic").unwrap();
    let d = s
        .upload(
            USER,
            &kb.id,
            "synthetic.docx",
            include_bytes!("fixtures/mineru-v1/synthetic.docx"),
        )
        .unwrap();
    let ready = s.index(USER, &kb.id, &d.id).await.unwrap();
    assert_eq!(ready.status, "READY");
    assert_eq!(v.image_inputs.load(Ordering::SeqCst), 0);
    assert_eq!(v.calls.load(Ordering::SeqCst), 1);
    let before = f.client.calls.load(Ordering::SeqCst);
    let mut settings = f.state.rag.settings_for_corpus(&kb.id).unwrap();
    settings.embedding_model_code = Some("fixture-multimodal".into());
    f.state
        .rag
        .configure_corpus(&kb.id, settings.clone())
        .unwrap();
    let image_ready = s.index(USER, &kb.id, &d.id).await.unwrap();
    assert_eq!(image_ready.status, "READY", "{:?}", image_ready.error);
    assert_eq!(parses.load(Ordering::SeqCst), 1);
    assert_eq!(v.calls.load(Ordering::SeqCst), 1);
    assert_eq!(v.image_inputs.load(Ordering::SeqCst), 1);
    assert_eq!(
        image_ready.parsing_artifact.as_ref().unwrap().resource_code,
        ready.parsing_artifact.as_ref().unwrap().resource_code
    );
    assert!(f.client.calls.load(Ordering::SeqCst) > before);
    let count = f.client.calls.load(Ordering::SeqCst);
    settings.fulltext = false;
    f.state
        .rag
        .configure_corpus(&kb.id, settings.clone())
        .unwrap();
    assert_eq!(s.index(USER, &kb.id, &d.id).await.unwrap().status, "READY");
    assert_eq!(
        f.client.calls.load(Ordering::SeqCst),
        count,
        "fulltext change must reuse vectors"
    );
    assert_eq!(
        parses.load(Ordering::SeqCst),
        1,
        "index configuration must not create parsing work"
    );
    assert_eq!(v.image_inputs.load(Ordering::SeqCst), 1);
    settings.graph = true;
    settings.graph_model_code = Some("synthetic-graph".into());
    f.state
        .rag
        .configure_corpus(&kb.id, settings.clone())
        .unwrap();
    let previous_inputs = v.inputs.lock().unwrap().len();
    let graph_ready = s.index(USER, &kb.id, &d.id).await.unwrap();
    assert_eq!(graph_ready.status, "READY", "{:?}", graph_ready.error);
    let graph_calls = v.graph_calls.load(Ordering::SeqCst);
    assert!(graph_calls > 0);
    let graph_vector_calls = f.client.calls.load(Ordering::SeqCst);
    assert!(
        graph_vector_calls > count,
        "graph entities require their own node vectors"
    );
    let node_inputs = v.inputs.lock().unwrap()[previous_inputs..].to_vec();
    assert!(!node_inputs.is_empty());
    assert!(
        node_inputs
            .iter()
            .all(|text| text == "Synthetic\nConcept\nSynthetic"),
        "enabling graph must reuse body vectors and embed only its new node projections: {node_inputs:?}"
    );
    assert_eq!(v.image_inputs.load(Ordering::SeqCst), 1);
    settings.fulltext = true;
    f.state
        .rag
        .configure_corpus(&kb.id, settings.clone())
        .unwrap();
    let reused = s.index(USER, &kb.id, &d.id).await.unwrap();
    assert_eq!(reused.status, "READY", "{:?}", reused.error);
    assert_eq!(
        f.client.calls.load(Ordering::SeqCst),
        graph_vector_calls,
        "the next generation reuses both body and graph-node vectors"
    );
    assert_eq!(
        v.graph_calls.load(Ordering::SeqCst),
        graph_calls,
        "unchanged graph must remap cached evidence to the new generation"
    );
    settings.graph = false;
    f.state
        .rag
        .configure_corpus(&kb.id, settings.clone())
        .unwrap();
    assert_eq!(s.index(USER, &kb.id, &d.id).await.unwrap().status, "READY");
    assert_eq!(f.client.calls.load(Ordering::SeqCst), graph_vector_calls);
    assert_eq!(v.graph_calls.load(Ordering::SeqCst), graph_calls);
    settings.embedding_model_code = Some("fixture-embedding".into());
    f.state.rag.configure_corpus(&kb.id, settings).unwrap();
    let text_ready = s.index(USER, &kb.id, &d.id).await.unwrap();
    assert_eq!(text_ready.status, "READY");
    assert_eq!(
        v.image_inputs.load(Ordering::SeqCst),
        1,
        "text mode must not send pixels"
    );
    assert_eq!(v.calls.load(Ordering::SeqCst), 1);
    assert_eq!(parses.load(Ordering::SeqCst), 1);
    assert!(
        v.inputs
            .lock()
            .unwrap()
            .iter()
            .any(|text| text.contains("cobalt-orbit"))
    );
}

#[tokio::test]
async fn quality_publication_retains_descriptions_and_raw_image_vectors_without_reparsing() {
    use eliteforge_agent_core::{
        ChunkSourceSpan, EvaluationPartition, EvaluationProvenance, RetrievalEvaluationCase,
        RetrievalEvidenceTarget,
    };
    use std::io::{Cursor, Read, Write};
    use tron_backend::application::{
        document::{ChunkConfig, ChunkMode},
        knowledge_quality::{KnowledgeQualityService, QualityOptimizationOptions},
    };
    let original = docx_archive();
    let mut archive = zip::ZipArchive::new(Cursor::new(original)).unwrap();
    let mut output = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).unwrap();
        let name = entry.name().to_owned();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        if name == "middle_json.json" {
            let mut middle: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            let image = middle["pages"][0]["blocks"][1].clone();
            let mut blocks = Vec::new();
            for topic in 0..8 {
                // Fixed windows remain complete and retain every character, but
                // cannot satisfy a two-paragraph question through one block.
                // Auto groups the same heading's two paragraphs into one unit.
                let body = format!(
                    "orchid{topic} unique evidence with applicable conditions and exceptions. "
                )
                .repeat(20)
                .chars()
                .take(391)
                .collect::<String>();
                let conditions=format!("Synthetic conditions for Topic {topic}, including exceptions and applicable scope. ").repeat(20).chars().take(599).collect::<String>();
                blocks.push(serde_json::json!({"type":"paragraph_title","index":topic*3,"content":format!("Topic {topic}"),"level":1}));
                blocks
                    .push(serde_json::json!({"type":"paragraph","index":topic*3+1,"content":body}));
                blocks.push(
                    serde_json::json!({"type":"paragraph","index":topic*3+2,"content":conditions}),
                );
            }
            blocks.push(image);
            middle["pages"][0]["blocks"] = serde_json::json!(blocks);
            bytes = serde_json::to_vec(&middle).unwrap();
        }
        output
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        output.write_all(&bytes).unwrap();
    }
    let f = Fixture::new();
    let v = Vision::new(f.client.clone());
    let (s, parses) = service(&f, &v, true, output.finish().unwrap().into_inner());
    let kb = s.create(USER, "Quality images", "").unwrap();
    let mut settings = f.state.rag.settings_for_corpus(&kb.id).unwrap();
    settings.embedding_model_code = Some("fixture-multimodal".into());
    settings.graph = true;
    settings.graph_model_code = Some("synthetic-graph".into());
    settings.rerank_model_code = None;
    f.state.rag.configure_corpus(&kb.id, settings).unwrap();
    let doc = s
        .upload(
            USER,
            &kb.id,
            "synthetic.docx",
            include_bytes!("fixtures/mineru-v1/synthetic.docx"),
        )
        .unwrap();
    s.configure(
        USER,
        &kb.id,
        &doc.id,
        Some(ChunkConfig {
            mode: ChunkMode::Fixed,
            max_chars: 200,
            overlap_chars: 0,
            heading_depth: 3,
        }),
    )
    .unwrap();
    let before = s.index(USER, &kb.id, &doc.id).await.unwrap();
    assert_eq!(before.status, "READY", "{:?}", before.error);
    let quality = KnowledgeQualityService::new(s.clone());
    let (fingerprint, excerpts) = quality.source_excerpts(USER, &kb.id, 0, 24).await.unwrap();
    let cases = (0..8)
        .map(|topic| {
            let excerpt = excerpts
                .iter()
                .find(|e| e.text.starts_with(&format!("orchid{topic}")))
                .unwrap();
            let conditions = excerpts
                .iter()
                .find(|e| {
                    e.text
                        .starts_with(&format!("Synthetic conditions for Topic {topic}"))
                })
                .unwrap();
            RetrievalEvaluationCase {
                id: format!("case-{topic}"),
                query: format!("orchid{topic}"),
                expected: vec![
                    RetrievalEvidenceTarget {
                        document_id: doc.id.clone(),
                        span: ChunkSourceSpan {
                            block_id: excerpt.block_id.clone(),
                            start: 0,
                            end: 391,
                        },
                    },
                    RetrievalEvidenceTarget {
                        document_id: doc.id.clone(),
                        span: ChunkSourceSpan {
                            block_id: conditions.block_id.clone(),
                            start: 0,
                            end: 599,
                        },
                    },
                ],
                partition: if topic % 2 == 0 {
                    EvaluationPartition::Training
                } else {
                    EvaluationPartition::Holdout
                },
                provenance: EvaluationProvenance::HumanLabeled,
            }
        })
        .collect();
    quality
        .save_dataset(USER, &kb.id, &fingerprint, cases)
        .await
        .unwrap();
    let images = v.image_inputs.load(Ordering::SeqCst);
    let descriptions = v.calls.load(Ordering::SeqCst);
    let run = quality
        .start(
            USER,
            &kb.id,
            "preserve-image-evidence",
            QualityOptimizationOptions {
                top_k: 1,
                max_trials: 1,
                candidates: vec![ChunkConfig {
                    mode: ChunkMode::Auto,
                    max_chars: 1600,
                    overlap_chars: 120,
                    heading_depth: 3,
                }],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let run = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        loop {
            let current = quality.run(USER, &kb.id, &run.id).unwrap();
            if !matches!(
                current.status.as_str(),
                "QUEUED" | "RUNNING" | "PUBLISHING" | "CANCEL_REQUESTED"
            ) {
                break current;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(run.status, "PUBLISHED", "{:?}", run.error);
    assert_eq!(parses.load(Ordering::SeqCst), 1);
    assert_eq!(v.calls.load(Ordering::SeqCst), descriptions);
    assert_eq!(
        v.image_inputs.load(Ordering::SeqCst),
        images,
        "unchanged image input must reuse the published pixel vector"
    );
    let after = s.detail(USER, &kb.id).unwrap().documents.remove(0);
    assert_ne!(after.active_source_code, before.active_source_code);
    assert_eq!(
        after.image_description_artifacts.len(),
        before.image_description_artifacts.len()
    );
    let conn = rusqlite::Connection::open(&f.state.config.database_url).unwrap();
    let count:usize=conn.query_row(&format!("SELECT count(*) FROM rag_{}_units WHERE source_code=?1 AND unit_type='IMAGE' AND embedding_status='INDEXED'",kb.id),[after.active_source_code.as_deref().unwrap()],|r|r.get(0)).unwrap();
    assert_eq!(
        count, 1,
        "optimized publication must retain the raw image vector unit"
    );
    let graphs:usize=conn.query_row("SELECT count(*) FROM rag_graph_active_sources a JOIN rag_graph_generations g ON g.generation_id=a.generation_id WHERE g.corpus_id=?1 AND g.source_code=?2",rusqlite::params![kb.id,after.active_source_code.as_deref().unwrap()],|r|r.get(0)).unwrap();
    assert_eq!(
        graphs, 1,
        "quality publication must activate the tested real graph with the image index"
    );
}
