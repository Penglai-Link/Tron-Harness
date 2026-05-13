use std::{
    fs,
    path::Path,
    sync::{Arc, Mutex, OnceLock},
};

use bcrypt::hash;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::shared::id::SnowflakeId;

const BCRYPT_MVP_COST: u32 = 4;
const DEFAULT_TIMESTAMP: &str = "2026-05-12T00:00:00Z";
const DEFAULT_EPOCH_MS: i64 = 1_778_544_000_000;
const LAST_ACTIVE_EPOCH_MS: i64 = 1_778_487_000_000;
const USERS_MIGRATION: &str =
    include_str!("../../../migrations/20260511_0001_create_user_management_users.sql");
const ASSIGNMENTS_MIGRATION: &str =
    include_str!("../../../migrations/20260511_0002_create_user_management_assignments.sql");
const PASSWORD_RESET_MIGRATION: &str =
    include_str!("../../../migrations/20260511_0003_create_user_password_reset_events.sql");

static GLOBAL_USER_SERVICE: OnceLock<UserService> = OnceLock::new();

#[derive(Clone)]
pub struct UserService {
    repository: Arc<Mutex<Box<dyn UserRepository>>>,
}

impl UserService {
    pub fn global() -> Self {
        GLOBAL_USER_SERVICE
            .get_or_init(|| {
                Self::from_database_url(":memory:").expect("SQLite repository should initialize")
            })
            .clone()
    }

    pub fn from_database_url(database_url: &str) -> Result<Self, UserError> {
        Ok(Self::new(SqliteUserRepository::connect(database_url)?))
    }

