//! Explicitly scoped validation of the user-selected CCS PDF. Local phase performs no inference.
mod common;
#[path="support/knowledge_base_fixture.rs"] mod fixture;
use fixture::*;
use std::{path::Path,sync::Arc};
use tron_backend::{adapters::outbound::mineru_documents::decode_archive,application::knowledge_base::{KnowledgeBaseError,parsing::*,local_documents::{ChunkMode,ChunkConfig}}};
const ROOT:&str="/Users/clouds3n/Documents/Codex/2026-10-03/task/mineru-local-deployment/data/ccs-real-validation";
const EVIDENCE:&str="/Users/clouds3n/Documents/Codex/2026-10-03/task/knowledge-base-implementation-evidence";
const NAME:&str="CCS Guidelines on Cardiac Risk in Noncardiac Surgery.pdf";
struct ExistingParse;
#[async_trait::async_trait]
impl KnowledgeDocumentParser for ExistingParse {
 fn supports_local_visual_reparse(&self)->bool{true}
 fn default_tier(&self)->ParsingTier{ParsingTier::Standard}
 async fn capabilities(&self)->Result<ParsingCapabilities,KnowledgeBaseError>{Ok(ParsingCapabilities{default_tier:ParsingTier::Standard,available_tiers:vec![ParsingTier::Standard],parser_version:"4.0.10".into(),office_tier:ParsingTier::Flash})}
 async fn parse(&self,name:&str,bytes:&[u8],_tier:Option<ParsingTier>)->Result<StructuredParse,KnowledgeBaseError>{
  use sha2::{Digest,Sha256};
  if name!=NAME || format!("{:x}",Sha256::digest(bytes))!="9155eab808f17ec894a106a3a91d8396b1f9e388b86db2a5960882a413d37be4" {return Err(KnowledgeBaseError::Invalid("only the explicitly approved CCS PDF is accepted".into()));}
  let archive=std::fs::read(Path::new(ROOT).join("result.zip")).map_err(|_|KnowledgeBaseError::Invalid("completed local parse required; no new parser job".into()))?;
  decode_archive(&archive,ParsingTier::Standard,"4.0.10","job_eaa725cacbcfb6ddd8179345")
 }
}
#[tokio::test]
#[ignore="requires explicitly selected local CCS PDF and its completed cached MinerU result"]
async fn local_pdf_preview_preserves_pages_private_images_and_reports_symbol_loss(){
 let f=Fixture::new();let service=f.service.clone().with_document_parser(Arc::new(ExistingParse));
 let kb=service.create(USER,"CCS PDF 本地验证（隔离）","Only the explicitly selected document; no external inference in this phase").unwrap();
 let original=std::fs::read(Path::new(ROOT).join("source.pdf")).unwrap();let doc=service.upload(USER,&kb.id,NAME,&original).unwrap();
 service.configure(USER,&kb.id,&doc.id,Some(ChunkConfig{mode:ChunkMode::Fixed,max_chars:8000,overlap_chars:0,heading_depth:2})).unwrap();
 let preview=service.preview(USER,&kb.id,&doc.id).await.unwrap();
 assert!(preview.chunk_count<=20,"do not silently omit preview chunks");
 assert!(preview.chunks.iter().any(|c|c.location.page==Some(3)&&!c.location.image_resource_codes.is_empty()&&c.text.contains("Figure 1")));
 assert!(preview.chunks.iter().any(|c|c.location.page==Some(5)&&c.text.contains("3.9")&&c.text.contains("6.0")&&c.text.contains("10.1")&&c.text.contains("15.0")&&c.text.contains("2.8%-5.4%")));
 assert!(preview.chunks.iter().any(|c|c.location.page==Some(6)&&c.text.contains("21.8")&&c.text.contains("19.0%-24.8%")));
 assert!(preview.chunks.iter().any(|c|c.location.page==Some(5)&&c.text.contains('\u{1}')),"original parse symbol loss must not be silently repaired");
 assert!(preview.warnings.iter().any(|w|w.contains("控制字符")));
 assert_eq!(f.client.calls.load(std::sync::atomic::Ordering::SeqCst),0);
 let detail=service.detail(USER,&kb.id).unwrap();let artifact=detail.documents[0].parsing_artifact.as_ref().unwrap();assert_eq!(artifact.image_resource_codes.len(),5);
 for code in &artifact.image_resource_codes {assert!(f.state.resources.download_binary_to_file("OTHER-USER",tron_backend::application::resource::ResourceBinaryDownloadCommand{workspace_code:WORKSPACE.into(),resource_code:code.clone()}).is_err());}
 let result=serde_json::json!({"scope":"isolated temporary KB, current source, not deployed desktop backend","mineruJobId":"job_eaa725cacbcfb6ddd8179345","chunkConfig":{"mode":"FIXED","maxChars":8000,"overlapChars":0},"preview":preview,"registeredPrivateImages":artifact.image_resource_codes.len(),"embeddingCalls":0,"visionCalls":0,"overallClinicalTextExactness":"FAIL: comparator and micro-unit losses preserved and reported; images required for visual confirmation"});
 std::fs::write(Path::new(EVIDENCE).join("ccs-local-preview.json"),serde_json::to_vec_pretty(&result).unwrap()).unwrap();
}

