use serde::{Deserialize, Serialize};

use super::UserError;

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
pub enum UserStatus {
    Active,
    Archived,
}

pub(crate) fn parse_user_status(value: &str) -> Result<UserStatus, UserError> {
    match value {
        "Active" => Ok(UserStatus::Active),
        "Archived" => Ok(UserStatus::Archived),
        _ => Err(UserError::InvalidStatus),
    }
}