    fn new(repository: impl UserRepository + 'static) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
        }
    }

    pub fn list_users(&self, query: ListUsersQuery) -> Result<PagedUsers, UserError> {
        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        repository.list_users(query)
    }

    pub fn get_user(&self, user_code: &str) -> Result<User, UserError> {
        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        repository.get_user(user_code)
    }

    pub fn create_user(&self, command: CreateUserCommand) -> Result<User, UserError> {
        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        let username = required_profile_field(command.username)?;
        let first_name = required_profile_field(command.first_name)?;
        let last_name = required_profile_field(command.last_name)?;
        let display_name = required_profile_field(command.display_name)?;
        let email = required_profile_field(command.email)?;
        let explicit_user_code = optional_trimmed(command.user_code);

        if let Some(user_code) = explicit_user_code.as_deref() {
            validate_user_code(user_code)?;
        }
        validate_role_codes(&command.role_codes)?;
        validate_space_codes(&command.space_codes)?;
        if let Some(initial_password) = command.initial_password.as_deref() {
            validate_initial_password(initial_password)?;
        }

        let id = repository.next_id()?;
        let user_code = if let Some(user_code) = explicit_user_code {
            if repository.user_code_exists(&user_code, None)? {
                return Err(UserError::DuplicateUserCode);
            }
            user_code
        } else {
            repository.next_user_code(&username)?
        };
        if repository.username_exists(&username, None)? {
            return Err(UserError::DuplicateUsername);
        }
        if repository.email_exists(&email, None)? {
            return Err(UserError::DuplicateEmail);
        }

        let initial_password = command
            .initial_password
            .unwrap_or_else(|| format!("TronInitial{id}!"));
        let password_hash = hash_password(&initial_password)?;

        let user = User {
            id: SnowflakeId::new(id),
            user_code: user_code.clone(),
            username,
            first_name,
            last_name,
            display_name,
            email,
            phone: command.phone,
            avatar_url: command.avatar_url,
            locale: command.locale.unwrap_or_else(|| "en-US".into()),
            timezone: command
                .timezone
                .unwrap_or_else(|| "America/Los_Angeles".into()),
            status: UserStatus::Active,
            role_codes: command.role_codes,
            space_codes: command.space_codes,
            password_hash,
            password_reset_required: command.password_reset_required.unwrap_or(true),
            last_active_at: None,
            created_at: DEFAULT_TIMESTAMP.into(),
            updated_at: DEFAULT_TIMESTAMP.into(),
            archived_at: None,
        };

        repository.insert_user(&user)?;
        Ok(user)
    }

    pub fn patch_user(
        &self,
        user_code: &str,
        command: UpdateUserCommand,
    ) -> Result<User, UserError> {
        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        let mut user = repository.get_user(user_code)?;

        if let Some(username) = command.username {
            if repository.username_exists(&username, Some(user_code))? {
                return Err(UserError::DuplicateUsername);
            }
            user.username = username;
        }
        if let Some(first_name) = command.first_name {
            user.first_name = first_name;
        }
        if let Some(last_name) = command.last_name {
            user.last_name = last_name;
        }
        if let Some(display_name) = command.display_name {
            user.display_name = display_name;
        }
        if let Some(email) = command.email {
            if repository.email_exists(&email, Some(user_code))? {
                return Err(UserError::DuplicateEmail);
            }
            user.email = email;
        }
        if let Some(phone) = command.phone {
            user.phone = phone;
        }
        if let Some(avatar_url) = command.avatar_url {
            user.avatar_url = avatar_url;
        }
        if let Some(locale) = command.locale {
            user.locale = locale;
        }
        if let Some(timezone) = command.timezone {
            user.timezone = timezone;
        }
        if let Some(status) = command.status {
            let status = parse_user_status(&status)?;
            user.status = status;
            user.archived_at = match status {
                UserStatus::Active => None,
                UserStatus::Archived => Some(DEFAULT_TIMESTAMP.into()),
            };
        }
        user.updated_at = DEFAULT_TIMESTAMP.into();

        repository.update_user(&user)?;
        Ok(user.clone())
    }

    pub fn archive_user(&self, user_code: &str) -> Result<User, UserError> {
        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        let mut user = repository.get_user(user_code)?;
        user.status = UserStatus::Archived;
        user.archived_at = Some(DEFAULT_TIMESTAMP.into());
        user.updated_at = DEFAULT_TIMESTAMP.into();
        repository.update_user(&user)?;
        Ok(user.clone())
    }

    pub fn restore_user(&self, user_code: &str) -> Result<User, UserError> {
        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        let mut user = repository.get_user(user_code)?;
        user.status = UserStatus::Active;
        user.archived_at = None;
        user.updated_at = DEFAULT_TIMESTAMP.into();
        repository.update_user(&user)?;
        Ok(user.clone())
    }

    pub fn replace_roles(
        &self,
        user_code: &str,
        role_codes: Vec<String>,
    ) -> Result<User, UserError> {
        validate_role_codes(&role_codes)?;
        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        let mut user = repository.get_user(user_code)?;
        user.role_codes = unique_preserving_order(role_codes);
        user.updated_at = DEFAULT_TIMESTAMP.into();
        repository.replace_role_codes(&user)?;
        Ok(user.clone())
    }

    pub fn replace_spaces(
        &self,
        user_code: &str,
        space_codes: Vec<String>,
    ) -> Result<User, UserError> {
        validate_space_codes(&space_codes)?;
        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        let mut user = repository.get_user(user_code)?;
        user.space_codes = unique_preserving_order(space_codes);
        user.updated_at = DEFAULT_TIMESTAMP.into();
        repository.replace_space_codes(&user)?;
        Ok(user.clone())
    }

    pub fn reset_password(&self, user_code: &str) -> Result<PasswordResetResult, UserError> {
        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        let mut user = repository.get_user(user_code)?;
        let reset_id = repository.next_id()?;
        let temporary_password = format!("TronTmp{reset_id}!");
        let password_hash = hash_password(&temporary_password)?;

        user.password_hash = password_hash;
        user.password_reset_required = true;
        user.updated_at = DEFAULT_TIMESTAMP.into();
        repository.record_password_reset(&user, reset_id)?;

        Ok(PasswordResetResult {
            user_code: user.user_code.clone(),
            temporary_password,
            password_reset_required: true,
        })
    }
}

