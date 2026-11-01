mod common;
#[path = "support/knowledge_base_fixture.rs"]
mod fixture;
use fixture::*;
use serde_json::json;
use std::sync::atomic::Ordering;
use tron_backend::{
    adapters::outbound::persistence::sqlite::SqliteKnowledgeBaseRepository,
    application::{
        knowledge_base::local_documents::{ChunkConfig, ChunkMode},
        resource::{
            ResourceBinaryDownloadCommand, ResourceDeleteCommand, ResourceGetQuery,
            ResourcePageQuery, ResourceUpdateCommand,
        },
    },
};

#[tokio::test]
async fn knowledge_base_defaults_and_graph_only_publication_use_real_source_generations() {
    use tron_backend::{application::{rag::*,knowledge_base::ingestion::IngestionKind},domain::rag::graph::GraphPartition,
        adapters::outbound::persistence::SqliteRagRepository};
    use tron_backend::application::rag::graph::RagGraphIndexRepository;
    let f=Fixture::new();let kb=f.service.create(USER,"Service manual","").unwrap();
    let settings=f.state.rag.settings_for_corpus(&kb.id).unwrap();assert!(settings.graph);
    assert_eq!(settings.embedding_model_code.as_deref(),Some("AUTO:MultimodalEmbedding"));
    f.state.rag.configure_corpus(&kb.id,RagSettings{fulltext:true,vector:false,..settings.clone()}).unwrap();
    let doc=f.service.upload(USER,&kb.id,"manual.md",b"# Services\nAlpha calls Beta.").unwrap();
    let ready=f.service.index(USER,&kb.id,&doc.id).await.unwrap();assert_eq!(ready.status,"READY","{:?}",ready.error);
    assert!(ready.embedding_dimension.is_none(),"chunk vectors remain off");
    let tasks=f.service.ingestion_tasks(USER,&kb.id,&doc.id).unwrap();let task=tasks.iter().find(|t|t.kind==IngestionKind::Indexing).unwrap();
    let index=SqliteRagRepository::connect(&f.state.config.database_url).unwrap().graph_index_repository().unwrap();
    let partition=GraphPartition{corpus_id:kb.id.clone(),scope_type:"SPACE".into(),scope_code:task.workspace_code.clone()};
    let generations=index.active_generations(&[partition.clone()]).unwrap();assert_eq!(generations.len(),1);
    assert_eq!(generations[0].expected_projections,2);assert_eq!(generations[0].task_id,task.id);
    assert_eq!(task.completed_units,ready.chunk_count,"node embedding progress must not corrupt chunk counters");
    let second=f.service.upload(USER,&kb.id,"second.md",b"# Other services\nAlpha calls Beta. Additional source.").unwrap();
    let second=f.service.index(USER,&kb.id,&second.id).await.unwrap();assert_eq!(second.status,"READY","{:?}",second.error);
    assert_eq!(index.active_generations(&[partition.clone()]).unwrap().len(),2);
    f.service.remove_document(USER,&kb.id,&second.id).unwrap();
    let remaining=index.active_generations(&[partition.clone()]).unwrap();assert_eq!(remaining.len(),1);
    assert_eq!(remaining[0].id,generations[0].id,"deleting one support must preserve the other source generation");
    f.state.rag.configure_corpus(&kb.id,RagSettings{fulltext:true,vector:false,graph:false,..settings}).unwrap();
    let republished=f.service.index(USER,&kb.id,&doc.id).await.unwrap();assert_eq!(republished.status,"READY","{:?}",republished.error);
    assert!(index.active_generations(&[partition]).unwrap().is_empty());
}

