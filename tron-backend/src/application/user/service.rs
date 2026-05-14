use std::sync::{Arc, Mutex};

use crate::{
    application::user::{CreateUserCommand, PasswordHasher, UpdateUserCommand, UserRepository},
    domain::user::{
        ListUsersQuery, PagedUsers, PasswordResetResult, User, UserError, UserStatistics,
        UserStatus, optional_trimmed, parse_user_status, required_profile_field,
        unique_preserving_order, validate_initial_password, validate_role_codes,
        validate_space_codes, validate_user_code,
    },
    shared::id::SnowflakeId,
};

const DEFAULT_TIMESTAMP: &str = "2026-05-12T00:00:00Z";

#[derive(Clone)]
pub struct UserService {
    repository: Arc<Mutex<Box<dyn UserRepository>>>,
    password_hasher: Arc<dyn PasswordHasher>,
}

impl UserService {
    pub fn new(
        repository: impl UserRepository + 'static,
        password_hasher: impl PasswordHasher + 'static,
    ) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
            password_hasher: Arc::new(password_hasher),
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

    pub fn statistics(&self) -> Result<UserStatistics, UserError> {
        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        repository.statistics()
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
        let password_hash = self.password_hasher.hash_password(&initial_password)?;

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
        let password_hash = self.password_hasher.hash_password(&temporary_password)?;

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
