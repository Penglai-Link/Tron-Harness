mod common;
#[path = "support/knowledge_base_fixture.rs"]
mod fixture;

use fixture::*;
use rusqlite::{Connection, params};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tron_backend::{
    adapters::outbound::persistence::sqlite::SqliteKnowledgeBaseRepository,
    application::{
        knowledge_base::{
            KnowledgeBaseService,
            local_documents::{ChunkConfig, ChunkMode},
        },
        llm::{LlmRuntimeModelResolver, LlmRuntimeService},
        model::ModelCommand,
        model_provider::ProviderAwareLlmRuntimeResolver,
    },
    domain::model::{ModelProtocol, ModelType},
};

const AUTO: &str = "PERSON_AI:ROUTE:EmbeddingAuto";

// All configuration writes below target the fixture's temporary database, never ~/.tron.
fn connect_auto(f: &Fixture) {
    Connection::open(&f.state.config.database_url).unwrap().execute_batch(
        "INSERT INTO provider_connections(provider_code,server_base_url,status,created_at,updated_at)
         VALUES('PersonAi','https://synthetic.invalid','Connected',1,1)
         ON CONFLICT(provider_code) DO UPDATE SET status='Connected';
         INSERT OR REPLACE INTO default_model_selections
         (scope_key,workspace_code,model_type,selection_kind,provider_code,route_code,created_at,updated_at)
         VALUES('platform',NULL,'EMBEDDING','ProviderRoute','PersonAi','EmbeddingAuto',1,1);",
    ).unwrap();
}

fn fixed_selection(f: &Fixture, scope: &str, code: &str) {
    Connection::open(&f.state.config.database_url)
        .unwrap()
        .execute(
            "INSERT OR REPLACE INTO default_model_selections
         (scope_key,workspace_code,model_type,selection_kind,model_code,created_at,updated_at)
         VALUES(?1,?1,'EMBEDDING','Fixed',?2,1,1)",
            params![scope, code],
        )
        .unwrap();
}

fn resolver(f: &Fixture) -> ProviderAwareLlmRuntimeResolver {
    ProviderAwareLlmRuntimeResolver::new(f.state.models.clone(), f.state.model_providers.clone())
}

fn service(f: &Fixture, llm: LlmRuntimeService) -> KnowledgeBaseService {
    KnowledgeBaseService::new(
        SqliteKnowledgeBaseRepository::connect(&f.state.config.database_url).unwrap(),
        f.state.rag.clone(),
        f.state.resources.clone(),
        f.state.agent_management.clone(),
        llm,
    )
}

fn run() -> eliteforge_agent_core::RunContext {
    eliteforge_agent_core::ModelInvocationOptions::for_call(
        &eliteforge_agent_core::RunLimits::default(),
    )
    .unwrap()
    .run
    .unwrap()
}

#[test]
fn explicit_embedding_route_obeys_workspace_selection_and_connection_state() {
    let f = Fixture::new();
    connect_auto(&f);
    let r = resolver(&f);
    for code in [None, Some(AUTO), Some(" PERSON_AI:ROUTE:EmbeddingAuto ")] {
        assert_eq!(
            r.embedding_runtime_model_config(WORKSPACE, code)
                .unwrap()
                .unwrap()
                .model_code,
            AUTO
        );
    }
    assert!(
        r.embedding_runtime_model_config(WORKSPACE, Some("PERSON_AI:ROUTE:Unknown"))
            .unwrap()
            .is_none()
    );
    // Platform-owned libraries resolve the platform default directly.
    assert_eq!(r.embedding_runtime_model_config("PLATFORM", None).unwrap().unwrap().model_code, AUTO);
    fixed_selection(&f, WORKSPACE, "missing-fixed-model");
    assert!(
        r.embedding_runtime_model_config(WORKSPACE, Some(AUTO))
            .unwrap()
            .is_none()
    );
    fixed_selection(&f, "nonexistent-workspace", "missing-fixed-model");
    assert!(
        r.embedding_runtime_model_config("nonexistent-workspace", Some(AUTO))
            .is_err()
    );
    Connection::open(&f.state.config.database_url)
        .unwrap()
        .execute(
            "DELETE FROM default_model_selections WHERE scope_key=?1 AND model_type='EMBEDDING'",
            [WORKSPACE],
        )
        .unwrap();
    // Removing the workspace override restores the existing platform fallback.
    assert!(
        r.embedding_runtime_model_config(WORKSPACE, Some(AUTO))
            .unwrap()
            .is_some()
    );
    Connection::open(&f.state.config.database_url)
        .unwrap()
        .execute(
            "UPDATE provider_connections SET status='AuthRequired' WHERE provider_code='PersonAi'",
            [],
        )
        .unwrap();
    assert!(
        r.embedding_runtime_model_config(WORKSPACE, Some(AUTO))
            .is_err()
    );
    assert!(r.embedding_runtime_model_config(WORKSPACE, None).is_err());
}

