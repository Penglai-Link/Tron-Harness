use rusqlite::{Connection, OptionalExtension, params};

use super::framework_storage::{SharedDatabase, business_transaction};
use crate::{
    application::topic::TopicRepository,
    domain::topic::{TopicError, TopicMemo, TopicReference, TopicReferenceKind},
};

pub struct SqliteTopicRepository {
    connection: SharedDatabase,
}

impl SqliteTopicRepository {
    pub fn connect(database_url: &str) -> Result<Self, TopicError> {
        let connection =
            SharedDatabase::open(database_url).map_err(|_| TopicError::StorageFailure)?;
        connection.call(
            |db| super::schema::initialize_business(db).map_err(|_| TopicError::StorageFailure),
            || TopicError::StorageFailure,
        )?;
        Ok(Self { connection })
    }
}

fn read(db: &Connection, project_code: &str) -> Result<TopicMemo, TopicError> {
    let details: Option<(String, i64)> = db
        .query_row(
            "SELECT notes,revision FROM topic_memos WHERE project_code=?1",
            [project_code],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|_| TopicError::StorageFailure)?;
    let (notes, revision) = details.unwrap_or_default();
    let mut statement = db.prepare("SELECT reference_code,kind,target_code,connection_code,label,url FROM topic_references WHERE project_code=?1 ORDER BY rowid").map_err(|_| TopicError::StorageFailure)?;
    let references = statement
        .query_map([project_code], |row| {
            let kind: String = row.get(1)?;
            let connection_code: String = row.get(3)?;
            Ok(TopicReference {
                reference_code: row.get(0)?,
                kind: TopicReferenceKind::parse(&kind).ok_or(rusqlite::Error::InvalidQuery)?,
                target_code: row.get(2)?,
                connection_code: (!connection_code.is_empty()).then_some(connection_code),
                label: row.get(4)?,
                url: row.get(5)?,
                available: true,
                directory_path: None,
            })
        })
        .map_err(|_| TopicError::StorageFailure)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| TopicError::StorageFailure)?;
    Ok(TopicMemo {
        project_code: project_code.into(),
        notes,
        revision: u64::try_from(revision).map_err(|_| TopicError::StorageFailure)?,
        references,
    })
}

fn ensure_revision(db: &Connection, project_code: &str, revision: u64) -> Result<(), TopicError> {
    db.execute("INSERT INTO topic_memos(project_code,notes,revision) VALUES (?1,'',0) ON CONFLICT(project_code) DO NOTHING", [project_code]).map_err(|_| TopicError::StorageFailure)?;
    let current: i64 = db
        .query_row(
            "SELECT revision FROM topic_memos WHERE project_code=?1",
            [project_code],
            |row| row.get(0),
        )
        .map_err(|_| TopicError::StorageFailure)?;
    if u64::try_from(current).ok() != Some(revision) {
        return Err(TopicError::Conflict);
    }
    Ok(())
}

fn bump(db: &Connection, project_code: &str) -> Result<(), TopicError> {
    db.execute(
        "UPDATE topic_memos SET revision=revision+1 WHERE project_code=?1",
        [project_code],
    )
    .map_err(|_| TopicError::StorageFailure)?;
    Ok(())
}

impl TopicRepository for SqliteTopicRepository {
    fn get(&self, project_code: &str) -> Result<TopicMemo, TopicError> {
        let code = project_code.to_owned();
        self.connection.call_read(
            move |db| {
                // Read the revision and references from the same SQLite snapshot.
                let tx = db
                    .unchecked_transaction()
                    .map_err(|_| TopicError::StorageFailure)?;
                let memo = read(&tx, &code)?;
                tx.commit().map_err(|_| TopicError::StorageFailure)?;
                Ok(memo)
            },
            || TopicError::StorageFailure,
        )
    }

    fn save_notes(
        &self,
        project_code: &str,
        revision: u64,
        notes: &str,
    ) -> Result<TopicMemo, TopicError> {
        let code = project_code.to_owned();
        let notes = notes.to_owned();
        self.connection.call(
            move |db| {
                let tx = business_transaction(db, rusqlite::TransactionBehavior::Immediate)
                    .map_err(|_| TopicError::StorageFailure)?;
                ensure_revision(&tx, &code, revision)?;
                tx.execute(
                    "UPDATE topic_memos SET notes=?2,revision=revision+1 WHERE project_code=?1",
                    params![code, notes],
                )
                .map_err(|_| TopicError::StorageFailure)?;
                let memo = read(&tx, &code)?;
                tx.commit().map_err(|_| TopicError::StorageFailure)?;
                Ok(memo)
            },
            || TopicError::StorageFailure,
        )
    }

    fn add_reference(
        &self,
        project_code: &str,
        revision: u64,
        reference: &TopicReference,
    ) -> Result<TopicMemo, TopicError> {
        let code = project_code.to_owned();
        let reference = reference.clone();
        self.connection.call(move |db| {
            let tx = business_transaction(db, rusqlite::TransactionBehavior::Immediate).map_err(|_| TopicError::StorageFailure)?;
            ensure_revision(&tx, &code, revision)?;
            let inserted = tx.execute("INSERT INTO topic_references(reference_code,project_code,kind,target_code,connection_code,label,url) VALUES (?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(project_code,kind,target_code,connection_code) DO NOTHING", params![reference.reference_code,code,reference.kind.as_str(),reference.target_code,reference.connection_code.as_deref().unwrap_or(""),reference.label,reference.url]).map_err(|_| TopicError::StorageFailure)?;
            if inserted != 0 { bump(&tx, &code)?; }
            let memo = read(&tx, &code)?;
            tx.commit().map_err(|_| TopicError::StorageFailure)?;
            Ok(memo)
        }, || TopicError::StorageFailure)
    }

    fn remove_reference(
        &self,
        project_code: &str,
        revision: u64,
        reference_code: &str,
    ) -> Result<TopicMemo, TopicError> {
        let code = project_code.to_owned();
        let reference_code = reference_code.to_owned();
        self.connection.call(
            move |db| {
                let tx = business_transaction(db, rusqlite::TransactionBehavior::Immediate)
                    .map_err(|_| TopicError::StorageFailure)?;
                ensure_revision(&tx, &code, revision)?;
                let changed = tx
                    .execute(
                        "DELETE FROM topic_references WHERE project_code=?1 AND reference_code=?2",
                        params![code, reference_code],
                    )
                    .map_err(|_| TopicError::StorageFailure)?;
                if changed == 0 {
                    return Err(TopicError::NotFound);
                }
                bump(&tx, &code)?;
                let memo = read(&tx, &code)?;
                tx.commit().map_err(|_| TopicError::StorageFailure)?;
                Ok(memo)
            },
            || TopicError::StorageFailure,
        )
    }
}
