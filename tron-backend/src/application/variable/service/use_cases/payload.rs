use super::*;

pub(super) struct StoredPayload {
    pub(super) value: String,
    pub(super) is_sensitive: bool,
    pub(super) value_nonce: Option<String>,
    pub(super) value_tag: Option<String>,
    pub(super) value_algorithm: Option<String>,
    pub(super) value_key_version: Option<String>,
}

pub(super) struct PlatformVariableIdentity<'a> {
    key: &'a str,
}

impl<'a> PlatformVariableIdentity<'a> {
    pub(super) fn new(key: &'a str) -> Self {
        Self { key }
    }

    pub(super) fn aad(&self) -> String {
        format!("platform:{}", self.key)
    }
}

pub(super) struct WorkspaceVariableIdentity<'a> {
    workspace_code: &'a str,
    key: &'a str,
}

impl<'a> WorkspaceVariableIdentity<'a> {
    pub(super) fn new(workspace_code: &'a str, key: &'a str) -> Self {
        Self {
            workspace_code,
            key,
        }
    }

    pub(super) fn aad(&self) -> String {
        format!("workspace:{}:{}", self.workspace_code, self.key)
    }
}

pub(super) fn build_stored_payload(
    cipher: &dyn VariableCipher,
    aad: &[u8],
    value: String,
    is_sensitive: bool,
) -> Result<StoredPayload, VariableError> {
    if !is_sensitive {
        return Ok(non_sensitive_payload(value));
    }
    sensitive_payload(cipher.encrypt(&value, aad)?)
}

pub(super) fn updated_platform_payload(
    cipher: &dyn VariableCipher,
    existing: &PlatformVariable,
    command: &UpdateVariableCommand,
) -> Result<StoredPayload, VariableError> {
    let target_sensitive = command.is_sensitive.unwrap_or(existing.is_sensitive);
    let mode = command.value_mode.unwrap_or(VariableValueMode::Replace);
    let aad = PlatformVariableIdentity::new(&existing.key).aad();
    updated_payload(
        cipher,
        &platform_to_variable(existing.clone()),
        aad.as_bytes(),
        command.value.as_deref(),
        mode,
        target_sensitive,
    )
}

pub(super) fn updated_workspace_payload(
    cipher: &dyn VariableCipher,
    existing: &Variable,
    workspace_code: &str,
    command: &UpdateVariableCommand,
) -> Result<StoredPayload, VariableError> {
    let target_sensitive = command.is_sensitive.unwrap_or(existing.is_sensitive);
    let mode = command.value_mode.unwrap_or(VariableValueMode::Replace);
    let aad = WorkspaceVariableIdentity::new(workspace_code, &existing.key).aad();
    updated_payload(
        cipher,
        existing,
        aad.as_bytes(),
        command.value.as_deref(),
        mode,
        target_sensitive,
    )
}

pub(super) fn reveal_stored_value(
    cipher: &dyn VariableCipher,
    variable: &Variable,
    aad: &[u8],
) -> Result<String, VariableError> {
    if !variable.is_sensitive {
        return Ok(variable.value.clone());
    }
    cipher.decrypt(&encrypted_value(variable)?, aad)
}

fn updated_payload(
    cipher: &dyn VariableCipher,
    existing: &Variable,
    aad: &[u8],
    value: Option<&str>,
    mode: VariableValueMode,
    target_sensitive: bool,
) -> Result<StoredPayload, VariableError> {
    match mode {
        VariableValueMode::Preserve if existing.is_sensitive && !target_sensitive => {
            Err(VariableError::SensitiveDowngradeValueRequired)
        }
        VariableValueMode::Preserve if !existing.is_sensitive && target_sensitive => {
            Err(VariableError::SensitiveUpgradeValueRequired)
        }
        VariableValueMode::Preserve => Ok(copy_payload(existing, target_sensitive)),
        VariableValueMode::Replace => {
            let value = value.ok_or_else(|| {
                if !existing.is_sensitive && target_sensitive {
                    VariableError::SensitiveUpgradeValueRequired
                } else if existing.is_sensitive && !target_sensitive {
                    VariableError::SensitiveDowngradeValueRequired
                } else {
                    VariableError::ValueRequired
                }
            })?;
            build_stored_payload(cipher, aad, value.to_string(), target_sensitive)
        }
    }
}

fn encrypted_value(variable: &Variable) -> Result<EncryptedVariableValue, VariableError> {
    Ok(EncryptedVariableValue {
        ciphertext: variable.value.clone(),
        nonce: variable
            .value_nonce
            .clone()
            .ok_or(VariableError::DecryptFailed)?,
        tag: variable
            .value_tag
            .clone()
            .ok_or(VariableError::DecryptFailed)?,
        algorithm: variable
            .value_algorithm
            .clone()
            .ok_or(VariableError::DecryptFailed)?,
        key_version: variable
            .value_key_version
            .clone()
            .ok_or(VariableError::DecryptFailed)?,
    })
}

fn sensitive_payload(encrypted: EncryptedVariableValue) -> Result<StoredPayload, VariableError> {
    Ok(StoredPayload {
        value: encrypted.ciphertext,
        is_sensitive: true,
        value_nonce: Some(encrypted.nonce),
        value_tag: Some(encrypted.tag),
        value_algorithm: Some(encrypted.algorithm),
        value_key_version: Some(encrypted.key_version),
    })
}

fn non_sensitive_payload(value: String) -> StoredPayload {
    StoredPayload {
        value,
        is_sensitive: false,
        value_nonce: None,
        value_tag: None,
        value_algorithm: None,
        value_key_version: None,
    }
}

fn copy_payload(existing: &Variable, target_sensitive: bool) -> StoredPayload {
    if !target_sensitive {
        return non_sensitive_payload(existing.value.clone());
    }
    StoredPayload {
        value: existing.value.clone(),
        is_sensitive: true,
        value_nonce: existing.value_nonce.clone(),
        value_tag: existing.value_tag.clone(),
        value_algorithm: existing.value_algorithm.clone(),
        value_key_version: existing.value_key_version.clone(),
    }
}
