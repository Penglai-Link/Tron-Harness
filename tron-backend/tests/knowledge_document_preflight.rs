mod common;
#[path = "support/knowledge_base_fixture.rs"]
mod fixture;
use fixture::*;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use tron_backend::application::{
    document::preflight::*,
    knowledge_base::{local_documents::*, parsing::*, *},
};

#[derive(Clone)]
struct CountingParser {
    calls: Arc<AtomicUsize>,
    inputs: Arc<Mutex<Vec<(String, Vec<u8>, OcrMode)>>>,
    mode: OcrMode,
}
#[async_trait::async_trait]
impl KnowledgeDocumentParser for CountingParser {
    fn default_tier(&self) -> ParsingTier {
        ParsingTier::Standard
    }
    fn with_ocr_mode(
        &self,
        mode: OcrMode,
    ) -> Result<Arc<dyn KnowledgeDocumentParser>, KnowledgeBaseError> {
        Ok(Arc::new(Self {
            mode,
            ..self.clone()
        }))
    }
    fn profile_identity(&self) -> serde_json::Value {
        serde_json::json!({"parser":"counting","mode":self.mode})
    }
    async fn capabilities(&self) -> Result<ParsingCapabilities, KnowledgeBaseError> {
        Ok(ParsingCapabilities {
            default_tier: ParsingTier::Standard,
            default_ocr_mode: OcrMode::Text,
            available_tiers: vec![ParsingTier::Standard, ParsingTier::Advanced],
            parser_version: "fixture-v1".into(),
            office_tier: ParsingTier::Flash,
        })
    }
    async fn parse(
        &self,
        filename: &str,
        bytes: &[u8],
        tier: Option<ParsingTier>,
    ) -> Result<StructuredParse, KnowledgeBaseError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.inputs
            .lock()
            .unwrap()
            .push((filename.into(), bytes.to_vec(), self.mode));
        Ok(StructuredParse {
            document: parse_local_document(
                "result.md",
                b"# Original evidence\nThe supplier parsed the original document exactly once.",
            )
            .unwrap(),
            middle: serde_json::json!([]),
            structured: serde_json::json!({}),
            markdown: "Original evidence".into(),
            assets: vec![],
            effective_tier: tier.unwrap_or_default(),
            parser_version: "fixture-v1".into(),
            vendor_job_id: "supplier-task".into(),
        })
    }
}
fn parser() -> CountingParser {
    CountingParser {
        calls: Arc::new(AtomicUsize::new(0)),
        inputs: Arc::new(Mutex::new(vec![])),
        mode: OcrMode::Auto,
    }
}