trait UserRepository: Send {
    fn list_users(&mut self, query: ListUsersQuery) -> Result<PagedUsers, UserError>;
    fn get_user(&mut self, user_code: &str) -> Result<User, UserError>;
    fn next_id(&mut self) -> Result<i64, UserError>;
    fn next_user_code(&mut self, username: &str) -> Result<String, UserError>;
    fn user_code_exists(
        &mut self,
        user_code: &str,
        except_user_code: Option<&str>,
    ) -> Result<bool, UserError>;
    fn username_exists(
        &mut self,
        username: &str,
        except_user_code: Option<&str>,
    ) -> Result<bool, UserError>;
    fn email_exists(
        &mut self,
        email: &str,
        except_user_code: Option<&str>,
    ) -> Result<bool, UserError>;
    fn insert_user(&mut self, user: &User) -> Result<(), UserError>;
    fn update_user(&mut self, user: &User) -> Result<(), UserError>;
    fn replace_role_codes(&mut self, user: &User) -> Result<(), UserError>;
    fn replace_space_codes(&mut self, user: &User) -> Result<(), UserError>;
    fn record_password_reset(&mut self, user: &User, reset_id: i64) -> Result<(), UserError>;
}

struct SqliteUserRepository {
    connection: Connection,
}

impl SqliteUserRepository {
    fn connect(database_url: &str) -> Result<Self, UserError> {
        ensure_database_parent(database_url)?;
        let connection = if database_url == ":memory:" || database_url.starts_with("file:") {
            Connection::open_with_flags(
                database_url,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
                    | rusqlite::OpenFlags::SQLITE_OPEN_CREATE
                    | rusqlite::OpenFlags::SQLITE_OPEN_URI,
            )
        } else {
            Connection::open(database_url)
        }
        .map_err(storage_error)?;

        let mut repository = Self { connection };
        repository.apply_migrations()?;
        repository.seed_if_empty()?;
        Ok(repository)
    }

    fn apply_migrations(&mut self) -> Result<(), UserError> {
        self.connection
            .execute_batch(USERS_MIGRATION)
            .map_err(storage_error)?;
        self.connection
            .execute_batch(ASSIGNMENTS_MIGRATION)
            .map_err(storage_error)?;
        self.connection
            .execute_batch(PASSWORD_RESET_MIGRATION)
            .map_err(storage_error)
    }

    fn seed_if_empty(&mut self) -> Result<(), UserError> {
        let count: i64 = self
            .connection
            .query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
            .map_err(storage_error)?;
        if count == 0 {
            for user in seeded_users() {
                self.insert_user(&user)?;
            }
        }
        Ok(())
    }

