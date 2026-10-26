//! Real authorized CCS-only validation. Production SDK calls; isolated storage and read-only auth metadata.
use super::*;
use eliteforge_agent_core as fw;
use rusqlite::{Connection, OpenFlags, params_from_iter};
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
use tron_backend::{
    adapters::outbound::{
        crypto::Sm4GcmVariableCipher,
        model::{
            HttpLlmRuntimeClient, HttpModelClient, HttpModelProviderClient,
            PersonAiModelProviderClient,
        },
        persistence::sqlite::SqliteKnowledgeBaseRepository,
    },
    application::{
        agent::{FrameworkHost, ModelClient, ModelStreamSink, ReactAgentBuilder},
        knowledge_base::KnowledgeBaseService,
        llm::{LlmRuntimeClient, LlmRuntimeRerankHit, LlmRuntimeService},
        model::RuntimeModelConfig,
        model_provider::{ModelProviderService, ProviderAwareLlmRuntimeResolver},
    },
    config::VariableCryptoConfig,
    domain::agent::{AgentModelConfig, AgentRunContext, ModelRequest, ModelResponse, ToolRegistry},
    shared::event_bus::EventBusHandle,
};
type EF<'a> = Pin<Box<dyn Future<Output = Result<(Vec<f32>, Option<String>), String>> + Send + 'a>>;
#[derive(Clone)]
struct CappedRuntime {
    inner: HttpLlmRuntimeClient,
    calls: Arc<AtomicUsize>,
    vision: Arc<AtomicUsize>,
    models: Arc<Mutex<Vec<(usize, Option<String>)>>>,
}
impl CappedRuntime {
    fn admit(&self) -> Result<(), String> {
        if self.calls.fetch_add(1, Ordering::SeqCst) >= 48 {
            Err("CCS-only embedding cap reached".into())
        } else {
            Ok(())
        }
    }
    fn record(&self, r: &Result<(Vec<f32>, Option<String>), String>) {
        if let Ok((v, m)) = r {
            self.models.lock().unwrap().push((v.len(), m.clone()));
        }
    }
}
impl LlmRuntimeClient for CappedRuntime {
    fn model_authorization_scope(&self, c: &RuntimeModelConfig) -> Result<Option<String>, String> {
        self.inner.model_authorization_scope(c)
    }
    fn generate_vision<'a>(
        &'a self,
        c: &'a RuntimeModelConfig,
        p: &'a str,
        m: &'a str,
        b: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move {
            if self.vision.fetch_add(1, Ordering::SeqCst) >= 2 {
                return Err("CCS image-description cap reached".into());
            }
            self.inner.generate_vision(c, p, m, b).await
        })
    }
    fn embed_batch_with_model<'a>(
        &'a self,
        c: &'a RuntimeModelConfig,
        inputs: &'a [String],
        dimensions: usize,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(Vec<fw::EmbeddingVector>, Option<String>), String>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            self.admit()?;
            let result = self
                .inner
                .embed_batch_with_model(c, inputs, dimensions)
                .await;
            if let Ok((vectors, provider)) = &result {
                self.models
                    .lock()
                    .unwrap()
                    .extend(vectors.iter().map(|v| (v.values.len(), provider.clone())));
            }
            result
        })
    }
    fn embed_with_model<'a>(&'a self, c: &'a RuntimeModelConfig, s: &'a str) -> EF<'a> {
        Box::pin(async move {
            self.admit()?;
            let r = self.inner.embed_with_model(c, s).await;
            self.record(&r);
            r
        })
    }
    fn embed_query_with_model_in_run<'a>(
        &'a self,
        c: &'a RuntimeModelConfig,
        s: &'a str,
        d: usize,
        r: &'a fw::RunContext,
    ) -> EF<'a> {
        Box::pin(async move {
            self.admit()?;
            let result = self.inner.embed_query_with_model_in_run(c, s, d, r).await;
            self.record(&result);
            result
        })
    }
    fn generate<'a>(
        &'a self,
        c: &'a RuntimeModelConfig,
        p: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        self.inner.generate(c, p)
    }
    fn embed<'a>(
        &'a self,
        _: &'a RuntimeModelConfig,
        _: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<f32>, String>> + Send + 'a>> {
        Box::pin(async { Err("identified SDK embedding required".into()) })
    }
    fn rerank<'a>(
        &'a self,
        c: &'a RuntimeModelConfig,
        q: &'a str,
        d: &'a [String],
        n: Option<usize>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<LlmRuntimeRerankHit>, String>> + Send + 'a>> {
        self.inner.rerank(c, q, d, n)
    }
}
#[derive(Clone)]
struct ObservedModel {
    inner: Arc<dyn ModelClient>,
    records: Arc<Mutex<Vec<serde_json::Value>>>,
    calls: Arc<AtomicUsize>,
}
impl ModelClient for ObservedModel {
    fn bind_for_run(&self, c: &AgentModelConfig) -> Result<Option<Arc<dyn ModelClient>>, String> {
        Ok(self.inner.bind_for_run(c)?.map(|inner| {
            Arc::new(Self {
                inner,
                records: self.records.clone(),
                calls: self.calls.clone(),
            }) as Arc<dyn ModelClient>
        }))
    }
    fn authorization_scope(&self, c: &AgentModelConfig) -> Result<Option<String>, String> {
        self.inner.authorization_scope(c)
    }
    fn generate<'a>(
        &'a self,
        c: &'a AgentModelConfig,
        r: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        self.inner.generate(c, r)
    }
    fn generate_once<'a>(
        &'a self,
        c: &'a AgentModelConfig,
        r: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        self.inner.generate_once(c, r)
    }
    fn generate_native_once<'a>(
        &'a self,
        c: &'a AgentModelConfig,
        r: &'a fw::ChatRequest,
        p: &'a ModelRequest,
        s: Option<ModelStreamSink>,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async move {
            if self.calls.fetch_add(1, Ordering::SeqCst) >= 3 {
                return Err("CCS visual-QA call cap reached".into());
            }
            use sha2::{Digest, Sha256};
            let media=r.messages.iter().flat_map(|m|m.content.0.iter()).filter_map(|b|if let fw::ContentBlock::Media{media}=b{let sha=match &media.source{fw::MediaSource::Base64(v)=>format!("{:x}",Sha256::digest(v.as_bytes())),fw::MediaSource::Url(_)=>"URL_NOT_ALLOWED".into()};Some(serde_json::json!({"name":media.name,"mimeType":media.mime_type,"estimatedTokens":media.estimated_tokens,"encodedDataSha256":sha}))}else{None}).collect::<Vec<_>>();
            let text = r
                .messages
                .iter()
                .map(|m| m.content.text())
                .collect::<Vec<_>>()
                .join("\n");
            self.records.lock().unwrap().push(serde_json::json!({"media":media,"mediaCount":media.len(),"nativeAdmittedText":text}));
            self.inner.generate_native_once(c, r, p, s).await
        })
    }
    fn close_observed<'a>(
        &'a self,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<eliteforge_agent_plugin_core::ShutdownReport, String>>
                + Send
                + 'a,
        >,
    > {
        self.inner.close_observed()
    }
    fn close<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        self.inner.close()
    }
}
fn copy_authorization(
    f: &Fixture,
) -> (
    ModelProviderService,
    Connection,
    Vec<rusqlite::types::Value>,
) {
    let home = std::env::var_os("HOME").expect("home required");
    let home = Path::new(&home);
    let source = Connection::open_with_flags(
        home.join(".tron/tron.sqlite3"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .expect("read-only auth source unavailable");
    let columns = "provider_code,server_base_url,status,credentials_encrypted,authorized_at,authorization_expires_at,last_sync_at,last_error,updated_at,person_ai_user_id,person_ai_username,created_at";
    let before = source
        .query_row(
            &format!("SELECT {columns} FROM provider_connections WHERE provider_code='PersonAi'"),
            [],
            |r| {
                (0..12)
                    .map(|i| r.get(i))
                    .collect::<Result<Vec<rusqlite::types::Value>, _>>()
            },
        )
        .expect("existing auth unavailable");
    assert!(matches!(&before[2],rusqlite::types::Value::Text(v) if v=="Connected"||v=="Degraded"));
    let yaml: serde_yaml::Value = serde_yaml::from_str(
        &std::fs::read_to_string(home.join(".tron/application.yaml")).unwrap(),
    )
    .unwrap();
    let key = yaml["variableCrypto"]["keyBase64"]
        .as_str()
        .expect("existing cipher key missing")
        .to_owned();
    let cipher = Arc::new(
        Sm4GcmVariableCipher::from_config(VariableCryptoConfig { key_base64: key })
            .expect("existing cipher invalid"),
    );
    let isolated = Connection::open(&f.state.config.database_url).unwrap();
    isolated.execute(&format!("INSERT OR REPLACE INTO provider_connections({columns}) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)"),params_from_iter(before.iter())).unwrap();
    let device: String = source
        .query_row(
            "SELECT device_id FROM person_ai_device_identity WHERE singleton_id=1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    isolated
        .execute(
            "INSERT OR REPLACE INTO person_ai_device_identity(singleton_id,device_id) VALUES(1,?1)",
            [device],
        )
        .unwrap();
    for table in ["provider_catalog_models", "default_model_selections"] {
        let mut s = source.prepare(&format!("SELECT * FROM {table}")).unwrap();
        let names = s
            .column_names()
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>();
        let rows = s
            .query_map([], |r| {
                (0..names.len())
                    .map(|i| r.get::<_, rusqlite::types::Value>(i))
                    .collect::<Result<Vec<_>, _>>()
            })
            .unwrap();
        let placeholders = (1..=names.len())
            .map(|i| format!("?{i}"))
            .collect::<Vec<_>>()
            .join(",");
        for row in rows {
            isolated
                .execute(
                    &format!(
                        "INSERT OR REPLACE INTO {table}({}) VALUES({placeholders})",
                        names.join(",")
                    ),
                    params_from_iter(row.unwrap().iter()),
                )
                .unwrap();
        }
    }
    drop(isolated);
    let providers = ModelProviderService::connect(&f.state.config.database_url, cipher).unwrap();
    (providers, source, before)
}
pub async fn run() {
    assert_eq!(
        std::env::var("TRON_CCS_PDF_LIVE").as_deref(),
        Ok("1"),
        "explicit scoped PDF live opt-in required"
    );
    // Require source-grounded local fidelity regression success before any external call.
    let local: serde_json::Value = serde_json::from_slice(
        &std::fs::read(Path::new(EVIDENCE).join("ccs-product-reprocessed-preview.json"))
            .expect("local PDF fidelity checks must pass first"),
    )
    .unwrap();
    assert!(
        local["preview"]["fidelityIssues"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let f = Fixture::new();
    let (providers, source, before) = copy_authorization(&f);
    let models = Arc::new(
        HttpModelClient::new(Arc::new(PersonAiModelProviderClient::new(
            HttpModelProviderClient::new(
                eliteforge_agent_official_plugin_model_http::ModelHttpClient::new(),
            ),
            providers.clone(),
        )))
        .with_model_executor(providers.model_executor()),
    );
    let embedding_calls = Arc::new(AtomicUsize::new(0));
    let vision_calls = Arc::new(AtomicUsize::new(0));
    let measured = Arc::new(Mutex::new(vec![]));
    let llm = LlmRuntimeService::new(
        Arc::new(ProviderAwareLlmRuntimeResolver::new(
            f.state.models.clone(),
            providers.clone(),
        )),
        Arc::new(CappedRuntime {
            inner: HttpLlmRuntimeClient::with_model_client(models.clone()),
            calls: embedding_calls.clone(),
            vision: vision_calls.clone(),
            models: measured.clone(),
        }),
    );
    let service = KnowledgeBaseService::new(
        SqliteKnowledgeBaseRepository::connect(&f.state.config.database_url).unwrap(),
        f.state.rag.clone(),
        f.state.resources.clone(),
        f.state.agent_management.clone(),
        llm,
    )
    .with_document_parser(Arc::new(ReprocessedParse))
    .with_visual_context(true)
    .with_image_descriptions(tron_backend::config::ImageDescriptionConfig {
        enabled: true,
        model_code: "PERSON_AI:FIXED:High:qwen3.8-flash".into(),
        max_images_per_document: 2,
        timeout_seconds: 30,
    });
    let kb = service
        .create(
            USER,
            "CCS approved real SDK validation",
            "Isolated temporary storage; only the explicitly approved PDF",
        )
        .unwrap();
    let doc = service
        .upload(
            USER,
            &kb.id,
            NAME,
            &std::fs::read(Path::new(ROOT).join("source.pdf")).unwrap(),
        )
        .unwrap();
    service
        .configure(
            USER,
            &kb.id,
            &doc.id,
            Some(ChunkConfig {
                mode: ChunkMode::Fixed,
                max_chars: 8000,
                overlap_chars: 0,
                heading_depth: 2,
            }),
        )
        .unwrap();
    service.preview(USER, &kb.id, &doc.id).await.unwrap();
    service
        .reparse_visual_document(USER, &kb.id, &doc.id)
        .await
        .unwrap();
    let ready = service.index(USER, &kb.id, &doc.id).await.unwrap();
    let mut report = serde_json::json!({"status":"running","sourceSha256":"9155eab808f17ec894a106a3a91d8396b1f9e388b86db2a5960882a413d37be4","document":ready,"productionWrites":false,"globalVisualFlagChanged":false,"qa":[]});
    let report_path = Path::new(EVIDENCE).join("ccs-real-sdk-visual-qa.json");
    std::fs::write(&report_path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    assert_eq!(
        ready.status,
        "READY",
        "{}",
        ready.error.as_deref().unwrap_or("indexing incomplete")
    );
    assert_eq!(ready.embedding_dimension, Some(1024));
    assert!(
        service
            .search_for_agent(USER, WORKSPACE, &f.agent, "RCRI", 8000)
            .await
            .unwrap()
            .hits
            .is_empty()
    );
    service
        .set_grants(USER, &kb.id, vec![f.agent.clone()])
        .unwrap();
    let host = FrameworkHost::temporary(Some(f.state.resources.clone())).unwrap();
    host.bind_knowledge_bases(service.clone()).unwrap();
    let requests = Arc::new(Mutex::new(vec![]));
    let qa_calls = Arc::new(AtomicUsize::new(0));
    let observed = Arc::new(ObservedModel {
        inner: models.clone(),
        records: requests.clone(),
        calls: qa_calls.clone(),
    });
    host.bind_model_client(observed.clone()).unwrap();
    let harness = ReactAgentBuilder::new(
        observed,
        Arc::new(ToolRegistry::default()),
        EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    let mut chat = providers
        .resolve_chat_selection(WORKSPACE, None, Some("qwen3.8-flash"), None)
        .unwrap()
        .unwrap();
    chat.max_tokens = Some(2048);
    chat.max_retries = Some(0);
    chat.temperature = Some(0.0);
    let questions = [
        "From Table 2, what is the pooled 30-day risk for the highest RCRI group? State the actual comparator, risk estimate and 95% CI. Cite the document ID and PDF page.",
        "What is the preoperative serum creatinine threshold in Table 1? Compare the NT-proBNP unit printed in Figure 1 with Table 3, preserving any source discrepancy. Give exact values, units, document ID and PDF pages.",
        "Follow Figure 1's arrows for elective surgery when natriuretic peptide testing is unavailable: which monitoring and care actions follow? Distinguish this from the negative biomarker branch. Cite the document ID and PDF page. Do not infer unseen arrow directions.",
    ];
    for (i, q) in questions.iter().enumerate() {
        let ctx:AgentRunContext=serde_json::from_value(serde_json::json!({"agentCode":f.agent,"conversationCode":format!("ccs-live-{i}"),"runCode":format!("ccs-live-{i}"),"workspaceCode":WORKSPACE,"userCode":USER,"locale":"en-US","agentKind":"REACT","model":chat,"variables":[],"skills":[],"tools":[],"executionScope":{},"limits":{"maxSteps":1,"maxToolCalls":1,"maxParallelToolCalls":1},"messages":[{"role":"user","parts":[{"type":"text","text":q}]}],"observations":[],"startedAt":"CCS authorized validation"})).unwrap();
        let result = tokio::time::timeout(std::time::Duration::from_secs(90), harness.run(ctx))
            .await
            .expect("CCS QA timeout")
            .expect("native SDK QA failed");
        let record = requests.lock().unwrap().last().cloned().unwrap();
        report["qa"].as_array_mut().unwrap().push(serde_json::json!({"question":q,"answer":result.final_output,"sdkExecution":result.sdk_execution,"admittedRequest":record}));
        report["embeddingCalls"] = serde_json::json!(embedding_calls.load(Ordering::SeqCst));
        report["imageDescriptionCalls"] = serde_json::json!(vision_calls.load(Ordering::SeqCst));
        report["qaCalls"] = serde_json::json!(qa_calls.load(Ordering::SeqCst));
        std::fs::write(&report_path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    service.set_grants(USER, &kb.id, vec![]).unwrap();
    let revoked = service
        .search_for_agent(USER, WORKSPACE, &f.agent, "RCRI", 8000)
        .await
        .unwrap();
    assert!(revoked.hits.is_empty());
    let after=source.query_row("SELECT provider_code,server_base_url,status,credentials_encrypted,authorized_at,authorization_expires_at,last_sync_at,last_error,updated_at,person_ai_user_id,person_ai_username,created_at FROM provider_connections WHERE provider_code='PersonAi'",[],|r|(0..12).map(|i|r.get(i)).collect::<Result<Vec<rusqlite::types::Value>,_>>()).unwrap();
    assert!(before == after, "original authorization metadata changed");
    report["status"] = serde_json::json!("completed_pending_source_answer_review");
    report["observedEmbedding"] = serde_json::json!(*measured.lock().unwrap());
    report["revokedHitsEmpty"] = serde_json::json!(true);
    std::fs::write(&report_path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    assert!(host.close().await.is_success());
    models.close().await;
    for qa in report["qa"].as_array().unwrap() {
        assert!(
            qa["admittedRequest"]["mediaCount"].as_u64().unwrap() > 0,
            "model did not receive an image; see persisted report"
        );
        assert!(
            qa["answer"].as_str().unwrap().contains(&doc.id),
            "answer lacks document ID citation; see persisted report"
        );
    }
}

/// Exercise admitted raw media through the real Agent loop without publishing the known-bad parsed KB.
pub async fn run_raw_images() {
    use base64::Engine;
    use sha2::{Digest, Sha256};
    assert_eq!(std::env::var("TRON_CCS_PDF_LIVE").as_deref(), Ok("1"));
    let f = Fixture::new();
    let (providers, source, before) = copy_authorization(&f);
    let models = Arc::new(
        HttpModelClient::new(Arc::new(PersonAiModelProviderClient::new(
            HttpModelProviderClient::new(
                eliteforge_agent_official_plugin_model_http::ModelHttpClient::new(),
            ),
            providers.clone(),
        )))
        .with_model_executor(providers.model_executor()),
    );
    let host = FrameworkHost::temporary(Some(f.state.resources.clone())).unwrap();
    let records = Arc::new(Mutex::new(vec![]));
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = Arc::new(ObservedModel {
        inner: models.clone(),
        records: records.clone(),
        calls: calls.clone(),
    });
    host.bind_model_client(observed.clone()).unwrap();
    let harness = ReactAgentBuilder::new(
        observed,
        Arc::new(ToolRegistry::default()),
        EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    let mut chat = providers
        .resolve_chat_selection(WORKSPACE, None, Some("qwen3.8-flash"), None)
        .unwrap()
        .unwrap();
    chat.max_tokens = Some(1024);
    chat.max_retries = Some(0);
    chat.temperature = Some(0.0);
    chat.context_window_tokens = Some(128000);
    let parsed = OfficialBasicParse
        .parse(
            NAME,
            &std::fs::read(Path::new(ROOT).join("source.pdf")).unwrap(),
            None,
        )
        .await
        .unwrap();
    let checks = [
        (
            "images/page_4_table_5.jpg",
            "Read only the attached original Table 1 image. Transcribe the preoperative serum creatinine threshold exactly, including the comparison sign, numerical value and SI unit. Preserve the printed unit. Say when unreadable; do not use a text extraction or outside medical knowledge.",
        ),
        (
            "images/page_4_table_15.jpg",
            "Read only the attached original Table 2 image. Transcribe the last Total RCRI points group including the exact comparator, the risk estimate, and its full 95% CI. Preserve every number and sign. Say when unreadable.",
        ),
    ];
    let path = Path::new(EVIDENCE).join("ccs-real-sdk-raw-image-qa.json");
    let mut report = serde_json::json!({"status":"running","scope":"two approved original table crops via native Framework Agent; no KB publication or RAG answer claim","sourceSha256":"9155eab808f17ec894a106a3a91d8396b1f9e388b86db2a5960882a413d37be4","productionWrites":false,"globalVisualFlagChanged":false,"embeddingCalls":0,"qa":[]});
    for (i, (asset_path, q)) in checks.iter().enumerate() {
        let asset = parsed
            .assets
            .iter()
            .find(|a| a.path == *asset_path)
            .unwrap();
        let ctx:AgentRunContext=serde_json::from_value(serde_json::json!({"agentCode":f.agent,"conversationCode":format!("ccs-raw-{i}"),"runCode":format!("ccs-raw-{i}"),"workspaceCode":WORKSPACE,"userCode":USER,"locale":"en-US","agentKind":"REACT","model":chat,"variables":[],"skills":[],"tools":[],"executionScope":{},"limits":{"maxSteps":1,"maxToolCalls":1,"maxParallelToolCalls":1},"messages":[{"role":"user","parts":[{"type":"text","text":q},{"type":"image","mime_type":"image/jpeg","data":base64::engine::general_purpose::STANDARD.encode(&asset.bytes)}]}],"observations":[],"startedAt":"CCS authorized original-image validation"})).unwrap();
        let outcome =
            tokio::time::timeout(std::time::Duration::from_secs(90), harness.run(ctx)).await;
        let admitted = records.lock().unwrap().last().cloned();
        match outcome{
   Ok(Ok(result))=>report["qa"].as_array_mut().unwrap().push(serde_json::json!({"question":q,"imagePath":asset_path,"imageBytesSha256":format!("{:x}",Sha256::digest(&asset.bytes)),"imageBytes":asset.bytes.len(),"answer":result.final_output,"sdkExecution":result.sdk_execution,"admittedRequest":admitted})),
   other=>{report["status"]=serde_json::json!("FAILED_NATIVE_IMAGE_RUN");report["error"]=serde_json::json!(format!("{other:?}"));std::fs::write(&path,serde_json::to_vec_pretty(&report).unwrap()).unwrap();let _=host.close().await;models.close().await;panic!("native image validation failed; see persisted evidence");}
  }
        std::fs::write(&path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    let after=source.query_row("SELECT provider_code,server_base_url,status,credentials_encrypted,authorized_at,authorization_expires_at,last_sync_at,last_error,updated_at,person_ai_user_id,person_ai_username,created_at FROM provider_connections WHERE provider_code='PersonAi'",[],|r|(0..12).map(|i|r.get(i)).collect::<Result<Vec<rusqlite::types::Value>,_>>()).unwrap();
    assert_eq!(before, after, "original auth changed");
    report["chatCalls"] = serde_json::json!(calls.load(Ordering::SeqCst));
    report["authorizationUnchanged"] = serde_json::json!(true);
    report["status"] = serde_json::json!("COMPLETED_PENDING_SOURCE_REVIEW");
    std::fs::write(&path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    assert!(host.close().await.is_success());
    models.close().await;
    for qa in report["qa"].as_array().unwrap() {
        assert_eq!(
            qa["admittedRequest"]["mediaCount"], 1,
            "native request did not retain exact original media"
        );
    }
    let a = report["qa"][0]["answer"].as_str().unwrap();
    assert!(
        a.contains("177") && (a.contains("μmol") || a.contains("µmol")),
        "micro-unit failed; persisted actual answer"
    );
    let a = report["qa"][1]["answer"].as_str().unwrap();
    assert!(
        a.contains("≥") && a.contains("15.0") && a.contains("11.1") && a.contains("20.0"),
        "table comparator/value failed; persisted actual answer"
    );
    report["status"] = serde_json::json!("PASS_NATIVE_RAW_IMAGE_QA_ONLY");
    std::fs::write(&path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
}