#[tokio::test]
async fn real_resolver_pins_fixed_space_for_multiple_chunks_rebuild_and_queries() {
    let f = Fixture::new();
    connect_auto(&f);
    let model = f
        .state
        .models
        .create_platform_model(ModelCommand {
            display_name: "Synthetic stable embedding".into(),
            vendor: "Synthetic".into(),
            protocol: ModelProtocol::OpenAiCompatibleEmbedding,
            model_type: ModelType::Embedding,
            model_capabilities: vec![],
            base_url: "https://synthetic.invalid/v1".into(),
            api_path: Some("embeddings".into()),
            api_key_ref: "synthetic-only".into(),
            model_name: "fixed-embedding-v1".into(),
            enabled: true,
            description: None,
            thinking_enabled: false,
            supported_reasoning_efforts: vec![],
            default_reasoning_effort: None,
            structured_output_enabled: false,
            responses_stateful_enabled: false,
            temperature: None,
            max_tokens: None,
            context_window_tokens: None,
            top_p: None,
            max_retries: Some(0),
            allow_inheritance: None,
        })
        .unwrap();
    f.state
        .models
        .set_platform_default(&model.model_code)
        .unwrap();
    fixed_selection(&f, "PLATFORM", &model.model_code);
    let llm = LlmRuntimeService::new(Arc::new(resolver(&f)), Arc::new(f.client.clone()));
    let svc = service(&f, llm);
    let kb = svc.create(USER, "Pinned space", "").unwrap();
    let doc = svc
        .upload(
            USER,
            &kb.id,
            "chunks.md",
            "orchid evidence for a fixed embedding space. "
                .repeat(20)
                .as_bytes(),
        )
        .unwrap();
    svc.configure(
        USER,
        &kb.id,
        &doc.id,
        Some(ChunkConfig {
            mode: ChunkMode::Fixed,
            max_chars: 200,
            overlap_chars: 20,
            heading_depth: 2,
        }),
    )
    .unwrap();
    let first = svc.index(USER, &kb.id, &doc.id).await.unwrap();
    assert_eq!(first.status, "READY", "{:?}", first.error);
    assert!(first.chunk_count > 1);
    assert_eq!(
        first.embedding_model_code.as_deref(),
        Some(model.model_code.as_str())
    );
    assert_eq!(first.embedding_dimension, Some(1024));
    assert_eq!(f.client.calls.load(Ordering::SeqCst), first.chunk_count);
    svc.set_grants(USER, &kb.id, vec![f.agent.clone()])
        .unwrap();
    // A new default cannot switch the published index to an automatic query space.
    Connection::open(&f.state.config.database_url)
        .unwrap()
        .execute(
            "DELETE FROM default_model_selections WHERE scope_key=?1 AND model_type='EMBEDDING'",
            [WORKSPACE],
        )
        .unwrap();
    let ctx = svc
        .search_for_agent_in_run(USER, WORKSPACE, &f.agent, "unseen paraphrase", 1200, &run())
        .await
        .unwrap();
    assert!(!ctx.hits.is_empty());
    assert_eq!(f.client.calls.load(Ordering::SeqCst), first.chunk_count + 1);
    let rebuilt = svc.index(USER, &kb.id, &doc.id).await.unwrap();
    assert_eq!(rebuilt.status, "READY");
    assert_eq!(rebuilt.index_revision, first.index_revision + 1);
    assert_eq!(
        rebuilt.embedding_space_fingerprint,
        first.embedding_space_fingerprint
    );
    let calls = f.client.calls.load(Ordering::SeqCst);
    Connection::open(&f.state.config.database_url)
        .unwrap()
        .execute(
            "UPDATE models SET model_name='fixed-embedding-v2' WHERE model_code=?1",
            [&model.model_code],
        )
        .unwrap();
    assert!(
        svc.search_for_agent_in_run(USER, WORKSPACE, &f.agent, "unseen paraphrase", 1200, &run())
            .await
            .unwrap()
            .hits
            .is_empty()
    );
    assert_eq!(f.client.calls.load(Ordering::SeqCst), calls);
    svc.set_grants(USER, &kb.id, vec![]).unwrap();
    assert!(
        svc.search_for_agent_in_run(USER, WORKSPACE, &f.agent, "orchid", 1200, &run())
            .await
            .unwrap()
            .hits
            .is_empty()
    );
    assert_eq!(f.client.calls.load(Ordering::SeqCst), calls);
}