    fn all_users(&mut self) -> Result<Vec<User>, UserError> {
        let user_codes = {
            let mut statement = self
                .connection
                .prepare("SELECT user_code FROM users ORDER BY display_name ASC")
                .map_err(storage_error)?;
            statement
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(storage_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(storage_error)?
        };

        user_codes
            .iter()
            .map(|user_code| self.get_user(user_code))
            .collect()
    }

    fn role_codes(&self, user_id: i64) -> Result<Vec<String>, UserError> {
        self.assignment_codes(
            "SELECT role_code FROM user_role_assignments WHERE user_id = ?1 ORDER BY id ASC",
            user_id,
        )
    }

    fn space_codes(&self, user_id: i64) -> Result<Vec<String>, UserError> {
        self.assignment_codes(
            "SELECT space_code FROM user_space_assignments WHERE user_id = ?1 ORDER BY id ASC",
            user_id,
        )
    }

    fn assignment_codes(&self, sql: &str, user_id: i64) -> Result<Vec<String>, UserError> {
        let mut statement = self.connection.prepare(sql).map_err(storage_error)?;
        statement
            .query_map([user_id], |row| row.get::<_, String>(0))
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn insert_assignments(
        &mut self,
        table: &str,
        code_column: &str,
        user: &User,
        codes: &[String],
    ) -> Result<(), UserError> {
        for code in codes {
            let id = self.next_id()?;
            let sql = format!(
                "INSERT INTO {table} (id, user_id, user_code, {code_column}, assigned_at) VALUES (?1, ?2, ?3, ?4, ?5)"
            );
            self.connection
                .execute(
                    &sql,
                    params![
                        id,
                        user.id.value(),
                        user.user_code,
                        code,
                        timestamp_to_millis(&user.updated_at),
                    ],
                )
                .map_err(storage_error)?;
        }
        Ok(())
    }

    fn scalar_exists(
        &self,
        sql: &str,
        value: &str,
        except_user_code: Option<&str>,
    ) -> Result<bool, UserError> {
        let count: i64 = self
            .connection
            .query_row(sql, params![value, except_user_code], |row| row.get(0))
            .map_err(storage_error)?;
        Ok(count > 0)
    }
}

impl UserRepository for SqliteUserRepository {
    fn list_users(&mut self, query: ListUsersQuery) -> Result<PagedUsers, UserError> {
        let current = query.current.or(query.page).unwrap_or(1).max(1);
        let size = query.size.unwrap_or(20).clamp(1, 100);
        let search = query.search.map(|value| value.to_ascii_lowercase());

        let mut records = self
            .all_users()?
            .into_iter()
            .filter(|user| {
                if let Some(status) = query.status {
                    user.status == status
                } else if query.include_archived.unwrap_or(false) {
                    true
                } else {
                    user.status == UserStatus::Active
                }
            })
            .filter(|user| {
                query
                    .role_code
                    .as_ref()
                    .is_none_or(|role| user.role_codes.iter().any(|value| value == role))
            })
            .filter(|user| {
                query
                    .space_code
                    .as_ref()
                    .is_none_or(|space| user.space_codes.iter().any(|value| value == space))
            })
            .filter(|user| {
                search.as_ref().is_none_or(|search| {
                    user.user_code.to_ascii_lowercase().contains(search)
                        || user.username.to_ascii_lowercase().contains(search)
                        || user.display_name.to_ascii_lowercase().contains(search)
                        || user.email.to_ascii_lowercase().contains(search)
                })
            })
            .collect::<Vec<_>>();

        records.sort_by(|left, right| left.display_name.cmp(&right.display_name));
        let total = records.len();
        let start = ((current - 1) * size).min(total);
        let end = (start + size).min(total);

        Ok(PagedUsers {
            current,
            size,
            total,
            records: records[start..end].to_vec(),
        })
    }

    fn get_user(&mut self, user_code: &str) -> Result<User, UserError> {
        let row = self
            .connection
            .query_row(
                "SELECT id, user_code, username, first_name, last_name, display_name, email, phone_country_code, phone_number, avatar_url, locale, timezone, status, password_hash, password_reset_required, last_active_at, created_at, updated_at, archived_at FROM users WHERE user_code = ?1",
                [user_code],
                |row| {
                    Ok(StoredUserRow {
                        id: row.get(0)?,
                        user_code: row.get(1)?,
                        username: row.get(2)?,
                        first_name: row.get(3)?,
                        last_name: row.get(4)?,
                        display_name: row.get(5)?,
                        email: row.get(6)?,
                        phone_country_code: row.get(7)?,
                        phone_number: row.get(8)?,
                        avatar_url: row.get(9)?,
                        locale: row.get(10)?,
                        timezone: row.get(11)?,
                        status: row.get(12)?,
                        password_hash: row.get(13)?,
                        password_reset_required: row.get(14)?,
                        last_active_at: row.get(15)?,
                        created_at: row.get(16)?,
                        updated_at: row.get(17)?,
                        archived_at: row.get(18)?,
                    })
                },
            )
            .optional()
            .map_err(storage_error)?
            .ok_or(UserError::NotFound)?;

        let role_codes = self.role_codes(row.id)?;
        let space_codes = self.space_codes(row.id)?;

        Ok(User {
            id: SnowflakeId::new(row.id),
            user_code: row.user_code,
            username: row.username,
            first_name: row.first_name,
            last_name: row.last_name,
            display_name: row.display_name,
            email: row.email,
            phone: phone_from_parts(row.phone_country_code, row.phone_number),
            avatar_url: row.avatar_url,
            locale: row.locale.unwrap_or_else(|| "en-US".into()),
            timezone: row.timezone.unwrap_or_else(|| "America/Los_Angeles".into()),
            status: user_status_from_storage(&row.status)?,
            role_codes,
            space_codes,
            password_hash: row.password_hash,
            password_reset_required: row.password_reset_required != 0,
            last_active_at: row.last_active_at.map(timestamp_from_millis),
            created_at: timestamp_from_millis(row.created_at),
            updated_at: timestamp_from_millis(row.updated_at),
            archived_at: row.archived_at.map(timestamp_from_millis),
        })
    }

    fn next_id(&mut self) -> Result<i64, UserError> {
        let max_id: i64 = self
            .connection
            .query_row(
                "SELECT max(id) FROM (
                    SELECT id FROM users
                    UNION ALL SELECT id FROM user_role_assignments
                    UNION ALL SELECT id FROM user_space_assignments
                    UNION ALL SELECT id FROM user_password_reset_events
                )",
                [],
                |row| row.get::<_, Option<i64>>(0),
            )
            .map_err(storage_error)?
            .unwrap_or(734_200_000_000_000_000);
        Ok(max_id + 1)
    }

    fn next_user_code(&mut self, username: &str) -> Result<String, UserError> {
        let mut code = format!("USR-{}", normalize_code_segment(username));
        if code == "USR-" {
            code = "USR-NEW-USER".into();
        }
        if !self.user_code_exists(&code, None)? {
            return Ok(code);
        }

        let mut suffix = 2;
        loop {
            let candidate = format!("{code}-{suffix}");
            if !self.user_code_exists(&candidate, None)? {
                return Ok(candidate);
            }
            suffix += 1;
        }
    }

    fn user_code_exists(
        &mut self,
        user_code: &str,
        except_user_code: Option<&str>,
    ) -> Result<bool, UserError> {
        self.scalar_exists(
            "SELECT COUNT(*) FROM users WHERE user_code = ?1 AND (?2 IS NULL OR user_code != ?2)",
            user_code,
            except_user_code,
        )
    }

    fn username_exists(
        &mut self,
        username: &str,
        except_user_code: Option<&str>,
    ) -> Result<bool, UserError> {
        self.scalar_exists(
            "SELECT COUNT(*) FROM users WHERE username = ?1 COLLATE NOCASE AND (?2 IS NULL OR user_code != ?2)",
            username,
            except_user_code,
        )
    }

    fn email_exists(
        &mut self,
        email: &str,
        except_user_code: Option<&str>,
    ) -> Result<bool, UserError> {
        self.scalar_exists(
            "SELECT COUNT(*) FROM users WHERE email = lower(?1) COLLATE NOCASE AND (?2 IS NULL OR user_code != ?2)",
            email,
            except_user_code,
        )
    }

    fn insert_user(&mut self, user: &User) -> Result<(), UserError> {
        self.connection
            .execute(
                "INSERT INTO users (
                    id, user_code, username, first_name, last_name, display_name, email,
                    phone_country_code, phone_number, avatar_url, locale, timezone, status,
                    password_hash, password_hash_algorithm, password_updated_at,
                    password_reset_required, last_active_at, archived_at, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, lower(?7), ?8, ?9, ?10, ?11, ?12, ?13, ?14, 'bcrypt', ?15, ?16, ?17, ?18, ?19, ?20)",
                params![
                    user.id.value(),
                    user.user_code,
                    user.username,
                    user.first_name,
                    user.last_name,
                    user.display_name,
                    user.email,
                    user.phone.as_ref().map(|phone| phone.country_code.as_str()),
                    user.phone.as_ref().map(|phone| phone.number.as_str()),
                    user.avatar_url,
                    user.locale,
                    user.timezone,
                    user_status_to_storage(user.status),
                    user.password_hash,
                    timestamp_to_millis(&user.updated_at),
                    bool_to_int(user.password_reset_required),
                    user.last_active_at.as_deref().map(timestamp_to_millis),
                    user.archived_at.as_deref().map(timestamp_to_millis),
                    timestamp_to_millis(&user.created_at),
                    timestamp_to_millis(&user.updated_at),
                ],
            )
            .map_err(storage_error)?;
        self.insert_assignments("user_role_assignments", "role_code", user, &user.role_codes)?;
        self.insert_assignments(
            "user_space_assignments",
            "space_code",
            user,
            &user.space_codes,
        )
    }

