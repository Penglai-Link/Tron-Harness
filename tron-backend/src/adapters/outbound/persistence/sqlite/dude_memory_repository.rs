use super::framework_storage::SharedDatabase;
use crate::{application::dude_memory::DudeMemoryRepository, domain::dude_memory::*};
use rusqlite::{OptionalExtension, params};
use std::collections::BTreeMap;

pub struct SqliteDudeMemoryRepository {
    database: SharedDatabase,
}
impl SqliteDudeMemoryRepository {
    pub fn connect(database_url: &str) -> Result<Self, DudeMemoryError> {
        Ok(Self {
            database: SharedDatabase::open(database_url).map_err(storage)?,
        })
    }
}
fn storage(error: impl std::fmt::Display) -> DudeMemoryError {
    DudeMemoryError::Storage(error.to_string())
}
fn encode(value: &impl serde::Serialize) -> Result<String, DudeMemoryError> {
    serde_json::to_string(value).map_err(storage)
}
fn decode<T: serde::de::DeserializeOwned>(text: String) -> Result<T, DudeMemoryError> {
    serde_json::from_str(&text).map_err(storage)
}
impl DudeMemoryRepository for SqliteDudeMemoryRepository {
    fn list_batches(
        &mut self,
        user: &str,
        dude: &str,
    ) -> Result<Vec<MemoryBatch>, DudeMemoryError> {
        let (user, dude) = (user.to_owned(), dude.to_owned());
        self.database.call(move |c| {
            let mut statement=c.prepare("SELECT record_json FROM dude_memory_batches WHERE user_code=?1 AND dude_code=?2 ORDER BY created_at DESC,batch_code DESC").map_err(storage)?;
            let rows=statement.query_map(params![user,dude],|r|r.get::<_,String>(0)).map_err(storage)?;
            rows.map(|row|decode(row.map_err(storage)?)).collect()
        },||storage("memory database unavailable"))
    }
    fn get_batch(&mut self, code: &str) -> Result<MemoryBatch, DudeMemoryError> {
        let code = code.to_owned();
        self.database.call(
            move |c| {
                decode(
                    c.query_row(
                        "SELECT record_json FROM dude_memory_batches WHERE batch_code=?1",
                        [code],
                        |r| r.get(0),
                    )
                    .optional()
                    .map_err(storage)?
                    .ok_or(DudeMemoryError::NotFound)?,
                )
            },
            || storage("memory database unavailable"),
        )
    }
    fn cursor_versions(
        &mut self,
        user: &str,
        dude: &str,
    ) -> Result<BTreeMap<String, String>, DudeMemoryError> {
        let (user, dude) = (user.to_owned(), dude.to_owned());
        self.database.call(move|c| {
            let mut statement=c.prepare("SELECT source_key,source_version FROM dude_memory_cursors WHERE user_code=?1 AND dude_code=?2").map_err(storage)?;
            statement.query_map(params![user,dude],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).map_err(storage)?.map(|row|row.map_err(storage)).collect()
        },||storage("memory database unavailable"))
    }
    fn save_batch(
        &mut self,
        previous: Option<&MemoryBatch>,
        next: &MemoryBatch,
        commit_cursors: bool,
    ) -> Result<(), DudeMemoryError> {
        let previous = previous.map(encode).transpose()?;
        let next = next.clone();
        let json = encode(&next)?;
        self.database.call(move|c| {
            let tx=super::framework_storage::business_transaction(c, rusqlite::TransactionBehavior::Immediate).map_err(storage)?;
            let status=if next.status==MemoryBatchStatus::Completed {"Completed"} else {"Active"};
            let changed=if let Some(previous)=previous {
                tx.execute("UPDATE dude_memory_batches SET record_json=?1,status=?2 WHERE batch_code=?3 AND record_json=?4",params![json,status,next.batch_code,previous]).map_err(storage)?
            } else {
                tx.execute("INSERT OR IGNORE INTO dude_memory_batches(batch_code,user_code,dude_code,status,record_json,created_at) VALUES(?1,?2,?3,?4,?5,?6)",params![next.batch_code,next.user_code,next.dude_code,status,json,next.created_at]).map_err(storage)?
            };
            if changed!=1 { return Err(DudeMemoryError::Conflict); }
            if commit_cursors {
                for source in &next.sources {
                    tx.execute("INSERT INTO dude_memory_cursors(user_code,dude_code,source_key,source_version) VALUES(?1,?2,?3,?4) ON CONFLICT(user_code,dude_code,source_key) DO UPDATE SET source_version=excluded.source_version",params![next.user_code,next.dude_code,source.identity(),source.version]).map_err(storage)?;
                }
            }
            tx.commit().map_err(storage)
        },||storage("memory database unavailable"))
    }
    fn list_candidates(&mut self, user: &str) -> Result<Vec<SkillCandidate>, DudeMemoryError> {
        let user = user.to_owned();
        self.database.call(move|c| {
            let mut statement=c.prepare("SELECT record_json FROM dude_skill_candidates WHERE user_code=?1 ORDER BY created_at DESC,candidate_code DESC").map_err(storage)?;
            statement.query_map([user],|r|r.get::<_,String>(0)).map_err(storage)?.map(|row|decode(row.map_err(storage)?)).collect()
        },||storage("memory database unavailable"))
    }
    fn get_candidate(&mut self, code: &str) -> Result<SkillCandidate, DudeMemoryError> {
        let code = code.to_owned();
        self.database.call(
            move |c| {
                decode(
                    c.query_row(
                        "SELECT record_json FROM dude_skill_candidates WHERE candidate_code=?1",
                        [code],
                        |r| r.get(0),
                    )
                    .optional()
                    .map_err(storage)?
                    .ok_or(DudeMemoryError::NotFound)?,
                )
            },
            || storage("memory database unavailable"),
        )
    }
    fn save_candidate(
        &mut self,
        previous: Option<&SkillCandidate>,
        next: &SkillCandidate,
    ) -> Result<(), DudeMemoryError> {
        let previous = previous.map(encode).transpose()?;
        let next = next.clone();
        let json = encode(&next)?;
        self.database.call(move|c| {
            let changed=if let Some(previous)=previous {
                c.execute("UPDATE dude_skill_candidates SET record_json=?1 WHERE candidate_code=?2 AND record_json=?3",params![json,next.candidate_code,previous]).map_err(storage)?
            } else {
                c.execute("INSERT OR IGNORE INTO dude_skill_candidates(candidate_code,user_code,dude_code,batch_code,skill_name,record_json,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![next.candidate_code,next.user_code,next.dude_code,next.batch_code,next.skill_name,json,next.created_at]).map_err(storage)?
            };
            if changed!=1 { return Err(DudeMemoryError::Conflict); } Ok(())
        },||storage("memory database unavailable"))
    }
}
