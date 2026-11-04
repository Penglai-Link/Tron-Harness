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

#[derive(Clone)]
pub struct ProfileRegistryService {
    pub(super) connection: Arc<Mutex<Connection>>,
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
    pub avatar_url: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CachedProfileAvatar {
    pub content_type: String,
    pub image_bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonAiAccountSummary {
    pub nickname: Option<String>,
    pub vip_level: Option<i32>,
    pub remaining_points: Option<f64>,
    pub today_checked_in: Option<bool>,
    pub today_reward_points: Option<f64>,
    pub updated_at: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapProfileStatus {
    pub time_zone: String,
    pub has_profiles: bool,
    pub active_profile_id: Option<String>,
    pub active_profile: Option<ProfileRecord>,
    pub avatar_url: Option<String>,
    pub account: Option<PersonAiAccountSummary>,
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
        crate::adapters::outbound::persistence::sqlite::schema::initialize_profiles(&connection)?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    pub fn list(&self) -> Result<Vec<ProfileRecord>, ProfileRegistryError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ProfileRegistryError::LockPoisoned)?;
        let records = {
            let mut statement = connection.prepare(
                "SELECT profile_id, person_ai_user_id, person_ai_username, status,
                        created_at, last_activated_at
                 FROM profiles ORDER BY last_activated_at DESC, created_at DESC",
            )?;
            let rows = statement.query_map([], map_profile)?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        records
            .into_iter()
            .map(|record| profile_with_cached_avatar_url(&connection, record))
            .collect()
    }

    pub fn active(&self) -> Result<Option<ProfileRecord>, ProfileRegistryError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ProfileRegistryError::LockPoisoned)?;
        let profile = connection
            .query_row(
                "SELECT profile_id, person_ai_user_id, person_ai_username, status,
                        created_at, last_activated_at
                 FROM profiles
                 WHERE status = 'ACTIVE'
                 ORDER BY last_activated_at DESC, created_at DESC
                 LIMIT 1",
                [],
                map_profile,
            )
            .optional()
            .map_err(ProfileRegistryError::from)?;
        profile
            .map(|record| profile_with_cached_avatar_url(&connection, record))
            .transpose()
    }

