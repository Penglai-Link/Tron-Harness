use super::framework_storage::SharedDatabase;
use crate::{
    application::assistant_memory::AssistantMemoryRepository, domain::assistant_memory::*,
};
use rusqlite::{OptionalExtension, params};
use std::collections::BTreeMap;

pub struct SqliteAssistantMemoryRepository {
    database: SharedDatabase,
}
impl SqliteAssistantMemoryRepository {
    pub fn connect(database_url: &str) -> Result<Self, AssistantMemoryError> {
        Ok(Self {
            database: SharedDatabase::open(database_url).map_err(storage)?,
        })
    }
}
fn storage(error: impl std::fmt::Display) -> AssistantMemoryError {
    AssistantMemoryError::Storage(error.to_string())
}
fn encode(value: &impl serde::Serialize) -> Result<String, AssistantMemoryError> {
    serde_json::to_string(value).map_err(storage)
}
fn decode<T: serde::de::DeserializeOwned>(text: String) -> Result<T, AssistantMemoryError> {
    serde_json::from_str(&text).map_err(storage)
}
impl AssistantMemoryRepository for SqliteAssistantMemoryRepository {
    fn list_batches(
        &mut self,
        user: &str,
        assistant: &str,
    ) -> Result<Vec<MemoryBatch>, AssistantMemoryError> {
        let (user, assistant) = (user.to_owned(), assistant.to_owned());
        self.database.call(move |c| {
            let mut statement=c.prepare("SELECT record_json FROM assistant_memory_batches WHERE user_code=?1 AND assistant_code=?2 ORDER BY created_at DESC,batch_code DESC").map_err(storage)?;
            let rows=statement.query_map(params![user,assistant],|r|r.get::<_,String>(0)).map_err(storage)?;
            rows.map(|row|decode(row.map_err(storage)?)).collect()
        },||storage("memory database unavailable"))
    }
    fn get_batch(&mut self, code: &str) -> Result<MemoryBatch, AssistantMemoryError> {
        let code = code.to_owned();
        self.database.call(
            move |c| {
                decode(
                    c.query_row(
                        "SELECT record_json FROM assistant_memory_batches WHERE batch_code=?1",
                        [code],
                        |r| r.get(0),
                    )
                    .optional()
                    .map_err(storage)?
                    .ok_or(AssistantMemoryError::NotFound)?,
                )
            },
            || storage("memory database unavailable"),
        )
    }
    fn cursor_versions(
        &mut self,
        user: &str,
        assistant: &str,
    ) -> Result<BTreeMap<String, String>, AssistantMemoryError> {
        let (user, assistant) = (user.to_owned(), assistant.to_owned());
        self.database.call(move|c| {
            let mut statement=c.prepare("SELECT source_key,source_version FROM assistant_memory_cursors WHERE user_code=?1 AND assistant_code=?2").map_err(storage)?;
            statement.query_map(params![user,assistant],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).map_err(storage)?.map(|row|row.map_err(storage)).collect()
        },||storage("memory database unavailable"))
    }
    fn save_batch(
        &mut self,
        previous: Option<&MemoryBatch>,
        next: &MemoryBatch,
        commit_cursors: bool,
    ) -> Result<(), AssistantMemoryError> {
        let previous = previous.map(encode).transpose()?;
        let next = next.clone();
        let json = encode(&next)?;
        self.database.call(move|c| {
            let tx=super::framework_storage::business_transaction(c, rusqlite::TransactionBehavior::Immediate).map_err(storage)?;
            let status=if next.status==MemoryBatchStatus::Completed {"Completed"} else {"Active"};
            let changed=if let Some(previous)=previous {
                tx.execute("UPDATE assistant_memory_batches SET record_json=?1,status=?2 WHERE batch_code=?3 AND record_json=?4",params![json,status,next.batch_code,previous]).map_err(storage)?
            } else {
                tx.execute("INSERT OR IGNORE INTO assistant_memory_batches(batch_code,user_code,assistant_code,status,record_json,created_at) VALUES(?1,?2,?3,?4,?5,?6)",params![next.batch_code,next.user_code,next.assistant_code,status,json,next.created_at]).map_err(storage)?
            };
            if changed!=1 { return Err(AssistantMemoryError::Conflict); }
            if commit_cursors {
                for source in &next.sources {
                    tx.execute("INSERT INTO assistant_memory_cursors(user_code,assistant_code,source_key,source_version) VALUES(?1,?2,?3,?4) ON CONFLICT(user_code,assistant_code,source_key) DO UPDATE SET source_version=excluded.source_version",params![next.user_code,next.assistant_code,source.identity(),source.version]).map_err(storage)?;
                }
            }
            tx.commit().map_err(storage)
        },||storage("memory database unavailable"))
    }
    fn list_candidates(&mut self, user: &str) -> Result<Vec<SkillCandidate>, AssistantMemoryError> {
        let user = user.to_owned();
        self.database.call(move|c| {
            let mut statement=c.prepare("SELECT record_json FROM assistant_skill_candidates WHERE user_code=?1 ORDER BY created_at DESC,candidate_code DESC").map_err(storage)?;
            statement.query_map([user],|r|r.get::<_,String>(0)).map_err(storage)?.map(|row|decode(row.map_err(storage)?)).collect()
        },||storage("memory database unavailable"))
    }
    fn get_candidate(&mut self, code: &str) -> Result<SkillCandidate, AssistantMemoryError> {
        let code = code.to_owned();
        self.database.call(
            move |c| {
                decode(
                    c.query_row(
                        "SELECT record_json FROM assistant_skill_candidates WHERE candidate_code=?1",
                        [code],
                        |r| r.get(0),
                    )
                    .optional()
                    .map_err(storage)?
                    .ok_or(AssistantMemoryError::NotFound)?,
                )
            },
            || storage("memory database unavailable"),
        )
    }
    fn save_candidate(
        &mut self,
        previous: Option<&SkillCandidate>,
        next: &SkillCandidate,
    ) -> Result<(), AssistantMemoryError> {
        let previous = previous.map(encode).transpose()?;
        let next = next.clone();
        let json = encode(&next)?;
        self.database.call(move|c| {
            let changed=if let Some(previous)=previous {
                c.execute("UPDATE assistant_skill_candidates SET record_json=?1 WHERE candidate_code=?2 AND record_json=?3",params![json,next.candidate_code,previous]).map_err(storage)?
            } else {
                c.execute("INSERT OR IGNORE INTO assistant_skill_candidates(candidate_code,user_code,assistant_code,batch_code,skill_name,record_json,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![next.candidate_code,next.user_code,next.assistant_code,next.batch_code,next.skill_name,json,next.created_at]).map_err(storage)?
            };
            if changed!=1 { return Err(AssistantMemoryError::Conflict); } Ok(())
        },||storage("memory database unavailable"))
    }
}
