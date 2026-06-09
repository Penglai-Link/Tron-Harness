use std::{fs, path::Path};

use crate::{
    domain::user::{PhoneNumber, UserError, UserSourceType, UserStatus},
    shared::time::{
        timestamp_from_millis as shared_timestamp_from_millis,
        timestamp_to_millis as shared_timestamp_to_millis,
    },
};

pub(super) struct StoredUserRow {
    pub(super) id: i64,
    pub(super) user_code: String,
    pub(super) source_type: UserSourceType,
    pub(super) username: String,
    pub(super) first_name: String,
    pub(super) last_name: String,
    pub(super) display_name: String,
    pub(super) email: String,
    pub(super) phone_country_code: Option<String>,
    pub(super) phone_number: Option<String>,
    pub(super) avatar_url: Option<String>,
    pub(super) locale: Option<String>,
    pub(super) timezone: Option<String>,
    pub(super) status: String,
    pub(super) password_hash: String,
    pub(super) password_reset_required: i64,
    pub(super) last_active_at: Option<i64>,
    pub(super) created_at: i64,
    pub(super) updated_at: i64,
    pub(super) archived_at: Option<i64>,
}

pub(super) fn ensure_database_parent(database_url: &str) -> Result<(), UserError> {
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

pub(super) fn bool_to_int(value: bool) -> i64 {
    i64::from(value)
}

pub(super) fn phone_from_parts(
    country_code: Option<String>,
    number: Option<String>,
) -> Option<PhoneNumber> {
    match (country_code, number) {
        (Some(country_code), Some(number)) => Some(PhoneNumber {
            country_code,
            number,
        }),
        _ => None,
    }
}

pub(super) fn user_status_to_storage(status: UserStatus) -> &'static str {
    match status {
        UserStatus::Active => "active",
        UserStatus::Archived => "archived",
    }
}

pub(super) fn user_status_from_storage(value: &str) -> Result<UserStatus, UserError> {
    match value {
        "active" => Ok(UserStatus::Active),
        "archived" => Ok(UserStatus::Archived),
        _ => Err(UserError::StorageFailure),
    }
}

pub(super) fn user_source_type_from_provider_code(value: Option<String>) -> UserSourceType {
    match value.as_deref() {
        Some("github") => UserSourceType::GitHub,
        Some("ldap") => UserSourceType::Ldap,
        _ => UserSourceType::Internal,
    }
}

pub(super) fn timestamp_to_millis(value: &str) -> i64 {
    shared_timestamp_to_millis(value)
}

pub(super) fn timestamp_from_millis(value: i64) -> String {
    shared_timestamp_from_millis(value)
}

pub(super) fn storage_error(_error: rusqlite::Error) -> UserError {
    UserError::StorageFailure
}
