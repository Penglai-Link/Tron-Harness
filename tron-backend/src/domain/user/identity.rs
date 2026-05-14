use super::UserError;

pub(crate) fn validate_user_code(user_code: &str) -> Result<(), UserError> {
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

pub(crate) fn normalize_code_segment(value: &str) -> String {
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
