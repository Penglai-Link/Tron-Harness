use super::*;

#[derive(Clone)]
pub struct KnowledgeLibrarySettings {
    pub name: String,
    pub description: String,
    pub default_parse_preference: DocumentParsePreference,
    pub index_settings: RagSettings,
}

impl KnowledgeBaseService {
    pub fn library_settings(
        &self,
        user: &str,
        id: &str,
    ) -> Result<KnowledgeLibrarySettings, KnowledgeBaseError> {
        let library = self.owned(user, id)?;
        Ok(KnowledgeLibrarySettings {
            name: library.name,
            description: library.description,
            default_parse_preference: library.default_parse_preference,
            index_settings: self.rag.settings_for_corpus(id)?,
        })
    }

    pub fn update_library_settings(
        &self,
        user: &str,
        id: &str,
        mut settings: KnowledgeLibrarySettings,
    ) -> Result<KnowledgeLibrarySettings, KnowledgeBaseError> {
        self.owned(user, id)?;
        settings.name = settings.name.trim().into();
        settings.description = settings.description.trim().into();
        if settings.name.is_empty()
            || settings.name.chars().count() > 120
            || settings.description.chars().count() > 2000
        {
            return Err(KnowledgeBaseError::invalid(
                "knowledge.base.settings_invalid",
            ));
        }
        settings
            .index_settings
            .validate()
            .map_err(|_| KnowledgeBaseError::invalid("knowledge.base.index_settings_invalid"))?;
        if settings.index_settings.graph && settings.index_settings.graph_model_code.is_none() {
            return Err(KnowledgeBaseError::invalid(
                "knowledge.base.graph_model_required",
            ));
        }
        // Update only library settings, never a stale copy of documents or grants.
        let index_changed = self.repository.save_library_settings(user, id, &settings)?;
        if index_changed {
            // Existing supplement pipeline reuses parsed artifacts and compatible vectors.
            self.supplement_corpus_indexes(user, id)?;
        }
        Ok(settings)
    }
}
