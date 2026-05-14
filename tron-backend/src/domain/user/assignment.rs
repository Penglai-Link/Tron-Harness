use super::UserError;

pub(crate) fn validate_role_codes(role_codes: &[String]) -> Result<(), UserError> {
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

pub(crate) fn validate_space_codes(space_codes: &[String]) -> Result<(), UserError> {
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

pub(crate) fn unique_preserving_order(values: Vec<String>) -> Vec<String> {
    values.into_iter().fold(Vec::new(), |mut acc, value| {
        if !acc.contains(&value) {
            acc.push(value);
        }
        acc
    })
}
