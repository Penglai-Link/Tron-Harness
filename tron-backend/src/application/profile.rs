use std::{
    env,
    error::Error,
    fmt, fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use chrono::Utc;
use rand::random;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

const CONTROL_DATABASE_ENV: &str = "TRON_BACKEND_CONTROL_DATABASE_URL";
const ACTIVE_PROFILE_ENV: &str = "TRON_PROFILE_ID";
const PROFILE_REGISTRY_MIGRATION: &str =
    include_str!("../../migrations/20260921_0001_create_profile_registry.sql");

#[derive(Clone)]
pub struct ProfileRegistryService {
    connection: Arc<Mutex<Connection>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileRecord {
    pub profile_id: String,
    pub person_ai_user_id: String,
    pub person_ai_username: String,
    pub status: String,
    pub created_at: String,
    pub last_activated_at: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapProfileStatus {
    pub has_profiles: bool,
    pub active_profile_id: Option<String>,
    pub active_profile: Option<ProfileRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CurrentProfileContext {
    pub profile_id: String,
    pub person_ai_user_id: String,
    pub person_ai_username: String,
}

#[derive(Debug)]
pub enum ProfileRegistryError {
    Storage(rusqlite::Error),
    LockPoisoned,
    ProfileNotFound(String),
    InvalidInput(&'static str),
}

impl fmt::Display for ProfileRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(error) => write!(formatter, "profile registry storage failed: {error}"),
            Self::LockPoisoned => formatter.write_str("profile registry lock is poisoned"),
            Self::ProfileNotFound(profile_id) => {
                write!(formatter, "profile `{profile_id}` was not found")
            }
            Self::InvalidInput(field) => write!(formatter, "{field} is required"),
        }
    }
}

impl Error for ProfileRegistryError {}

impl From<rusqlite::Error> for ProfileRegistryError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Storage(error)
    }
}

impl ProfileRegistryService {
    pub fn from_environment() -> Result<Self, ProfileRegistryError> {
        let database_url = env::var(CONTROL_DATABASE_ENV)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(default_control_database_url);
        Self::new(database_url)
    }

