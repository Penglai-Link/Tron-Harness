use serde::Serialize;

use crate::shared::id::SnowflakeId;

use super::{PhoneNumber, UserStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum UserSourceType {
    Internal,
    GitHub,
    #[serde(rename = "LDAP")]
    Ldap,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: SnowflakeId,
    pub user_code: String,
    pub source_type: UserSourceType,
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