#[tokio::test]
async fn singleton_auto_route_indexes_rebuilds_queries_and_detects_observed_model_change() {
    let f = Fixture::new();
    connect_auto(&f);
    *f.client.provider_model.lock().unwrap() = Some("observed-model-a".into());
    let llm = LlmRuntimeService::new(Arc::new(resolver(&f)), Arc::new(f.client.clone()));
    assert!(
        llm.embedding_space_fingerprint(WORKSPACE, Some(AUTO), 1024)
            .is_ok()
    );
    let svc = service(&f, llm);
    let kb = svc
        .create(USER, "Single model route", "")
        .unwrap();
    let doc = svc
        .upload(
            USER,
            &kb.id,
            "chunks.md",
            "orchid vector evidence. ".repeat(30).as_bytes(),
        )
        .unwrap();
    svc.configure(
        USER,
        &kb.id,
        &doc.id,
        Some(ChunkConfig {
            mode: ChunkMode::Fixed,
            max_chars: 200,
            overlap_chars: 20,
            heading_depth: 2,
        }),
    )
    .unwrap();
    let first = svc.index(USER, &kb.id, &doc.id).await.unwrap();
    assert_eq!(first.status, "READY", "{:?}", first.error);
    assert!(first.chunk_count > 1);
    assert_eq!(first.embedding_dimension, Some(1024));
    assert_eq!(
        first.embedding_provider_model.as_deref(),
        Some("observed-model-a")
    );
    svc.set_grants(USER, &kb.id, vec![f.agent.clone()])
        .unwrap();
    let hits = svc
        .search_for_agent_in_run(USER, WORKSPACE, &f.agent, "unseen paraphrase", 1200, &run())
        .await
        .unwrap();
    assert!(!hits.hits.is_empty());
    let rebuilt = svc.index(USER, &kb.id, &doc.id).await.unwrap();
    assert_eq!(rebuilt.status, "READY");
    assert_eq!(rebuilt.index_revision, first.index_revision + 1);
    *f.client.provider_model.lock().unwrap() = Some("observed-model-b".into());
    assert!(
        svc.search_for_agent_in_run(USER, WORKSPACE, &f.agent, "unseen paraphrase", 1200, &run())
            .await
            .unwrap()
            .hits
            .is_empty()
    );
    let changed = svc
        .detail(USER, &kb.id)
        .unwrap()
        .documents
        .remove(0);
    assert_eq!(changed.status, "NEEDS_REBUILD");
    assert!(
        changed
            .error
            .as_deref()
            .unwrap()
            .contains("实际Embedding模型")
    );
    assert_eq!(changed.active_source_code, rebuilt.active_source_code);
    let refreshed = svc.index(USER, &kb.id, &doc.id).await.unwrap();
    assert_eq!(refreshed.status, "READY");
    assert_eq!(
        refreshed.embedding_provider_model.as_deref(),
        Some("observed-model-b")
    );
    assert!(
        !svc.search_for_agent_in_run(USER, WORKSPACE, &f.agent, "unseen paraphrase", 1200, &run())
            .await
            .unwrap()
            .hits
            .is_empty()
    );
    let calls = f.client.calls.load(Ordering::SeqCst);
    svc.set_grants(USER, &kb.id, vec![]).unwrap();
    assert!(
        svc.search_for_agent_in_run(USER, WORKSPACE, &f.agent, "orchid", 1200, &run())
            .await
            .unwrap()
            .hits
            .is_empty()
    );
    assert_eq!(f.client.calls.load(Ordering::SeqCst), calls);
}