#[tokio::test]
async fn unresolved_embedding_identity_recovers_on_retry_without_reparsing() {
    use std::sync::{Arc, atomic::AtomicBool};
    use tron_backend::{
        application::{knowledge_base::ingestion::IngestionKind, llm::LlmRuntimeModelResolver, model::RuntimeModelConfig},
        domain::model::ModelError,
    };
    struct RecoveringResolver(Arc<AtomicBool>);
    impl LlmRuntimeModelResolver for RecoveringResolver {
        fn embedding_runtime_model_config(
            &self,
            workspace: &str,
            model: Option<&str>,
        ) -> Result<Option<RuntimeModelConfig>, ModelError> {
            if !self.0.load(Ordering::SeqCst) {
                return Err(ModelError::RuntimeResolutionFailed("No tagged embedding candidates".into()));
            }
            EmbeddingResolver.embedding_runtime_model_config(workspace, model)
        }
        fn rerank_runtime_model_config(
            &self, _: &str, _: Option<&str>,
        ) -> Result<Option<RuntimeModelConfig>, ModelError> { Ok(None) }
    }
    let available = Arc::new(AtomicBool::new(false));
    let f = Fixture::with_resolver(Arc::new(RecoveringResolver(available.clone())));
    let kb = f.service.create(USER, "Recovery", "").unwrap();
    let doc = f.service.upload(USER, &kb.id, "guide.pdf", &pdf()).unwrap();
    let failed = f.service.index(USER, &kb.id, &doc.id).await.unwrap();
    assert_eq!(failed.status, "FAILED");
    assert!(failed.error.unwrap().contains("No tagged embedding candidates"));
    assert_eq!(f.client.calls.load(Ordering::SeqCst), 0);
    let before = f.service.ingestion_tasks(USER, &kb.id, &doc.id).unwrap();
    let parsing = before.iter().find(|t| t.kind == IngestionKind::Parsing).unwrap();
    assert_eq!(parsing.state, "COMPLETED", "{:?}",parsing.error);
    let index = before.iter().find(|t| t.kind == IngestionKind::Indexing).unwrap();
    assert!(index.configuration["embeddingModel"].is_null());
    assert!(index.chunks_resource_code.is_some());

    available.store(true, Ordering::SeqCst);
    f.service.retry_index(USER, &kb.id, &doc.id).unwrap();
    let ready = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let doc = f.service.detail(USER, &kb.id).unwrap().documents.remove(0);
            if doc.status == "READY" || doc.status == "FAILED" { break doc; }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }).await.unwrap();
    assert_eq!(ready.status, "READY", "{:?}", ready.error);
    let after = f.service.ingestion_tasks(USER, &kb.id, &doc.id).unwrap();
    let same_parse = after.iter().find(|t| t.id == parsing.id).unwrap();
    assert_eq!(same_parse.attempt, parsing.attempt, "successful parsing must not rerun");
    let same_index = after.iter().find(|t| t.id == index.id).unwrap();
    assert_eq!(same_index.state, "READY");
    assert!(same_index.chunks_resource_code.is_none(), "published task checkpoints are retired");
    assert_eq!(same_index.configuration, index.configuration, "input lineage remains immutable");
    assert_eq!(same_index.embedding_model_code(), Some("fixture-embedding"));
    assert!(same_index.embedding_fingerprint().is_some());
    f.service.stop_ingestion_workers();
}
#[tokio::test]
async fn four_real_formats_are_private_indexed_granted_and_retrieved_with_locations() {
    let f = Fixture::new();
    let kb = f.service.create(USER, "知识资料", "四格式").unwrap();
    assert!(f.service.detail("OTHER-USER", &kb.id).is_err());
    assert!(f.service.detail(USER, &kb.id).is_ok());
    for (name, bytes) in [
        ("guide.md", b"# Orchid\norchid markdown evidence".to_vec()),
        ("guide.docx", office()),
        ("guide.xlsx", spreadsheet()),
        ("guide.pdf", pdf()),
    ] {
        let doc = f.service.upload(USER, &kb.id, name, &bytes).unwrap();
        let preview = f.service.preview(USER, &kb.id, &doc.id).await.unwrap();
        assert!(preview.chunk_count > 0);
        assert!(
            preview
                .chunks
                .iter()
                .any(|c| c.text.to_lowercase().contains("orchid"))
        );
        let indexed = f.service.index(USER, &kb.id, &doc.id).await.unwrap();
        assert_eq!(indexed.status, "READY", "{:?}", indexed.error);
        assert_eq!(indexed.embedding_dimension, Some(1024));
        assert!(
            f.state
                .resources
                .download_binary_to_file(
                    "OTHER-USER",
                    ResourceBinaryDownloadCommand {
                        workspace_code: "PLATFORM".into(),
                        resource_code: doc.resource_code.clone()
                    }
                )
                .is_err()
        );
        assert!(
            f.state
                .resources
                .get_resource(
                    "OTHER-USER",
                    ResourceGetQuery {
                        workspace_code: "PLATFORM".into(),
                        resource_code: doc.resource_code.clone()
                    }
                )
                .is_err()
        );
        assert!(
            f.state
                .resources
                .update_resource(
                    USER,
                    ResourceUpdateCommand {
                        workspace_code: "PLATFORM".into(),
                        resource_code: doc.resource_code.clone(),
                        name: Some("tampered".into()),
                        description: None,
                        allow_inheritance: None
                    }
                )
                .is_err()
        );
        assert!(
            f.state
                .resources
                .delete_resource(
                    USER,
                    ResourceDeleteCommand {
                        workspace_code: "PLATFORM".into(),
                        resource_code: doc.resource_code.clone()
                    }
                )
                .is_err()
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
    }
    let calls = f.client.calls.load(Ordering::SeqCst);
    assert!(
        f.service
            .search_for_agent(USER, WORKSPACE, &f.agent, "orchid", 1200)
            .await
            .unwrap()
            .hits
            .is_empty()
    );
    assert_eq!(f.client.calls.load(Ordering::SeqCst), calls);
    f.service
        .set_grants(USER, &kb.id, vec![f.agent.clone()])
        .unwrap();
    let context = f
        .service
        .search_for_agent(USER, WORKSPACE, &f.agent, "orchid", 1200)
        .await
        .unwrap();
    assert_eq!(context.hits.len(), 4);
    assert!(context.text.contains("[文档:"));
    assert!(context.text.contains("orchid"));
    let small = f
        .service
        .search_for_agent(USER, WORKSPACE, &f.agent, "orchid", 32)
        .await
        .unwrap();
    assert!(!small.hits.is_empty());
    assert!(small.text.contains("orchid"));
    assert!(!small.text.starts_with("[文档:"));
    assert!(context.hits.iter().all(|h| !h.location.is_null()));
    assert!(
        f.service
            .search_for_agent("OTHER-USER", WORKSPACE, &f.agent, "orchid", 1200)
            .await
            .unwrap()
            .hits
            .is_empty()
    );
    // The same authorized Agent can retrieve the library in every assigned space.
    assert!(
        !f.service
            .search_for_agent(USER, "COMPLIANCE", &f.agent, "orchid", 1200)
            .await
            .unwrap()
            .hits
            .is_empty()
    );
    assert!(
        f.service
            .search_for_agent(USER, WORKSPACE, "FORGED_AGENT", "orchid", 1200)
            .await
            .unwrap()
            .hits
            .is_empty()
    );
    f.service.set_grants(USER, &kb.id, vec![]).unwrap();
    assert!(
        f.service
            .search_for_agent(USER, WORKSPACE, &f.agent, "orchid", 1200)
            .await
            .unwrap()
            .hits
            .is_empty()
    );
    let page = f
        .state
        .resources
        .query_resources(
            "OTHER-USER",
            ResourcePageQuery {
                workspace_code: "PLATFORM".into(),
                kinds: vec![],
                source_kind: None,
                search: None,
                current: 1,
                size: 50,
            },
        )
        .unwrap();
    assert!(!page.records.iter().any(|r| r.description == "知识库原件"));
    assert!(
        f.service
            .upload(USER, &kb.id, "old.doc", b"legacy")
            .is_err()
    );
    assert!(
        f.service
            .upload(USER, &kb.id, "old.xls", b"legacy")
            .is_err()
    );
}
#[tokio::test]
async fn per_document_configuration_rebuild_and_failure_preserve_published_generation() {
    let f = Fixture::new();
    let kb = f.service.create(USER, "Rebuild", "").unwrap();
    let content = format!("orchid {}", "evidence ".repeat(250));
    let a = f
        .service
        .upload(USER, &kb.id, "a.md", content.as_bytes())
        .unwrap();
    let b = f
        .service
        .upload(USER, &kb.id, "b.md", b"orchid independent")
        .unwrap();
    let first = f.service.index(USER, &kb.id, &a.id).await.unwrap();
    let small = ChunkConfig {
        mode: ChunkMode::Fixed,
        max_chars: 200,
        overlap_chars: 20,
        ..Default::default()
    };
    f.service
        .configure(USER, &kb.id, &a.id, Some(small))
        .unwrap();
    assert!(
        f.service
            .preview(USER, &kb.id, &a.id)
            .await
            .unwrap()
            .chunk_count
            > first.chunk_count
    );
    assert_eq!(
        f.service
            .preview(USER, &kb.id, &b.id)
            .await
            .unwrap()
            .chunk_count,
        1
    );
    f.client.failing.store(true, Ordering::SeqCst);
    let failed = f.service.index(USER, &kb.id, &a.id).await.unwrap();
    assert_eq!(failed.status, "FAILED");
    assert_eq!(failed.active_source_code, first.active_source_code);
    assert_eq!(failed.index_revision, first.index_revision);
    f.service
        .set_grants(USER, &kb.id, vec![f.agent.clone()])
        .unwrap();
    let old = f
        .service
        .search_for_agent(USER, WORKSPACE, &f.agent, "orchid", 1200)
        .await
        .unwrap();
    assert_eq!(
        old.hits[0].source_code,
        first.active_source_code.clone().unwrap()
    );
    f.client.failing.store(false, Ordering::SeqCst);
    let rebuilt = f.service.index(USER, &kb.id, &a.id).await.unwrap();
    assert_eq!(rebuilt.status, "READY");
    assert_ne!(rebuilt.active_source_code, first.active_source_code);
    assert_eq!(rebuilt.index_revision, 2);
    let retrieved = f
        .service
        .search_for_agent(USER, WORKSPACE, &f.agent, "orchid", 1200)
        .await
        .unwrap();
    assert!(
        retrieved
            .hits
            .iter()
            .all(|h| Some(&h.source_code) == rebuilt.active_source_code.as_ref())
    );
    f.service.remove_document(USER, &kb.id, &a.id).unwrap();
    assert!(
        f.service
            .search_for_agent(USER, WORKSPACE, &f.agent, "orchid", 1200)
            .await
            .unwrap()
            .hits
            .is_empty()
    );
    assert!(
        f.state
            .resources
            .get_resource(
                USER,
                ResourceGetQuery {
                    workspace_code: "PLATFORM".into(),
                    resource_code: a.resource_code
                }
            )
            .is_ok()
    );
}
#[tokio::test]
async fn knowledge_base_corpus_is_automatic_and_owner_filtered() {
    let f = Fixture::new();
    let kb = f.service.create(USER, "Private", "").unwrap();
    let owned = std::collections::BTreeSet::from([kb.id.clone()]);
    assert!(
        f.state
            .rag
            .corpus_summaries(&owned)
            .unwrap()
            .iter()
            .any(|corpus| corpus.id == kb.id && corpus.managed_source_id.as_ref() == Some(&kb.id))
    );
    assert!(
        !f.state
            .rag
            .corpus_summaries(&Default::default())
            .unwrap()
            .iter()
            .any(|corpus| corpus.id == kb.id)
    );
}

#[tokio::test]
async fn image_description_model_is_chosen_by_rag_and_pinned_to_the_index_task() {
    use tron_backend::application::rag::RagSettings;
    let f = Fixture::new();
    let library = f.service.create(USER, "Image policy", "").unwrap();
    let document = f.service.upload(USER, &library.id, "guide.md", b"orchid").unwrap();
    f.service.start_index(USER, &library.id, &document.id).unwrap();
    let default_task = f.service.ingestion_tasks(USER, &library.id, &document.id).unwrap();
    assert_eq!(default_task[0].configuration["imageDescription"]["enabled"], true);

    let configured = f.service.create(USER, "Image policy enabled", "").unwrap();
    f.state.rag.configure_corpus(&configured.id, RagSettings {
        image_description_model_code: Some("synthetic-vision".into()),
        embedding_model_code: Some("synthetic-embedding".into()),
        ..Default::default()
    }).unwrap();
    let image_document = f.service.upload(USER, &configured.id, "image.md", b"orchid").unwrap();
    f.service.start_index(USER, &configured.id, &image_document.id).unwrap();
    let task = f.service.ingestion_tasks(USER, &configured.id, &image_document.id).unwrap();
    assert_eq!(task[0].configuration["imageDescription"]["enabled"], true);
    assert_eq!(task[0].configuration["imageDescription"]["modelCode"], "synthetic-vision");
    f.state.rag.configure_corpus(&configured.id, RagSettings {
        embedding_model_code: Some("synthetic-embedding".into()),
        ..Default::default()
    }).unwrap();
    assert_eq!(f.service.ingestion_tasks(USER, &configured.id, &image_document.id).unwrap()[0].configuration["imageDescription"]["modelCode"], "synthetic-vision");
    f.service.stop_ingestion_workers();
}
#[tokio::test]
async fn reopening_storage_preserves_metadata_and_explicit_rebuild_publishes_a_new_generation() {
    let f = Fixture::new();
    let kb = f.service.create(USER, "Recover", "").unwrap();
    let doc = f
        .service
        .upload(USER, &kb.id, "guide.md", b"orchid recovery")
        .unwrap();
    let indexed = f.service.index(USER, &kb.id, &doc.id).await.unwrap();
    let repo = SqliteKnowledgeBaseRepository::connect(&f.state.config.database_url).unwrap();
    let mut interrupted = indexed.clone();
    interrupted.revision += 1;
    interrupted.status = "INDEXING".into();
    repo.save_document(&kb.id, &interrupted, indexed.revision)
        .unwrap();
    drop(repo);
    let reopened = SqliteKnowledgeBaseRepository::connect(&f.state.config.database_url).unwrap();
    let recovered = reopened.document(&kb.id, &doc.id).unwrap();
    assert_eq!(recovered.status, "INDEXING");
    assert_eq!(recovered.active_source_code, indexed.active_source_code);
    assert_eq!(recovered.revision, interrupted.revision);
    let retried = f.service.index(USER, &kb.id, &doc.id).await.unwrap();
    assert_eq!(retried.status, "READY");
}

#[tokio::test]
async fn product_http_contract_upload_configure_preview_index_and_grants() {
    use axum::{
        body::Body,
        http::{Method, Request, StatusCode, header},
    };
    use tower::ServiceExt;
    let f = Fixture::new();
    let mut state = f.state.clone();
    state.knowledge_bases = f.service.clone();
    let app = tron_backend::app::build_router_with_state(state).unwrap();
    let created = common::authenticated_request(
        &app,
        Method::POST,
        "/api/v1/knowledge-bases",
        Some(json!({"name":"HTTP Library","description":"synthetic","defaultParsePreference":"Auto"})),
    )
    .await;
    assert_eq!(created.status(), StatusCode::OK);
    let kb = common::response_json(created).await;
    let id = kb["data"]["id"].as_str().unwrap();
    let boundary = "kb-fixture";
    let body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"workspaceCode\"\r\n\r\n{WORKSPACE}\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"guide.md\"\r\nContent-Type: text/markdown\r\n\r\n# Guide\norchid HTTP contract\r\n--{boundary}--\r\n"
    );
    let uploaded = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/knowledge-bases/{id}/documents"))
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(uploaded.status(), StatusCode::OK);
    let uploaded = common::response_json(uploaded).await;
    assert_eq!(
        uploaded["data"]["status"], "QUEUED",
        "upload must persist a queued task immediately"
    );
    assert!(uploaded["data"]["taskId"].is_string());
    let doc = uploaded["data"]["id"].as_str().unwrap();
    wait_http_ready(&app, id, doc).await;
    for suffix in ["preview", "index"] {
        let response = common::authenticated_request(
            &app,
            Method::POST,
            &format!("/api/v1/knowledge-bases/{id}/documents/{doc}/{suffix}"),
            Some(json!({"workspaceCode":WORKSPACE})),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let value = common::response_json(response).await;
        if suffix == "preview" {
            assert!(value["data"]["chunks"][0]["location"].is_object())
        } else {
            assert_eq!(value["data"]["status"], "QUEUED");
            wait_http_ready(&app, id, doc).await;
        }
    }
    let configured=common::authenticated_request(&app,Method::PATCH,&format!("/api/v1/knowledge-bases/{id}/documents/{doc}"),Some(json!({"workspaceCode":WORKSPACE,"chunkConfig":{"mode":"FIXED","maxChars":200,"overlapChars":20,"headingDepth":2}}))).await;
    assert_eq!(configured.status(), StatusCode::OK);
    assert_eq!(
        common::response_json(configured).await["data"]["status"],
        "QUEUED"
    );
    wait_http_ready(&app, id, doc).await;
    let granted = common::authenticated_request(
        &app,
        Method::PUT,
        &format!("/api/v1/knowledge-bases/{id}/grants"),
        Some(json!({"workspaceCode":WORKSPACE,"agentCodes":[f.agent]})),
    )
    .await;
    assert_eq!(granted.status(), StatusCode::OK);
    let granted = common::response_json(granted).await;
    assert_eq!(granted["data"]["agentCodes"].as_array().unwrap().len(), 1);
    let corpus = common::authenticated_request(
        &app,
        Method::GET,
        &format!("/api/v1/rag/corpora/{id}"),
        None,
    )
    .await;
    assert_eq!(corpus.status(), StatusCode::NOT_FOUND);
    let denied = common::authenticated_request(
        &app,
        Method::GET,
        &format!("/api/v1/knowledge-bases/{id}?workspaceCode=COMPLIANCE"),
        None,
    )
    .await;
    assert_eq!(denied.status(), StatusCode::OK);
}

#[tokio::test]
async fn vector_retrieval_uses_one_authorized_query_embedding_and_skips_zero_or_stale_space() {
    let f = Fixture::new();
    let kb = f.service.create(USER, "Vectors", "").unwrap();
    let doc = f
        .service
        .upload(USER, &kb.id, "guide.md", b"orchid semantic evidence")
        .unwrap();
    let indexed = f.service.index(USER, &kb.id, &doc.id).await.unwrap();
    let run = eliteforge_agent_core::ModelInvocationOptions::for_call(
        &eliteforge_agent_core::RunLimits::default(),
    )
    .unwrap()
    .run
    .unwrap();
    let count = f.client.calls.load(Ordering::SeqCst);
    assert!(
        f.service
            .search_for_agent_in_run(
                USER,
                WORKSPACE,
                &f.agent,
                "semantic paraphrase not present",
                1200,
                &run
            )
            .await
            .unwrap()
            .hits
            .is_empty()
    );
    assert_eq!(f.client.calls.load(Ordering::SeqCst), count);
    f.service
        .set_grants(USER, &kb.id, vec![f.agent.clone()])
        .unwrap();
    for (query, budget) in [("orchid", 0), ("", 1200)] {
        assert!(
            f.service
                .search_for_agent_in_run(USER, WORKSPACE, &f.agent, query, budget, &run)
                .await
                .unwrap()
                .hits
                .is_empty()
        )
    }
    assert_eq!(f.client.calls.load(Ordering::SeqCst), count);
    let context = f
        .service
        .search_for_agent_in_run(USER, WORKSPACE, &f.agent, "unseen paraphrase", 1200, &run)
        .await
        .unwrap();
    assert_eq!(context.hits.len(), 1);
    assert_eq!(f.client.calls.load(Ordering::SeqCst), count + 1);
    let repository = SqliteKnowledgeBaseRepository::connect(&f.state.config.database_url).unwrap();
    let mut stale = indexed.clone();
    stale.embedding_space_fingerprint = Some("old-model-config".into());
    stale.revision += 1;
    repository
        .save_document(&kb.id, &stale, indexed.revision)
        .unwrap();
    let fulltext = f
        .service
        .search_for_agent_in_run(USER, WORKSPACE, &f.agent, "orchid", 1200, &run)
        .await
        .unwrap();
    assert_eq!(fulltext.hits.len(), 1);
    assert_eq!(f.client.calls.load(Ordering::SeqCst), count + 1);
    assert_ne!(fulltext.source_version, context.source_version);
    run.cancellation.cancel();
    assert!(
        f.service
            .search_for_agent_in_run(USER, WORKSPACE, &f.agent, "orchid", 1200, &run)
            .await
            .is_err()
    );
    assert_eq!(f.client.calls.load(Ordering::SeqCst), count + 1);
}

#[tokio::test]
async fn same_owner_agent_resource_reads_require_grants_and_removed_docs_stay_private() {
    let f = Fixture::new();
    let kb = f.service.create(USER, "Resource ACL", "").unwrap();
    let doc = f
        .service
        .upload(USER, &kb.id, "guide.md", b"orchid private original")
        .unwrap();
    let read = |service: &tron_backend::application::resource::ResourceService| {
        service.download_binary_to_file(
            USER,
            ResourceBinaryDownloadCommand {
                workspace_code: "PLATFORM".into(),
                resource_code: doc.resource_code.clone(),
            },
        )
    };
    assert!(read(&f.state.resources).is_ok());
    assert!(read(&f.state.resources.for_agent(None)).is_err());
    assert!(
        read(
            &f.state
                .resources
                .for_agent_in_workspace(Some(&f.agent), WORKSPACE)
        )
        .is_err()
    );
    assert!(read(&f.state.resources.for_agent(Some("FORGED_AGENT"))).is_err());
    f.service
        .set_grants(USER, &kb.id, vec![f.agent.clone()])
        .unwrap();
    assert!(
        read(
            &f.state
                .resources
                .for_agent_in_workspace(Some(&f.agent), WORKSPACE)
        )
        .is_ok()
    );
    assert!(
        read(
            &f.state
                .resources
                .for_agent_in_workspace(Some(&f.agent), "COMPLIANCE")
        )
        .is_ok()
    );
    f.service.set_grants(USER, &kb.id, vec![]).unwrap();
    assert!(
        read(
            &f.state
                .resources
                .for_agent_in_workspace(Some(&f.agent), WORKSPACE)
        )
        .is_err()
    );
    f.service
        .set_grants(USER, &kb.id, vec![f.agent.clone()])
        .unwrap();
    f.service.remove_document(USER, &kb.id, &doc.id).unwrap();
    assert!(
        read(
            &f.state
                .resources
                .for_agent_in_workspace(Some(&f.agent), WORKSPACE)
        )
        .is_err()
    );
    assert!(read(&f.state.resources).is_ok());
}

#[tokio::test]
async fn automatic_ingestion_preserves_recorded_parser_failure_without_a_review_workflow() {
    use tron_backend::application::knowledge_base::fidelity::FidelityIssue;
    let f = Fixture::new();
    let kb = f.service.create(USER, "Automatic lifecycle", "").unwrap();
    let ready = f
        .service
        .upload_and_index(USER, &kb.id, "clean.md", b"orchid automatic evidence")
        .await
        .unwrap();
    assert_eq!(ready.status, "READY");
    assert_eq!(ready.index_revision, 1);
    let rebuilt = f
        .service
        .configure_and_index(
            USER,
            &kb.id,
            &ready.id,
            Some(ChunkConfig {
                mode: ChunkMode::Fixed,
                max_chars: 200,
                overlap_chars: 0,
                heading_depth: 2,
            }),
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(rebuilt.status, "READY");
    assert_eq!(rebuilt.index_revision, 2);
    let original = pdf();
    let bad = f
        .service
        .upload(USER, &kb.id, "known-failure.pdf", &original)
        .unwrap();
    f.service.preview(USER, &kb.id, &bad.id).await.unwrap();
    let repo = SqliteKnowledgeBaseRepository::connect(&f.state.config.database_url).unwrap();
    let mut failed = repo.document(&kb.id, &bad.id).unwrap();
    failed
        .parsing_artifact
        .as_mut()
        .unwrap()
        .fidelity_issues
        .push(FidelityIssue {
            kind: "RECORDED_PARSER_ERROR".into(),
            page: Some(1),
            blocks: vec![],
            source_excerpt: "upstream extraction lost a source comparator".into(),
            image_resource_codes: vec![],
            source_region: None,
            candidate_region: None,
            candidate_excerpt: None,
        });
    let revision = failed.revision;
    failed.revision += 1;
    repo.save_document(&kb.id, &failed, revision).unwrap();
    let before = f.client.calls.load(Ordering::SeqCst);
    let result = f.service.index(USER, &kb.id, &bad.id).await.unwrap();
    assert_eq!(result.status, "FAILED");
    assert!(result.error.as_ref().unwrap().contains("source comparator"),"{:?}",result.error);
    assert!(result.active_source_code.is_none());
    assert_eq!(before, f.client.calls.load(Ordering::SeqCst));
    let configured = f
        .service
        .configure_and_index(USER, &kb.id, &bad.id, None, None, None)
        .await
        .unwrap();
    assert_eq!(configured.status, "FAILED");
    assert_eq!(before, f.client.calls.load(Ordering::SeqCst));
    f.service
        .set_grants(USER, &kb.id, vec![f.agent.clone()])
        .unwrap();
    let hits = f
        .service
        .search_for_agent(USER, WORKSPACE, &f.agent, "orchid", 1200)
        .await
        .unwrap()
        .hits;
    assert!(hits.iter().any(|h| h.document_id == ready.id));
    assert!(!hits.iter().any(|h| h.document_id == bad.id));
    let (_, record) = f
        .state
        .resources
        .download_binary_to_file(
            USER,
            ResourceBinaryDownloadCommand {
                workspace_code: "PLATFORM".into(),
                resource_code: bad.resource_code.clone(),
            },
        )
        .unwrap();
    let mut bytes = vec![];
    f.state
        .resources
        .download_binary_storage(&record, &mut bytes)
        .unwrap();
    assert_eq!(bytes, original);
    assert!(
        f.state
            .resources
            .for_agent_in_workspace(Some(&f.agent), WORKSPACE)
            .download_binary_to_file(
                USER,
                ResourceBinaryDownloadCommand {
                    workspace_code: "PLATFORM".into(),
                    resource_code: bad.resource_code
                }
            )
            .is_err()
    );
}

async fn wait_http_ready(app: &axum::Router, kb: &str, document: &str) {
    tokio::time::timeout(std::time::Duration::from_secs(8), async {
        loop {
            let response = common::authenticated_request(
                app,
                axum::http::Method::GET,
                &format!("/api/v1/knowledge-bases/{kb}?workspaceCode={WORKSPACE}"),
                None,
            )
            .await;
            let value = common::response_json(response).await;
            let doc = value["data"]["documents"]
                .as_array()
                .unwrap()
                .iter()
                .find(|d| d["id"] == document)
                .unwrap();
            assert_ne!(doc["status"], "FAILED", "{doc}");
            if doc["status"] == "READY" {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn durable_task_http_contract_cancel_retry_and_owner_scope() {
    use axum::http::{Method, StatusCode};
    let f = Fixture::new();
    f.client.blocking.store(true, Ordering::SeqCst);
    let kb = f.service.create(USER, "Task API", "").unwrap();
    let doc = f
        .service
        .upload_and_start_index(USER, &kb.id, "task.md", b"orchid durable HTTP")
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while f.service.detail(USER, &kb.id).unwrap().documents[0].status != "EMBEDDING" {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let mut state = f.state.clone();
    state.knowledge_bases = f.service.clone();
    let app = tron_backend::app::build_router_with_state(state).unwrap();
    let root = format!("/api/v1/knowledge-bases/{}/documents/{}", kb.id, doc.id);
    let get = common::authenticated_request(
        &app,
        Method::GET,
        &format!("{root}/tasks?workspaceCode={WORKSPACE}"),
        None,
    )
    .await;
    assert_eq!(get.status(), StatusCode::OK);
    let tasks = common::response_json(get).await;
    assert_eq!(tasks["code"], "SUCCESS");
    assert_eq!(tasks["data"].as_array().unwrap().len(), 2);
    assert_eq!(tasks["data"][0]["kind"], "INDEXING");
    assert_eq!(tasks["data"][1]["kind"], "PARSING");
    assert_eq!(tasks["data"][1]["state"], "COMPLETED");
    let rag_tasks = common::authenticated_request(&app, Method::GET, "/api/v1/rag/tasks", None).await;
    assert_eq!(rag_tasks.status(), StatusCode::OK);
    let rag_tasks = common::response_json(rag_tasks).await;
    let matching = rag_tasks["data"].as_array().unwrap().iter()
        .filter(|task| task["documentId"] == doc.id).collect::<Vec<_>>();
    assert_eq!(matching.len(), 1);
    assert_eq!(matching[0]["id"], tasks["data"][0]["id"]);
    assert_eq!(tasks["data"][0]["stage"], "Embedding");
    assert_eq!(tasks["data"][0]["state"], "RUNNING");
    assert!(tasks["data"][0].get("configuration").is_none());
    assert!(tasks["data"][0].get("leaseOwner").is_none());
    let denied = common::authenticated_request(
        &app,
        Method::GET,
        &format!("{root}/tasks?workspaceCode=COMPLIANCE"),
        None,
    )
    .await;
    assert!(denied.status().is_success());
    let cancelled = common::authenticated_request(
        &app,
        Method::POST,
        &format!("{root}/cancel"),
        Some(json!({"workspaceCode":WORKSPACE})),
    )
    .await;
    assert_eq!(cancelled.status(), StatusCode::OK);
    let cancelled = common::response_json(cancelled).await;
    assert_eq!(cancelled["data"]["status"], "CANCELLED");
    assert!(cancelled["data"]["activeSourceCode"].is_null());
    let retried = common::authenticated_request(
        &app,
        Method::POST,
        &format!("{root}/retry"),
        Some(json!({"workspaceCode":WORKSPACE})),
    )
    .await;
    assert_eq!(retried.status(), StatusCode::OK);
    let retried = common::response_json(retried).await;
    assert_eq!(retried["data"]["taskId"], tasks["data"][0]["id"]);
    assert!(matches!(
        retried["data"]["status"].as_str(),
        Some("QUEUED" | "EMBEDDING")
    ));
    f.client.blocking.store(false, Ordering::SeqCst);
    wait_http_ready(&app, &kb.id, &doc.id).await;
    f.service.stop_ingestion_workers();
}

#[tokio::test]
async fn library_settings_metadata_and_noop_preserve_documents_grants_and_tasks() {
    use tron_backend::application::knowledge_base::DocumentParsePreference;
    let f = Fixture::new();
    let kb = f.service.create(USER, "Manual", "Original").unwrap();
    let doc = f.service.upload(USER, &kb.id, "guide.pdf", &pdf()).unwrap();
    let before = f.service.detail(USER, &kb.id).unwrap();
    let tasks = f.service.ingestion_tasks(USER, &kb.id, &doc.id).unwrap();
    let mut settings = f.service.library_settings(USER, &kb.id).unwrap();
    f.service
        .update_library_settings(USER, &kb.id, settings.clone())
        .unwrap();
    settings.name = "  Updated manual  ".into();
    settings.description = " Updated description ".into();
    settings.default_parse_preference = DocumentParsePreference::Recognition;
    let saved = f
        .service
        .update_library_settings(USER, &kb.id, settings)
        .unwrap();
    assert_eq!(saved.name, "Updated manual");
    let after = f.service.detail(USER, &kb.id).unwrap();
    assert_eq!(
        serde_json::to_value(before.documents).unwrap(),
        serde_json::to_value(after.documents).unwrap()
    );
    assert_eq!(before.agent_codes, after.agent_codes);
    assert_eq!(
        f.service
            .ingestion_tasks(USER, &kb.id, &doc.id)
            .unwrap()
            .len(),
        tasks.len()
    );
    assert_eq!(f.client.calls.load(Ordering::SeqCst), 0);
    let reread = f.service.library_settings(USER, &kb.id).unwrap();
    assert_eq!(reread.description, "Updated description");
    assert_eq!(
        serde_json::to_value(reread.default_parse_preference).unwrap(),
        "Recognition"
    );
    assert_eq!(
        f.state
            .rag
            .corpus_summaries(&[kb.id.clone()].into())
            .unwrap()
            .iter()
            .find(|c| c.id == kb.id)
            .unwrap()
            .name,
        "Updated manual"
    );
    let mut invalid = reread.clone();
    invalid.name = " ".into();
    invalid.index_settings.graph = false;
    assert!(
        f.service
            .update_library_settings(USER, &kb.id, invalid)
            .is_err()
    );
    assert!(
        f.service
            .update_library_settings("OTHER-USER", &kb.id, reread)
            .is_err()
    );
    assert!(
        f.service
            .library_settings(USER, &kb.id)
            .unwrap()
            .index_settings
            .graph
    );
    assert_eq!(
        f.service.detail(USER, &kb.id).unwrap().name,
        "Updated manual"
    );
}

#[tokio::test]
async fn library_settings_http_roundtrip_is_owner_scoped_and_localized() {
    let f = Fixture::new();
    let kb = f.service.create(USER, "Manual", "Original").unwrap();
    let mut state = f.state.clone();
    state.knowledge_bases = f.service.clone();
    let app = tron_backend::app::build_router_with_state(state).unwrap();
    async fn request(
        app: &axum::Router,
        method: &str,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> (u16, serde_json::Value) {
        let response =
            common::authenticated_request(app, method.parse().unwrap(), path, body).await;
        (
            response.status().as_u16(),
            common::response_json(response).await,
        )
    }
    let path = format!("/api/v1/knowledge-bases/{}/settings", kb.id);
    let (status, read) = request(&app, "GET", &path, None).await;
    assert_eq!(status, 200, "{read}");
    let mut settings = read["data"].clone();
    settings["name"] = json!("Updated through API");
    let (status, saved) = request(&app, "PUT", &path, Some(settings)).await;
    assert_eq!(status, 200, "{saved}");
    assert_eq!(saved["code"], "0");
    assert_eq!(saved["data"]["name"], "Updated through API");
    let (_, read) = request(&app, "GET", &path, None).await;
    assert_eq!(read["data"], saved["data"]);
    let mut invalid = read["data"].clone();
    invalid["name"] = json!("");
    let (status, error) = request(&app, "PUT", &path, Some(invalid)).await;
    assert_eq!(status, 400, "{error}");
    assert_eq!(error["data"], json!({}));
    use tower::ServiceExt;
    let mut invalid = read["data"].clone();
    invalid["name"] = json!("");
    let response = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("PUT")
                .uri(&path)
                .header("content-type", "application/json")
                .header("accept-language", "en-US")
                .body(axum::body::Body::from(invalid.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 400);
    let error = common::response_json(response).await;
    assert_eq!(
        error["message"],
        "Invalid knowledge base name, description or index settings"
    );
    let foreign = f
        .service
        .create("OTHER-USER", "Private library", "")
        .unwrap();
    let (status, error) = request(
        &app,
        "GET",
        &format!("/api/v1/knowledge-bases/{}/settings", foreign.id),
        None,
    )
    .await;
    assert_eq!(status, 404, "{error}");
    assert_eq!(error["data"], json!({}));
}

#[tokio::test]
async fn local_text_indexing_does_not_reconstruct_a_remote_parser() {
    use std::sync::Arc;
    use tron_backend::application::knowledge_base::{KnowledgeBaseError, parsing::*};
    struct StrictRemoteParser;
    #[async_trait::async_trait]
    impl KnowledgeDocumentParser for StrictRemoteParser {
        fn task_snapshot(
            &self,
            _: &serde_json::Value,
        ) -> Result<Option<Arc<dyn KnowledgeDocumentParser>>, KnowledgeBaseError> {
            panic!("local text must not reconstruct the remote parser")
        }
        fn default_tier(&self) -> ParsingTier {
            ParsingTier::Standard
        }
        async fn capabilities(&self) -> Result<ParsingCapabilities, KnowledgeBaseError> {
            panic!("local text must not query the remote parser")
        }
        async fn parse(
            &self,
            _: &str,
            _: &[u8],
            _: Option<ParsingTier>,
        ) -> Result<StructuredParse, KnowledgeBaseError> {
            panic!("local text must not use the remote parser")
        }
    }
    let f = Fixture::new();
    let service = f
        .service
        .clone()
        .with_document_parser(Arc::new(StrictRemoteParser));
    let kb = service.create(USER, "Local text", "").unwrap();
    let document = service
        .upload(USER, &kb.id, "guide.md", b"# Guide\n\nLocal text is searchable.")
        .unwrap();
    assert!(document.parse_plan.as_ref().unwrap().features.local_text);
    let indexed = service.index(USER, &kb.id, &document.id).await.unwrap();
    assert_eq!(indexed.status, "READY", "{:?}", indexed.error);
    service.stop_ingestion_workers();
}

#[tokio::test]
async fn library_settings_index_change_supplements_cached_pdf_without_parser_resubmission() {
    let f = Fixture::new();
    let kb = f.service.create(USER, "Cached document", "").unwrap();
    let doc = f.service.upload(USER, &kb.id, "guide.pdf", &pdf()).unwrap();
    let ready = f.service.index(USER, &kb.id, &doc.id).await.unwrap();
    assert_eq!(ready.status, "READY", "{:?}", ready.error);
    use tron_backend::application::knowledge_base::{KnowledgeBaseError, parsing::*};
    struct RejectParser;
    #[async_trait::async_trait]
    impl KnowledgeDocumentParser for RejectParser {
        fn default_tier(&self) -> ParsingTier {
            ParsingTier::Standard
        }
        async fn capabilities(&self) -> Result<ParsingCapabilities, KnowledgeBaseError> {
            Err(KnowledgeBaseError::Storage(
                "Parser must not be used for cached indexing".into(),
            ))
        }
        async fn parse(
            &self,
            _: &str,
            _: &[u8],
            _: Option<ParsingTier>,
        ) -> Result<StructuredParse, KnowledgeBaseError> {
            panic!("Cached indexing must never call the parser")
        }
    }
    let service = f
        .service
        .clone()
        .with_document_parser(std::sync::Arc::new(RejectParser));
    let embedding_calls = f.client.calls.load(Ordering::SeqCst);
    let mut settings = f.service.library_settings(USER, &kb.id).unwrap();
    settings.index_settings.graph = false;
    service
        .update_library_settings(USER, &kb.id, settings)
        .unwrap();
    let completed = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let current = service.detail(USER, &kb.id).unwrap().documents.remove(0);
            if current.status == "READY" || current.status == "FAILED" {
                break current;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(completed.status, "READY", "{:?}", completed.error);
    let after_tasks = service.ingestion_tasks(USER, &kb.id, &doc.id).unwrap();
    assert!(
        after_tasks
            .iter()
            .any(|t| t.configuration["reuseParsedOnly"] == true)
    );
    assert_eq!(
        f.client.calls.load(Ordering::SeqCst),
        embedding_calls,
        "compatible body vectors must be reused"
    );
    assert_eq!(
        completed.parsing_artifact.as_ref().unwrap().resource_code,
        ready.parsing_artifact.as_ref().unwrap().resource_code
    );
    service.stop_ingestion_workers();
}
