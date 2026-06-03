use serde::Deserialize;

const DEFAULT_VARIABLE_CRYPTO_KEY_VERSION: &str = "test-v1";
const DEFAULT_VARIABLE_CRYPTO_KEY_BASE64: &str = "EBESExQVFhcYGRobHB0eHw==";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VariableCryptoConfig {
    pub active_key_version: Option<String>,
    pub keys: Vec<VariableCryptoKeyConfig>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VariableCryptoKeyConfig {
    pub version: String,
    pub key_base64: String,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(super) struct VariableCryptoFileConfig {
    active_key_version: Option<String>,
    keys: Option<Vec<VariableCryptoKeyFileConfig>>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct VariableCryptoKeyFileConfig {
    version: Option<String>,
    key_base64: Option<String>,
}

impl Default for VariableCryptoConfig {
    fn default() -> Self {
        Self {
            active_key_version: None,
            keys: Vec::new(),
        }
    }
}

impl VariableCryptoConfig {
    pub fn test_default() -> Self {
        Self {
            active_key_version: Some(DEFAULT_VARIABLE_CRYPTO_KEY_VERSION.into()),
            keys: vec![VariableCryptoKeyConfig {
                version: DEFAULT_VARIABLE_CRYPTO_KEY_VERSION.into(),
                key_base64: DEFAULT_VARIABLE_CRYPTO_KEY_BASE64.into(),
            }],
        }
    }

    pub(super) fn apply_overrides(&mut self, overrides: VariableCryptoFileConfig) {
        if let Some(value) = overrides.active_key_version {
            self.active_key_version = normalize_config_string(value);
        }
        if let Some(keys) = overrides.keys {
            self.keys = keys
                .into_iter()
                .filter_map(|key| {
                    Some(VariableCryptoKeyConfig {
                        version: normalize_config_string(key.version?)?,
                        key_base64: normalize_config_string(key.key_base64?)?,
                    })
                })
                .collect();
        }
    }
}

fn normalize_config_string(value: String) -> Option<String> {
    let trimmed = value.trim().to_string();
    (!trimmed.is_empty()).then_some(trimmed)
}
