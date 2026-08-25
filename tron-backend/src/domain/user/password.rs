use super::UserError;

#[derive(Debug, Clone)]
pub struct PasswordResetResult {
    pub user_code: String,
    pub temporary_password: String,
    pub password_reset_required: bool,
}

pub(crate) fn validate_initial_password(value: &str) -> Result<(), UserError> {
    let has_min_length = value.chars().count() >= 12;
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