fn count_vectors(f: &Fixture, kb: &str) -> usize {
    let c = Connection::open(&f.state.config.database_url).unwrap();
    tron_backend::shared::sqlite_extensions::default_sqlite_extension_context()
        .prepare_connection(&c)
        .unwrap();
    c.query_row(
        &format!("SELECT count(*) FROM \"rag_{kb}_units_vec_d1024\""),
        [],
        |r| r.get(0),
    )
    .unwrap()
}
#[tokio::test]
async fn only_1024_index_geometry_is_published_and_retrieved() {
    let f = Fixture::new();
    let kb = f.service.create(USER, "1024 only", "").unwrap();
    let d = f
        .service
        .upload(
            USER,
            &kb.id,
            "proof.md",
            b"synthetic 1024 geometry fact",
        )
        .unwrap();
    let ready = f
        .service
        .index(USER, &kb.id, &d.id)
        .await
        .unwrap();
    assert_eq!(ready.status, "READY");
    assert_eq!(ready.embedding_dimension, Some(1024));
    assert_eq!(count_vectors(&f, &kb.id), 1);
    f.service
        .set_grants(USER, &kb.id, vec![f.agent.clone()])
        .unwrap();
    assert!(
        f.service
            .search_for_agent(USER, WORKSPACE, &f.agent, "unseen paraphrase", 1200)
            .await
            .unwrap()
            .hits
            .is_empty()
    );
    assert!(
        !f.service
            .search_for_agent_in_run(USER, WORKSPACE, &f.agent, "unseen paraphrase", 1200, &run())
            .await
            .unwrap()
            .hits
            .is_empty()
    );
    for invalid in [1536, 4096, 8193] {
        f.client.dimensions.store(invalid, Ordering::SeqCst);
        let d = f
            .service
            .upload(
                USER,
                &kb.id,
                &format!("invalid{invalid}.md"),
                b"invalid geometry",
            )
            .unwrap();
        let failed = f
            .service
            .index(USER, &kb.id, &d.id)
            .await
            .unwrap();
        assert_eq!(failed.status, "FAILED");
        assert!(failed.active_source_code.is_none(), "invalid vectors must not publish any part of the generation");
        assert_eq!(failed.embedding_dimension, None);
        assert!(failed.error.as_deref().unwrap().contains("要求1024维"));
        assert_eq!(count_vectors(&f, &kb.id), 1);
    }
}
#[tokio::test]
async fn query_geometry_change_requires_rebuild_and_failed_rebuild_keeps_1024_generation() {
    let f = Fixture::new();
    let kb = f
        .service
        .create(USER, "Geometry rejection", "")
        .unwrap();
    let d = f
        .service
        .upload(
            USER,
            &kb.id,
            "proof.md",
            b"synthetic 1024 geometry fact",
        )
        .unwrap();
    let ready = f
        .service
        .index(USER, &kb.id, &d.id)
        .await
        .unwrap();
    f.service
        .set_grants(USER, &kb.id, vec![f.agent.clone()])
        .unwrap();
    f.client.dimensions.store(4096, Ordering::SeqCst);
    assert!(
        f.service
            .search_for_agent_in_run(USER, WORKSPACE, &f.agent, "unseen paraphrase", 1200, &run())
            .await
            .unwrap()
            .hits
            .is_empty()
    );
    assert_eq!(
        f.service.detail(USER, &kb.id).unwrap().documents[0].status,
        "NEEDS_REBUILD"
    );
    let failed = f
        .service
        .index(USER, &kb.id, &d.id)
        .await
        .unwrap();
    assert_eq!(failed.status, "FAILED");
    assert_eq!(failed.active_source_code, ready.active_source_code);
    assert_eq!(failed.index_revision, ready.index_revision);
    assert_eq!(failed.embedding_dimension, Some(1024));
    f.client.dimensions.store(1024, Ordering::SeqCst);
    assert_eq!(
        f.service
            .index(USER, &kb.id, &d.id)
            .await
            .unwrap()
            .status,
        "READY"
    );
    assert!(
        !f.service
            .search_for_agent_in_run(USER, WORKSPACE, &f.agent, "unseen paraphrase", 1200, &run())
            .await
            .unwrap()
            .hits
            .is_empty()
    );
}
#[tokio::test]
async fn obsolete_4096_metadata_cannot_dispatch_queries_and_needs_explicit_rebuild() {
    let f = Fixture::new();
    let kb = f
        .service
        .create(USER, "Obsolete metadata", "")
        .unwrap();
    let d = f
        .service
        .upload(
            USER,
            &kb.id,
            "proof.md",
            b"synthetic geometry fact",
        )
        .unwrap();
    let ready = f
        .service
        .index(USER, &kb.id, &d.id)
        .await
        .unwrap();
    f.service
        .set_grants(USER, &kb.id, vec![f.agent.clone()])
        .unwrap();
    // Simulate a persisted old geometry record without using or copying user data.
    let c = Connection::open(&f.state.config.database_url).unwrap();
    let columns: Vec<String> = c
        .prepare("PRAGMA table_info(knowledge_base_documents)")
        .unwrap()
        .query_map([], |r| r.get(1))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let payload = columns.iter().find(|s| s.contains("payload")).unwrap();
    c.execute(&format!("UPDATE knowledge_base_documents SET {payload}=json_set({payload},'$.embeddingDimension',4096)"),[]).unwrap();
    let calls = f.client.calls.load(Ordering::SeqCst);
    assert!(
        f.service
            .search_for_agent_in_run(USER, WORKSPACE, &f.agent, "unseen paraphrase", 1200, &run())
            .await
            .unwrap()
            .hits
            .is_empty()
    );
    assert_eq!(f.client.calls.load(Ordering::SeqCst), calls);
    let old = f
        .service
        .detail(USER, &kb.id)
        .unwrap()
        .documents
        .remove(0);
    assert_eq!(old.status, "NEEDS_REBUILD");
    assert_eq!(old.active_source_code, ready.active_source_code);
    let rebuilt = f
        .service
        .index(USER, &kb.id, &d.id)
        .await
        .unwrap();
    assert_eq!(rebuilt.status, "READY");
    assert_eq!(rebuilt.embedding_dimension, Some(1024));
}
#[tokio::test]
async fn mixed_chunk_dimensions_cannot_replace_published_1024_vectors() {
    let f = Fixture::new();
    let kb = f
        .service
        .create(USER, "Chunk rejection", "")
        .unwrap();
    let d = f
        .service
        .upload(
            USER,
            &kb.id,
            "proof.md",
            "synthetic text. ".repeat(50).as_bytes(),
        )
        .unwrap();
    f.service
        .configure(
            USER,
            &kb.id,
            &d.id,
            Some(ChunkConfig {
                mode: ChunkMode::Fixed,
                max_chars: 200,
                overlap_chars: 20,
                heading_depth: 2,
            }),
        )
        .unwrap();
    let ready = f
        .service
        .index(USER, &kb.id, &d.id)
        .await
        .unwrap();
    assert_eq!(ready.status, "READY");
    assert!(ready.chunk_count > 1);
    f.client
        .dimension_sequence
        .lock()
        .unwrap()
        .extend([1024, 4096]);
    let failed = f
        .service
        .index(USER, &kb.id, &d.id)
        .await
        .unwrap();
    assert_eq!(failed.status, "FAILED");
    assert_eq!(failed.active_source_code, ready.active_source_code);
    assert_eq!(failed.index_revision, ready.index_revision);
    assert_eq!(failed.embedding_dimension, Some(1024));
}
