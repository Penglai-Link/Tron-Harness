mod common;
#[path = "support/knowledge_base_fixture.rs"]
mod fixture;
use fixture::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tron_backend::{
    adapters::outbound::mineru_documents::{MineruKnowledgeDocumentParser, decode_archive},
    application::{
        knowledge_base::{KnowledgeBaseError, parsing::*},
        resource::ResourceBinaryDownloadCommand,
    },
    config::MineruConfig,
};

const DOCX: &[u8] = include_bytes!("fixtures/mineru-v1/synthetic.docx.zip");
const XLSX: &[u8] = include_bytes!("fixtures/mineru-v1/synthetic.xlsx.zip");
const PDF: &[u8] = include_bytes!("fixtures/mineru-v1/synthetic.pdf.zip");
const SCAN: &[u8] = include_bytes!("fixtures/mineru-v1/synthetic-scanned.pdf.zip");
struct Replay {
    calls: Arc<AtomicUsize>,
}
#[async_trait::async_trait]
impl KnowledgeDocumentParser for Replay {
    fn default_tier(&self) -> ParsingTier {
        ParsingTier::Standard
    }
    async fn capabilities(&self) -> Result<ParsingCapabilities, KnowledgeBaseError> {
        Ok(ParsingCapabilities { default_ocr_mode: tron_backend::application::knowledge_base::parsing::OcrMode::Auto,
            default_tier: ParsingTier::Standard,
            available_tiers: vec![ParsingTier::Flash, ParsingTier::Standard],
            parser_version: "4.0.10".into(),
            office_tier: ParsingTier::Flash,
        })
    }
    async fn parse(
        &self,
        name: &str,
        _bytes: &[u8],
        tier: Option<ParsingTier>,
    ) -> Result<StructuredParse, KnowledgeBaseError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let (archive, actual) = if name.ends_with(".docx") {
            (DOCX, ParsingTier::Flash)
        } else if name.ends_with(".xlsx") {
            (XLSX, ParsingTier::Flash)
        } else if name.contains("scanned") {
            (SCAN, ParsingTier::Standard)
        } else {
            (PDF, ParsingTier::Standard)
        };
        if tier.is_some_and(|t| t != actual) {
            return Err(KnowledgeBaseError::Invalid(
                "Unsupported synthetic tier".into(),
            ));
        }
        decode_archive(archive, actual, "4.0.10", "replayed-existing-job")
    }
}
#[tokio::test]
async fn real_structured_outputs_use_resource_cache_with_image_acl_and_1024_index() {
    let f = Fixture::new();
    let calls = Arc::new(AtomicUsize::new(0));
    let service = f.service.clone().with_document_parser(Arc::new(Replay {
        calls: calls.clone(),
    }));
    let kb = service
        .create(USER, "Structured", "synthetic only")
        .unwrap();
    for name in [
        "synthetic.docx",
        "synthetic.xlsx",
        "synthetic.pdf",
        "synthetic-scanned.pdf",
    ] {
        let doc = service
            .upload(USER, &kb.id, name, if name.ends_with(".docx") { include_bytes!("fixtures/mineru-v1/synthetic.docx").as_slice() } else if name.ends_with(".xlsx") { include_bytes!("fixtures/mineru-v1/synthetic.xlsx").as_slice() } else { b"synthetic replay input" })
            .unwrap();
        let preview = service
            .preview(USER, &kb.id, &doc.id)
            .await
            .unwrap();
        assert!(
            preview
                .chunks
                .iter()
                .any(|c| c.text.contains("Alpha") && c.text.contains("120"))
        );
        assert!(preview.chunks.iter().any(|c| !c.location.blocks.is_empty()));
        let cached = service
            .detail(USER, &kb.id)
            .unwrap()
            .documents
            .into_iter()
            .find(|d| d.id == doc.id)
            .unwrap();
        let artifact = cached.parsing_artifact.as_ref().unwrap();
        assert!(!artifact.image_resource_codes.is_empty());
        let expected = if name.ends_with(".pdf") {
            ParsingTier::Standard
        } else {
            ParsingTier::Flash
        };
        assert_eq!(artifact.effective_tier, expected);
        let before = calls.load(Ordering::SeqCst);
        let indexed = service
            .index(USER, &kb.id, &doc.id)
            .await
            .unwrap();
        assert_eq!(indexed.status, "READY", "{:?}", indexed.error);
        assert_eq!(indexed.embedding_dimension, Some(1024));
        assert_eq!(
            calls.load(Ordering::SeqCst),
            before,
            "index must reuse preview cache"
        );
        for resource in
            std::iter::once(&artifact.resource_code).chain(artifact.image_resource_codes.iter())
        {
            let command = || ResourceBinaryDownloadCommand {
                workspace_code: WORKSPACE.into(),
                resource_code: resource.clone(),
            };
            assert!(
                f.state
                    .resources
                    .download_binary_to_file(USER, command())
                    .is_ok()
            );
            assert!(
                f.state
                    .resources
                    .download_binary_to_file("OTHER-USER", command())
                    .is_err()
            );
            assert!(
                f.state
                    .resources
                    .for_agent(Some(&f.agent))
                    .download_binary_to_file(USER, command())
                    .is_err()
            );
            assert!(
                f.state
                    .resources
                    .for_agent(None)
                    .download_binary_to_file(USER, command())
                    .is_err()
            );
            service
                .set_grants(USER, &kb.id, vec![f.agent.clone()])
                .unwrap();
            assert!(
                f.state
                    .resources
                    .for_agent(Some(&f.agent))
                    .download_binary_to_file(USER, command())
                    .is_ok()
            );
            service.set_grants(USER, &kb.id, vec![]).unwrap();
            assert!(
                f.state
                    .resources
                    .for_agent(Some(&f.agent))
                    .download_binary_to_file(USER, command())
                    .is_err()
            );
        }
        service.set_grants(USER, &kb.id, vec![f.agent.clone()]).unwrap();
        let retrieved=service.search_for_agent(USER,WORKSPACE,&f.agent,"Alpha",8000).await.unwrap();
        assert!(!retrieved.hits.is_empty());
        // Removing the isolated image object proves default-off performs no image IO.
        let (_,image_record)=f.state.resources.download_binary_to_file(USER,ResourceBinaryDownloadCommand{workspace_code:WORKSPACE.into(),resource_code:artifact.image_resource_codes[0].clone()}).unwrap();
        let key=image_record.storage_locator.strip_prefix("storage://local-default/").unwrap();
        let image_path=f.directory.path().join("files").join(key);
        let saved=image_path.with_extension("synthetic-disabled-probe");std::fs::rename(&image_path,&saved).unwrap();
        let disabled=service.visual_evidence_for_agent(USER,WORKSPACE,&f.agent,&retrieved.hits).unwrap();
        assert!(disabled.evidence.is_empty());assert!(!disabled.warnings.is_empty());
        let missing=service.clone().with_visual_context(true).visual_evidence_for_agent(USER,WORKSPACE,&f.agent,&retrieved.hits).unwrap();
        assert!(!missing.warnings.is_empty());
        assert!(missing.evidence.iter().all(|e|e.resource_code!=artifact.image_resource_codes[0]));
        std::fs::rename(&saved,&image_path).unwrap();
        // Explicitly enable only this isolated synthetic fixture, never user configuration.
        let visual_service=service.clone().with_visual_context(true);
        let visual=visual_service.visual_evidence_for_agent(USER,WORKSPACE,&f.agent,&retrieved.hits).unwrap();
        assert!(!visual.evidence.is_empty(),"{name}: expected registered image resource");
        assert!(visual.evidence.iter().all(|e|e.source_reference.contains(&doc.id) && !e.bytes.is_empty()));
        // Exercise real registered synthetic images through native SDK preparation
        // and budgeting. No Contributor/Agent loop or provider inference is invoked.
        let mut content=vec![eliteforge_agent_core::ContentBlock::Text{text:"Synthetic Alpha source evidence".into()}];
        for evidence in &visual.evidence {
            let media=eliteforge_agent_memory::prepare_inline_image_evidence(&evidence.bytes,Some(evidence.source_reference.clone()),Default::default()).unwrap();
            content.push(eliteforge_agent_core::ContentBlock::Media{media});
        }
        let source=eliteforge_agent_core::ContextBlock::data("knowledge_base",eliteforge_agent_core::Content::from(content));
        let projected=eliteforge_agent_memory::project_multimodal_data(&source,eliteforge_agent_core::ContextBudget{max_tokens:16000,max_messages:1},&eliteforge_agent_core::ModelRef::new("synthetic","vision"),&eliteforge_agent_core::ConservativeTokenEstimator,4,1200,true).await.unwrap();
        assert!(projected.messages[0].content.has_media(),"{name}: real prepared images must fit visual budget");
        assert!(projected.messages[0].content.text().contains(&doc.id));assert!(projected.messages[0].content.text().contains("SDK resized evidence"));
        assert!(projected.report.final_tokens<=16000);
        assert!(visual_service.visual_evidence_for_agent("OTHER-USER",WORKSPACE,&f.agent,&retrieved.hits).is_err());
        assert!(visual_service.visual_evidence_for_agent(USER,"OTHER-WORKSPACE",&f.agent,&retrieved.hits).is_err());
        assert!(visual_service.visual_evidence_for_agent(USER,WORKSPACE,"unknown-agent",&retrieved.hits).is_err());
        let mut forged=retrieved.hits.clone();forged[0].location["imageResourceCodes"]=serde_json::json!([artifact.resource_code]);
        assert!(visual_service.visual_evidence_for_agent(USER,WORKSPACE,&f.agent,&forged).is_err());
        service.set_grants(USER,&kb.id,vec![]).unwrap();
        assert!(visual_service.visual_evidence_for_agent(USER,WORKSPACE,&f.agent,&retrieved.hits).is_err());
        service
            .remove_document(USER, &kb.id, &doc.id)
            .unwrap();
        service.set_grants(USER,&kb.id,vec![f.agent.clone()]).unwrap();
        assert!(visual_service.visual_evidence_for_agent(USER,WORKSPACE,&f.agent,&retrieved.hits).is_err());
        service
            .set_grants(USER, &kb.id, vec![f.agent.clone()])
            .unwrap();
        let image = artifact.image_resource_codes[0].clone();
        assert!(
            f.state
                .resources
                .for_agent(Some(&f.agent))
                .download_binary_to_file(
                    USER,
                    ResourceBinaryDownloadCommand {
                        workspace_code: WORKSPACE.into(),
                        resource_code: image
                    }
                )
                .is_err()
        );
        service.set_grants(USER, &kb.id, vec![]).unwrap();
    }
    assert_eq!(calls.load(Ordering::SeqCst), 4);
}
#[test]
fn actual_pdf_tier_mismatch_is_rejected_and_table_structure_is_retained() {
    assert!(decode_archive(PDF, ParsingTier::Basic, "4.0.10", "job").is_err());
    let parsed = decode_archive(PDF, ParsingTier::Standard, "4.0.10", "job").unwrap();
    assert!(parsed.middle.to_string().contains("<table>"));
    assert!(
        parsed
            .document
            .sections
            .iter()
            .any(|s| s.text.contains("<table>"))
    );
    assert!(
        parsed
            .document
            .sections
            .iter()
            .all(|s| s.location.page == Some(1))
    );
}
#[tokio::test]
async fn mineru_v1_transport_uses_upload_job_and_zip_without_source_download() {
    use axum::{
        Json, Router,
        extract::State,
        routing::{get, post, put},
    };
    use serde_json::{Value, json};
    #[derive(Clone)]
    struct Seen(Arc<std::sync::Mutex<Vec<Value>>>);
    async fn upload(State(seen): State<Seen>, Json(value): Json<Value>) -> Json<Value> {
        assert_eq!(value["mime_type"], "application/pdf");
        seen.0.lock().unwrap().push(value);
        Json(
            json!({"id":"upload_test","status":"pending","upload_url":"/v1/uploads/upload_test/content"}),
        )
    }
    async fn start(State(seen): State<Seen>, Json(value): Json<Value>) -> Json<Value> {
        assert_eq!(value["tier"], "standard");
        seen.0.lock().unwrap().push(value);
        Json(json!({"job_id":"job_test","status":"queued"}))
    }
    let seen = Seen(Default::default());
    let router=Router::new().route("/v1/health",get(||async{Json(json!({"status":"ok","version":"4.0.10"}))})).route("/v1/tiers",get(||async{Json(json!({"data":[{"id":"flash"},{"id":"standard"}]}))})).route("/v1/uploads",post(upload)).route("/v1/uploads/upload_test/content",put(||async{axum::http::StatusCode::OK})).route("/v1/uploads/upload_test/complete",post(||async{Json(json!({"file":{"id":"file_test"}}))})).route("/v1/parse/jobs",post(start)).route("/v1/parse/jobs/job_test",get(||async{Json(json!({"job_id":"job_test","status":"completed","files":[{"output_files":{"zip":{"file_id":"file_zip"}}}]}))})).route("/v1/files/file_zip/content",get(||async{PDF})).with_state(seen.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut config = MineruConfig::default();
    config.base_url = format!("http://{address}");
    let parser = MineruKnowledgeDocumentParser::new(config);
    let parsed = parser
        .parse("synthetic.pdf", b"synthetic transport", None)
        .await
        .unwrap();
    assert!(!parsed.assets.is_empty());
    assert_eq!(seen.0.lock().unwrap().len(), 2);
    server.abort();
}

#[tokio::test]
async fn cancellation_preserves_parsed_resources_and_retry_reuses_cache() {
    let f = Fixture::new();
    let calls = Arc::new(AtomicUsize::new(0));
    let service = f.service.clone().with_document_parser(Arc::new(Replay {
        calls: calls.clone(),
    }));
    let kb = service
        .create(USER, "Cancel synthetic", "")
        .unwrap();
    let doc = service
        .upload(
            USER,
            &kb.id,
            "synthetic.pdf",
            b"synthetic cancel",
        )
        .unwrap();
    f.client.blocking.store(true, Ordering::SeqCst);
    let owned = service.clone();
    let kb_id = kb.id.clone();
    let doc_id = doc.id.clone();
    let work = tokio::spawn(async move { owned.index(USER, &kb_id, &doc_id).await });
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let current = service
                .detail(USER, &kb.id)
                .unwrap()
                .documents
                .remove(0);
            if current.parsing_artifact.is_some() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    work.abort();
    assert!(work.await.unwrap_err().is_cancelled());
    let interrupted = service
        .detail(USER, &kb.id)
        .unwrap()
        .documents
        .remove(0);
    assert_eq!(interrupted.status, "FAILED");
    assert!(interrupted.parsing_artifact.is_some());
    assert!(!interrupted.derived_resource_codes.is_empty());
    f.client.blocking.store(false, Ordering::SeqCst);
    let ready = service
        .index(USER, &kb.id, &doc.id)
        .await
        .unwrap();
    assert_eq!(ready.status, "READY");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn retry_resumes_exact_hash_inflight_vendor_job_without_upload_or_new_job(){
 use axum::{Json,Router,routing::{get,post},extract::State};use serde_json::json;use sha2::{Digest,Sha256};
 #[derive(Clone)]struct Counter(Arc<AtomicUsize>);
 async fn job(State(count):State<Counter>)->Json<serde_json::Value>{let completed=count.0.fetch_add(1,Ordering::SeqCst)>0;Json(json!({"job_id":"job_existing","status":if completed{"completed"}else{"running"},"tier":"standard","files":[{"name":"synthetic.pdf","file_id":"file_existing","parse":if completed{json!({"parser_version":"4.0.10"})}else{serde_json::Value::Null},"output_files":{"zip":{"file_id":"file_zip"}}}]}))}
 let count=Counter(Arc::new(AtomicUsize::new(0)));let sha=format!("{:x}",Sha256::digest(b"resume transport"));
 let router=Router::new().route("/v1/health",get(||async{Json(json!({"status":"ok","version":"4.0.10"}))})).route("/v1/tiers",get(||async{Json(json!({"data":[{"id":"standard"}]}))})).route("/v1/parse/jobs",get(||async{Json(json!({"data":[{"job_id":"job_existing"}]}))}).post(||async{panic!("retry must not submit a new parse job");#[allow(unreachable_code)] Json(json!({}))})).route("/v1/uploads",post(||async{panic!("retry must not upload again");#[allow(unreachable_code)]Json(json!({}))})).route("/v1/parse/jobs/job_existing",get(job)).route("/v1/files/file_existing",get(move||{let sha=sha.clone();async move{Json(json!({"sha256sum":sha}))}})).route("/v1/files/file_zip/content",get(||async{PDF})).with_state(count.clone());
 let listener=tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();let addr=listener.local_addr().unwrap();let task=tokio::spawn(async move{axum::serve(listener,router).await.unwrap()});let mut config=MineruConfig::default();config.base_url=format!("http://{addr}");let parser=MineruKnowledgeDocumentParser::new(config);let parsed=parser.parse("synthetic.pdf",b"resume transport",None).await.unwrap();assert_eq!(parsed.vendor_job_id,"job_existing");assert_eq!(count.0.load(Ordering::SeqCst),2);task.abort();
}

#[cfg(target_os = "macos")]
#[tokio::test]
#[ignore = "Historical retired reprocessing workflow; evidence retained, normal ingestion covered in knowledge_base_api"]
async fn independent_clean_ocr_cannot_clear_original_risks_or_publish_vectors() {
    struct TwoExtractions { calls: AtomicUsize }
    #[async_trait::async_trait]
    impl KnowledgeDocumentParser for TwoExtractions {
        fn default_tier(&self)->ParsingTier { ParsingTier::Standard }
        fn supports_local_visual_reparse(&self)->bool { true }
        async fn capabilities(&self)->Result<ParsingCapabilities,KnowledgeBaseError> { Ok(ParsingCapabilities { default_ocr_mode: tron_backend::application::knowledge_base::parsing::OcrMode::Auto,default_tier:ParsingTier::Standard, available_tiers:vec![ParsingTier::Standard],parser_version:"4.0.10".into(),office_tier:ParsingTier::Flash}) }
        async fn parse(&self,_:&str,_:&[u8],_:Option<ParsingTier>)->Result<StructuredParse,KnowledgeBaseError> {
            let mut result=decode_archive(PDF,ParsingTier::Standard,"4.0.10","independent-replay-no-job")?;
            if self.calls.fetch_add(1,Ordering::SeqCst)==0 {
                result.document.sections[0].text.push('\u{0001}');
                let value=result.middle["pages"][0]["blocks"][0]["content"][0]["content"].as_str().unwrap().to_owned()+"\u{0001}";
                result.middle["pages"][0]["blocks"][0]["content"][0]["content"]=serde_json::Value::String(value);
            }
            Ok(result)
        }
    }
    use pdf_extract::{Document, Stream, dictionary};
    let mut pdf=Document::with_version("1.7");let pages=pdf.new_object_id();
    let content=pdf.add_object(Stream::new(dictionary!{},b"q 0 0 0 rg 10 10 40 40 re f Q".to_vec()));
    let page=pdf.add_object(dictionary!{"Type"=>"Page","Parent"=>pages,"MediaBox"=>vec![0.into(),0.into(),300.into(),200.into()],"Contents"=>content,"Resources"=>dictionary!{}});
    pdf.objects.insert(pages,dictionary!{"Type"=>"Pages","Kids"=>vec![page.into()],"Count"=>1}.into());
    let catalog=pdf.add_object(dictionary!{"Type"=>"Catalog","Pages"=>pages});pdf.trailer.set("Root",catalog);
    let mut original=vec![];pdf.save_to(&mut original).unwrap();
    let rendered=eliteforge_agent_memory::render_pdf_evidence(&original,Default::default()).unwrap();
    let pixels=image::load_from_memory(&rendered[0].png).unwrap().to_rgb8();
    let dark=pixels.pixels().filter(|p|p.0.iter().all(|v|*v<40)).count();
    assert!(dark>40_000,"PDF points must explicitly scale into raster pixels; blank margins must not shrink the source: {dark}");
    let mut rotated=pdf.clone();rotated.get_object_mut(page).unwrap().as_dict_mut().unwrap().set("Rotate",90);
    let mut rotated_bytes=vec![];rotated.save_to(&mut rotated_bytes).unwrap();
    let rotated=eliteforge_agent_memory::render_pdf_evidence(&rotated_bytes,Default::default()).unwrap();
    assert!((rotated[0].width_points-200.0).abs()<0.01 && (rotated[0].height_points-300.0).abs()<0.01);
    assert!(rotated[0].width<rotated[0].height);
    let f=Fixture::new();let service=f.service.clone().with_document_parser(Arc::new(TwoExtractions{calls:AtomicUsize::new(0)}));
    let kb=service.create(USER,"Independent OCR guard fixture","").unwrap();
    let uploaded=service.upload(USER,&kb.id,"independent-encoding.pdf",&original).unwrap();
    let before=service.preview(USER,&kb.id,&uploaded.id).await.unwrap();
    assert!(before.fidelity_issues.iter().any(|i|i.kind=="UNRESOLVED_TEXT_GLYPH"));
    let after=service.reparse_visual_document(USER,&kb.id,&uploaded.id).await.unwrap();
    assert_eq!(after.status,"REVIEW_REQUIRED");
    let artifact=after.parsing_artifact.as_ref().unwrap();
    assert!(artifact.fidelity_issues.iter().any(|i|i.kind=="ORIGINAL_FIDELITY_UNRESOLVED"));
    assert!(!artifact.fidelity_issues.iter().any(|i|i.kind=="UNRESOLVED_TEXT_GLYPH"),"the fresh candidate itself has no bad characters");
    let source=after.visual_source.as_ref().unwrap();
    assert!(source.original_parse_resource_code.is_some());
    // Repeating the cached preview must also retain the original quarantine.
    let repeated=service.preview(USER,&kb.id,&uploaded.id).await.unwrap();
    assert!(repeated.fidelity_issues.iter().any(|i|i.kind=="ORIGINAL_FIDELITY_UNRESOLVED"));
    let indexed=service.index(USER,&kb.id,&uploaded.id).await.unwrap();
    assert_eq!(indexed.status,"REVIEW_REQUIRED");assert!(indexed.active_source_code.is_none());
    assert_eq!(f.client.calls.load(Ordering::SeqCst),0);
    service.set_grants(USER,&kb.id,vec![f.agent.clone()]).unwrap();
    assert!(service.search_for_agent(USER,WORKSPACE,&f.agent,"ORBIT",800).await.unwrap().hits.is_empty());
    let (_,record)=f.state.resources.download_binary_to_file(USER,ResourceBinaryDownloadCommand{workspace_code:WORKSPACE.into(),resource_code:source.derived_resource_code.clone()}).unwrap();
    let mut bytes=vec![];f.state.resources.download_binary_storage(&record,&mut bytes).unwrap();
    let derived=Document::load_mem(&bytes).unwrap();
    let id=derived.get_pages()[&1];
    let media=derived.get_object(id).unwrap().as_dict().unwrap().get(b"MediaBox").unwrap().as_array().unwrap();
    assert!((media[2].as_float().unwrap()-300.0).abs()<0.01);
    assert!((media[3].as_float().unwrap()-200.0).abs()<0.01);
    let (_,record)=f.state.resources.download_binary_to_file(USER,ResourceBinaryDownloadCommand{workspace_code:WORKSPACE.into(),resource_code:uploaded.resource_code.clone()}).unwrap();
    let mut unchanged=vec![];f.state.resources.download_binary_storage(&record,&mut unchanged).unwrap();assert_eq!(original,unchanged);
}
