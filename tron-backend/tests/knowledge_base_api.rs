mod common;
#[path = "support/knowledge_base_fixture.rs"]
mod fixture;
use fixture::*;
use serde_json::json;
use std::sync::atomic::Ordering;
use tron_backend::{
    adapters::outbound::persistence::sqlite::{SqliteKnowledgeBaseRepository, SqliteRagRepository},
    application::{
        knowledge_base::local_documents::{ChunkConfig, ChunkMode},
        rag::{RagConfigRepository, RagCorpusManagementInput, RagManagementRepository},
        resource::{
            ResourceBinaryDownloadCommand, ResourceDeleteCommand, ResourceGetQuery,
            ResourcePageQuery, ResourceUpdateCommand,
        },
    },
    domain::rag::{RagProfileRecord, RagProfileType, RagStatus},
};
#[tokio::test]
async fn four_real_formats_are_private_indexed_granted_and_retrieved_with_locations() {
    let f = Fixture::new();
    let kb = f
        .service
        .create(USER, "知识资料", "四格式")
        .unwrap();
    assert!(f.service.detail("OTHER-USER", &kb.id).is_err());
    assert!(f.service.detail(USER, &kb.id).is_ok());
    for (name, bytes) in [
        ("guide.md", b"# Orchid\norchid markdown evidence".to_vec()),
        ("guide.docx", office()),
        ("guide.xlsx", spreadsheet()),
        ("guide.pdf", pdf()),
    ] {
        let doc = f
            .service
            .upload(USER, &kb.id, name, &bytes)
            .unwrap();
        let preview = f.service.preview(USER, &kb.id, &doc.id).await.unwrap();
        assert!(preview.chunk_count > 0);
        assert!(
            preview
                .chunks
                .iter()
                .any(|c| c.text.to_lowercase().contains("orchid"))
        );
        let indexed = f
            .service
            .index(USER, &kb.id, &doc.id)
            .await
            .unwrap();
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
    f.service
        .set_grants(USER, &kb.id, vec![])
        .unwrap();
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
    let first = f
        .service
        .index(USER, &kb.id, &a.id)
        .await
        .unwrap();
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
            .await.unwrap()
            .chunk_count
            > first.chunk_count
    );
    assert_eq!(
        f.service
            .preview(USER, &kb.id, &b.id)
            .await.unwrap()
            .chunk_count,
        1
    );
    f.client.failing.store(true, Ordering::SeqCst);
    let failed = f
        .service
        .index(USER, &kb.id, &a.id)
        .await
        .unwrap();
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
    let rebuilt = f
        .service
        .index(USER, &kb.id, &a.id)
        .await
        .unwrap();
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
    f.service
        .remove_document(USER, &kb.id, &a.id)
        .unwrap();
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
async fn generic_rag_cannot_expose_product_corpus_or_mix_it_into_legacy_profiles() {
    let f = Fixture::new();
    let legacy = SqliteRagRepository::connect(&f.state.config.database_url).unwrap();
    let mut placeholder = legacy.get_corpus("code-graph").unwrap().unwrap();
    placeholder.status = RagStatus::Reserved;
    legacy.save_managed_corpus(&placeholder).unwrap();
    let kb = f.service.create(USER, "Private", "").unwrap();
    assert!(f.state.rag.get_corpus(&kb.id).is_err());
    assert!(f.state.rag.get_corpus_management(&kb.id).is_err());
    assert!(
        !f.state
            .rag
            .list_corpora()
            .unwrap()
            .iter()
            .any(|c| c.corpus_id == kb.id)
    );
    assert!(
        !f.state
            .rag
            .profile_catalog()
            .unwrap()
            .corpora
            .iter()
            .any(|c| c.corpus_id == kb.id)
    );
    let repository = SqliteRagRepository::connect(&f.state.config.database_url).unwrap();
    let mut alias = repository.get_corpus(&kb.id).unwrap().unwrap();
    alias.corpus_id = "legacy-alias".into();
    repository.save_managed_corpus(&alias).unwrap();
    assert!(f.state.rag.get_corpus(&alias.corpus_id).is_err());
    assert!(f.state.rag.get_storage_binding(&alias.corpus_id).is_err());
    let aliased_profile = RagProfileRecord {
        profile_id: "alias-profile".into(),
        profile_type: RagProfileType::Retrieval,
        status: RagStatus::Enabled,
        primary_corpus_id: Some(alias.corpus_id.clone()),
        corpus_ids: vec![alias.corpus_id.clone()],
        required_source_version: Some("v1".into()),
        config: json!({}),
        filters: json!({}),
        degrade_policy: "SKIP_FAILED_ROUTE".into(),
    };
    repository.save_profile(&aliased_profile).unwrap();
    assert!(
        f.state
            .rag
            .get_profile(&aliased_profile.profile_id)
            .is_err()
    );
    let normal = f.state.rag.get_corpus_management("memory-card").unwrap();
    let mut payload = serde_json::to_value(normal).unwrap();
    payload["corpusId"] = json!("attempted-kb-table-alias");
    payload["storageBinding"]["targets"][0]["table"] = json!(format!("rag_{}_units", kb.id));
    let request: RagCorpusManagementInput = serde_json::from_value(payload).unwrap();
    assert!(
        f.state
            .rag
            .verify_corpus_management(request.clone())
            .is_err()
    );
    assert!(
        f.state
            .rag
            .create_corpus_management(request.clone())
            .is_err()
    );
    assert!(
        f.state
            .rag
            .update_corpus_management("memory-card", request)
            .is_err()
    );
    assert!(f.state.rag.get_corpus("memory-card").is_ok());
    let mixed = RagProfileRecord {
        profile_id: "mixed-protected".into(),
        profile_type: RagProfileType::Retrieval,
        status: RagStatus::Enabled,
        primary_corpus_id: Some("memory-card".into()),
        corpus_ids: vec!["memory-card".into(), kb.id.clone()],
        required_source_version: Some("v1".into()),
        config: json!({"routes":[{"corpusId":kb.id}]}),
        filters: json!({}),
        degrade_policy: "SKIP_FAILED_ROUTE".into(),
    };
    repository.save_profile(&mixed).unwrap();
    assert!(f.state.rag.get_profile(&mixed.profile_id).is_err());
    assert!(
        !f.state
            .rag
            .list_profiles(Default::default())
            .unwrap()
            .iter()
            .any(|p| p.profile_id == mixed.profile_id)
    );
    assert!(repository.get_corpus(&kb.id).unwrap().is_some());
}
#[tokio::test]
async fn interrupted_index_claim_recovers_only_new_document_metadata() {
    let f = Fixture::new();
    let kb = f.service.create(USER, "Recover", "").unwrap();
    let doc = f
        .service
        .upload(USER, &kb.id, "guide.md", b"orchid recovery")
        .unwrap();
    let indexed = f
        .service
        .index(USER, &kb.id, &doc.id)
        .await
        .unwrap();
    let repo = SqliteKnowledgeBaseRepository::connect(&f.state.config.database_url).unwrap();
    let mut interrupted = indexed.clone();
    interrupted.revision += 1;
    interrupted.status = "INDEXING".into();
    repo.save_document(&kb.id, &interrupted, indexed.revision)
        .unwrap();
    drop(repo);
    let reopened = SqliteKnowledgeBaseRepository::connect(&f.state.config.database_url).unwrap();
    let recovered = reopened.document(&kb.id, &doc.id).unwrap();
    assert_eq!(recovered.status, "FAILED");
    assert_eq!(recovered.active_source_code, indexed.active_source_code);
    assert_eq!(recovered.revision, interrupted.revision + 1);
    let retried = f
        .service
        .index(USER, &kb.id, &doc.id)
        .await
        .unwrap();
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
        Some(json!({"workspaceCode":WORKSPACE,"name":"HTTP Library","description":"synthetic"})),
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
    assert_eq!(uploaded["data"]["status"], "QUEUED", "upload must persist a queued task immediately");
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
    assert_eq!(corpus.status(), StatusCode::FORBIDDEN);
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
        .upload(
            USER,
            &kb.id,
            "guide.md",
            b"orchid semantic evidence",
        )
        .unwrap();
    let indexed = f
        .service
        .index(USER, &kb.id, &doc.id)
        .await
        .unwrap();
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
    let kb = f
        .service
        .create(USER, "Resource ACL", "")
        .unwrap();
    let doc = f
        .service
        .upload(
            USER,
            &kb.id,
            "guide.md",
            b"orchid private original",
        )
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
    assert!(read(&f.state.resources.for_agent_in_workspace(Some(&f.agent), WORKSPACE)).is_err());
    assert!(read(&f.state.resources.for_agent(Some("FORGED_AGENT"))).is_err());
    f.service
        .set_grants(USER, &kb.id, vec![f.agent.clone()])
        .unwrap();
    assert!(read(&f.state.resources.for_agent_in_workspace(Some(&f.agent), WORKSPACE)).is_ok());
    assert!(
        read(
            &f.state
                .resources
                .for_agent_in_workspace(Some(&f.agent), "COMPLIANCE")
        )
        .is_ok()
    );
    f.service
        .set_grants(USER, &kb.id, vec![])
        .unwrap();
    assert!(read(&f.state.resources.for_agent_in_workspace(Some(&f.agent), WORKSPACE)).is_err());
    f.service
        .set_grants(USER, &kb.id, vec![f.agent.clone()])
        .unwrap();
    f.service
        .remove_document(USER, &kb.id, &doc.id)
        .unwrap();
    assert!(read(&f.state.resources.for_agent_in_workspace(Some(&f.agent), WORKSPACE)).is_err());
    assert!(read(&f.state.resources).is_ok());
}

#[tokio::test]
async fn automatic_ingestion_preserves_recorded_parser_failure_without_a_review_workflow() {
    use tron_backend::application::knowledge_base::fidelity::FidelityIssue;
    let f=Fixture::new();let kb=f.service.create(USER,"Automatic lifecycle","").unwrap();
    let ready=f.service.upload_and_index(USER,&kb.id,"clean.md",b"orchid automatic evidence").await.unwrap();
    assert_eq!(ready.status,"READY");assert_eq!(ready.index_revision,1);
    let rebuilt=f.service.configure_and_index(USER,&kb.id,&ready.id,Some(ChunkConfig{mode:ChunkMode::Fixed,max_chars:200,overlap_chars:0,heading_depth:2}),None,None).await.unwrap();
    assert_eq!(rebuilt.status,"READY");assert_eq!(rebuilt.index_revision,2);
    let original=pdf();let bad=f.service.upload(USER,&kb.id,"known-failure.pdf",&original).unwrap();
    f.service.preview(USER,&kb.id,&bad.id).await.unwrap();
    let repo=SqliteKnowledgeBaseRepository::connect(&f.state.config.database_url).unwrap();
    let mut failed=repo.document(&kb.id,&bad.id).unwrap();
    failed.parsing_artifact.as_mut().unwrap().fidelity_issues.push(FidelityIssue{kind:"RECORDED_PARSER_ERROR".into(),page:Some(1),blocks:vec![],source_excerpt:"upstream extraction lost a source comparator".into(),image_resource_codes:vec![],source_region:None,candidate_region:None,candidate_excerpt:None});
    let revision=failed.revision;failed.revision+=1;repo.save_document(&kb.id,&failed,revision).unwrap();
    let before=f.client.calls.load(Ordering::SeqCst);
    let result=f.service.index(USER,&kb.id,&bad.id).await.unwrap();
    assert_eq!(result.status,"FAILED");assert!(result.error.as_ref().unwrap().contains("source comparator"));assert!(result.active_source_code.is_none());
    assert_eq!(before,f.client.calls.load(Ordering::SeqCst));
    let configured=f.service.configure_and_index(USER,&kb.id,&bad.id,None,None,None).await.unwrap();assert_eq!(configured.status,"FAILED");
    assert_eq!(before,f.client.calls.load(Ordering::SeqCst));
    f.service.set_grants(USER,&kb.id,vec![f.agent.clone()]).unwrap();
    let hits=f.service.search_for_agent(USER,WORKSPACE,&f.agent,"orchid",1200).await.unwrap().hits;
    assert!(hits.iter().any(|h|h.document_id==ready.id));assert!(!hits.iter().any(|h|h.document_id==bad.id));
    let (_,record)=f.state.resources.download_binary_to_file(USER,ResourceBinaryDownloadCommand{workspace_code:WORKSPACE.into(),resource_code:bad.resource_code.clone()}).unwrap();let mut bytes=vec![];f.state.resources.download_binary_storage(&record,&mut bytes).unwrap();assert_eq!(bytes,original);
    assert!(f.state.resources.for_agent_in_workspace(Some(&f.agent),WORKSPACE).download_binary_to_file(USER,ResourceBinaryDownloadCommand{workspace_code:WORKSPACE.into(),resource_code:bad.resource_code}).is_err());
}

async fn wait_http_ready(app: &axum::Router, kb: &str, document: &str) {
    tokio::time::timeout(std::time::Duration::from_secs(8), async {
        loop {
            let response = common::authenticated_request(app, axum::http::Method::GET, &format!("/api/v1/knowledge-bases/{kb}?workspaceCode={WORKSPACE}"), None).await;
            let value = common::response_json(response).await;
            let doc = value["data"]["documents"].as_array().unwrap().iter().find(|d| d["id"] == document).unwrap();
            assert_ne!(doc["status"], "FAILED", "{doc}");
            if doc["status"] == "READY" { break; }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }).await.unwrap();
}

#[tokio::test]
async fn durable_task_http_contract_cancel_retry_and_owner_scope() {
    use axum::http::{Method, StatusCode};
    let f = Fixture::new();
    f.client.blocking.store(true, Ordering::SeqCst);
    let kb = f.service.create(USER, "Task API", "").unwrap();
    let doc = f.service.upload_and_start_index(USER, &kb.id, "task.md", b"orchid durable HTTP").unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while f.service.detail(USER,&kb.id).unwrap().documents[0].status != "EMBEDDING" {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }).await.unwrap();
    let mut state=f.state.clone(); state.knowledge_bases=f.service.clone();
    let app=tron_backend::app::build_router_with_state(state).unwrap();
    let root=format!("/api/v1/knowledge-bases/{}/documents/{}",kb.id,doc.id);
    let get=common::authenticated_request(&app,Method::GET,&format!("{root}/tasks?workspaceCode={WORKSPACE}"),None).await;
    assert_eq!(get.status(),StatusCode::OK);
    let tasks=common::response_json(get).await;
    assert_eq!(tasks["code"],"SUCCESS"); assert_eq!(tasks["data"].as_array().unwrap().len(),1);
    assert_eq!(tasks["data"][0]["stage"],"Embedding"); assert_eq!(tasks["data"][0]["state"],"RUNNING");
    assert!(tasks["data"][0].get("configuration").is_none()); assert!(tasks["data"][0].get("leaseOwner").is_none());
    let denied=common::authenticated_request(&app,Method::GET,&format!("{root}/tasks?workspaceCode=COMPLIANCE"),None).await;
    assert!(denied.status().is_success());
    let cancelled=common::authenticated_request(&app,Method::POST,&format!("{root}/cancel"),Some(json!({"workspaceCode":WORKSPACE}))).await;
    assert_eq!(cancelled.status(),StatusCode::OK);
    let cancelled=common::response_json(cancelled).await;
    assert_eq!(cancelled["data"]["status"],"CANCELLED"); assert!(cancelled["data"]["activeSourceCode"].is_null());
    let retried=common::authenticated_request(&app,Method::POST,&format!("{root}/retry"),Some(json!({"workspaceCode":WORKSPACE}))).await;
    assert_eq!(retried.status(),StatusCode::OK);
    let retried=common::response_json(retried).await;
    assert_eq!(retried["data"]["taskId"],tasks["data"][0]["id"]);
    assert!(matches!(retried["data"]["status"].as_str(),Some("QUEUED"|"EMBEDDING")));
    f.client.blocking.store(false,Ordering::SeqCst);
    wait_http_ready(&app,&kb.id,&doc.id).await;
    f.service.stop_ingestion_workers();
}
