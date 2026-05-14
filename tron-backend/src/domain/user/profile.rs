use serde::{Deserialize, Serialize};

use super::UserError;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhoneNumber {
    pub country_code: String,
    pub number: String,
}

pub(crate) fn required_profile_field(value: Option<String>) -> Result<String, UserError> {
    let value = value.ok_or(UserError::RequiredProfileField)?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(UserError::RequiredProfileField)
    } else {
        Ok(trimmed.to_owned())
    }
}

pub(crate) fn optional_trimmed(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_owned())
    })
}