#[tokio::test]
#[ignore="requires the explicitly approved CCS PDF and completed local parse; no external calls"]
async fn confirmed_comparator_and_micro_unit_loss_block_index_publication(){
 let f=Fixture::new();let service=f.service.clone().with_document_parser(Arc::new(ExistingParse));
 let kb=service.create(USER,"CCS fidelity regression (isolated)","Actual pages5/6 and their original crops are ground truth").unwrap();
 let original=std::fs::read(Path::new(ROOT).join("source.pdf")).unwrap();let doc=service.upload(USER,&kb.id,NAME,&original).unwrap();
 service.configure(USER,&kb.id,&doc.id,Some(ChunkConfig{mode:ChunkMode::Fixed,max_chars:8000,overlap_chars:0,heading_depth:2})).unwrap();
 let preview=service.preview(USER,&kb.id,&doc.id).await.unwrap();
 let page5=preview.chunks.iter().find(|c|c.location.page==Some(5)).unwrap();
 // Human-checked original page5/crops: final score >=3 and creatinine >177 μmol/L.
 // The cached raw parse demonstrably loses both, and must remain unmodified as evidence.
 assert!(page5.text.contains('\u{1}'));assert!(page5.text.contains("177 mmol/L"));
 let indexed=service.index(USER,&kb.id,&doc.id).await.unwrap();
 assert_eq!(indexed.status,"REVIEW_REQUIRED","known incorrect comparator/unit text must never become READY");
 assert!(indexed.active_source_code.is_none());
 assert_eq!(f.client.calls.load(std::sync::atomic::Ordering::SeqCst),0,"reject before embedding");
 service.set_grants(USER,&kb.id,vec![f.agent.clone()]).unwrap();
 let context=service.search_for_agent(USER,WORKSPACE,&f.agent,"RCRI",8000).await.unwrap();assert!(context.hits.is_empty());
}

