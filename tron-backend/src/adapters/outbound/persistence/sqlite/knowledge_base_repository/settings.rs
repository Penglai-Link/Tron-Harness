use super::*;
use crate::application::{knowledge_base::KnowledgeLibrarySettings, rag::RagSettings};

impl SqliteKnowledgeBaseRepository {
    pub fn save_library_settings(
        &self,
        user: &str,
        id: &str,
        settings: &KnowledgeLibrarySettings,
    ) -> Result<bool, KnowledgeBaseError> {
        let (user, id, settings) = (user.to_owned(), id.to_owned(), settings.clone());
        self.connection.call(move |connection| {
            let tx = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(connection, rusqlite::TransactionBehavior::Immediate).map_err(storage)?;
            let owner: Option<String> = tx.query_row("SELECT owner_user_code FROM knowledge_bases WHERE id=?1", [&id], |r| r.get(0)).optional().map_err(storage)?;
            match owner {
                None => return Err(KnowledgeBaseError::NotFound),
                Some(owner) if owner != user => return Err(KnowledgeBaseError::Forbidden),
                _ => {}
            }
            let capabilities: String = tx.query_row("SELECT capabilities_json FROM rag_corpora WHERE corpus_id=?1", [&id], |r| r.get(0)).map_err(storage)?;
            let capabilities: serde_json::Value = decode(&capabilities)?;
            let old = RagSettings::from_capabilities(&capabilities)?;
            let changed = encode(&old)? != encode(&settings.index_settings)?;
            tx.execute("UPDATE knowledge_bases SET payload_json=json_set(payload_json,'$.name',?2,'$.description',?3,'$.defaultParsePreference',json(?4)) WHERE id=?1", params![id,settings.name,settings.description,encode(&settings.default_parse_preference)?]).map_err(storage)?;
            tx.execute("UPDATE rag_corpora SET display_name=?2,description=?3,capabilities_json=json_set(capabilities_json,'$.settings',json(?4)),updated_at=?5 WHERE corpus_id=?1", params![id,settings.name,settings.description,encode(&settings.index_settings)?,chrono::Utc::now().timestamp_millis()]).map_err(storage)?;
            tx.commit().map_err(storage)?;
            Ok(changed)
        }, || storage("database worker stopped"))
    }
}
