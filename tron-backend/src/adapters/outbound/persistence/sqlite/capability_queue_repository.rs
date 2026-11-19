use super::framework_storage::SharedDatabase;
use crate::application::capability::{
    CAPABILITY_OPERATION_LOG_LIMIT, CAPABILITY_QUEUE_LIMIT, CapabilityEnqueueRecord,
    CapabilityOperationRecord, CapabilityQueueItem, CapabilityQueueOperation,
    CapabilityQueueOutcome, CapabilityQueueRepository, CapabilityQueueStatus,
};
use rusqlite::{Connection, OptionalExtension, params};

/// SQLite adapter for the durable capability queue. The queue shares the
/// business database through [`SharedDatabase`], so transactions, WAL settings
/// and the periodic freelist reclaim use the same configured connection.
pub struct SqliteCapabilityQueueRepository(SharedDatabase);

impl SqliteCapabilityQueueRepository {
    pub fn connect(url: &str) -> Result<Self, String> {
        SharedDatabase::open(url)
            .map(Self)
            .map_err(|error| error.to_string())
    }
}

fn failure(error: rusqlite::Error) -> String {
    error.to_string()
}

fn read_operation(
    row: &rusqlite::Row<'_>,
    index: usize,
) -> rusqlite::Result<CapabilityQueueOperation> {
    let value: String = row.get(index)?;
    CapabilityQueueOperation::parse(&value).ok_or_else(|| {
        rusqlite::Error::InvalidColumnType(index, "operation".into(), rusqlite::types::Type::Text)
    })
}

fn read_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<CapabilityQueueItem> {
    Ok(CapabilityQueueItem {
        id: row.get(0)?,
        pack_id: row.get(1)?,
        pack_name: row.get(2)?,
        operation: read_operation(row, 3)?,
        force: row.get(4)?,
        locale: row.get(5)?,
        created_at: row.get(6)?,
        status: CapabilityQueueStatus::Queued,
    })
}

fn items(connection: &Connection) -> Result<Vec<CapabilityQueueItem>, String> {
    let sql = format!(
        "SELECT item_id,pack_id,pack_name,operation,force,locale,created_at \
         FROM capability_queue_items ORDER BY sequence LIMIT {CAPABILITY_QUEUE_LIMIT}"
    );
    let mut query = connection.prepare(&sql).map_err(failure)?;
    query
        .query_map([], read_item)
        .map_err(failure)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(failure)
}

fn log(connection: &Connection) -> Result<Vec<CapabilityOperationRecord>, String> {
    let sql = format!(
        "SELECT item_id,pack_id,pack_name,operation,force,succeeded,error,error_code,finished_at \
         FROM capability_operation_log ORDER BY sequence DESC LIMIT {CAPABILITY_OPERATION_LOG_LIMIT}"
    );
    let mut query = connection.prepare(&sql).map_err(failure)?;
    query
        .query_map([], |row| {
            Ok(CapabilityOperationRecord {
                item_id: row.get(0)?,
                pack_id: row.get(1)?,
                pack_name: row.get(2)?,
                operation: read_operation(row, 3)?,
                force: row.get(4)?,
                succeeded: row.get(5)?,
                error: row.get(6)?,
                error_code: row.get(7)?,
                finished_at: row.get(8)?,
            })
        })
        .map_err(failure)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(failure)
}

impl CapabilityQueueRepository for SqliteCapabilityQueueRepository {
    fn items(&mut self) -> Result<Vec<CapabilityQueueItem>, String> {
        self.0.call_read(
            |connection| items(connection),
            || "capability queue storage is unavailable".into(),
        )
    }

    fn log(&mut self) -> Result<Vec<CapabilityOperationRecord>, String> {
        self.0.call_read(
            |connection| log(connection),
            || "capability queue storage is unavailable".into(),
        )
    }

