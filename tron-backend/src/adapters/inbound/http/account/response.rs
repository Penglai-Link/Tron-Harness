use serde::Serialize;

use crate::{
    application::user::PasswordChangeResult,
    domain::{
        auth::AuthMethod,
        user::{PhoneNumber, User, UserSourceType, UserStatus},
    },
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountProfileDto {
    user_code: String,
    source_type: UserSourceType,
    username: String,
    first_name: String,
    last_name: String,
    display_name: String,
    email: String,
    phone: Option<PhoneNumber>,
    avatar_url: Option<String>,
    locale: String,
    timezone: String,
    status: UserStatus,
    role_codes: Vec<String>,
    space_codes: Vec<String>,
    password_change_allowed: bool,
    password_reset_required: bool,
    updated_at: String,
}

impl AccountProfileDto {
    pub fn new(user: User, auth_method: AuthMethod) -> Self {
        let password_change_allowed =
            auth_method == AuthMethod::Internal && user.source_type == UserSourceType::Internal;

        Self {
            user_code: user.user_code,
            source_type: user.source_type,
            username: user.username,
            first_name: user.first_name,
            last_name: user.last_name,
            display_name: user.display_name,
            email: user.email,
            phone: user.phone,
            avatar_url: user.avatar_url,
            locale: user.locale,
            timezone: user.timezone,
            status: user.status,
            role_codes: user.role_codes,
            space_codes: user.space_codes,
            password_change_allowed,
            password_reset_required: user.password_reset_required,
            updated_at: user.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordChangeDto {
    changed: bool,
    password_reset_required: bool,
}

impl From<PasswordChangeResult> for PasswordChangeDto {
    fn from(result: PasswordChangeResult) -> Self {
        Self {
            changed: result.changed,
            password_reset_required: result.password_reset_required,
        }
    }
}
