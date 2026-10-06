mod common;
#[path = "support/knowledge_base_fixture.rs"]
mod fixture;
use eliteforge_agent_core as fw;
use fixture::*;
use rusqlite::{Connection, OpenFlags, params, params_from_iter, types::Value};
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
type EmbeddingFuture<'a> =
    Pin<Box<dyn Future<Output = Result<(Vec<f32>, Option<String>), String>> + Send + 'a>>;
struct CappedClient {
    inner: HttpLlmRuntimeClient,
    calls: Arc<AtomicUsize>,
    observed: Arc<Mutex<Vec<(usize, Option<String>)>>>,
}
impl CappedClient {
    fn admit(&self) -> Result<(), String> {
        if self.calls.fetch_add(1, Ordering::SeqCst) >= 2 {
            Err("synthetic verification call cap reached".into())
        } else {
            Ok(())
        }
    }
    fn record(&self, result: &Result<(Vec<f32>, Option<String>), String>) {
        if let Ok((v, m)) = result {
            self.observed.lock().unwrap().push((v.len(), m.clone()));
        }
    }
}
impl LlmRuntimeClient for CappedClient {
    fn embed_with_model<'a>(
        &'a self,
        c: &'a RuntimeModelConfig,
        s: &'a str,
    ) -> EmbeddingFuture<'a> {
        Box::pin(async move {
            self.admit()?;
            let r = tokio::time::timeout(
                std::time::Duration::from_secs(45),
                self.inner.embed_with_model(c, s),
            )
            .await
            .map_err(|_| "synthetic embedding timeout".to_owned())?;
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
    ) -> EmbeddingFuture<'a> {
        Box::pin(async move {
            self.admit()?;
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(45),
                self.inner.embed_query_with_model_in_run(c, s, d, r),
            )
            .await
            .map_err(|_| "synthetic query timeout".to_owned())?;
            self.record(&result);
            result
        })
    }
    fn generate<'a>(
        &'a self,
        _: &'a RuntimeModelConfig,
        _: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async { Err("unused".into()) })
    }
    fn embed<'a>(
        &'a self,
        _: &'a RuntimeModelConfig,
        _: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<f32>, String>> + Send + 'a>> {
        Box::pin(async { Err("use identified embedding".into()) })
    }
    fn rerank<'a>(
        &'a self,
        _: &'a RuntimeModelConfig,
        _: &'a str,
        _: &'a [String],
        _: Option<usize>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<LlmRuntimeRerankHit>, String>> + Send + 'a>> {
        Box::pin(async { Err("unused".into()) })
    }
}
// Read only existing authorization metadata, never user documents. Secrets are never logged.
const COLUMNS: &str = "provider_code,server_base_url,status,credentials_encrypted,authorized_at,authorization_expires_at,last_sync_at,last_error,updated_at,person_ai_user_id,person_ai_username,created_at";
fn authorization_row(source: &Connection) -> Vec<Value> {
    source
        .query_row(
            &format!("SELECT {COLUMNS} FROM provider_connections WHERE provider_code='PersonAi'"),
            [],
            |row| (0..12).map(|i| row.get(i)).collect(),
        )
        .expect("existing authorization row unavailable")
}
#[derive(Default)]
struct Capture(Mutex<Vec<fw::ChatRequest>>);
impl ModelClient for Capture {
    fn generate<'a>(
        &'a self,
        _: &'a AgentModelConfig,
        _: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async { Ok(ModelResponse::final_output("done")) })
    }
    fn generate_once<'a>(
        &'a self,
        c: &'a AgentModelConfig,
        r: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        self.generate(c, r)
    }
    fn generate_native_once<'a>(
        &'a self,
        _: &'a AgentModelConfig,
        request: &'a fw::ChatRequest,
        _: &'a ModelRequest,
        _: Option<ModelStreamSink>,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async move {
            self.0.lock().unwrap().push(request.clone());
            Ok(ModelResponse::final_output("done"))
        })
    }
    fn close<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async {})
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
        Box::pin(async { Ok(Default::default()) })
    }
}
fn context(f: &Fixture, run: &str, identity: bool) -> AgentRunContext {
    serde_json::from_value(serde_json::json!({
        "agentCode": if identity {Some(&f.agent)} else {None}, "conversationCode":run, "runCode":run, "workspaceCode":WORKSPACE,"userCode":USER,"locale":"en-US","agentKind":"REACT",
        "model":{"modelCode":"fixture-chat","protocol":"CHAT_COMPLETIONS","baseUrl":"http://127.0.0.1:1/v1","apiKey":"synthetic","modelName":"fixture","maxTokens":128,"contextWindowTokens":8000},
        "variables":[],"skills":[],"tools":[],"executionScope":{},"limits":{"maxSteps":2,"maxToolCalls":2,"maxParallelToolCalls":1},"messages":[{"role":"user","parts":[{"type":"text","text":"ambience telemetry"}]}],"observations":[],"startedAt":"fixture"
    })).unwrap()
}

