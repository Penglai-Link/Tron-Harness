use super::*;

impl SqliteKnowledgeBaseRepository {
    pub fn acl_revisions(
        &self,
        ids: &[String],
    ) -> Result<std::collections::BTreeMap<String, u64>, KnowledgeBaseError> {
        let ids = serde_json::to_string(ids).map_err(storage)?;
        self.connection.call_read(move |connection| {
            let mut q = connection.prepare("SELECT id,acl_revision FROM knowledge_bases WHERE id IN (SELECT value FROM json_each(?1))").map_err(storage)?;
            q.query_map([ids],|r|Ok((r.get(0)?,r.get(1)?))).map_err(storage)?.collect::<Result<_,_>>().map_err(storage)
        }, || storage("database worker stopped"))
    }
    pub fn library_derived_resources(&self, id: &str) -> Result<Vec<String>, KnowledgeBaseError> {
        let id = id.to_owned();
        self.connection.call_read(move |connection| {
            let mut q = connection.prepare("SELECT r.resource_code FROM knowledge_base_documents d JOIN knowledge_base_derived_resources r ON r.document_id=d.id WHERE d.knowledge_base_id=?1").map_err(storage)?;
            q.query_map([id],|r|r.get(0)).map_err(storage)?.collect::<Result<_,_>>().map_err(storage)
        }, || storage("database worker stopped"))
    }
    pub fn platform_library_summaries(
        &self,
        user: &str,
    ) -> Result<Vec<KnowledgeBase>, KnowledgeBaseError> {
        let user = user.to_owned();
        self.connection.call_read(move |connection| {
            let mut libraries = {
                let mut statement = connection.prepare("SELECT k.payload_json,count(d.id) FROM knowledge_bases k LEFT JOIN knowledge_base_documents d ON d.knowledge_base_id=k.id WHERE k.owner_user_code=?1 GROUP BY k.id ORDER BY k.id").map_err(storage)?;
                statement.query_map([&user], |row| Ok((row.get::<_, String>(0)?, row.get::<_, usize>(1)?))).map_err(storage)?
                    .map(|row| {
                        let (raw, count) = row.map_err(storage)?;
                        let mut library: KnowledgeBase = decode(&raw)?;
                        library.documents.clear();
                        library.agent_codes.clear();
                        library.document_count = count;
                        Ok(library)
                    }).collect::<Result<Vec<_>, KnowledgeBaseError>>()?
            };
            let positions = libraries.iter().enumerate().map(|(index, library)| (library.id.clone(), index))
                .collect::<std::collections::BTreeMap<_, _>>();
            let mut statement = connection.prepare("SELECT g.knowledge_base_id,g.agent_code FROM knowledge_base_grants g JOIN knowledge_bases k ON k.id=g.knowledge_base_id WHERE k.owner_user_code=?1 ORDER BY g.knowledge_base_id,g.agent_code").map_err(storage)?;
            let rows = statement.query_map([&user], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))).map_err(storage)?;
            for row in rows {
                let (library_id, agent_code) = row.map_err(storage)?;
                if let Some(&index) = positions.get(&library_id) {
                    libraries[index].agent_codes.push(agent_code);
                }
            }
            Ok(libraries)
        }, || storage("database worker stopped"))
    }
    pub fn authorized_libraries_with_documents(
        &self,
        user: &str,
        agent: &str,
    ) -> Result<Vec<KnowledgeBase>, KnowledgeBaseError> {
        let user = user.to_owned();
        let agent = agent.to_owned();
        self.connection.call_read(move |connection| {
            let mut libraries = {
                let mut statement = connection.prepare("SELECT k.payload_json FROM knowledge_bases k WHERE k.owner_user_code=?1 AND EXISTS(SELECT 1 FROM knowledge_base_grants g WHERE g.knowledge_base_id=k.id AND g.agent_code=?2) ORDER BY k.id").map_err(storage)?;
                statement.query_map(params![&user,&agent], |row| row.get::<_, String>(0)).map_err(storage)?
                    .map(|row| {
                        let mut library: KnowledgeBase = decode(&row.map_err(storage)?)?;
                        library.documents.clear();
                        Ok(library)
                    }).collect::<Result<Vec<_>, KnowledgeBaseError>>()?
            };
            let positions = libraries.iter().enumerate().map(|(index, library)| (library.id.clone(), index))
                .collect::<std::collections::BTreeMap<_, _>>();
            let mut statement = connection.prepare("SELECT d.knowledge_base_id,d.payload_json FROM knowledge_base_documents d JOIN knowledge_bases k ON k.id=d.knowledge_base_id WHERE k.owner_user_code=?1 AND EXISTS(SELECT 1 FROM knowledge_base_grants g WHERE g.knowledge_base_id=k.id AND g.agent_code=?2) ORDER BY d.knowledge_base_id,d.id").map_err(storage)?;
            let rows = statement.query_map(params![&user,&agent], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))).map_err(storage)?;
            for row in rows {
                let (library_id, raw) = row.map_err(storage)?;
                if let Some(&index) = positions.get(&library_id) {
                    libraries[index].documents.push(decode(&raw)?);
                }
            }
            for library in &mut libraries { library.document_count = library.documents.len(); }
            Ok(libraries)
        }, || storage("database worker stopped"))
    }
    pub fn platform_libraries_with_documents(
        &self,
        user: &str,
    ) -> Result<Vec<KnowledgeBase>, KnowledgeBaseError> {
        let user = user.to_owned();
        self.connection.call_read(move |connection| {
            let mut libraries = {
                let mut statement = connection.prepare("SELECT payload_json FROM knowledge_bases WHERE owner_user_code=?1 ORDER BY id").map_err(storage)?;
                statement.query_map([&user], |row| row.get::<_, String>(0)).map_err(storage)?
                    .map(|row| decode::<KnowledgeBase>(&row.map_err(storage)?))
                    .collect::<Result<Vec<_>, _>>()?
            };
            let positions = libraries.iter().enumerate().map(|(index, library)| (library.id.clone(), index))
                .collect::<std::collections::BTreeMap<_, _>>();
            let mut statement = connection.prepare("SELECT d.knowledge_base_id,d.payload_json FROM knowledge_base_documents d JOIN knowledge_bases k ON k.id=d.knowledge_base_id WHERE k.owner_user_code=?1 ORDER BY d.knowledge_base_id,d.id").map_err(storage)?;
            let documents = statement.query_map([&user], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))).map_err(storage)?;
            for document in documents {
                let (library_id, raw) = document.map_err(storage)?;
                if let Some(&index) = positions.get(&library_id) {
                    libraries[index].documents.push(decode::<KnowledgeBaseDocument>(&raw)?);
                }
            }
            for library in &mut libraries { library.document_count = library.documents.len(); }
            Ok(libraries)
        }, || storage("database worker stopped"))
    }
    pub fn platform_libraries(&self, user: &str) -> Result<Vec<KnowledgeBase>, KnowledgeBaseError> {
        let user = user.to_owned();
        self.connection.call_read(move |connection| {
            let mut statement = connection.prepare("SELECT payload_json FROM knowledge_bases WHERE owner_user_code=?1 ORDER BY id").map_err(storage)?;
            let rows = statement.query_map([user], |row| row.get::<_,String>(0)).map_err(storage)?;
            rows.map(|row| decode(&row.map_err(storage)?)).collect()
        }, || storage("database worker stopped"))
    }
}
