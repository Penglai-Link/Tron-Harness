use crate::domain::user::{PhoneNumber, UserError};

#[derive(Debug, Clone)]
pub struct UpdateAccountProfileCommand {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub display_name: Option<String>,
    pub phone: Option<Option<PhoneNumber>>,
    pub avatar_url: Option<Option<String>>,
    pub locale: Option<String>,
    pub timezone: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChangePasswordCommand {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountError {
    User(UserError),
    CurrentPasswordIncorrect,
    PasswordChangeUnsupported,
    PasswordUnchanged,
    PasswordVerifyFailed,
}

impl From<UserError> for AccountError {
    fn from(error: UserError) -> Self {
        Self::User(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PasswordChangeResult {
    pub changed: bool,
    pub password_reset_required: bool,
}
