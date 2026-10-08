use super::*;
use std::collections::BTreeSet;

impl SqliteKnowledgeBaseRepository {
    pub fn set_agent_libraries(
        &self,
        user: &str,
        workspace: &str,
        agent: &str,
        library_ids: &[String],
    ) -> Result<(), KnowledgeBaseError> {
        let (user, workspace, agent, library_ids) = (
            user.to_owned(),
            workspace.to_owned(),
            agent.to_owned(),
            library_ids.to_vec(),
        );
        self.connection.call(move |connection| {
            let tx = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(connection, rusqlite::TransactionBehavior::Immediate).map_err(storage)?;
            let owned = {
                let mut statement = tx.prepare("SELECT id FROM knowledge_bases WHERE owner_user_code=?1 AND workspace_code=?2").map_err(storage)?;
                statement.query_map(params![user, workspace], |row| row.get::<_, String>(0)).map_err(storage)?
                    .collect::<Result<BTreeSet<_>, _>>().map_err(storage)?
            };
            let selected = library_ids.into_iter().collect::<BTreeSet<_>>();
            if !selected.is_subset(&owned) { return Err(KnowledgeBaseError::NotFound); }
            for id in owned {
                let granted = tx.query_row("SELECT EXISTS(SELECT 1 FROM knowledge_base_grants WHERE knowledge_base_id=?1 AND agent_code=?2)", params![id, agent], |row| row.get::<_, bool>(0)).map_err(storage)?;
                if granted == selected.contains(&id) { continue; }
                if selected.contains(&id) {
                    tx.execute("INSERT INTO knowledge_base_grants(knowledge_base_id,agent_code) VALUES(?1,?2)", params![id, agent]).map_err(storage)?;
                } else {
                    tx.execute("DELETE FROM knowledge_base_grants WHERE knowledge_base_id=?1 AND agent_code=?2", params![id, agent]).map_err(storage)?;
                }
                tx.execute("UPDATE knowledge_bases SET acl_revision=acl_revision+1 WHERE id=?1", [&id]).map_err(storage)?;
            }
            tx.commit().map_err(storage)
        }, || storage("database worker stopped"))
    }
}