    pub fn new(database_url: impl AsRef<str>) -> Result<Self, ProfileRegistryError> {
        let database_url = database_url.as_ref();
        if database_url != ":memory:" && !database_url.starts_with("file:") {
            if let Some(parent) = Path::new(database_url).parent() {
                fs::create_dir_all(parent).map_err(|_| {
                    ProfileRegistryError::Storage(rusqlite::Error::InvalidPath(
                        parent.to_path_buf(),
                    ))
                })?;
            }
        }
        let connection = Connection::open(database_url)?;
        connection.execute_batch("PRAGMA foreign_keys = ON;")?;
        connection.execute_batch(PROFILE_REGISTRY_MIGRATION)?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    pub fn list(&self) -> Result<Vec<ProfileRecord>, ProfileRegistryError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ProfileRegistryError::LockPoisoned)?;
        let mut statement = connection.prepare(
            "SELECT profile_id, person_ai_user_id, person_ai_username, status,
                    created_at, last_activated_at
             FROM profiles ORDER BY last_activated_at DESC, created_at DESC",
        )?;
        let rows = statement.query_map([], map_profile)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn active(&self) -> Result<Option<ProfileRecord>, ProfileRegistryError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ProfileRegistryError::LockPoisoned)?;
        connection
            .query_row(
                "SELECT p.profile_id, p.person_ai_user_id, p.person_ai_username, p.status,
                        p.created_at, p.last_activated_at
                 FROM profile_runtime_state s
                 JOIN profiles p ON p.profile_id = s.active_profile_id
                 WHERE s.singleton = 1",
                [],
                map_profile,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn bootstrap_status(&self) -> Result<BootstrapProfileStatus, ProfileRegistryError> {
        let active_profile = self.active()?;
        let has_profiles = {
            let connection = self
                .connection
                .lock()
                .map_err(|_| ProfileRegistryError::LockPoisoned)?;
            connection.query_row("SELECT EXISTS(SELECT 1 FROM profiles)", [], |row| {
                row.get(0)
            })?
        };
        Ok(BootstrapProfileStatus {
            has_profiles,
            active_profile_id: active_profile
                .as_ref()
                .map(|profile| profile.profile_id.clone()),
            active_profile,
        })
    }

    pub fn register_or_update(
        &self,
        person_ai_user_id: &str,
        person_ai_username: &str,
    ) -> Result<ProfileRecord, ProfileRegistryError> {
        let person_ai_user_id = person_ai_user_id.trim();
        let person_ai_username = person_ai_username.trim();
        if person_ai_user_id.is_empty() {
            return Err(ProfileRegistryError::InvalidInput("personAiUserId"));
        }
        if person_ai_username.is_empty() {
            return Err(ProfileRegistryError::InvalidInput("personAiUsername"));
        }

        let connection = self
            .connection
            .lock()
            .map_err(|_| ProfileRegistryError::LockPoisoned)?;
        let existing_id = connection
            .query_row(
                "SELECT profile_id FROM profiles WHERE person_ai_user_id = ?1",
                [person_ai_user_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let pending_id = if existing_id.is_none() {
            connection
                .query_row(
                    "SELECT profile_id FROM profiles
                     WHERE status = 'PENDING' ORDER BY created_at DESC LIMIT 1",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .optional()?
        } else {
            None
        };
        let profile_id = existing_id
            .clone()
            .or_else(|| pending_id.clone())
            .unwrap_or_else(|| format!("profile-{:032x}", random::<u128>()));
        let now = Utc::now().to_rfc3339();
        if pending_id.is_some() && existing_id.is_none() {
            connection.execute(
                "UPDATE profiles SET person_ai_user_id = ?2, person_ai_username = ?3,
                 status = 'ACTIVE', last_activated_at = ?4 WHERE profile_id = ?1",
                params![profile_id, person_ai_user_id, person_ai_username, now],
            )?;
        } else {
            connection.execute(
                "INSERT INTO profiles(profile_id, person_ai_user_id, person_ai_username, status, created_at)
                 VALUES (?1, ?2, ?3, 'ACTIVE', ?4)
                 ON CONFLICT(person_ai_user_id) DO UPDATE SET
                     person_ai_username = excluded.person_ai_username,
                     status = 'ACTIVE'",
                params![profile_id, person_ai_user_id, person_ai_username, now],
            )?;
        }
        connection.execute(
            "UPDATE profiles SET status = 'INACTIVE'
             WHERE profile_id <> ?1 AND status = 'ACTIVE'",
            [profile_id.as_str()],
        )?;
        connection.execute(
            "UPDATE profile_runtime_state SET active_profile_id = ?1 WHERE singleton = 1",
            [profile_id.as_str()],
        )?;
        connection
            .query_row(
                "SELECT profile_id, person_ai_user_id, person_ai_username, status,
                        created_at, last_activated_at
                 FROM profiles WHERE person_ai_user_id = ?1",
                [person_ai_user_id],
                map_profile,
            )
            .map_err(Into::into)
    }

    pub fn create_pending(&self) -> Result<ProfileRecord, ProfileRegistryError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ProfileRegistryError::LockPoisoned)?;
        let transaction = connection.unchecked_transaction()?;
        let existing = transaction
            .query_row(
                "SELECT profile_id, person_ai_user_id, person_ai_username, status,
                        created_at, last_activated_at
                 FROM profiles WHERE status = 'PENDING'
                 ORDER BY created_at DESC LIMIT 1",
                [],
                map_profile,
            )
            .optional()?;
        if let Some(profile) = existing {
            transaction.execute(
                "UPDATE profiles SET status = 'INACTIVE' WHERE status = 'ACTIVE'",
                [],
            )?;
            transaction.execute(
                "UPDATE profiles SET status = 'PENDING' WHERE profile_id = ?1",
                [profile.profile_id.as_str()],
            )?;
            transaction.execute(
                "UPDATE profile_runtime_state SET active_profile_id = ?1 WHERE singleton = 1",
                [profile.profile_id.as_str()],
            )?;
            transaction.commit()?;
            return Ok(ProfileRecord {
                status: "PENDING".to_owned(),
                ..profile
            });
        }
        let profile_id = format!("profile-{:032x}", random::<u128>());
        let now = Utc::now().to_rfc3339();
        transaction.execute(
            "UPDATE profiles SET status = 'INACTIVE' WHERE status = 'ACTIVE'",
            [],
        )?;
        transaction.execute(
            "INSERT INTO profiles(
                profile_id, person_ai_user_id, person_ai_username, status, created_at
             ) VALUES (?1, '', '', 'PENDING', ?2)",
            params![profile_id, now],
        )?;
        transaction.execute(
            "UPDATE profile_runtime_state SET active_profile_id = ?1 WHERE singleton = 1",
            [profile_id.as_str()],
        )?;
        let profile = transaction
            .query_row(
                "SELECT profile_id, person_ai_user_id, person_ai_username, status,
                        created_at, last_activated_at
                 FROM profiles WHERE profile_id = ?1",
                [profile_id.as_str()],
                map_profile,
            )
            .map_err(ProfileRegistryError::from)?;
        transaction.commit()?;
        Ok(profile)
    }

    pub fn activate(&self, profile_id: &str) -> Result<ProfileRecord, ProfileRegistryError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ProfileRegistryError::LockPoisoned)?;
        let transaction = connection.unchecked_transaction()?;
        let status = transaction
            .query_row(
                "SELECT status FROM profiles WHERE profile_id = ?1",
                [profile_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let Some(status) = status else {
            return Err(ProfileRegistryError::ProfileNotFound(profile_id.to_owned()));
        };
        if status == "PENDING" {
            return Err(ProfileRegistryError::InvalidInput(
                "pending profile requires PersonAI authorization",
            ));
        }
        let now = Utc::now().to_rfc3339();
        transaction.execute(
            "UPDATE profiles SET status = 'INACTIVE' WHERE status = 'ACTIVE'",
            [],
        )?;
        transaction.execute(
            "UPDATE profiles SET status = 'ACTIVE', last_activated_at = ?2 WHERE profile_id = ?1",
            params![profile_id, now],
        )?;
        transaction.execute(
            "UPDATE profile_runtime_state SET active_profile_id = ?1 WHERE singleton = 1",
            [profile_id],
        )?;
        let profile = transaction
            .query_row(
                "SELECT profile_id, person_ai_user_id, person_ai_username, status,
                        created_at, last_activated_at
                 FROM profiles WHERE profile_id = ?1",
                [profile_id],
                map_profile,
            )
            .map_err(ProfileRegistryError::from)?;
        transaction.commit()?;
        Ok(profile)
    }

    pub fn active_profile_id_from_environment() -> Option<String> {
        env::var(ACTIVE_PROFILE_ENV)
            .ok()
            .filter(|value| !value.trim().is_empty())
    }

    pub fn current_context(&self) -> Result<CurrentProfileContext, ProfileRegistryError> {
        let profile = self
            .active()?
            .ok_or(ProfileRegistryError::ProfileNotFound("active".into()))?;
        if profile.status != "ACTIVE"
            || Self::active_profile_id_from_environment()
                .is_some_and(|profile_id| profile_id != profile.profile_id)
        {
            return Err(ProfileRegistryError::ProfileNotFound(profile.profile_id));
        }
        Ok(CurrentProfileContext {
            profile_id: profile.profile_id,
            person_ai_user_id: profile.person_ai_user_id,
            person_ai_username: profile.person_ai_username,
        })
    }
}

