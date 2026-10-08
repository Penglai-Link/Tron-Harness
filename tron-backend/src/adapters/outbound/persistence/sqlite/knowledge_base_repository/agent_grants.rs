use super::*;
use std::collections::BTreeSet;

impl SqliteKnowledgeBaseRepository {
    pub fn replace_library_agent_grants(
        &self,
        user: &str,
        id: &str,
        agents: &[String],
    ) -> Result<(), KnowledgeBaseError> {
        let (user, id, selected) = (
            user.to_owned(),
            id.to_owned(),
            agents.iter().cloned().collect::<BTreeSet<_>>(),
        );
        self.connection.call(move |connection| {
            let tx = super::super::framework_storage::business_transaction(connection, rusqlite::TransactionBehavior::Immediate).map_err(storage)?;
            let owned: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM knowledge_bases WHERE id=?1 AND owner_user_code=?2)", params![id, user], |row| row.get(0)).map_err(storage)?;
            if !owned { return Err(KnowledgeBaseError::NotFound); }
            tx.execute("DELETE FROM knowledge_base_grants WHERE knowledge_base_id=?1", [&id]).map_err(storage)?;
            for agent in selected { tx.execute("INSERT INTO knowledge_base_grants VALUES(?1,?2)", params![id, agent]).map_err(storage)?; }
            tx.execute("UPDATE knowledge_bases SET acl_revision=acl_revision+1 WHERE id=?1", [&id]).map_err(storage)?;
            tx.commit().map_err(storage)
        }, || storage("database worker stopped"))
    }

    /// A knowledge grant belongs to the Agent and follows it into assigned spaces.
    pub fn set_agent_libraries(
        &self,
        user: &str,
        agent: &str,
        library_ids: &[String],
    ) -> Result<(), KnowledgeBaseError> {
        let (user, agent, library_ids) = (user.to_owned(), agent.to_owned(), library_ids.to_vec());
        self.connection.call(move |connection| {
            let tx = super::super::framework_storage::business_transaction(connection, rusqlite::TransactionBehavior::Immediate).map_err(storage)?;
            let owned = {
                let mut statement = tx.prepare("SELECT id FROM knowledge_bases WHERE owner_user_code=?1").map_err(storage)?;
                statement.query_map([user], |row| row.get::<_, String>(0)).map_err(storage)?.collect::<Result<BTreeSet<_>, _>>().map_err(storage)?
            };
            let selected = library_ids.into_iter().collect::<BTreeSet<_>>();
            if !selected.is_subset(&owned) { return Err(KnowledgeBaseError::NotFound); }
            for id in owned {
                let granted: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM knowledge_base_grants WHERE knowledge_base_id=?1 AND agent_code=?2)", params![id, agent], |row| row.get(0)).map_err(storage)?;
                if granted == selected.contains(&id) { continue; }
                if selected.contains(&id) {
                    tx.execute("INSERT INTO knowledge_base_grants VALUES(?1,?2)", params![id, agent]).map_err(storage)?;
                } else {
                    tx.execute("DELETE FROM knowledge_base_grants WHERE knowledge_base_id=?1 AND agent_code=?2", params![id, agent]).map_err(storage)?;
                }
                tx.execute("UPDATE knowledge_bases SET acl_revision=acl_revision+1 WHERE id=?1", [&id]).map_err(storage)?;
            }
            tx.commit().map_err(storage)
        }, || storage("database worker stopped"))
    }
}
