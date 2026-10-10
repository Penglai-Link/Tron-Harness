
use rusqlite::{Connection, params};

use crate::{
    application::memory::{MemoryError, MemoryWorkDomain, MemoryWorkDomainRepository},
    shared::time::{timestamp_from_millis, timestamp_to_millis},
};

struct SqliteMemoryWorkDomainRepositorySql<'a> {
    connection: &'a mut Connection,
}
pub struct SqliteMemoryWorkDomainRepository {
    connection: crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase,
}

impl<'a> SqliteMemoryWorkDomainRepositorySql<'a> {
    pub fn initialize(
        _database_url: &str,
        connection: &'a mut Connection,
    ) -> Result<Self, MemoryError> {
        crate::adapters::outbound::persistence::sqlite::schema::initialize_business(connection)
            .map_err(storage_error)?;
        Ok(Self { connection })
    }

}

impl MemoryWorkDomainRepository for SqliteMemoryWorkDomainRepositorySql<'_> {
    fn list_domains(&mut self) -> Result<Vec<MemoryWorkDomain>, MemoryError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT domain_code, name, description, status, created_at, updated_at
                 FROM memory_work_domains
                 ORDER BY status ASC, name ASC, domain_code ASC",
            )
            .map_err(storage_error)?;
        statement
            .query_map([], map_domain)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn create_domain(&mut self, domain: &MemoryWorkDomain) -> Result<(), MemoryError> {
        self.connection
            .execute(
                "INSERT INTO memory_work_domains (
                     domain_code, name, description, status, created_at, updated_at
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    domain.domain_code,
                    domain.name,
                    domain.description,
                    domain.status,
                    timestamp_to_millis(&domain.created_at),
                    timestamp_to_millis(&domain.updated_at),
                ],
            )
            .map_err(|error| {
                if unique_violation(&error) {
                    MemoryError::Conflict("memory work domain already exists".into())
                } else {
                    storage_error(error)
                }
            })?;
        Ok(())
    }

    fn update_domain(&mut self, domain: &MemoryWorkDomain) -> Result<(), MemoryError> {
        let updated = self
            .connection
            .execute(
                "UPDATE memory_work_domains
                 SET name = ?2,
                     description = ?3,
                     status = ?4,
                     updated_at = ?5
                 WHERE domain_code = ?1",
                params![
                    domain.domain_code,
                    domain.name,
                    domain.description,
                    domain.status,
                    timestamp_to_millis(&domain.updated_at),
                ],
            )
            .map_err(storage_error)?;
        if updated == 0 {
            return Err(MemoryError::NotFound(
                "memory work domain was not found".into(),
            ));
        }
        Ok(())
    }
}

fn map_domain(row: &rusqlite::Row<'_>) -> rusqlite::Result<MemoryWorkDomain> {
    Ok(MemoryWorkDomain {
        domain_code: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        status: row.get(3)?,
        created_at: timestamp_from_millis(row.get(4)?),
        updated_at: timestamp_from_millis(row.get(5)?),
    })
}

fn unique_violation(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(code, _)
            if code.code == rusqlite::ErrorCode::ConstraintViolation
    )
}

fn storage_error(error: rusqlite::Error) -> MemoryError {
    MemoryError::Storage(error.to_string())
}

impl SqliteMemoryWorkDomainRepository {
    pub fn connect(database_url: &str) -> Result<Self, MemoryError> {
        let connection = crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase::open(database_url).map_err(storage_error)?;
        let database_url = database_url.to_owned();
        connection.call(
            move |connection| {
                SqliteMemoryWorkDomainRepositorySql::initialize(&database_url, connection)
                    .map(|_| ())
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )?;
        Ok(Self { connection })
    }
}
impl MemoryWorkDomainRepository for SqliteMemoryWorkDomainRepository {
    fn list_domains(&mut self) -> Result<Vec<MemoryWorkDomain>, MemoryError> {
        self.connection.call(
            move |connection| SqliteMemoryWorkDomainRepositorySql { connection }.list_domains(),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn create_domain(&mut self, domain: &MemoryWorkDomain) -> Result<(), MemoryError> {
        let domain = domain.to_owned();
        self.connection.call(
            move |connection| {
                SqliteMemoryWorkDomainRepositorySql { connection }.create_domain(&domain)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn update_domain(&mut self, domain: &MemoryWorkDomain) -> Result<(), MemoryError> {
        let domain = domain.to_owned();
        self.connection.call(
            move |connection| {
                SqliteMemoryWorkDomainRepositorySql { connection }.update_domain(&domain)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
}