#[tokio::test]
async fn preflight_has_no_supplier_side_effects_and_old_office_is_forwarded_unchanged_once() {
    let f = Fixture::new();
    let p = parser();
    let service = f.service.clone().with_document_parser(Arc::new(p.clone()));
    let kb = service.create(USER, "Auto", "").unwrap();
    assert_eq!(kb.default_parse_preference, DocumentParsePreference::Auto);
    let mut source = b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1".to_vec();
    source.extend_from_slice(b"synthetic binary document payload");
    for name in ["report.doc", "slides.ppt", "book.xls"] {
        let plan = service
            .preflight_document(USER, &kb.id, name, &source, None)
            .unwrap();
        assert_eq!(plan.method, DocumentExtractionMethod::Recognition);
        // Legacy Office has a fixed OCR policy even if the library or upload
        // asks for native extraction or precise layout.
        for preference in [
            DocumentParsePreference::Native,
            DocumentParsePreference::Layout,
        ] {
            assert_eq!(
                service
                    .preflight_document(USER, &kb.id, name, &source, Some(preference))
                    .unwrap()
                    .method,
                DocumentExtractionMethod::Recognition
            );
        }
    }
    assert!(
        service
            .preflight_document("OTHER-USER", &kb.id, "report.doc", &source, None)
            .is_err()
    );
    assert!(
        service
            .preflight_document(USER, &kb.id, "program.exe", &source, None)
            .is_err()
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 0);
    assert_eq!(service.detail(USER, &kb.id).unwrap().document_count, 0);
    let plan = service
        .preflight_document(USER, &kb.id, "report.doc", &source, None)
        .unwrap();
    let mut stale = plan.clone();
    stale.features.source_sha256 = "stale".into();
    assert!(
        service
            .upload_with_parsing_options(
                USER,
                &kb.id,
                "report.doc",
                &source,
                Some("stale"),
                Some(DocumentParsePreference::Auto),
                Some(&stale)
            )
            .is_err()
    );
    assert_eq!(service.detail(USER, &kb.id).unwrap().document_count, 0);
    let doc = service.upload(USER, &kb.id, "report.doc", &source).unwrap();
    assert_eq!(doc.parse_plan.as_ref().unwrap(), &plan);
    let ready = service.index(USER, &kb.id, &doc.id).await.unwrap();
    assert_eq!(ready.status, "READY", "{:?}", ready.error);
    assert_eq!(p.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        p.inputs.lock().unwrap()[0],
        ("report.doc".into(), source.clone(), OcrMode::Ocr)
    );
    service
        .configure(
            USER,
            &kb.id,
            &doc.id,
            Some(ChunkConfig {
                max_chars: 400,
                overlap_chars: 40,
                ..Default::default()
            }),
        )
        .unwrap();
    let rebuilt = service.index(USER, &kb.id, &doc.id).await.unwrap();
    assert_eq!(rebuilt.status, "READY", "{:?}", rebuilt.error);
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        1,
        "rebuilding chunks must reuse the parsing artifact"
    );
    assert_eq!(
        service.detail(USER, &kb.id).unwrap().documents[0].parse_plan,
        Some(plan)
    );
}

#[tokio::test]
async fn text_is_read_directly_and_a_saved_preference_is_selected_before_indexing() {
    let f = Fixture::new();
    let p = parser();
    let service = f.service.clone().with_document_parser(Arc::new(p.clone()));
    let kb = service
        .create_with_parsing_settings(
            USER,
            "Precise",
            "",
            Default::default(),
            DocumentParsePreference::Layout,
        )
        .unwrap();
    let source = "# This remains literal text\n中文 evidence\n";
    let doc = service
        .upload(USER, &kb.id, "notes.txt", source.as_bytes())
        .unwrap();
    assert!(doc.parse_plan.as_ref().unwrap().features.local_text);
    let ready = service.index(USER, &kb.id, &doc.id).await.unwrap();
    assert_eq!(ready.status, "READY", "{:?}", ready.error);
    assert_eq!(p.calls.load(Ordering::SeqCst), 0);
    let plan = service
        .preflight_document(USER, &kb.id, "paper.pdf", &fixture::pdf(), None)
        .unwrap();
    assert_eq!(plan.preference, DocumentParsePreference::Layout);
    assert_eq!(plan.method, DocumentExtractionMethod::Layout);
    assert_eq!(
        service
            .detail(USER, &kb.id)
            .unwrap()
            .default_parse_preference,
        DocumentParsePreference::Layout
    );
}

#[test]
fn unreadable_pdf_structure_is_unknown_quality_not_a_preflight_upload_failure() {
    let bytes = b"%PDF-1.7\n1 0 obj\n<< /Type /Catalog >>\nendobj\ntrailer broken\n%%EOF";
    let facts = inspect_document("recoverable.pdf", bytes).unwrap();
    assert!(facts.uncertain);
    assert_eq!(facts.units, 0, "unavailable page count must not be invented");
    let plan = FeatureBasedDocumentRouter.plan(facts, DocumentParsePreference::Auto);
    assert_eq!(plan.method, DocumentExtractionMethod::Layout);
    assert_eq!(plan.reason, "uncertain_structure");
    assert!(inspect_document("fake.pdf", b"not a PDF").is_err());
    let mut large = b"%PDF-1.7\n".to_vec();
    large.resize(4 * 1024 * 1024 + 1, b' ');
    let facts = inspect_document("large.pdf", &large).unwrap();
    assert!(facts.uncertain);
    assert_eq!(facts.inspected_units, 0);
    assert_eq!(facts.units, 0);
}

