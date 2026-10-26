//! Document-level chunking and parsing settings, with parse-cache invalidation.
use super::*;

impl KnowledgeBaseService {
    pub async fn configure_parsing(
        &self,
        user: &str,
        kb: &str,
        doc: &str,
        config: Option<ChunkConfig>,
        tier: Option<Option<ParsingTier>>,
        ocr: Option<Option<OcrMode>>,
    ) -> Result<KnowledgeBaseDocument, KnowledgeBaseError> {
        let mut document = self
            .prepare_parsing_configuration(user, kb, doc, config, tier, ocr)
            .await?;
        let revision = document.revision;
        document.revision += 1;
        self.repository.save_document(kb, &document, revision)?;
        Ok(document)
    }
    pub(super) async fn prepare_parsing_configuration(
        &self,
        user: &str,
        kb: &str,
        doc: &str,
        config: Option<ChunkConfig>,
        tier: Option<Option<ParsingTier>>,
        ocr: Option<Option<OcrMode>>,
    ) -> Result<KnowledgeBaseDocument, KnowledgeBaseError> {
        self.owned(user, kb)?;
        if let Some(Some(selected)) = tier {
            let filename = self
                .repository
                .document(kb, doc)?
                .file_name
                .to_ascii_lowercase();
            let extension = std::path::Path::new(&filename)
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or_default();
            if crate::application::document::preflight::DIRECT_READ_EXTENSIONS.contains(&extension)
            {
                return Err(KnowledgeBaseError::Invalid(
                    "Markdown使用直接读取，无需解析质量覆盖".into(),
                ));
            }
            let capabilities = self.parsing_capabilities(user).await?;
            if !capabilities.available_tiers.contains(&selected) {
                return Err(KnowledgeBaseError::Invalid("选定解析质量不可用".into()));
            }
        }
        if let Some(Some(mode)) = ocr {
            let document = self.repository.document(kb, doc)?;
            let extension = std::path::Path::new(&document.file_name)
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            if !crate::application::document::preflight::PARSE_EXTENSIONS
                .contains(&extension.as_str())
            {
                return Err(KnowledgeBaseError::Invalid("此文件不需要文字识别".into()));
            }
            self.parser
                .as_ref()
                .ok_or_else(|| KnowledgeBaseError::Invalid("文档解析服务未配置".into()))?
                .with_ocr_mode(mode)?;
        }
        self.prepare_document_configuration(user, kb, doc, config, tier, ocr)
    }
    pub fn configure(
        &self,
        user: &str,
        kb: &str,
        doc: &str,
        config: Option<ChunkConfig>,
    ) -> Result<KnowledgeBaseDocument, KnowledgeBaseError> {
        self.configure_document(user, kb, doc, config, None, None)
    }
    fn configure_document(
        &self,
        user: &str,
        kb: &str,
        doc: &str,
        config: Option<ChunkConfig>,
        tier: Option<Option<ParsingTier>>,
        ocr: Option<Option<OcrMode>>,
    ) -> Result<KnowledgeBaseDocument, KnowledgeBaseError> {
        let mut document = self.prepare_document_configuration(user, kb, doc, config, tier, ocr)?;
        let revision = document.revision;
        document.revision += 1;
        self.repository.save_document(kb, &document, revision)?;
        Ok(document)
    }
    pub(super) fn prepare_document_configuration(
        &self,
        user: &str,
        kb: &str,
        doc: &str,
        config: Option<ChunkConfig>,
        tier: Option<Option<ParsingTier>>,
        ocr: Option<Option<OcrMode>>,
    ) -> Result<KnowledgeBaseDocument, KnowledgeBaseError> {
        self.owned(user, kb)?;
        if let Some(c) = &config {
            c.validate()
                .map_err(|e| KnowledgeBaseError::Invalid(e.to_string()))?;
            if !self.chunk_strategies.descriptors().iter().any(|d| d.id == c.mode.strategy_id()) {
                return Err(KnowledgeBaseError::Invalid("切片策略未注册".into()));
            }
        }
        let mut doc = self.repository.document(kb, doc)?;
        if ingestion_worker::in_progress(&doc.status) {
            return Err(KnowledgeBaseError::Conflict);
        }
        doc.chunk_config = config;
        let previous_parsing = (doc.parsing_tier, doc.ocr_mode);
        if let Some(tier) = tier {
            doc.parsing_tier = tier;
        }
        if let Some(ocr) = ocr {
            doc.ocr_mode = ocr;
        }
        if tier.is_some() || ocr.is_some() {
            let preference = if doc.parsing_tier == Some(ParsingTier::Advanced) {
                DocumentParsePreference::Layout
            } else if doc.ocr_mode == Some(OcrMode::Ocr) {
                DocumentParsePreference::Recognition
            } else if doc.parsing_tier.is_none() && doc.ocr_mode.is_none() {
                DocumentParsePreference::Auto
            } else {
                DocumentParsePreference::Native
            };
            self.replan_document(user, kb, &mut doc, preference)?;
            if previous_parsing != (doc.parsing_tier, doc.ocr_mode) {
                // Index-only rebuilds may reuse cached parsing, but an explicit
                // parsing change must enqueue a new parse. Derived resources
                // stay registered until successful publication cleans them up.
                doc.parsing_artifact = None;
            }
        }
        doc.status = if fidelity::blocked(&doc) {
            "REVIEW_REQUIRED"
        } else if doc.active_source_code.is_some() {
            "NEEDS_REBUILD"
        } else {
            "UPLOADED"
        }
        .into();
        doc.error = fidelity::blocked(&doc)
            .then(|| format!("解析质量校验失败，索引未发布：{}；请核对原件及图片",fidelity::failure_reason(&doc)));
        Ok(doc)
    }
}