#[tokio::test]
#[ignore="requires the approved PDF and existing local parse; isolated database only"]
async fn legacy_publication_and_resource_reads_cannot_bypass_review_and_other_docs_stay_ready(){
 let f=Fixture::new();let service=f.service.clone().with_document_parser(Arc::new(ExistingParse)).with_visual_context(true);
 let kb=service.create(USER,"Legacy fidelity isolation regression","").unwrap();
 let seed=service.upload(USER,&kb.id,"legacy-seed.md",b"RCRI legacy known-bad text").unwrap();
 let seed=service.index(USER,&kb.id,&seed.id).await.unwrap();assert_eq!(seed.status,"READY");
 let clean=service.upload(USER,&kb.id,"unaffected.md",b"orchid unaffected evidence").unwrap();
 let clean=service.index(USER,&kb.id,&clean.id).await.unwrap();assert_eq!(clean.status,"READY");
 let doc=service.upload(USER,&kb.id,NAME,&std::fs::read(Path::new(ROOT).join("source.pdf")).unwrap()).unwrap();
 service.preview(USER,&kb.id,&doc.id).await.unwrap();
 let repo=tron_backend::adapters::outbound::persistence::sqlite::SqliteKnowledgeBaseRepository::connect(&f.state.config.database_url).unwrap();
 let mut old=repo.document(&kb.id,&doc.id).unwrap();
 let original_revision=old.revision;
 let mut payload=serde_json::to_value(&old).unwrap();
 payload["status"]=serde_json::json!("READY");payload["error"]=serde_json::Value::Null;
 payload["activeSourceCode"]=serde_json::json!(seed.active_source_code);
 payload["indexRevision"]=serde_json::json!(1);
 // Simulate a genuinely old payload, where new fields did not exist. Never touch production.
 payload["parsingArtifact"].as_object_mut().unwrap().remove("fidelityChecked");
 payload["parsingArtifact"].as_object_mut().unwrap().remove("fidelityIssues");
 old=serde_json::from_value(payload).unwrap();assert!(!old.parsing_artifact.as_ref().unwrap().fidelity_checked);
 old.revision+=1;repo.save_document(&kb.id,&old,original_revision).unwrap();
 repo.delete_document(&kb.id,&seed.id).unwrap();
 service.set_grants(USER,&kb.id,vec![f.agent.clone()]).unwrap();
 let artifact=old.parsing_artifact.as_ref().unwrap();
 let resources=f.state.resources.for_agent_in_workspace(Some(&f.agent),WORKSPACE);
 let command=|code:&str|tron_backend::application::resource::ResourceBinaryDownloadCommand{workspace_code:WORKSPACE.into(),resource_code:code.into()};
 assert!(resources.download_binary_to_file(USER,command(&artifact.resource_code)).is_err(),"unchecked legacy cache cannot be exposed by generic Agent file tools");
 let forged=tron_backend::application::knowledge_base::KnowledgeBaseHit{knowledge_base_id:kb.id.clone(),document_id:old.id.clone(),resource_code:old.resource_code.clone(),resource_revision:old.resource_revision,source_code:old.active_source_code.clone().unwrap(),chunk_code:"stale-cached-hit".into(),title:NAME.into(),text:"RCRI cached text".into(),location:serde_json::json!({"page":5,"imageResourceCodes":artifact.image_resource_codes}),score:1.0};
 assert!(service.visual_evidence_for_agent(USER,WORKSPACE,&f.agent,&[forged]).is_err(),"cached hits must be revalidated before media reads");
 assert!(service.search_for_agent(USER,WORKSPACE,&f.agent,"RCRI",8000).await.unwrap().hits.is_empty());
 let quarantined=repo.document(&kb.id,&old.id).unwrap();assert_eq!(quarantined.status,"REVIEW_REQUIRED");assert!(quarantined.parsing_artifact.as_ref().unwrap().fidelity_checked);
 let refreshed=service.configure(USER,&kb.id,&old.id,None).unwrap();assert_eq!(refreshed.status,"REVIEW_REQUIRED","changing splitting must not clear unresolved findings");
 for code in std::iter::once(&artifact.resource_code).chain(artifact.image_resource_codes.iter()).chain(std::iter::once(&old.resource_code)) {assert!(resources.download_binary_to_file(USER,command(code)).is_err());assert!(f.state.resources.download_binary_to_file(USER,command(code)).is_ok(),"owner retains original and audit resources");}
 assert_eq!(repo.document(&kb.id,&clean.id).unwrap().status,"READY");
 let unaffected=service.search_for_agent(USER,WORKSPACE,&f.agent,"orchid",8000).await.unwrap();assert!(unaffected.hits.iter().any(|h|h.document_id==clean.id));
 service.set_grants(USER,&kb.id,vec![]).unwrap();
 assert!(service.search_for_agent(USER,WORKSPACE,&f.agent,"orchid",8000).await.unwrap().hits.is_empty());
 assert!(service.visual_evidence_for_agent(USER,WORKSPACE,&f.agent,&unaffected.hits).is_err());
}

