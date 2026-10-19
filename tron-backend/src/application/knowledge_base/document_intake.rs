use super::*;
use crate::application::document::preflight::{
    DocumentExtractionMethod, DocumentParsingRouter, FeatureBasedDocumentRouter, inspect_document,
};

pub(super) fn apply_plan(doc: &mut KnowledgeBaseDocument, plan: DocumentParsePlan) {
    doc.parsing_tier = Some(if plan.method == DocumentExtractionMethod::Layout {
        ParsingTier::Advanced
    } else {
        ParsingTier::Standard
    });
    doc.ocr_mode = Some(if plan.method == DocumentExtractionMethod::Recognition {
        OcrMode::Ocr
    } else {
        OcrMode::Text
    });
    doc.parse_plan = Some(plan);
}
impl KnowledgeBaseService {
    /// Local inspection only. Does not store resources, enqueue tasks or call suppliers.
    pub fn preflight_document(
        &self,
        user: &str,
        kb: &str,
        filename: &str,
        bytes: &[u8],
        preference: Option<DocumentParsePreference>,
    ) -> Result<DocumentParsePlan, KnowledgeBaseError> {
        let library = self.owned(user, kb)?;
        let features = inspect_document(filename, bytes)
            .map_err(|e| KnowledgeBaseError::Invalid(e.to_string()))?;
        let plan = FeatureBasedDocumentRouter.plan(
            features,
            preference.unwrap_or(library.default_parse_preference),
        );
        Ok(plan)
    }
    pub(super) fn replan_document(
        &self,
        user: &str,
        kb: &str,
        doc: &mut KnowledgeBaseDocument,
        preference: DocumentParsePreference,
    ) -> Result<(), KnowledgeBaseError> {
        let (_, original) = self.resources.download_binary_to_file(
            user,
            ResourceBinaryDownloadCommand {
                workspace_code: "PLATFORM".into(),
                resource_code: doc.resource_code.clone(),
            },
        )?;
        let mut bytes = Vec::new();
        self.resources
            .download_binary_storage(&original, &mut bytes)?;
        let plan = self.preflight_document(user, kb, &doc.file_name, &bytes, Some(preference))?;
        apply_plan(doc, plan);
        Ok(())
    }
}
