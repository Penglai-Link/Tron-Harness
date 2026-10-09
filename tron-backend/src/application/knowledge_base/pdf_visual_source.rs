//! Explicit local page-image reprocessing, bound to the immutable original and audit resources.
use super::*;
use serde::{Deserialize, Serialize};
use std::io;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PdfVisualSource {
    pub original_sha256: String,
    pub derived_sha256: String,
    pub derived_resource_code: String,
    pub audit_resource_code: String,
    pub page_count: usize,
    pub page_image_sha256: Vec<String>,
    #[serde(default)]
    pub original_parse_resource_code: Option<String>,
    pub original_findings: Vec<fidelity::FidelityIssue>,
    pub renderer: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Audit {
    original_resource_code: String,
    original_resource_revision: u64,
    original_sha256: String,
    derived_sha256: String,
    page_image_sha256: Vec<String>,
    #[serde(default)]
    original_parse_resource_code: Option<String>,
    original_findings: Vec<fidelity::FidelityIssue>,
    method: String,
    actor_user_code: String,
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
/// Page pixels replace text extraction from ambiguous PDF font maps; no glyph is guessed.
/// CoreGraphics renders locally. The existing configured MinerU performs the next parse.
fn image_pdf(source: &[u8]) -> Result<(Vec<u8>, Vec<String>), KnowledgeBaseError> {
    use pdf_extract::{
        Document, Object, Stream,
        content::{Content, Operation},
        dictionary,
    };
    let rendered = eliteforge_agent_memory::render_pdf_evidence(source, Default::default())
        .map_err(invalid_storage)?;
    let mut document = Document::with_version("1.7");
    let pages_id = document.new_object_id();
    let mut pages = Vec::new();
    let mut hashes = Vec::new();
    for page in rendered {
        hashes.push(digest(&page.png));
        let rgb = image::load_from_memory_with_format(&page.png, image::ImageFormat::Png)
            .map_err(invalid_storage)?
            .to_rgb8();
        let mut stream = Stream::new(
            dictionary! {"Type"=>"XObject","Subtype"=>"Image","Width"=>page.width as i64,"Height"=>page.height as i64,"ColorSpace"=>"DeviceRGB","BitsPerComponent"=>8},
            rgb.into_raw(),
        );
        stream.compress().map_err(invalid_storage)?;
        let image_id = document.add_object(stream);
        let resources =
            document.add_object(dictionary! {"XObject"=>dictionary!{"PageImage"=>image_id}});
        let content = Content {
            operations: vec![
                Operation::new("q", vec![]),
                Operation::new(
                    "cm",
                    vec![
                        (page.width_points as f32).into(),
                        0.into(),
                        0.into(),
                        (page.height_points as f32).into(),
                        0.into(),
                        0.into(),
                    ],
                ),
                Operation::new("Do", vec![Object::Name(b"PageImage".to_vec())]),
                Operation::new("Q", vec![]),
            ],
        };
        let stream = document.add_object(Stream::new(
            dictionary! {},
            content.encode().map_err(invalid_storage)?,
        ));
        let id=document.add_object(dictionary!{"Type"=>"Page","Parent"=>pages_id,"MediaBox"=>vec![0.into(),0.into(),(page.width_points as f32).into(),(page.height_points as f32).into()],"Contents"=>stream,"Resources"=>resources});
        pages.push(id.into());
    }
    let count = pages.len();
    document.objects.insert(
        pages_id,
        dictionary! {"Type"=>"Pages","Kids"=>pages,"Count"=>count as i64}.into(),
    );
    let catalog = document.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages_id});
    document.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).map_err(invalid_storage)?;
    if bytes.len() > local_documents::MAX_RAW_BYTES {
        return Err(KnowledgeBaseError::Invalid(
            "原页图像副本超过20 MiB，未替换原件".into(),
        ));
    }
    Ok((bytes, hashes))
}
impl KnowledgeBaseService {
    pub async fn reparse_visual_document(
        &self,
        user: &str,
        kb: &str,
        doc: &str,
    ) -> Result<KnowledgeBaseDocument, KnowledgeBaseError> {
        self.prepare_visual_reparse(user, kb, doc)
            .await?;
        self.preview(user, kb, doc).await?;
        self.owned(user, kb)?;
        self.repository.document(kb, doc)
    }

