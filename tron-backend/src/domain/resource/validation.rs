use super::ResourceError;

pub const MAX_RESOURCE_CODE_LENGTH: usize = 64;
pub const MAX_DEFINITION_CODE_LENGTH: usize = 64;

pub fn validate_resource_code(value: &str) -> Result<(), ResourceError> {
    validate_business_code(value, MAX_RESOURCE_CODE_LENGTH)
}

pub fn validate_definition_code(value: &str) -> Result<(), ResourceError> {
    validate_business_code(value, MAX_DEFINITION_CODE_LENGTH)
}

pub fn validate_version_code(value: &str) -> Result<(), ResourceError> {
    let Some(sequence) = value.strip_prefix('v') else {
        return Err(ResourceError::InvalidVersion);
    };
    let Some((number, suffix)) = sequence.split_once('-') else {
        return Err(ResourceError::InvalidVersion);
    };
    if number.is_empty()
        || number.starts_with('0')
        || !number.chars().all(|value| value.is_ascii_digit())
        || suffix.len() != 3
        || !suffix.chars().all(|value| value.is_ascii_alphanumeric())
    {
        return Err(ResourceError::InvalidVersion);
    }
    Ok(())
}

pub fn validate_binary_name(value: &str, max_length: usize) -> Result<(), ResourceError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > max_length
        || value == "."
        || value == ".."
        || value.contains('/')
        || value.contains('\\')
        || value.chars().any(|character| character.is_control())
    {
        return Err(ResourceError::InvalidInput);
    }
    Ok(())
}

fn validate_business_code(value: &str, max_length: usize) -> Result<(), ResourceError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > max_length
        || !value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
    {
        return Err(ResourceError::InvalidCode);
    }
    Ok(())
}
