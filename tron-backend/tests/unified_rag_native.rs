//! Opt-in verification of the fresh unified schema with native MinerU and configured models.
mod common;
#[path = "support/knowledge_base_fixture.rs"]
mod fixture;
use fixture::*;
use rusqlite::{Connection, OpenFlags, params, params_from_iter, types::Value};
use std::sync::{Arc, Mutex};
use tron_backend::{
    adapters::outbound::{
        crypto::Sm4GcmVariableCipher,
        mineru_documents::MineruKnowledgeDocumentParser,
        model::{
            HttpLlmRuntimeClient, HttpModelClient, HttpModelProviderClient,
            PersonAiModelProviderClient,
        },
        persistence::sqlite::{SqliteKnowledgeBaseRepository, SqliteRagRepository},
    },
    application::{
        knowledge_base::KnowledgeBaseService,
        llm::LlmRuntimeService,
        model_provider::{ModelProviderService, ProviderAwareLlmRuntimeResolver},
        rag::*,
        resource::ResourceBinaryDownloadCommand,
    },
    config::{AppConfig, VariableCryptoConfig},
};
const COLUMNS: &str = "provider_code,server_base_url,status,credentials_encrypted,authorized_at,authorization_expires_at,last_sync_at,last_error,updated_at,person_ai_user_id,person_ai_username,created_at";
#[tokio::test]
#[ignore = "requires TRON_UNIFIED_RAG_NATIVE_PDF pointing to a selected synthetic PDF and existing model authorization"]
async fn native_pdf_uses_the_unified_indices_and_evidence_locations() {
    let path = std::path::PathBuf::from(
        std::env::var("TRON_UNIFIED_RAG_NATIVE_PDF").expect("selected synthetic PDF required"),
    );
    assert!(
        path.starts_with("/tmp"),
        "use an isolated synthetic document"
    );
    let bytes = std::fs::read(&path).unwrap();
    assert!(bytes.len() < 1024 * 1024);
    let home = std::env::var_os("HOME").expect("home unavailable");
    let source = Connection::open_with_flags(
        std::path::Path::new(&home).join(".tron/tron.sqlite3"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .expect("read-only authorization source unavailable");
    let before: Vec<Value> = source
        .query_row(
            &format!("SELECT {COLUMNS} FROM provider_connections WHERE provider_code='PersonAi'"),
            [],
            |row| (0..12).map(|i| row.get(i)).collect(),
        )
        .unwrap();
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

    let llm = LlmRuntimeService::new(
        Arc::new(ProviderAwareLlmRuntimeResolver::new(
            f.state.models.clone(),
            providers,
        )),
        Arc::new(HttpLlmRuntimeClient::with_model_client(models)),
    );
    let rag = RagService::new(
        Arc::new(Mutex::new(
            SqliteRagRepository::connect(&f.state.config.database_url).unwrap(),
        )),
        DefaultRagRuntime::new(llm.clone(), f.state.llm_router.clone()),
    );
    let config =
        AppConfig::from_yaml_file(std::path::Path::new(&home).join(".tron/application.yaml"))
            .unwrap();
    assert!(
        config.mineru.nodes.is_empty() && config.mineru.base_url.starts_with("http://127.0.0.1:"),
        "local MinerU only"
    );
    let service = KnowledgeBaseService::new(
        SqliteKnowledgeBaseRepository::connect(&f.state.config.database_url).unwrap(),
        rag.clone(),
        f.state.resources.clone(),
        f.state.agent_management.clone(),
        llm,
    )
    .with_document_parser(Arc::new(MineruKnowledgeDocumentParser::new(config.mineru)))
    .with_visual_context(true);
    let kb = service
        .create(USER, "Unified native acceptance", "synthetic PDF only")
        .unwrap();
    let doc = service
        .upload(USER, &kb.id, "unified-rag-proof.pdf", &bytes)
        .unwrap();
    let indexed = tokio::time::timeout(
        std::time::Duration::from_secs(360),
        service.index(USER, &kb.id, &doc.id),
    )
    .await
    .expect("native parse/index timeout")
    .unwrap();
    println!("Native parse artifact: {:?}", indexed.parsing_artifact);
    println!(
        "Native document status={} chunks={} error={:?}",
        indexed.status, indexed.chunk_count, indexed.error
    );
    if let Some(artifact) = &indexed.parsing_artifact {
        let (_, record) = f
            .state
            .resources
            .download_binary_to_file(
                USER,
                ResourceBinaryDownloadCommand {
                    workspace_code: "PLATFORM".into(),
                    resource_code: artifact.resource_code.clone(),
                },
            )
            .unwrap();
        let mut data = vec![];
        f.state
            .resources
            .download_binary_storage(&record, &mut data)
            .unwrap();
        std::fs::write("/tmp/tron-rag-native-parse.json", data).unwrap();
    }
    assert_eq!(indexed.status, "READY");
    assert!(indexed.chunk_count >= 2);
    assert_eq!(indexed.embedding_dimension, Some(1024));
    let counts = rag
        .corpus_summaries(&std::collections::BTreeSet::from([kb.id.clone()]))
        .unwrap()
        .into_iter()
        .find(|c| c.id == kb.id)
        .unwrap()
        .counts;
    assert_eq!(counts.chunk_count, indexed.chunk_count);
    assert_eq!(counts.vector_count, indexed.chunk_count);
    service
        .set_grants(USER, &kb.id, vec![f.agent.clone()])
        .unwrap();
    let query = rag
        .search(
            &service,
            &RagIdentity {
                user_code: USER.into(),
                workspace_code: WORKSPACE.into(),
                agent_code: f.agent.clone(),
            },
            RagQueryInput {
                query: "Blue Harbor archive retention".into(),
                corpus_ids: vec![kb.id.clone()],
                filters: serde_json::json!({}),
                limit: 8,
            },
            None,
        )
        .await
        .unwrap();
    assert!(
        query
            .stages
            .iter()
            .any(|s| s.channel == "vector" && s.status == "COMPLETED" && s.hit_count > 0),
        "{:#?}",
        query.stages
    );
    assert!(
        query
            .stages
            .iter()
            .any(|s| s.channel == "fulltext" && s.status == "COMPLETED" && s.hit_count > 0)
    );
    assert!(
        query
            .evidence
            .iter()
            .any(|h| h.location["page"] == 1 && h.text.contains("27"))
    );
    let hits = service
        .search_for_agent(
            USER,
            WORKSPACE,
            &f.agent,
            "Blue Harbor archive retention",
            16000,
        )
        .await
        .unwrap()
        .hits;
    println!(
        "Native hit locations: {:?}",
        hits.iter().map(|h| &h.location).collect::<Vec<_>>()
    );
    let visual = service
        .visual_evidence_for_agent(USER, WORKSPACE, &f.agent, &hits)
        .unwrap();
    assert!(
        !visual.evidence.is_empty(),
        "real document image references must resolve: {:?}",
        visual.warnings
    );
    assert!(
        visual
            .evidence
            .iter()
            .all(|e| !e.bytes.is_empty() && e.source_reference.contains("页"))
    );
    let (_, original) = f
        .state
        .resources
        .download_binary_to_file(
            USER,
            ResourceBinaryDownloadCommand {
                workspace_code: "PLATFORM".into(),
                resource_code: doc.resource_code,
            },
        )
        .unwrap();
    let mut actual = vec![];
    f.state
        .resources
        .download_binary_storage(&original, &mut actual)
        .unwrap();
    assert_eq!(actual, bytes);
    service.set_grants(USER, &kb.id, vec![]).unwrap();
    assert!(
        service
            .search_for_agent(USER, WORKSPACE, &f.agent, "Blue Harbor", 1200)
            .await
            .unwrap()
            .hits
            .is_empty()
    );
    assert!(
        service
            .visual_evidence_for_agent(USER, WORKSPACE, &f.agent, &hits)
            .is_err()
    );
    assert!(
        rag.search(
            &service,
            &RagIdentity {
                user_code: USER.into(),
                workspace_code: WORKSPACE.into(),
                agent_code: f.agent.clone()
            },
            RagQueryInput {
                query: "Blue Harbor".into(),
                corpus_ids: vec![kb.id],
                filters: serde_json::json!({}),
                limit: 8
            },
            None
        )
        .await
        .is_err()
    );
    assert_eq!(
        before,
        source
            .query_row(
                &format!(
                    "SELECT {COLUMNS} FROM provider_connections WHERE provider_code='PersonAi'"
                ),
                [],
                |row| (0..12)
                    .map(|i| row.get::<_, Value>(i))
                    .collect::<Result<Vec<_>, _>>()
            )
            .unwrap()
    );
    println!(
        "Native acceptance passed: common chunks/vector indices, original bytes, page and image evidence, grant revocation"
    );
    f.state.agent_framework.close().await;
}