    /// Does not index or declare semantic correctness. Fresh parse/fidelity checks are still required.
    pub async fn prepare_visual_reparse(
        &self,
        user: &str,
        kb: &str,
        doc_id: &str,
    ) -> Result<KnowledgeBaseDocument, KnowledgeBaseError> {
        let workspace = "PLATFORM";
        self.owned(user, kb)?;
        let mut doc = self.repository.document(kb, doc_id)?;
        if !doc.file_name.to_ascii_lowercase().ends_with(".pdf") || doc.status != "REVIEW_REQUIRED"
        {
            return Err(KnowledgeBaseError::Invalid(
                "仅待核对的PDF可从原页图像重新识别".into(),
            ));
        }
        let parser = self
            .parser
            .as_ref()
            .ok_or_else(|| KnowledgeBaseError::Invalid("本地解析器未配置".into()))?;
        if !parser.supports_local_visual_reparse()
            || doc.parsing_tier.unwrap_or(parser.default_tier()) == ParsingTier::Flash
        {
            return Err(KnowledgeBaseError::Invalid(
                "重新识别需要本机MinerU及支持OCR的解析质量；不会外发或自动降级".into(),
            ));
        }
        let original = self.visual_source_bytes(
            user,
            workspace,
            &doc.resource_code,
            local_documents::MAX_RAW_BYTES,
        )?;
        if doc.visual_source.is_some() {
            return Ok(doc);
        }
        let findings = doc
            .parsing_artifact
            .as_ref()
            .map(|a| a.fidelity_issues.clone())
            .unwrap_or_default();
        if findings.is_empty() {
            return Err(KnowledgeBaseError::Invalid(
                "缺少原始风险来源，不能跳过核验".into(),
            ));
        }
        let original_sha256 = digest(&original);
        let (derived, hashes) = tokio::task::spawn_blocking(move || image_pdf(&original))
            .await
            .map_err(invalid_storage)??;
        self.owned(user, kb)?;
        let derived_sha256 = digest(&derived);
        let derived_resource_code = self.derived_resource(
            user,
            workspace,
            &doc,
            "original-page-images.pdf",
            ResourceKind::Pdf,
            &derived,
        )?;
        let original_parse_resource_code = doc
            .parsing_artifact
            .as_ref()
            .map(|a| a.resource_code.clone());
        let audit=Audit{original_parse_resource_code:original_parse_resource_code.clone(),original_resource_code:doc.resource_code.clone(),original_resource_revision:doc.resource_revision,original_sha256:original_sha256.clone(),derived_sha256:derived_sha256.clone(),page_image_sha256:hashes.clone(),original_findings:findings.clone(),method:"framework-coregraphics-local-raster-v2; original displayed crop-box geometry and page order; no font substitutions; fresh MinerU OCR required".into(),actor_user_code:user.into()};
        let audit_resource_code = self.derived_resource(
            user,
            workspace,
            &doc,
            "source-reprocessing-audit.json",
            ResourceKind::File,
            &serde_json::to_vec(&audit).map_err(invalid_storage)?,
        )?;
        let revision = doc.revision;
        doc.revision += 1;
        doc.visual_source = Some(PdfVisualSource {
            original_sha256,
            derived_sha256,
            derived_resource_code: derived_resource_code.clone(),
            audit_resource_code: audit_resource_code.clone(),
            page_count: hashes.len(),
            page_image_sha256: hashes,
            original_parse_resource_code,
            original_findings: findings,
            renderer: "Framework CoreGraphics local raster v2".into(),
        });
        doc.derived_resource_codes
            .extend([derived_resource_code, audit_resource_code]);
        // Retain REVIEW_REQUIRED and the raw parse until the new derived input is actually parsed.
        self.repository.save_document(kb, &doc, revision)?;
        Ok(doc)
    }
}

struct Bounded(Vec<u8>, usize);
impl io::Write for Bounded {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.0.len().saturating_add(bytes.len()) > self.1 {
            return Err(io::Error::other("visual source exceeds bound"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl KnowledgeBaseService {
    fn visual_source_bytes(
        &self,
        user: &str,
        workspace: &str,
        code: &str,
        limit: usize,
    ) -> Result<Vec<u8>, KnowledgeBaseError> {
        let (_, record) = self.resources.download_binary_to_file(
            user,
            ResourceBinaryDownloadCommand {
                workspace_code: workspace.into(),
                resource_code: code.into(),
            },
        )?;
        if record.byte_size > limit as u64 {
            return Err(KnowledgeBaseError::Invalid(
                "数据保真核验资源超出上限".into(),
            ));
        }
        let mut bytes = Bounded(Vec::new(), limit);
        self.resources
            .download_binary_storage(&record, &mut bytes)?;
        Ok(bytes.0)
    }
}
