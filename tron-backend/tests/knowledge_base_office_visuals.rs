//! Synthetic local chart evidence. Does not execute a Contributor or contact a model.
mod common;
#[path="support/knowledge_base_fixture.rs"] mod fixture;
use fixture::*;
use std::{sync::{Arc,atomic::{AtomicUsize,Ordering}},io::{Cursor,Read,Write}};
use tron_backend::{adapters::outbound::office_visuals::LocalOfficeChartRenderer, application::knowledge_base::{office_visuals::validate_office_chart_input,parsing::*,KnowledgeBaseError}};
const WORD:&[u8]=include_bytes!("fixtures/office-chart/native-chart.docx");
const EXCEL:&[u8]=include_bytes!("fixtures/office-chart/native-chart.xlsx");
fn mutate(bytes:&[u8], path:&str, data:&[u8])->Vec<u8> {
 let mut source=zip::ZipArchive::new(Cursor::new(bytes)).unwrap();let mut out=zip::ZipWriter::new(Cursor::new(Vec::new()));
 for i in 0..source.len() { let mut file=source.by_index(i).unwrap();if file.name()==path{continue;} let mut content=Vec::new();file.read_to_end(&mut content).unwrap();out.start_file(file.name(),zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated)).unwrap();out.write_all(&content).unwrap(); }
 out.start_file(path,zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated)).unwrap();out.write_all(data).unwrap();out.finish().unwrap().into_inner()
}
#[test]
fn source_mapping_and_active_content_rejection_are_explicit() {
 let word=validate_office_chart_input("chart.docx",WORD).unwrap();assert!(word.has_charts());assert!(word.chart_sources["document"][0].contains("Word段3"));assert!(word.chart_sources["document"][0].contains("Revenue!$B$2:$B$3"));
 let excel=validate_office_chart_input("chart.xlsx",EXCEL).unwrap();assert_eq!(excel.sheets,vec!["Revenue","Controls"]);assert!(excel.chart_sources["Revenue"][0].contains("$B$2:$B$3"));assert!(excel.chart_sources["Controls"].is_empty());
 let rel=br#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdDanger" Type="hyperlink" Target="file:///etc/passwd" TargetMode="External"/></Relationships>"#;
 assert!(validate_office_chart_input("chart.docx",&mutate(WORD,"word/_rels/document.xml.rels",rel)).is_err());
 assert!(validate_office_chart_input("chart.docx",&mutate(WORD,"word/vbaProject.bin",b"synthetic forbidden macro marker")).is_err());
 let xml=br#"<!DOCTYPE document [<!ENTITY local SYSTEM "file:///etc/passwd">]><document>&local;</document>"#;
 assert!(validate_office_chart_input("chart.docx",&mutate(WORD,"word/document.xml",xml)).is_err());
 let xml=br#"<worksheet><c><f>WEBSERVICE("https://example.invalid")</f></c></worksheet>"#;
 assert!(validate_office_chart_input("chart.xlsx",&mutate(EXCEL,"xl/worksheets/sheet1.xml",xml)).is_err());
}
struct Parse(Arc<AtomicUsize>);
#[async_trait::async_trait]
impl KnowledgeDocumentParser for Parse {
 fn default_tier(&self)->ParsingTier { ParsingTier::Flash }
 async fn capabilities(&self)->Result<ParsingCapabilities,KnowledgeBaseError> { Ok(ParsingCapabilities { default_ocr_mode: tron_backend::application::knowledge_base::parsing::OcrMode::Auto,default_tier:ParsingTier::Flash,available_tiers:vec![ParsingTier::Flash],parser_version:"synthetic-chart-native".into(),office_tier:ParsingTier::Flash}) }
 async fn parse(&self,name:&str,_bytes:&[u8],_tier:Option<ParsingTier>)->Result<StructuredParse,KnowledgeBaseError> {
  self.0.fetch_add(1,Ordering::SeqCst);
  let format=if name.ends_with(".xlsx"){"xlsx"}else{"docx"};
  let pages=if format=="xlsx" {2} else {1};
  let middle=serde_json::json!({"schema":"docvortex.middle","schema_version":"2.0","metadata":{"file_suffix":format},"extensions":{"mineru":{"tier":"flash"}},"pages":(0..pages).map(|i|serde_json::json!({"page_idx":i,"blocks":[{"type":"chart","index":0,"content":"Alpha revenue120, Beta revenue240"}]})).collect::<Vec<_>>()});
  // A synthetic equivalent of native MinerU chart-table output; local renderer
  // must supply actual PNG chart evidence in addition to this quantitative text.
  let mut archive=zip::ZipWriter::new(Cursor::new(Vec::new()));
  for (name,value) in [("middle_json.json",serde_json::to_vec(&middle).unwrap()),("structured_content.json",b"{}".to_vec()),("markdown.md",b"Alpha120 Beta240".to_vec())] { archive.start_file(name,zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated)).unwrap();archive.write_all(&value).unwrap(); }
  let bytes=archive.finish().unwrap().into_inner();
  tron_backend::adapters::outbound::mineru_documents::decode_archive(&bytes,ParsingTier::Flash,"synthetic-chart-native","synthetic-no-server-job")
 }
}
struct Render(Arc<AtomicUsize>);
#[async_trait::async_trait]
impl KnowledgeOfficeRenderer for Render {
 async fn render_pdf(&self,name:&str,bytes:&[u8])->Result<Vec<u8>,KnowledgeBaseError> {self.0.fetch_add(1,Ordering::SeqCst);LocalOfficeChartRenderer.render_pdf(name,bytes).await}
}
#[cfg(target_os="macos")]
#[tokio::test]
#[ignore="requires installed LibreOffice and macOS local sandbox; synthetic only"]
async fn local_charts_are_cached_private_and_preserve_original_sources() {
 let f=Fixture::new();let parses=Arc::new(AtomicUsize::new(0));let renders=Arc::new(AtomicUsize::new(0));
 let service=f.service.clone().with_document_parser(Arc::new(Parse(parses.clone()))).with_office_renderer(Arc::new(Render(renders.clone()))).with_visual_context(true);
 let kb=service.create(USER,"Synthetic chart evidence","no user data").unwrap();
 for (name,raw) in [("chart.docx",WORD),("chart.xlsx",EXCEL)] {
  let doc=service.upload(USER,&kb.id,name,raw).unwrap();
  let preview=service.preview(USER,&kb.id,&doc.id).await.unwrap();
  let visuals:Vec<_>=preview.chunks.iter().filter(|c|c.location.rendered_page.is_some()).collect();assert!(!visuals.is_empty());
  for chunk in &visuals {assert!(chunk.location.page.is_none());assert!(!chunk.location.chart_sources.is_empty());assert!(!chunk.location.image_resource_codes.is_empty());if name.ends_with("xlsx"){assert_eq!(chunk.location.sheet.as_deref(),Some("Revenue"));}else{assert!(chunk.location.chart_sources[0].contains("Word段3"));}}
  let pc=parses.load(Ordering::SeqCst);let rc=renders.load(Ordering::SeqCst);
  let cached=service.preview(USER,&kb.id,&doc.id).await.unwrap();assert_eq!(cached.chunks.len(),preview.chunks.len());assert_eq!(parses.load(Ordering::SeqCst),pc);assert_eq!(renders.load(Ordering::SeqCst),rc);
  let ready=service.index(USER,&kb.id,&doc.id).await.unwrap();assert_eq!(ready.status,"READY","{:?}",ready.error);assert_eq!(renders.load(Ordering::SeqCst),rc);
  service.set_grants(USER,&kb.id,vec![f.agent.clone()]).unwrap();
  let hits=service.search_for_agent(USER,WORKSPACE,&f.agent,"Alpha",8000).await.unwrap();
  let images=service.visual_evidence_for_agent(USER,WORKSPACE,&f.agent,&hits.hits).unwrap();assert!(!images.evidence.is_empty());
  assert!(service.visual_evidence_for_agent("OTHER-USER",WORKSPACE,&f.agent,&hits.hits).is_err());
 }
 assert_eq!(renders.load(Ordering::SeqCst),2);assert_eq!(parses.load(Ordering::SeqCst),2);
}
