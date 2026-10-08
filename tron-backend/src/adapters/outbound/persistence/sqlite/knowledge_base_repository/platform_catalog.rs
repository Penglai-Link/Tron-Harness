use super::*;

impl SqliteKnowledgeBaseRepository {
    pub fn platform_libraries(&self, user: &str) -> Result<Vec<KnowledgeBase>, KnowledgeBaseError> {
        let user = user.to_owned();
        self.connection.call(move |connection| {
            let mut statement = connection.prepare("SELECT payload_json FROM knowledge_bases WHERE owner_user_code=?1 ORDER BY id").map_err(storage)?;
            let rows = statement.query_map([user], |row| row.get::<_,String>(0)).map_err(storage)?;
            rows.map(|row| decode(&row.map_err(storage)?)).collect()
        }, || storage("database worker stopped"))
    }
}