#[test]
fn predefined_cid_encoding_uses_layout_without_invoking_unsupported_glyph_decoder() {
    let mut document = pdf_extract::Document::load_mem(&fixture::pdf()).unwrap();
    for object in document.objects.values_mut() {
        if let pdf_extract::Object::Dictionary(font) = object {
            if font.get(b"Type").and_then(pdf_extract::Object::as_name).ok() == Some(b"Font") {
                font.set("Subtype", pdf_extract::Object::Name(b"Type0".to_vec()));
                font.set("Encoding", pdf_extract::Object::Name(b"GBK-EUC-H".to_vec()));
            }
        }
    }
    let mut source = Vec::new();
    document.save_to(&mut source).unwrap();
    let facts = inspect_document("cid.pdf", &source).unwrap();
    assert!(facts.uncertain);
    assert_eq!(facts.units, 1);
    assert_eq!(facts.text_characters, 0);
    assert_eq!(FeatureBasedDocumentRouter.plan(facts, DocumentParsePreference::Auto).method, DocumentExtractionMethod::Layout);
}

#[test]
fn format_validation_uses_original_structure_and_rejects_disguised_or_unsupported_files() {
    assert_eq!(
        inspect_document("report.docx", &fixture::office())
            .unwrap()
            .format,
        "docx"
    );
    assert_eq!(
        inspect_document("book.xlsx", &fixture::spreadsheet())
            .unwrap()
            .format,
        "xlsx"
    );
    assert!(inspect_document("fake.pptx", &fixture::office()).is_err());
    assert!(inspect_document("fake.pdf", b"not a PDF").is_err());
    assert!(inspect_document("script.js", b"alert(1)").is_err());
    let utf16: Vec<u8> = [0xff, 0xfe]
        .into_iter()
        .chain("中文 text".encode_utf16().flat_map(u16::to_le_bytes))
        .collect();
    let doc = parse_local_document("notes.txt", &utf16).unwrap();
    assert!(doc.sections[0].text.contains("中文 text"));
    assert!(inspect_document("empty.txt", b"").is_err());
}

#[tokio::test]
async fn preflight_http_rejects_unsupported_formats_without_creating_documents_or_tasks() {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let f = Fixture::new();
    let kb = f.service.create(USER, "HTTP preflight", "").unwrap();
    let foreign = f.service.create("FOREIGN", "Private", "").unwrap();
    let mut state = f.state.clone();
    state.knowledge_bases = f.service.clone();
    let app = tron_backend::app::build_router_with_state(state).unwrap();
    let task_count = f
        .state
        .rag
        .record_task_views(USER)
        .unwrap()
        .into_iter()
        .filter(|task| task.corpus_id == kb.id)
        .count();
    for (id, name, expected) in [
        (kb.id.as_str(), "notes.txt", StatusCode::OK),
        (kb.id.as_str(), "script.js", StatusCode::BAD_REQUEST),
        (foreign.id.as_str(), "notes.txt", StatusCode::NOT_FOUND),
        ("missing-library", "notes.txt", StatusCode::NOT_FOUND),
    ] {
        let body = format!(
            "--preflight-contract\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\nContent-Type: text/plain\r\n\r\nSynthetic evidence\r\n--preflight-contract--\r\n"
        );
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/knowledge-bases/{id}/documents/preflight"))
                    .header(
                        "content-type",
                        "multipart/form-data; boundary=preflight-contract",
                    )
                    .header("accept-language", "en-US")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
        let json = common::response_json(response).await;
        if expected == StatusCode::OK {
            assert_eq!(json["code"], "0");
            assert_eq!(json["data"]["plan"]["features"]["localText"], true);
            assert_eq!(json["data"]["plan"]["preference"], "Auto");
        } else {
            assert_ne!(json["code"], "0");
            assert!(json["message"].as_str().unwrap().is_ascii());
        }
    }
    assert_eq!(f.service.detail(USER, &kb.id).unwrap().document_count, 0);
    assert_eq!(
        f.state
            .rag
            .record_task_views(USER)
            .unwrap()
            .into_iter()
            .filter(|task| task.corpus_id == kb.id)
            .count(),
        task_count
    );
    f.state.agent_framework.close().await;
}
