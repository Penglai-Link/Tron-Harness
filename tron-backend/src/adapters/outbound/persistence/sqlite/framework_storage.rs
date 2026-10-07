//! Composition bridge for synchronous business services and Framework storage.
//! Entire consistency operations run in one future so transaction binding is retained.
use std::{future::Future, sync::OnceLock};

pub(crate) fn blocking<T: Send>(future: impl Future<Output = T> + Send) -> T {
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    let runtime = RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("storage runtime")
    });
    let execute = || {
        std::thread::scope(|scope| {
            scope
                .spawn(|| runtime.block_on(future))
                .join()
                .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
        })
    };
    if tokio::runtime::Handle::try_current()
        .is_ok_and(|handle| handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread)
    {
        tokio::task::block_in_place(execute)
    } else {
        execute()
    }
}

pub(crate) fn chat_error(
    error: eliteforge_agent_core::StateError,
) -> crate::domain::chat::ChatError {
    use crate::domain::chat::ChatError;
    use eliteforge_agent_core::StateError;
    match error {
        StateError::NotFound => ChatError::SessionNotFound,
        StateError::Busy => ChatError::SessionRunActive,
        other => {
            eprintln!("Framework session storage failed: {other}");
            ChatError::StorageFailure
        }
    }
}

#[derive(Clone)]
pub(crate) struct SharedDatabase(
    std::sync::Arc<eliteforge_agent_official_plugin_storage_local::SqliteDatabase>,
);
impl SharedDatabase {
    pub fn open(path: &str) -> rusqlite::Result<Self> {
        use std::collections::BTreeMap;
        use std::sync::{Arc, Mutex, Weak};
        static DATABASES: OnceLock<
            Mutex<
                BTreeMap<
                    String,
                    Weak<eliteforge_agent_official_plugin_storage_local::SqliteDatabase>,
                >,
            >,
        > = OnceLock::new();
        if path == ":memory:" {
            return blocking(
                eliteforge_agent_official_plugin_storage_local::SqliteDatabase::open(path),
            )
            .map(|db| Self(Arc::new(db)));
        }
        let key = if path.starts_with("file:") {
            path.to_owned()
        } else {
            let absolute =
                std::path::absolute(path).map_err(|_| rusqlite::Error::InvalidPath(path.into()))?;
            let resolved = if absolute.exists() {
                std::fs::canonicalize(&absolute)
            } else {
                let parent = absolute
                    .parent()
                    .ok_or_else(|| rusqlite::Error::InvalidPath(absolute.clone()))?;
                std::fs::create_dir_all(parent)
                    .map_err(|_| rusqlite::Error::InvalidPath(absolute.clone()))?;
                std::fs::canonicalize(parent)
                    .map(|parent| parent.join(absolute.file_name().unwrap()))
            }
            .map_err(|_| rusqlite::Error::InvalidPath(absolute))?;
            resolved.to_string_lossy().into_owned()
        };
        let mut databases = DATABASES
            .get_or_init(Default::default)
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        if let Some(database) = databases.get(&key).and_then(Weak::upgrade) {
            return Ok(Self(database));
        }
        let database = Arc::new(blocking(
            eliteforge_agent_official_plugin_storage_local::SqliteDatabase::open(&key),
        )?);
        databases.insert(key, Arc::downgrade(&database));
        Ok(Self(database))
    }
    pub fn runtime(&self) -> eliteforge_agent_official_plugin_storage_local::SqliteDatabase {
        self.0.as_ref().clone()
    }
    pub fn call<T: Send + 'static, E: Send + 'static>(
        &self,
        operation: impl FnOnce(&mut rusqlite::Connection) -> Result<T, E> + Send + 'static,
        failure: impl FnOnce() -> E,
    ) -> Result<T, E> {
        self.0
            .call_blocking(operation)
            .map_err(|error| match error {
                tokio_rusqlite::Error::Error(error) => error,
                _ => failure(),
            })
    }
}

/// A business transaction joins the host transaction on the worker connection.
/// Its successful inner completion releases a savepoint, never the outer commit.
pub(crate) struct BusinessTransaction<'a> {
    connection: &'a rusqlite::Connection,
    savepoint: Option<String>,
    finished: bool,
}
pub(crate) fn business_transaction(
    connection: &rusqlite::Connection,
    behavior: rusqlite::TransactionBehavior,
) -> rusqlite::Result<BusinessTransaction<'_>> {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let savepoint = if connection.is_autocommit() {
        None
    } else {
        Some(format!(
            "tron_tx_{}",
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ))
    };
    if let Some(name) = &savepoint {
        connection.execute_batch(&format!("SAVEPOINT {name}"))?;
    } else {
        connection.execute_batch(match behavior {
            rusqlite::TransactionBehavior::Deferred => "BEGIN DEFERRED",
            rusqlite::TransactionBehavior::Exclusive => "BEGIN EXCLUSIVE",
            _ => "BEGIN IMMEDIATE",
        })?;
    }
    Ok(BusinessTransaction {
        connection,
        savepoint,
        finished: false,
    })
}
impl std::ops::Deref for BusinessTransaction<'_> {
    type Target = rusqlite::Connection;
    fn deref(&self) -> &Self::Target {
        self.connection
    }
}
impl BusinessTransaction<'_> {
    pub fn commit(mut self) -> rusqlite::Result<()> {
        self.connection.execute_batch(
            &self
                .savepoint
                .as_ref()
                .map(|name| format!("RELEASE {name}"))
                .unwrap_or("COMMIT".into()),
        )?;
        self.finished = true;
        Ok(())
    }
    pub fn rollback(mut self) -> rusqlite::Result<()> {
        self.rollback_inner()?;
        self.finished = true;
        Ok(())
    }
    fn rollback_inner(&self) -> rusqlite::Result<()> {
        self.connection.execute_batch(
            &self
                .savepoint
                .as_ref()
                .map(|name| format!("ROLLBACK TO {name}; RELEASE {name}"))
                .unwrap_or("ROLLBACK".into()),
        )
    }
}
impl Drop for BusinessTransaction<'_> {
    fn drop(&mut self) {
        if !self.finished {
            if let Err(error) = self.rollback_inner() {
                eprintln!("business transaction rollback failed: {error}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn database_path_aliases_share_the_same_native_connection() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("tron.sqlite3");
        let first = SharedDatabase::open(path.to_str().unwrap()).unwrap();
        let alias = directory.path().join(".").join("tron.sqlite3");
        let second = SharedDatabase::open(alias.to_str().unwrap()).unwrap();
        assert!(std::sync::Arc::ptr_eq(&first.0, &second.0));
        #[cfg(unix)]
        {
            let link = directory.path().join("alias.sqlite3");
            std::os::unix::fs::symlink(&path, &link).unwrap();
            let third = SharedDatabase::open(link.to_str().unwrap()).unwrap();
            assert!(std::sync::Arc::ptr_eq(&first.0, &third.0));
        }
    }
}
