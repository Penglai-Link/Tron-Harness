use super::ResourceError;

pub const MAX_RESOURCE_CODE_LENGTH: usize = 64;
pub const MAX_BINARY_NAME_LENGTH: usize = 255;

pub fn validate_resource_code(value: &str) -> Result<(), ResourceError> {
    validate_business_code(value, MAX_RESOURCE_CODE_LENGTH)
}

pub fn validate_binary_name(value: &str, max_length: usize) -> Result<(), ResourceError> {
    if value != value.trim()
        || value.is_empty()
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
    if value != value.trim()
        || value.is_empty()
        || value.len() > max_length
        || !value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
    {
        return Err(ResourceError::InvalidCode);
    }
    Ok(())
}