#[tokio::test]
#[ignore="approved CCS PDF and macOS local renderer; no external inference"]
async fn local_product_visual_source_keeps_original_audit_and_cannot_clear_gate_without_new_parse(){
 use sha2::{Digest,Sha256};use tron_backend::application::resource::ResourceBinaryDownloadCommand;
 let f=Fixture::new();let service=f.service.clone().with_document_parser(Arc::new(ExistingParse));
 let kb=service.create(USER,"CCS visual source regression","").unwrap();let bytes=std::fs::read(Path::new(ROOT).join("source.pdf")).unwrap();
 let doc=service.upload(USER,&kb.id,NAME,&bytes).unwrap();service.preview(USER,&kb.id,&doc.id).await.unwrap();
 assert!(service.prepare_visual_reparse("OTHER-USER",&kb.id,&doc.id).await.is_err());
 let prepared=service.prepare_visual_reparse(USER,&kb.id,&doc.id).await.unwrap();assert_eq!(prepared.status,"REVIEW_REQUIRED");
 let source=prepared.visual_source.as_ref().unwrap();assert_eq!(source.page_count,16);assert_eq!(source.original_sha256,format!("{:x}",Sha256::digest(&bytes)));assert_eq!(source.page_image_sha256.len(),16);assert!(!source.original_findings.is_empty());
 let command=|code:&str|ResourceBinaryDownloadCommand{workspace_code:WORKSPACE.into(),resource_code:code.into()};
 let (_,record)=f.state.resources.download_binary_to_file(USER,command(&source.derived_resource_code)).unwrap();let mut derived=Vec::new();f.state.resources.download_binary_storage(&record,&mut derived).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&derived)),source.derived_sha256);
 assert!(pdf_extract::extract_text_from_mem(&derived).unwrap().trim().is_empty(),"ambiguous font mappings must not reach the OCR input");
 let output=Path::new(ROOT).join("product-visual-reparse-v2");std::fs::create_dir_all(&output).unwrap();std::fs::write(output.join("source.pdf"),&derived).unwrap();std::fs::write(output.join("source-audit.json"),serde_json::to_vec_pretty(source).unwrap()).unwrap();
 for code in [&source.derived_resource_code,&source.audit_resource_code]{assert!(f.state.resources.download_binary_to_file("OTHER-USER",command(code)).is_err());}
 let repeated=service.prepare_visual_reparse(USER,&kb.id,&doc.id).await.unwrap();assert_eq!(repeated.visual_source.as_ref().unwrap().derived_sha256,source.derived_sha256);
 // The original replay parser intentionally rejects derived input: simply preparing the
 // source cannot clear findings or publish an index without an actual fresh successful parse.
 let indexed=service.index(USER,&kb.id,&doc.id).await.unwrap();assert_eq!(indexed.status,"REVIEW_REQUIRED");assert!(indexed.active_source_code.is_none());assert_eq!(f.client.calls.load(std::sync::atomic::Ordering::SeqCst),0);
 let (_,record)=f.state.resources.download_binary_to_file(USER,command(&doc.resource_code)).unwrap();let mut unchanged=Vec::new();f.state.resources.download_binary_storage(&record,&mut unchanged).unwrap();assert_eq!(bytes,unchanged);
}

