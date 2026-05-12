use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, OnceLock},
};

use bcrypt::hash;
use serde::{Deserialize, Serialize};

use crate::shared::id::SnowflakeId;

const BCRYPT_MVP_COST: u32 = 4;
const DEFAULT_TIMESTAMP: &str = "2026-05-12T00:00:00Z";

static GLOBAL_USER_SERVICE: OnceLock<UserService> = OnceLock::new();

#[derive(Clone)]
pub struct UserService {
    repository: Arc<Mutex<InMemoryUserRepository>>,
}

impl UserService {
    pub fn global() -> Self {
        GLOBAL_USER_SERVICE
            .get_or_init(|| Self::new(InMemoryUserRepository::seeded()))
            .clone()
    }

    fn new(repository: InMemoryUserRepository) -> Self {
        Self {
            repository: Arc::new(Mutex::new(repository)),
        }
    }

    pub fn list_users(&self, query: ListUsersQuery) -> PagedUsers {
        let repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        repository.list_users(query)
    }

    pub fn get_user(&self, user_code: &str) -> Result<User, UserError> {
        let repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        repository
            .users
            .get(user_code)
            .cloned()
            .ok_or(UserError::NotFound)
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

        validate_role_codes(&command.role_codes)?;
        validate_space_codes(&command.space_codes)?;
        if let Some(initial_password) = command.initial_password.as_deref() {
            validate_initial_password(initial_password)?;
        }

        if explicit_user_code
            .as_ref()
            .is_some_and(|user_code| repository.users.contains_key(user_code))
        {
            return Err(UserError::DuplicateUserCode);
        }
        if repository.username_exists(&username) {
            return Err(UserError::DuplicateUsername);
        }
        if repository.email_exists(&email) {
            return Err(UserError::DuplicateEmail);
        }

        let id = repository.next_id();
        let user_code = explicit_user_code.unwrap_or_else(|| repository.next_user_code(&username));
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

        repository.users.insert(user_code, user.clone());
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
        let user = repository
            .users
            .get_mut(user_code)
            .ok_or(UserError::NotFound)?;

        if let Some(username) = command.username {
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
            user.status = status;
            user.archived_at = match status {
                UserStatus::Active => None,
                UserStatus::Archived => Some(DEFAULT_TIMESTAMP.into()),
            };
        }
        user.updated_at = DEFAULT_TIMESTAMP.into();

        Ok(user.clone())
    }

    pub fn archive_user(&self, user_code: &str) -> Result<User, UserError> {
        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        let user = repository
            .users
            .get_mut(user_code)
            .ok_or(UserError::NotFound)?;
        user.status = UserStatus::Archived;
        user.archived_at = Some(DEFAULT_TIMESTAMP.into());
        user.updated_at = DEFAULT_TIMESTAMP.into();
        Ok(user.clone())
    }

    pub fn restore_user(&self, user_code: &str) -> Result<User, UserError> {
        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        let user = repository
            .users
            .get_mut(user_code)
            .ok_or(UserError::NotFound)?;
        user.status = UserStatus::Active;
        user.archived_at = None;
        user.updated_at = DEFAULT_TIMESTAMP.into();
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
        let user = repository
            .users
            .get_mut(user_code)
            .ok_or(UserError::NotFound)?;
        user.role_codes = unique_preserving_order(role_codes);
        user.updated_at = DEFAULT_TIMESTAMP.into();
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
        let user = repository
            .users
            .get_mut(user_code)
            .ok_or(UserError::NotFound)?;
        user.space_codes = unique_preserving_order(space_codes);
        user.updated_at = DEFAULT_TIMESTAMP.into();
        Ok(user.clone())
    }

    pub fn reset_password(&self, user_code: &str) -> Result<PasswordResetResult, UserError> {
        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        let reset_id = repository.next_id();
        let temporary_password = format!("TronTmp{reset_id}!");
        let password_hash = hash_password(&temporary_password)?;
        let user = repository
            .users
            .get_mut(user_code)
            .ok_or(UserError::NotFound)?;

        user.password_hash = password_hash;
        user.password_reset_required = true;
        user.updated_at = DEFAULT_TIMESTAMP.into();

        Ok(PasswordResetResult {
            user_code: user.user_code.clone(),
            temporary_password,
            password_reset_required: true,
        })
    }
}

#[derive(Debug)]
struct InMemoryUserRepository {
    users: BTreeMap<String, User>,
    next_id: i64,
}

impl InMemoryUserRepository {
    fn seeded() -> Self {
        let users = [
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
        .into_iter()
        .map(|user| (user.user_code.clone(), user))
        .collect();

        Self {
            users,
            next_id: 734_200_000_000_000_000,
        }
    }

    fn next_id(&mut self) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn next_user_code(&self, username: &str) -> String {
        let mut code = format!("USR-{}", normalize_code_segment(username));
        if code == "USR-" {
            code = "USR-NEW-USER".into();
        }
        if !self.users.contains_key(&code) {
            return code;
        }

        let mut suffix = 2;
        loop {
            let candidate = format!("{code}-{suffix}");
            if !self.users.contains_key(&candidate) {
                return candidate;
            }
            suffix += 1;
        }
    }

    fn username_exists(&self, username: &str) -> bool {
        self.users
            .values()
            .any(|user| user.username.eq_ignore_ascii_case(username))
    }

    fn email_exists(&self, email: &str) -> bool {
        self.users
            .values()
            .any(|user| user.email.eq_ignore_ascii_case(email))
    }

    fn list_users(&self, query: ListUsersQuery) -> PagedUsers {
        let current = query.current.or(query.page).unwrap_or(1).max(1);
        let size = query.size.unwrap_or(20).clamp(1, 100);
        let search = query.search.map(|value| value.to_ascii_lowercase());

        let mut records = self
            .users
            .values()
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
            .cloned()
            .collect::<Vec<_>>();

        records.sort_by(|left, right| left.display_name.cmp(&right.display_name));
        let total = records.len();
        let start = ((current - 1) * size).min(total);
        let end = (start + size).min(total);

        PagedUsers {
            current,
            size,
            total,
            records: records[start..end].to_vec(),
        }
    }
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
    pub status: Option<UserStatus>,
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
    DuplicateUserCode,
    DuplicateUsername,
    DuplicateEmail,
    RequiredProfileField,
    WeakInitialPassword,
    InvalidRoleCode,
    InvalidSpaceCode,
    PasswordHashFailed,
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