    fn has_items(&mut self) -> Result<bool, String> {
        self.0.call_read(
            |connection| {
                connection
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM capability_queue_items)",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(failure)
            },
            || "capability queue storage is unavailable".into(),
        )
    }

    fn enqueue(&mut self, item: CapabilityQueueItem) -> Result<CapabilityEnqueueRecord, String> {
        self.0.call(
            move |connection| {
                let transaction = connection.transaction().map_err(failure)?;
                let existing = {
                    let mut query = transaction
                        .prepare(
                            "SELECT item_id,pack_id,pack_name,operation,force,locale,created_at \
                             FROM capability_queue_items WHERE pack_id=?1 AND operation=?2 LIMIT 1",
                        )
                        .map_err(failure)?;
                    let mut rows = query
                        .query_map(
                            params![item.pack_id, item.operation.storage_value()],
                            read_item,
                        )
                        .map_err(failure)?;
                    rows.next().transpose().map_err(failure)?
                };
                if let Some(mut existing) = existing {
                    // A force confirmation upgrades a waiting normal uninstall
                    // instead of silently keeping the weaker request.
                    if item.operation == CapabilityQueueOperation::Uninstall
                        && item.force
                        && !existing.force
                    {
                        transaction
                            .execute(
                                "UPDATE capability_queue_items SET force=1 WHERE item_id=?1",
                                [&existing.id],
                            )
                            .map_err(failure)?;
                        transaction.commit().map_err(failure)?;
                        existing.force = true;
                    }
                    return Ok(CapabilityEnqueueRecord::Existing(existing));
                }
                let pending: i64 = transaction
                    .query_row("SELECT COUNT(*) FROM capability_queue_items", [], |row| {
                        row.get(0)
                    })
                    .map_err(failure)?;
                if pending >= CAPABILITY_QUEUE_LIMIT as i64 {
                    return Ok(CapabilityEnqueueRecord::Full);
                }
                transaction
                    .execute(
                        "INSERT INTO capability_queue_items( \
                             item_id,pack_id,pack_name,operation,force,locale,created_at) \
                         VALUES(?1,?2,?3,?4,?5,?6,?7)",
                        params![
                            item.id,
                            item.pack_id,
                            item.pack_name,
                            item.operation.storage_value(),
                            item.force,
                            item.locale,
                            item.created_at
                        ],
                    )
                    .map_err(failure)?;
                transaction.commit().map_err(failure)?;
                Ok(CapabilityEnqueueRecord::Queued {
                    item,
                    was_empty: pending == 0,
                })
            },
            || "capability queue storage is unavailable".into(),
        )
    }

    fn complete(
        &mut self,
        item_id: &str,
        record: &CapabilityOperationRecord,
    ) -> Result<(), String> {
        let item_id = item_id.to_owned();
        let record = record.clone();
        self.0.call(
            move |connection| {
                let transaction = connection.transaction().map_err(failure)?;
                transaction
                    .execute(
                        "DELETE FROM capability_queue_items WHERE item_id=?1",
                        [&item_id],
                    )
                    .map_err(failure)?;
                transaction
                    .execute(
                        "INSERT INTO capability_operation_log( \
                             item_id,pack_id,pack_name,operation,force,succeeded,error,error_code,finished_at) \
                         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9) \
                         ON CONFLICT(item_id) DO NOTHING",
                        params![
                            record.item_id,
                            record.pack_id,
                            record.pack_name,
                            record.operation.storage_value(),
                            record.force,
                            record.succeeded,
                            record.error,
                            record.error_code,
                            record.finished_at
                        ],
                    )
                    .map_err(failure)?;
                // Keep exactly the newest entries. With fewer than the limit the
                // subquery is NULL, so nothing is deleted.
                transaction
                    .execute(
                        "DELETE FROM capability_operation_log WHERE sequence < ( \
                             SELECT sequence FROM capability_operation_log \
                             ORDER BY sequence DESC LIMIT 1 OFFSET ?1)",
                        [CAPABILITY_OPERATION_LOG_LIMIT as i64 - 1],
                    )
                    .map_err(failure)?;
                transaction.commit().map_err(failure)
            },
            || "capability queue storage is unavailable".into(),
        )
    }

    fn outcome(&mut self, item_id: &str) -> Result<CapabilityQueueOutcome, String> {
        let item_id = item_id.to_owned();
        self.0.call_read(
            move |connection| {
                let pending: bool = connection
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM capability_queue_items WHERE item_id=?1)",
                        [&item_id],
                        |row| row.get(0),
                    )
                    .map_err(failure)?;
                if pending {
                    return Ok(CapabilityQueueOutcome::Pending);
                }
                let record: Option<(bool, Option<String>, Option<String>)> = connection
                    .query_row(
                        "SELECT succeeded,error,error_code FROM capability_operation_log WHERE item_id=?1",
                        [&item_id],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                    )
                    .optional()
                    .map_err(failure)?;
                Ok(match record {
                    Some((true, _, _)) => CapabilityQueueOutcome::Succeeded,
                    Some((false, error, code)) => CapabilityQueueOutcome::Failed {
                        error: error.unwrap_or_default(),
                        code,
                    },
                    None => CapabilityQueueOutcome::Pending,
                })
            },
            || "capability queue storage is unavailable".into(),
        )
    }
}