fn map_profile(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProfileRecord> {
    Ok(ProfileRecord {
        profile_id: row.get(0)?,
        person_ai_user_id: row.get(1)?,
        person_ai_username: row.get(2)?,
        status: row.get(3)?,
        created_at: row.get(4)?,
        last_activated_at: row.get(5)?,
    })
}

fn default_control_database_url() -> String {
    let home = env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home)
        .join(".tron/control/profiles.sqlite")
        .display()
        .to_string()
}

pub fn default_profile_data_dir(base_dir: impl AsRef<Path>, profile_id: &str) -> PathBuf {
    base_dir.as_ref().join("profiles").join(profile_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_updates_the_same_person_ai_profile() {
        let registry = ProfileRegistryService::new(":memory:").expect("registry");
        let first = registry
            .register_or_update("person-1", "alice")
            .expect("first profile");
        let updated = registry
            .register_or_update("person-1", "alice-new")
            .expect("updated profile");

        assert_eq!(first.profile_id, updated.profile_id);
        assert_eq!(updated.person_ai_username, "alice-new");
        assert_eq!(registry.list().expect("profiles").len(), 1);
    }

    #[test]
    fn activation_is_exclusive_and_status_is_recoverable() {
        let registry = ProfileRegistryService::new(":memory:").expect("registry");
        let first = registry
            .register_or_update("person-1", "alice")
            .expect("first profile");
        let second = registry
            .register_or_update("person-2", "bob")
            .expect("second profile");

        registry
            .activate(&first.profile_id)
            .expect("activate first");
        registry
            .activate(&second.profile_id)
            .expect("activate second");
        let status = registry.bootstrap_status().expect("status");

        assert_eq!(status.active_profile_id, Some(second.profile_id.clone()));
        assert_eq!(
            status.active_profile.expect("active").person_ai_user_id,
            "person-2"
        );
        assert_eq!(
            registry
                .list()
                .expect("profiles")
                .iter()
                .filter(|profile| profile.status == "ACTIVE")
                .count(),
            1
        );
    }

    #[test]
    fn pending_profile_is_completed_in_place() {
        let registry = ProfileRegistryService::new(":memory:").expect("registry");
        let pending = registry.create_pending().expect("pending profile");
        assert_eq!(pending.status, "PENDING");

        let completed = registry
            .register_or_update("person-1", "alice")
            .expect("completed profile");
        assert_eq!(completed.profile_id, pending.profile_id);
        assert_eq!(completed.status, "ACTIVE");
        assert_eq!(
            registry
                .current_context()
                .expect("context")
                .person_ai_user_id,
            "person-1"
        );
    }

    #[test]
    fn profile_data_is_nested_under_a_non_identity_directory() {
        assert_eq!(
            default_profile_data_dir("/tmp/tron", "profile-123"),
            PathBuf::from("/tmp/tron/profiles/profile-123")
        );
    }
}