struct ReprocessedParse;
#[async_trait::async_trait]
impl KnowledgeDocumentParser for ReprocessedParse {
 fn default_tier(&self)->ParsingTier{ParsingTier::Standard}
 fn supports_local_visual_reparse(&self)->bool{true}
 async fn capabilities(&self)->Result<ParsingCapabilities,KnowledgeBaseError>{ExistingParse.capabilities().await}
 async fn parse(&self,name:&str,bytes:&[u8],tier:Option<ParsingTier>)->Result<StructuredParse,KnowledgeBaseError>{
  use sha2::{Digest,Sha256};let sha=format!("{:x}",Sha256::digest(bytes));
  if sha=="9155eab808f17ec894a106a3a91d8396b1f9e388b86db2a5960882a413d37be4" {return ExistingParse.parse(name,bytes,tier).await;}
  let root=Path::new(ROOT).join("product-visual-reparse");let audit:serde_json::Value=serde_json::from_slice(&std::fs::read(root.join("source-audit.json")).map_err(|_|KnowledgeBaseError::Invalid("product-rendered source audit required".into()))?).map_err(|_|KnowledgeBaseError::Invalid("invalid local audit".into()))?;
  if name!=NAME || audit["derivedSha256"].as_str()!=Some(sha.as_str()) {return Err(KnowledgeBaseError::Invalid("only the product-rendered approved CCS PDF is accepted".into()));}
  let archive=std::fs::read(root.join("result.zip")).map_err(|_|KnowledgeBaseError::Invalid("completed local page-image OCR result required".into()))?;
  decode_archive(&archive,ParsingTier::Standard,"4.0.10","job_a86c09ece63e846cea460b5d")
 }
}
#[tokio::test]
#[ignore="approved CCS PDF and completed product-rendered OCR result; no external inference"]
async fn product_reprocessed_pdf_preserves_critical_values_units_and_french_accents(){
 let f=Fixture::new();let service=f.service.clone().with_document_parser(Arc::new(ReprocessedParse));
 let kb=service.create(USER,"CCS source/OCR fidelity verification","").unwrap();let original=std::fs::read(Path::new(ROOT).join("source.pdf")).unwrap();let doc=service.upload(USER,&kb.id,NAME,&original).unwrap();
 service.configure(USER,&kb.id,&doc.id,Some(ChunkConfig{mode:ChunkMode::Fixed,max_chars:8000,overlap_chars:0,heading_depth:2})).unwrap();
 service.preview(USER,&kb.id,&doc.id).await.unwrap();service.reparse_visual_document(USER,&kb.id,&doc.id).await.unwrap();let preview=service.preview(USER,&kb.id,&doc.id).await.unwrap();
 assert!(preview.fidelity_issues.is_empty(),"{:?}",preview.fidelity_issues);
 let page=|n:u32|preview.chunks.iter().filter(|c|c.location.page==Some(n)).map(|c|c.text.as_str()).collect::<Vec<_>>().join("\n");
 let p5=page(5);assert!(p5.contains("177 μmol/L")||p5.contains("177 µmol/L"),"micro-unit lost: {p5}");assert!(p5.contains("≥3")||p5.contains("≥ 3"),"comparison lost: {p5}");
 assert!(p5.contains("15.0")&&p5.contains("11.1")&&p5.contains("20.0"));
 let p6=page(6);assert!(p6.contains("300 ng/L")&&p6.contains("92 mg/L")&&p6.contains("21.8")&&p6.contains("19.0")&&p6.contains("24.8"));
 let p1=page(1);assert!(p1.contains("RÉSUMÉ")&&p1.contains("Société"),"French accent lost: {p1}");assert!(page(2).contains("périopératoire"));assert!(page(16).contains("Josée"));
 assert!(preview.chunks.iter().any(|c|c.location.page==Some(3)&&!c.location.image_resource_codes.is_empty()));
 let reviewed=service.detail(USER,&kb.id).unwrap().documents.remove(0);assert_ne!(reviewed.status,"REVIEW_REQUIRED");assert!(reviewed.visual_source.is_some());assert_eq!(f.client.calls.load(std::sync::atomic::Ordering::SeqCst),0);
 std::fs::write(Path::new(EVIDENCE).join("ccs-product-reprocessed-preview.json"),serde_json::to_vec_pretty(&serde_json::json!({"preview":preview,"document":reviewed,"scope":"real local OCR over product-rendered immutable source; critical-value/selected-French source checks only","externalModelCalls":0})).unwrap()).unwrap();
}

#[path="support/ccs_live_sdk.rs"] mod live_sdk;
#[tokio::test]
#[ignore="explicitly approved CCS PDF only; TRON_CCS_PDF_LIVE=1; real existing PersonAI SDK calls"]
async fn approved_ccs_pdf_real_index_retrieval_and_visual_qa(){live_sdk::run().await;}