/// Opt-in only: two synthetic embedding calls over the existing authorized SDK channel.
/// All KB/resources/configuration writes are isolated in a temporary directory.
#[tokio::test]
#[ignore = "requires explicit authorization and TRON_KB_LIVE_SYNTHETIC=1"]
async fn authorized_singleton_route_real_vectors_reach_native_contributor_with_acl() {
    assert!(
        std::env::var("TRON_KB_LIVE_SYNTHETIC").as_deref() == Ok("1"),
        "explicit synthetic verification opt-in required"
    );
    let home = std::env::var_os("HOME").expect("home unavailable");
    let source = Connection::open_with_flags(
        std::path::Path::new(&home).join(".tron/tron.sqlite3"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .expect("read-only authorization source unavailable");
    let before = authorization_row(&source);
    assert!(
        matches!(&before[2],Value::Text(v) if v=="Connected"||v=="Degraded"),
        "existing connection is not ready"
    );
    assert!(
        matches!(&before[3],Value::Text(v) if !v.is_empty()),
        "existing credentials unavailable"
    );
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    assert!(
        matches!(&before[5],Value::Integer(expiry) if *expiry>now+600_000),
        "existing authorization too close to expiry for isolated verification"
    );
    let yaml: serde_yaml::Value = serde_yaml::from_str(
        &std::fs::read_to_string(std::path::Path::new(&home).join(".tron/application.yaml"))
            .expect("configured cipher unavailable"),
    )
    .unwrap_or_else(|_| panic!("configured YAML cannot be parsed"));
    let key = yaml
        .get("variableCrypto")
        .and_then(|v| v.get("keyBase64"))
        .and_then(|v| v.as_str())
        .expect("configured cipher key unavailable")
        .to_owned();
    let cipher = Arc::new(
        Sm4GcmVariableCipher::from_config(VariableCryptoConfig { key_base64: key })
            .expect("configured cipher invalid"),
    );
    let f = Fixture::new();
    let isolated = Connection::open(&f.state.config.database_url).unwrap();
    isolated.execute(&format!("INSERT OR REPLACE INTO provider_connections({COLUMNS}) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)"),params_from_iter(before.iter())).unwrap();
    let device: String = source
        .query_row(
            "SELECT device_id FROM person_ai_device_identity WHERE singleton_id=1",
            [],
            |r| r.get(0),
        )
        .expect("existing device identity unavailable");
    isolated
        .execute(
            "INSERT OR REPLACE INTO person_ai_device_identity(singleton_id,device_id) VALUES(1,?1)",
            [device],
        )
        .unwrap();
    // Preserve the current platform embedding selection, rather than inventing a model config.
    let selection:(String,String,String)=source.query_row("SELECT selection_kind,provider_code,route_code FROM default_model_selections WHERE scope_key='platform' AND model_type='EMBEDDING'",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).expect("existing embedding selection unavailable");
    assert!(
        selection
            == (
                "ProviderRoute".into(),
                "PersonAi".into(),
                "EmbeddingAuto".into()
            ),
        "existing selection is not the authorized singleton route"
    );
    isolated.execute("INSERT OR REPLACE INTO default_model_selections(scope_key,workspace_code,model_type,selection_kind,provider_code,route_code,created_at,updated_at) VALUES('platform',NULL,'EMBEDDING',?1,?2,?3,1,1)",params![selection.0,selection.1,selection.2]).unwrap();
    drop(isolated);
    let providers = ModelProviderService::connect(&f.state.config.database_url, cipher)
        .expect("isolated provider initialization failed");
    let models = Arc::new(
        HttpModelClient::new(Arc::new(PersonAiModelProviderClient::new(
            HttpModelProviderClient::new(
                eliteforge_agent_official_plugin_model_http::ModelHttpClient::new(),
            ),
            providers.clone(),
        )))
        .with_model_executor(providers.model_executor()),
    );
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = Arc::new(Mutex::new(vec![]));
    let client = CappedClient {
        inner: HttpLlmRuntimeClient::with_model_client(models.clone()),
        calls: calls.clone(),
        observed: observed.clone(),
    };
    let llm = LlmRuntimeService::new(
        Arc::new(ProviderAwareLlmRuntimeResolver::new(
            f.state.models.clone(),
            providers,
        )),
        Arc::new(client),
    );
    let service = KnowledgeBaseService::new(
        SqliteKnowledgeBaseRepository::connect(&f.state.config.database_url).unwrap(),
        f.state.rag.clone(),
        f.state.resources.clone(),
        f.state.workspaces.clone(),
        f.state.agent_management.clone(),
        llm,
    );
    let kb = service
        .create(USER, WORKSPACE, "Synthetic verification", "synthetic only")
        .unwrap();
    let doc = service
        .upload(
            USER,
            WORKSPACE,
            &kb.id,
            "synthetic.md",
            b"SYNTHETIC_VECTOR_PROOF imaginary archive cooling uses violet frost.",
        )
        .unwrap();
    let indexed = service
        .index(USER, WORKSPACE, &kb.id, &doc.id)
        .await
        .unwrap();
    let measurements = observed.lock().unwrap().clone();
    println!(
        "synthetic live indexing: status={}, calls={}, observed={}",
        indexed.status,
        calls.load(Ordering::SeqCst),
        serde_json::to_string(&measurements).unwrap()
    );
    assert_eq!(
        indexed.status, "READY",
        "real embedding could not publish a vector index; stop after first call"
    );
    assert_eq!(indexed.embedding_dimension, Some(1024));
    assert_eq!(indexed.embedding_provider_model.as_deref(), Some("bge-m3"));
    assert_eq!(indexed.chunk_count, 1);
    // Prove the sole query has no lexical hit, so successful injection requires vector retrieval.
    assert!(
        service
            .search_for_agent(USER, WORKSPACE, &f.agent, "ambience telemetry", 1200)
            .await
            .unwrap()
            .hits
            .is_empty()
    );
    let host = FrameworkHost::temporary(Some(f.state.resources.clone())).unwrap();
    host.bind_knowledge_bases(service.clone()).unwrap();
    let capture = Arc::new(Capture::default());
    host.bind_model_client(capture.clone()).unwrap();
    let harness = ReactAgentBuilder::new(
        capture.clone(),
        Arc::new(ToolRegistry::default()),
        EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    harness
        .run(context(&f, "synthetic-no-grants", true))
        .await
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    service
        .set_grants(USER, WORKSPACE, &kb.id, vec![f.agent.clone()])
        .unwrap();
    // Plain full-text-only API must still miss after grant; it sends no embedding request.
    assert!(
        service
            .search_for_agent(USER, WORKSPACE, &f.agent, "ambience telemetry", 1200)
            .await
            .unwrap()
            .hits
            .is_empty()
    );
    harness
        .run(context(&f, "synthetic-with-grant", true))
        .await
        .unwrap();
    service.set_grants(USER, WORKSPACE, &kb.id, vec![]).unwrap();
    harness
        .run(context(&f, "synthetic-revoked", true))
        .await
        .unwrap();
    service
        .set_grants(USER, WORKSPACE, &kb.id, vec![f.agent.clone()])
        .unwrap();
    harness
        .run(context(&f, "synthetic-no-identity", false))
        .await
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    let texts: Vec<String> = capture
        .0
        .lock()
        .unwrap()
        .iter()
        .map(|r| {
            r.messages
                .iter()
                .map(|m| m.content.text())
                .collect::<Vec<_>>()
                .join("\n")
        })
        .collect();
    assert_eq!(texts.len(), 4);
    assert!(!texts[0].contains("SYNTHETIC_VECTOR_PROOF"));
    assert!(
        texts[1].contains("SYNTHETIC_VECTOR_PROOF"),
        "authorized native contributor did not receive real vector result"
    );
    assert!(!texts[2].contains("SYNTHETIC_VECTOR_PROOF"));
    assert!(!texts[3].contains("SYNTHETIC_VECTOR_PROOF"));
    assert!(
        authorization_row(&source) == before,
        "source authorization changed during isolated verification"
    );
    println!(
        "synthetic live vector/contributor: calls=2; grant=true; revoke=true; missing_identity=true; observed={}",
        serde_json::to_string(&*observed.lock().unwrap()).unwrap()
    );
    assert!(host.close().await.is_success());
    models.close().await;
}

/// Verify real HTTP-created MinerU docs through a native SDK loop on a copied DB.
/// Only one synthetic query embedding; model generation is captured locally.
#[tokio::test]
#[ignore="requires TRON_KB_REAL_MINERU_CONTRIBUTOR=1 after synthetic HTTP ingestion"]
async fn real_http_mineru_resources_reach_native_contributor_with_revocation() {
    assert_eq!(std::env::var("TRON_KB_REAL_MINERU_CONTRIBUTOR").as_deref(),Ok("1"));
    use tron_backend::{app::AppState,config::AppConfig,application::profile::ProfileRegistryService,shared::id::FixedSnowflakeIdProvider};
    let evidence=std::path::Path::new("/Users/clouds3n/Documents/Codex/2026-10-03/task/knowledge-base-implementation-evidence");
    let report:serde_json::Value=serde_json::from_slice(&std::fs::read(evidence.join("kb-local-http-results.json")).unwrap()).unwrap();
    assert_eq!(report["finalGrantsRevoked"],true);
    let kb_id=report["knowledgeBaseId"].as_str().unwrap();let workspace=report["workspace"].as_str().unwrap();let agent=report["syntheticGrantAgent"].as_str().unwrap();
    let isolated=tempfile::tempdir().unwrap();let database=isolated.path().join("native.sqlite3");
    assert!(std::process::Command::new("python3").arg("-c").arg("import sqlite3,sys,os; a=sqlite3.connect('file:/Users/clouds3n/.tron/tron.sqlite3?mode=ro',uri=True); b=sqlite3.connect(sys.argv[1]); a.backup(b); b.close(); a.close(); os.chmod(sys.argv[1],0o600)").arg(&database).status().unwrap().success());
    let owner:String=Connection::open(&database).unwrap().query_row("SELECT owner_user_code FROM knowledge_bases WHERE id=?1",[kb_id],|r|r.get(0)).unwrap();
    let config_path=isolated.path().join("application.yaml");std::fs::copy("/Users/clouds3n/.tron/application.yaml",&config_path).unwrap();
    let mut config=AppConfig::from_yaml_file(&config_path).unwrap();config.database_url=database.display().to_string();config.graph_storage_path=":memory:".into();config.skills_registry_root=isolated.path().join("skills").display().to_string();
    let profiles=ProfileRegistryService::new(":memory:").unwrap();profiles.register_or_update(&owner,"synthetic-validation").unwrap();let profile=profiles.list().unwrap().remove(0);profiles.activate(&profile.profile_id).unwrap();
    let state=AppState::new_with_profiles(config,Arc::new(FixedSnowflakeIdProvider::new(791000000000000000)),profiles).unwrap();
    let providers=state.model_providers.clone();
    let models=Arc::new(HttpModelClient::new(Arc::new(PersonAiModelProviderClient::new(HttpModelProviderClient::new(eliteforge_agent_official_plugin_model_http::ModelHttpClient::new()),providers.clone()))).with_model_executor(providers.model_executor()));
    let calls=Arc::new(AtomicUsize::new(0));let observed=Arc::new(Mutex::new(vec![]));
    let llm=LlmRuntimeService::new(Arc::new(ProviderAwareLlmRuntimeResolver::new(state.models.clone(),providers)),Arc::new(CappedClient{inner:HttpLlmRuntimeClient::with_model_client(models.clone()),calls:calls.clone(),observed:observed.clone()}));
    let service=KnowledgeBaseService::new(SqliteKnowledgeBaseRepository::connect(&state.config.database_url).unwrap(),state.rag.clone(),state.resources.clone(),state.workspaces.clone(),state.agent_management.clone(),llm);
    let host=FrameworkHost::temporary(Some(state.resources.clone())).unwrap();host.bind_knowledge_bases(service.clone()).unwrap();let capture=Arc::new(Capture::default());host.bind_model_client(capture.clone()).unwrap();
    let harness=ReactAgentBuilder::new(capture.clone(),Arc::new(ToolRegistry::default()),EventBusHandle::disabled()).with_framework_host(Some(host.clone())).build().unwrap();
    let context=|run:&str,identity:bool|serde_json::from_value(serde_json::json!({"agentCode":if identity{Some(agent)}else{None},"conversationCode":run,"runCode":run,"workspaceCode":workspace,"userCode":owner,"locale":"en-US","agentKind":"REACT","model":{"modelCode":"fixture-chat","protocol":"CHAT_COMPLETIONS","baseUrl":"http://127.0.0.1:1/v1","apiKey":"synthetic","modelName":"fixture","maxTokens":128,"contextWindowTokens":8000},"variables":[],"skills":[],"tools":[],"executionScope":{},"limits":{"maxSteps":2,"maxToolCalls":2,"maxParallelToolCalls":1},"messages":[{"role":"user","parts":[{"type":"text","text":"diagram pipeline before search"}]}],"observations":[],"startedAt":"synthetic"})).unwrap();
    harness.run(context("real-mineru-no-grant",true)).await.unwrap();assert_eq!(calls.load(Ordering::SeqCst),0);
    service.set_grants(&owner,workspace,kb_id,vec![agent.into()]).unwrap();
    let details=service.detail(&owner,workspace,kb_id).unwrap();let images=details.documents.iter().filter_map(|d|d.parsing_artifact.as_ref()).flat_map(|p|p.image_resource_codes.iter().cloned()).collect::<Vec<_>>();assert!(!images.is_empty());
    for code in &images {assert!(state.resources.for_agent(Some(agent)).download_binary_to_file(&owner,tron_backend::application::resource::ResourceBinaryDownloadCommand{workspace_code:workspace.into(),resource_code:code.clone()}).is_ok());}
    harness.run(context("real-mineru-granted",true)).await.unwrap();assert_eq!(calls.load(Ordering::SeqCst),1);
    service.set_grants(&owner,workspace,kb_id,vec![]).unwrap();
    for code in &images {assert!(state.resources.for_agent(Some(agent)).download_binary_to_file(&owner,tron_backend::application::resource::ResourceBinaryDownloadCommand{workspace_code:workspace.into(),resource_code:code.clone()}).is_err());}
    harness.run(context("real-mineru-revoked",true)).await.unwrap();service.set_grants(&owner,workspace,kb_id,vec![agent.into()]).unwrap();harness.run(context("real-mineru-missing-identity",false)).await.unwrap();assert_eq!(calls.load(Ordering::SeqCst),1);
    let texts=capture.0.lock().unwrap().iter().map(|r|r.messages.iter().map(|m|m.content.text()).collect::<Vec<_>>().join("\n")).collect::<Vec<_>>();assert_eq!(texts.len(),4);
    assert!(!texts[0].contains("ORBIT REPORT"));assert!(texts[1].contains("Alpha") && texts[1].contains("resource:KB_"),"real structured/image evidence did not reach native model request");assert!(!texts[2].contains("ORBIT REPORT"));assert!(!texts[3].contains("ORBIT REPORT"));
    std::fs::write(evidence.join("kb-real-native-contributor.json"),serde_json::to_vec_pretty(&serde_json::json!({"source":"actual HTTP-created synthetic MinerU KB, isolated online copy for grants/runtime","queryEmbeddingCalls":calls.load(Ordering::SeqCst),"observedEmbedding":*observed.lock().unwrap(),"nativeCaptureRequests":texts.len(),"grantedStructuredTextAndImageReferences":true,"revokedNoContext":true,"missingIdentityNoContext":true,"derivedImageAclGrantAndRevoke":true,"imagesChecked":images.len(),"defaultKbFinalGrants":"revoked; isolated grant tests do not mutate default DB"})).unwrap()).unwrap();
    assert!(host.close().await.is_success());state.agent_framework.close().await;models.close().await;
}
