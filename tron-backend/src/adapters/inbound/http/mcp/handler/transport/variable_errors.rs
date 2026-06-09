use crate::domain::variable::VariableError;

pub(super) fn variable_error_code(error: &VariableError) -> &'static str {
    match error {
        VariableError::NotFound => "VARIABLE_NOT_FOUND",
        VariableError::WorkspaceRequired => "VARIABLE_WORKSPACE_REQUIRED",
        VariableError::InvalidWorkspace => "VARIABLE_WORKSPACE_INVALID",
        VariableError::InvalidKey => "VARIABLE_KEY_INVALID",
        VariableError::ValueRequired => "VARIABLE_VALUE_REQUIRED",
        VariableError::DuplicateKey => "VARIABLE_KEY_DUPLICATE",
        VariableError::InheritedMutationDenied => "VARIABLE_INHERITED_MUTATION_INVALID",
        VariableError::EncryptionNotConfigured => "VARIABLE_ENCRYPTION_NOT_CONFIGURED",
        VariableError::EncryptFailed => "VARIABLE_ENCRYPT_FAILED",
        VariableError::DecryptFailed => "VARIABLE_DECRYPT_FAILED",
        VariableError::SensitiveUpgradeValueRequired => "VARIABLE_SENSITIVE_UPGRADE_VALUE_REQUIRED",
        VariableError::SensitiveDowngradeValueRequired => {
            "VARIABLE_SENSITIVE_DOWNGRADE_VALUE_REQUIRED"
        }
        VariableError::SensitiveOverrideRequired => "VARIABLE_SENSITIVE_OVERRIDE_REQUIRED",
        VariableError::StorageFailure => "VARIABLE_STORAGE_FAILED",
    }
}

pub(super) fn variable_jsonrpc_error_code(error: &VariableError) -> i64 {
    match error {
        VariableError::StorageFailure
        | VariableError::EncryptionNotConfigured
        | VariableError::EncryptFailed
        | VariableError::DecryptFailed => -32603,
        _ => -32602,
    }
}

pub(super) fn variable_jsonrpc_error_message(error: &VariableError) -> &'static str {
    match error {
        VariableError::NotFound => "variable was not found",
        VariableError::WorkspaceRequired => "workspaceCode is required",
        VariableError::InvalidWorkspace => "variable workspace must reference an active workspace",
        VariableError::InvalidKey => {
            "variable key must use letters, digits, underscore, hyphen, or dot"
        }
        VariableError::ValueRequired => "variable value is required",
        VariableError::DuplicateKey => "variable key already exists in the current workspace",
        VariableError::InheritedMutationDenied => {
            "override the inherited variable locally before modifying it"
        }
        VariableError::EncryptionNotConfigured => "Sensitive value encryption is not configured",
        VariableError::EncryptFailed => "Sensitive value could not be encrypted",
        VariableError::DecryptFailed => "Sensitive value could not be decrypted",
        VariableError::SensitiveUpgradeValueRequired => {
            "Sensitive upgrade requires a replacement value"
        }
        VariableError::SensitiveDowngradeValueRequired => {
            "Sensitive downgrade requires a replacement value"
        }
        VariableError::SensitiveOverrideRequired => {
            "Sensitive inherited variables require sensitive overrides"
        }
        VariableError::StorageFailure => "variable storage operation failed",
    }
}