struct OfficialBasicParse;
#[async_trait::async_trait]
impl KnowledgeDocumentParser for OfficialBasicParse {
 fn default_tier(&self)->ParsingTier{ParsingTier::Basic}
 async fn capabilities(&self)->Result<ParsingCapabilities,KnowledgeBaseError>{Ok(ParsingCapabilities{default_tier:ParsingTier::Basic,available_tiers:vec![ParsingTier::Basic],parser_version:"4.0.10".into(),office_tier:ParsingTier::Flash})}
 async fn parse(&self,name:&str,bytes:&[u8],_:Option<ParsingTier>)->Result<StructuredParse,KnowledgeBaseError>{
  use sha2::{Digest,Sha256};assert_eq!(name,NAME);assert_eq!(format!("{:x}",Sha256::digest(bytes)),"9155eab808f17ec894a106a3a91d8396b1f9e388b86db2a5960882a413d37be4");
  decode_archive(&std::fs::read(Path::new(ROOT).join("official-basic/result.zip")).unwrap(),ParsingTier::Basic,"4.0.10","job_db5f76d67ae74f16b4a8dabd")
 }
}
#[tokio::test]
#[ignore="explicit approved CCS sample and completed official Basic artifact; local-only isolated storage"]
async fn official_basic_known_failure_stays_failed_private_and_unpublished(){
 use tron_backend::adapters::outbound::persistence::sqlite::SqliteKnowledgeBaseRepository;
 use tron_backend::application::resource::ResourceBinaryDownloadCommand;
 use std::sync::atomic::Ordering;
 let f=Fixture::new();let service=f.service.clone().with_document_parser(Arc::new(OfficialBasicParse));
 let kb=service.create(USER,"CCS recorded upstream failure","").unwrap();let bytes=std::fs::read(Path::new(ROOT).join("source.pdf")).unwrap();let doc=service.upload(USER,&kb.id,NAME,&bytes).unwrap();
 let preview=service.preview(USER,&kb.id,&doc.id).await.unwrap();
 assert!(preview.chunks.iter().any(|c|c.text.contains("177 mmol/L")),"actual known incorrect micro-unit must remain in raw evidence");
 // This is verified developer evidence, not a product validator or a per-file production rule.
 let reason="Official MinerU Basic/auto CCS result retains a verified unit error: original page5 >177 μmol/L is extracted as >177 mmol/L. Existing known-bad data must not publish.";
 let repo=SqliteKnowledgeBaseRepository::connect(&f.state.config.database_url).unwrap();let mut recorded=repo.document(&kb.id,&doc.id).unwrap();let revision=recorded.revision;recorded.parser_failure=Some(reason.into());recorded.revision+=1;repo.save_document(&kb.id,&recorded,revision).unwrap();
 let failed=service.index(USER,&kb.id,&doc.id).await.unwrap();assert_eq!(failed.status,"FAILED");assert_eq!(failed.error.as_deref(),Some(reason));assert!(failed.active_source_code.is_none());assert_eq!(failed.index_revision,0);assert_eq!(f.client.calls.load(Ordering::SeqCst),0);
 let retry=service.configure_and_index(USER,&kb.id,&doc.id,None,None,None).await.unwrap();assert_eq!(retry.status,"FAILED");assert_eq!(f.client.calls.load(Ordering::SeqCst),0);
 service.set_grants(USER,&kb.id,vec![f.agent.clone()]).unwrap();assert!(service.search_for_agent(USER,WORKSPACE,&f.agent,"RCRI",8000).await.unwrap().hits.is_empty());
 let command=ResourceBinaryDownloadCommand{workspace_code:WORKSPACE.into(),resource_code:doc.resource_code.clone()};assert!(f.state.resources.for_agent_in_workspace(Some(&f.agent),WORKSPACE).download_binary_to_file(USER,command.clone()).is_err());
 let (_,record)=f.state.resources.download_binary_to_file(USER,command).unwrap();let mut original=vec![];f.state.resources.download_binary_storage(&record,&mut original).unwrap();assert_eq!(original,bytes);
 std::fs::write(Path::new(EVIDENCE).join("ccs-converged-known-failure.json"),serde_json::to_vec_pretty(&serde_json::json!({"status":"PASS_FAILURE_ISOLATION","parserJob":"job_db5f76d67ae74f16b4a8dabd","document":failed,"productionWrites":false,"embeddingCalls":0,"visionCalls":0,"retrievalHits":0,"originalUnchanged":true,"agentResourceBlocked":true,"failureRecordedByDeveloperTest":true})).unwrap()).unwrap();
}
#[tokio::test]
#[ignore="TRON_CCS_PDF_LIVE=1; approved original CCS table crops only; two real native Framework SDK QA calls"]
async fn approved_ccs_original_table_images_reach_native_sdk_agent(){live_sdk::run_raw_images().await;}
