use serde::Deserialize;

const DEFAULT_VARIABLE_CRYPTO_KEY_BASE64: &str = "EBESExQVFhcYGRobHB0eHw==";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VariableCryptoConfig {
    pub key_base64: String,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(super) struct VariableCryptoFileConfig {
    key_base64: Option<String>,
}

impl Default for VariableCryptoConfig {
    fn default() -> Self {
        Self {
            key_base64: String::new(),
        }
    }
}

impl VariableCryptoConfig {
    pub fn test_default() -> Self {
        Self {
            key_base64: DEFAULT_VARIABLE_CRYPTO_KEY_BASE64.into(),
        }
    }

    pub(super) fn apply_overrides(&mut self, overrides: VariableCryptoFileConfig) {
        if let Some(value) = overrides.key_base64 {
            self.key_base64 = normalize_config_string(value).unwrap_or_default();
        }
    }
}

fn normalize_config_string(value: String) -> Option<String> {
    let trimmed = value.trim().to_string();
    (!trimmed.is_empty()).then_some(trimmed)
}