    pub fn bootstrap_status(&self) -> Result<BootstrapProfileStatus, ProfileRegistryError> {
        let active_profile = self.active()?;
        let avatar_url = active_profile
            .as_ref()
            .and_then(|profile| profile.avatar_url.clone());
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
            account: match &active_profile {
                Some(profile) => self.cached_account(&profile.person_ai_user_id)?,
                None => None,
            },
            time_zone: match &active_profile {
                Some(profile) => self.time_zone(&profile.person_ai_user_id)?,
                None => "Asia/Shanghai".into(),
            },
            has_profiles,
            active_profile_id: active_profile
                .as_ref()
                .map(|profile| profile.profile_id.clone()),
            active_profile,
            avatar_url,
        })
    }

    pub fn time_zone(&self, user_id: &str) -> Result<String, ProfileRegistryError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ProfileRegistryError::LockPoisoned)?;
        Ok(connection
            .query_row(
                "SELECT time_zone FROM profile_preferences WHERE person_ai_user_id=?1",
                [user_id],
                |row| row.get(0),
            )
            .optional()?
            .unwrap_or_else(|| "Asia/Shanghai".into()))
    }

    pub fn set_time_zone(&self, user_id: &str, zone: &str) -> Result<String, ProfileRegistryError> {
        let zone = zone.trim();
        zone.parse::<chrono_tz::Tz>()
            .map_err(|_| ProfileRegistryError::InvalidInput("timeZone"))?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| ProfileRegistryError::LockPoisoned)?;
        connection.execute(
            "INSERT INTO profile_preferences(person_ai_user_id,time_zone,updated_at) VALUES(?1,?2,?3)
             ON CONFLICT(person_ai_user_id) DO UPDATE SET time_zone=excluded.time_zone,updated_at=excluded.updated_at",
            params![user_id, zone, crate::shared::time::current_timestamp()],
        )?;
        Ok(zone.into())
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
                     status = 'ACTIVE',
                     last_activated_at = excluded.created_at",
                params![profile_id, person_ai_user_id, person_ai_username, now],
            )?;
        }
        connection.execute(
            "UPDATE profiles SET status = 'INACTIVE'
             WHERE profile_id <> ?1 AND status = 'ACTIVE'",
            [profile_id.as_str()],
        )?;
        let profile = connection
            .query_row(
                "SELECT profile_id, person_ai_user_id, person_ai_username, status,
                        created_at, last_activated_at
                 FROM profiles WHERE person_ai_user_id = ?1",
                [person_ai_user_id],
                map_profile,
            )
            .map_err(ProfileRegistryError::from)?;
        profile_with_cached_avatar_url(&connection, profile)
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
        profile_with_cached_avatar_url(&connection, profile)
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
        profile_with_cached_avatar_url(&connection, profile)
    }

    pub fn remove(&self, profile_id: &str) -> Result<(), ProfileRegistryError> {
        let profile_id = profile_id.trim();
        if profile_id.is_empty() {
            return Err(ProfileRegistryError::InvalidInput("profileId"));
        }
        let connection = self
            .connection
            .lock()
            .map_err(|_| ProfileRegistryError::LockPoisoned)?;
        let transaction = connection.unchecked_transaction()?;
        let exists = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM profiles WHERE profile_id = ?1)",
            [profile_id],
            |row| row.get::<_, bool>(0),
        )?;
        if !exists {
            return Err(ProfileRegistryError::ProfileNotFound(profile_id.to_owned()));
        }
        let was_active = transaction.query_row(
            "SELECT status = 'ACTIVE' FROM profiles WHERE profile_id = ?1",
            [profile_id],
            |row| row.get::<_, bool>(0),
        )?;
        if was_active {
            if let Some(next_profile_id) = transaction
                .query_row(
                    "SELECT profile_id FROM profiles
                     WHERE profile_id <> ?1 AND status <> 'PENDING'
                     ORDER BY last_activated_at DESC, created_at DESC LIMIT 1",
                    [profile_id],
                    |row| row.get::<_, String>(0),
                )
                .optional()?
            {
                transaction.execute(
                    "UPDATE profiles SET status = 'ACTIVE', last_activated_at = ?2 WHERE profile_id = ?1",
                    params![next_profile_id, Utc::now().to_rfc3339()],
                )?;
            }
        }
        transaction.execute("DELETE FROM profiles WHERE profile_id = ?1", [profile_id])?;
        transaction.commit()?;
        Ok(())
    }

    pub fn current_context(&self) -> Result<CurrentProfileContext, ProfileRegistryError> {
        let profile = self
            .active()?
            .ok_or(ProfileRegistryError::ProfileNotFound("active".into()))?;
        if profile.status != "ACTIVE" {
            return Err(ProfileRegistryError::ProfileNotFound(profile.profile_id));
        }
        Ok(CurrentProfileContext {
            profile_id: profile.profile_id,
            person_ai_user_id: profile.person_ai_user_id,
            person_ai_username: profile.person_ai_username,
        })
    }

    pub fn cache_avatar(
        &self,
        person_ai_user_id: &str,
        content_type: &str,
        image_bytes: &[u8],
    ) -> Result<(), ProfileRegistryError> {
        let person_ai_user_id = person_ai_user_id.trim();
        if person_ai_user_id.is_empty() {
            return Err(ProfileRegistryError::InvalidInput("personAiUserId"));
        }
        let connection = self
            .connection
            .lock()
            .map_err(|_| ProfileRegistryError::LockPoisoned)?;
        connection.execute(
            "INSERT INTO profile_avatar_cache(person_ai_user_id, content_type, image_bytes, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(person_ai_user_id) DO UPDATE SET
                  content_type = excluded.content_type,
                  image_bytes = excluded.image_bytes,
                  updated_at = excluded.updated_at
             WHERE profile_avatar_cache.content_type <> excluded.content_type
                OR profile_avatar_cache.image_bytes <> excluded.image_bytes",
            params![
                person_ai_user_id,
                content_type.trim(),
                image_bytes,
                Utc::now().to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn clear_cached_avatar(&self, person_ai_user_id: &str) -> Result<(), ProfileRegistryError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ProfileRegistryError::LockPoisoned)?;
        connection.execute(
            "DELETE FROM profile_avatar_cache WHERE person_ai_user_id = ?1",
            [person_ai_user_id.trim()],
        )?;
        Ok(())
    }

    pub fn cached_avatar(
        &self,
        profile_id: &str,
    ) -> Result<Option<CachedProfileAvatar>, ProfileRegistryError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ProfileRegistryError::LockPoisoned)?;
        connection
            .query_row(
                "SELECT cache.content_type, cache.image_bytes
                 FROM profiles
                 INNER JOIN profile_avatar_cache AS cache
                   ON cache.person_ai_user_id = profiles.person_ai_user_id
                 WHERE profiles.profile_id = ?1",
                [profile_id],
                |row| {
                    Ok(CachedProfileAvatar {
                        content_type: row.get(0)?,
                        image_bytes: row.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn cached_avatar_url(
        &self,
        profile_id: &str,
    ) -> Result<Option<String>, ProfileRegistryError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ProfileRegistryError::LockPoisoned)?;
        cached_avatar_url_for_profile(&connection, profile_id)
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
        avatar_url: None,
    })
}

fn profile_with_cached_avatar_url(
    connection: &Connection,
    mut profile: ProfileRecord,
) -> Result<ProfileRecord, ProfileRegistryError> {
    profile.avatar_url = cached_avatar_url_for_profile(connection, &profile.profile_id)?;
    Ok(profile)
}

fn cached_avatar_url_for_profile(
    connection: &Connection,
    profile_id: &str,
) -> Result<Option<String>, ProfileRegistryError> {
    let updated_at = connection
        .query_row(
            "SELECT cache.updated_at
             FROM profiles
             INNER JOIN profile_avatar_cache AS cache
               ON cache.person_ai_user_id = profiles.person_ai_user_id
             WHERE profiles.profile_id = ?1",
            [profile_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    Ok(updated_at.map(|updated_at| {
        format!(
            "/api/v1/profiles/{}/avatar?v={}",
            profile_id,
            url::form_urlencoded::byte_serialize(updated_at.as_bytes()).collect::<String>()
        )
    }))
}

fn default_control_database_url() -> String {
    let home = env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home)
        .join(".tron/control/profiles.sqlite")
        .display()
        .to_string()
}