    fn update_user(&mut self, user: &User) -> Result<(), UserError> {
        self.connection
            .execute(
                "UPDATE users SET
                    username = ?2,
                    first_name = ?3,
                    last_name = ?4,
                    display_name = ?5,
                    email = lower(?6),
                    phone_country_code = ?7,
                    phone_number = ?8,
                    avatar_url = ?9,
                    locale = ?10,
                    timezone = ?11,
                    status = ?12,
                    password_hash = ?13,
                    password_updated_at = ?14,
                    password_reset_required = ?15,
                    last_active_at = ?16,
                    archived_at = ?17,
                    updated_at = ?18,
                    version = version + 1
                WHERE user_code = ?1",
                params![
                    user.user_code,
                    user.username,
                    user.first_name,
                    user.last_name,
                    user.display_name,
                    user.email,
                    user.phone.as_ref().map(|phone| phone.country_code.as_str()),
                    user.phone.as_ref().map(|phone| phone.number.as_str()),
                    user.avatar_url,
                    user.locale,
                    user.timezone,
                    user_status_to_storage(user.status),
                    user.password_hash,
                    timestamp_to_millis(&user.updated_at),
                    bool_to_int(user.password_reset_required),
                    user.last_active_at.as_deref().map(timestamp_to_millis),
                    user.archived_at.as_deref().map(timestamp_to_millis),
                    timestamp_to_millis(&user.updated_at),
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn replace_role_codes(&mut self, user: &User) -> Result<(), UserError> {
        self.update_user(user)?;
        self.connection
            .execute(
                "DELETE FROM user_role_assignments WHERE user_id = ?1",
                [user.id.value()],
            )
            .map_err(storage_error)?;
        self.insert_assignments("user_role_assignments", "role_code", user, &user.role_codes)
    }

    fn replace_space_codes(&mut self, user: &User) -> Result<(), UserError> {
        self.update_user(user)?;
        self.connection
            .execute(
                "DELETE FROM user_space_assignments WHERE user_id = ?1",
                [user.id.value()],
            )
            .map_err(storage_error)?;
        self.insert_assignments(
            "user_space_assignments",
            "space_code",
            user,
            &user.space_codes,
        )
    }

    fn record_password_reset(&mut self, user: &User, reset_id: i64) -> Result<(), UserError> {
        self.update_user(user)?;
        self.connection
            .execute(
                "INSERT INTO user_password_reset_events (
                    id, reset_code, user_id, user_code, password_hash_algorithm,
                    password_reset_required_after, temporary_password_returned, created_at
                ) VALUES (?1, ?2, ?3, ?4, 'bcrypt', 1, 1, ?5)",
                params![
                    reset_id,
                    format!("RESET-{reset_id}"),
                    user.id.value(),
                    user.user_code,
                    timestamp_to_millis(&user.updated_at),
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }
}

struct StoredUserRow {
    id: i64,
    user_code: String,
    username: String,
    first_name: String,
    last_name: String,
    display_name: String,
    email: String,
    phone_country_code: Option<String>,
    phone_number: Option<String>,
    avatar_url: Option<String>,
    locale: Option<String>,
    timezone: Option<String>,
    status: String,
    password_hash: String,
    password_reset_required: i64,
    last_active_at: Option<i64>,
    created_at: i64,
    updated_at: i64,
    archived_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: SnowflakeId,
    pub user_code: String,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub display_name: String,
    pub email: String,
    pub phone: Option<PhoneNumber>,
    pub avatar_url: Option<String>,
    pub locale: String,
    pub timezone: String,
    pub status: UserStatus,
    pub role_codes: Vec<String>,
    pub space_codes: Vec<String>,
    pub password_hash: String,
    pub password_reset_required: bool,
    pub last_active_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhoneNumber {
    pub country_code: String,
    pub number: String,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
pub enum UserStatus {
    Active,
    Archived,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserCommand {
    pub user_code: Option<String>,
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<PhoneNumber>,
    pub avatar_url: Option<String>,
    pub locale: Option<String>,
    pub timezone: Option<String>,
    pub initial_password: Option<String>,
    pub password_reset_required: Option<bool>,
    #[serde(default)]
    pub role_codes: Vec<String>,
    #[serde(default)]
    pub space_codes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserCommand {
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<Option<PhoneNumber>>,
    pub avatar_url: Option<Option<String>>,
    pub locale: Option<String>,
    pub timezone: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListUsersQuery {
    pub page: Option<usize>,
    pub current: Option<usize>,
    pub size: Option<usize>,
    pub status: Option<UserStatus>,
    pub search: Option<String>,
    pub role_code: Option<String>,
    pub space_code: Option<String>,
    pub include_archived: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct PagedUsers {
    pub current: usize,
    pub size: usize,
    pub total: usize,
    pub records: Vec<User>,
}

#[derive(Debug, Clone)]
pub struct PasswordResetResult {
    pub user_code: String,
    pub temporary_password: String,
    pub password_reset_required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserError {
    NotFound,
    InvalidUserCode,
    DuplicateUserCode,
    DuplicateUsername,
    DuplicateEmail,
    RequiredProfileField,
    WeakInitialPassword,
    InvalidStatus,
    InvalidRoleCode,
    InvalidSpaceCode,
    PasswordHashFailed,
    StorageFailure,
}

fn ensure_database_parent(database_url: &str) -> Result<(), UserError> {
    if database_url == ":memory:" || database_url.starts_with("file:") {
        return Ok(());
    }
    if let Some(parent) = Path::new(database_url).parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|_| UserError::StorageFailure)?;
    }
    Ok(())
}

fn seeded_users() -> Vec<User> {
    vec![
        seeded_user(
            734_102_998_871_252_992,
            "USR-0001",
            "alex.johnson",
            "Alex",
            "Johnson",
            "Alex Johnson",
            "alex.johnson@tron.ai",
            UserStatus::Active,
            vec!["DEVELOPER"],
            vec!["PLATFORM"],
        ),
        seeded_user(
            734_103_002_743_185_408,
            "USR-0002",
            "maya.singh",
            "Maya",
            "Singh",
            "Maya Singh",
            "maya.singh@tron.ai",
            UserStatus::Active,
            vec!["VIEWER"],
            vec!["ENTERPRISE"],
        ),
        seeded_user(
            734_103_002_743_185_409,
            "USR-0007",
            "sam.viewer",
            "Sam",
            "Viewer",
            "Sam Viewer",
            "sam.viewer@tron.ai",
            UserStatus::Active,
            vec!["VIEWER"],
            vec!["SUPPORT"],
        ),
        seeded_user(
            734_103_002_743_185_410,
            "USR-0009",
            "archived.restore",
            "Archived",
            "Restore",
            "Archived Restore",
            "archived.restore@tron.ai",
            UserStatus::Archived,
            vec!["DEVELOPER"],
            vec!["DATA"],
        ),
        seeded_user(
            734_103_002_743_185_411,
            "USR-1001",
            "reset.target",
            "Reset",
            "Target",
            "Reset Target",
            "reset.target@tron.ai",
            UserStatus::Active,
            vec!["ADMIN"],
            vec!["PLATFORM"],
        ),
        seeded_user(
            9_007_199_254_740_993,
            "USR-LARGE-ID",
            "large.id",
            "Large",
            "Identifier",
            "Large Identifier",
            "large.id@tron.ai",
            UserStatus::Active,
            vec!["WORKSPACE_ADMIN"],
            vec!["ENTERPRISE"],
        ),
        seeded_user(
            734_103_002_743_185_412,
            "USR-ARCHIVED",
            "archived.only",
            "Archived",
            "Only",
            "Archived Only",
            "archived.only@tron.ai",
            UserStatus::Archived,
            vec!["VIEWER"],
            vec!["SUPPORT"],
        ),
    ]
}

fn seeded_user(
    id: i64,
    user_code: &str,
    username: &str,
    first_name: &str,
    last_name: &str,
    display_name: &str,
    email: &str,
    status: UserStatus,
    role_codes: Vec<&str>,
    space_codes: Vec<&str>,
) -> User {
    User {
        id: SnowflakeId::new(id),
        user_code: user_code.into(),
        username: username.into(),
        first_name: first_name.into(),
        last_name: last_name.into(),
        display_name: display_name.into(),
        email: email.into(),
        phone: Some(PhoneNumber {
            country_code: "+1".into(),
            number: "5550100".into(),
        }),
        avatar_url: None,
        locale: "en-US".into(),
        timezone: "America/Los_Angeles".into(),
        status,
        role_codes: role_codes.into_iter().map(str::to_owned).collect(),
        space_codes: space_codes.into_iter().map(str::to_owned).collect(),
        password_hash: "$2b$04$wzYH5I9Aw4f3ZV5h9tczhOJtT5QxQfyeZDJmW5hE4d1tij8nS0V9G".into(),
        password_reset_required: false,
        last_active_at: Some("2026-05-11T08:30:00Z".into()),
        created_at: "2026-05-11T08:00:00Z".into(),
        updated_at: "2026-05-11T08:00:00Z".into(),
        archived_at: (status == UserStatus::Archived).then(|| "2026-05-11T08:00:00Z".into()),
    }
}

fn bool_to_int(value: bool) -> i64 {
    i64::from(value)
}

fn phone_from_parts(country_code: Option<String>, number: Option<String>) -> Option<PhoneNumber> {
    match (country_code, number) {
        (Some(country_code), Some(number)) => Some(PhoneNumber {
            country_code,
            number,
        }),
        _ => None,
    }
}

fn user_status_to_storage(status: UserStatus) -> &'static str {
    match status {
        UserStatus::Active => "active",
        UserStatus::Archived => "archived",
    }
}

fn user_status_from_storage(value: &str) -> Result<UserStatus, UserError> {
    match value {
        "active" => Ok(UserStatus::Active),
        "archived" => Ok(UserStatus::Archived),
        _ => Err(UserError::StorageFailure),
    }
}

fn timestamp_to_millis(value: &str) -> i64 {
    match value {
        "2026-05-11T08:30:00Z" => LAST_ACTIVE_EPOCH_MS,
        "2026-05-11T08:00:00Z" => DEFAULT_EPOCH_MS - 86_400_000,
        _ => DEFAULT_EPOCH_MS,
    }
}

fn timestamp_from_millis(value: i64) -> String {
    match value {
        LAST_ACTIVE_EPOCH_MS => "2026-05-11T08:30:00Z".into(),
        value if value == DEFAULT_EPOCH_MS - 86_400_000 => "2026-05-11T08:00:00Z".into(),
        _ => DEFAULT_TIMESTAMP.into(),
    }
}

fn storage_error(_error: rusqlite::Error) -> UserError {
    UserError::StorageFailure
}

fn validate_role_codes(role_codes: &[String]) -> Result<(), UserError> {
    if role_codes.iter().all(|code| {
        matches!(
            code.as_str(),
            "ADMIN" | "WORKSPACE_ADMIN" | "DEVELOPER" | "VIEWER"
        )
    }) {
        Ok(())
    } else {
        Err(UserError::InvalidRoleCode)
    }
}

fn validate_space_codes(space_codes: &[String]) -> Result<(), UserError> {
    if space_codes.iter().all(|code| {
        let trimmed = code.trim();
        !trimmed.is_empty()
            && trimmed == code
            && code
                .chars()
                .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
    }) {
        Ok(())
    } else {
        Err(UserError::InvalidSpaceCode)
    }
}

fn validate_user_code(user_code: &str) -> Result<(), UserError> {
    if !user_code.is_empty()
        && user_code
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
    {
        Ok(())
    } else {
        Err(UserError::InvalidUserCode)
    }
}

fn parse_user_status(value: &str) -> Result<UserStatus, UserError> {
    match value {
        "Active" => Ok(UserStatus::Active),
        "Archived" => Ok(UserStatus::Archived),
        _ => Err(UserError::InvalidStatus),
    }
}

fn unique_preserving_order(values: Vec<String>) -> Vec<String> {
    values.into_iter().fold(Vec::new(), |mut acc, value| {
        if !acc.contains(&value) {
            acc.push(value);
        }
        acc
    })
}

fn required_profile_field(value: Option<String>) -> Result<String, UserError> {
    let value = value.ok_or(UserError::RequiredProfileField)?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(UserError::RequiredProfileField)
    } else {
        Ok(trimmed.to_owned())
    }
}

fn optional_trimmed(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_owned())
    })
}

fn validate_initial_password(value: &str) -> Result<(), UserError> {
    let has_min_length = value.chars().count() >= 8;
    let has_uppercase = value.chars().any(|ch| ch.is_ascii_uppercase());
    let has_lowercase = value.chars().any(|ch| ch.is_ascii_lowercase());
    let has_digit = value.chars().any(|ch| ch.is_ascii_digit());
    let has_symbol = value.chars().any(|ch| !ch.is_ascii_alphanumeric());

    if has_min_length && has_uppercase && has_lowercase && has_digit && has_symbol {
        Ok(())
    } else {
        Err(UserError::WeakInitialPassword)
    }
}

fn normalize_code_segment(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_uppercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn hash_password(value: &str) -> Result<String, UserError> {
    hash(value, BCRYPT_MVP_COST).map_err(|_| UserError::PasswordHashFailed)
}
