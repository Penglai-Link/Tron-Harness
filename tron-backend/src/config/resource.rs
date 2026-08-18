use std::collections::BTreeMap;

use super::{ConfigError, paths::normalize_storage_path};

pub const DEFAULT_RESOURCE_STORAGE_PROFILE: &str = "local-default";
pub const DEFAULT_RESOURCE_SCHEMA_MAX_BYTES: usize = 1_048_576;
pub const DEFAULT_RESOURCE_ATTRIBUTES_MAX_BYTES: usize = 1_048_576;
pub const DEFAULT_RESOURCE_MAX_DEPTH: usize = 128;
pub const DEFAULT_RESOURCE_MAX_VALIDATION_ERRORS: usize = 100;
pub const DEFAULT_RESOURCE_MAX_BINARIES: usize = 500;
pub const DEFAULT_RESOURCE_MAX_BINARY_BYTES: u64 = 1_073_741_824;
pub const DEFAULT_RESOURCE_MAX_COMMIT_BYTES: u64 = 1_073_741_824;
pub const DEFAULT_RESOURCE_MAX_BINARY_NAME_LENGTH: usize = 255;
pub const DEFAULT_RESOURCE_CONNECTION_VERIFICATION_TIMEOUT_MS: u64 = 5_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceConfig {
    pub storage: ResourceStorageConfig,
    pub limits: ResourceLimits,
    pub connection_verification_timeout_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceStorageConfig {
    pub default_profile: String,
    pub profiles: BTreeMap<String, LocalResourceStorageProfile>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalResourceStorageProfile {
    pub root: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceLimits {
    pub schema_max_bytes: usize,
    pub attributes_max_bytes: usize,
    pub max_depth: usize,
    pub max_validation_errors: usize,
    pub max_binaries: usize,
    pub max_binary_bytes: u64,
    pub max_commit_bytes: u64,
    pub max_binary_name_length: usize,
}

impl ResourceConfig {
    pub fn with_default_root(root: String) -> Self {
        let mut profiles = BTreeMap::new();
        profiles.insert(
            DEFAULT_RESOURCE_STORAGE_PROFILE.to_string(),
            LocalResourceStorageProfile { root },
        );
        Self {
            storage: ResourceStorageConfig {
                default_profile: DEFAULT_RESOURCE_STORAGE_PROFILE.to_string(),
                profiles,
            },
            limits: ResourceLimits::default(),
            connection_verification_timeout_ms: DEFAULT_RESOURCE_CONNECTION_VERIFICATION_TIMEOUT_MS,
        }
    }

    pub fn test_default() -> Self {
        Self::with_default_root(
            std::env::temp_dir()
                .join("tron-resource-test")
                .display()
                .to_string(),
        )
    }

    pub(crate) fn apply_overrides(
        &mut self,
        overrides: ResourceFileConfig,
    ) -> Result<(), ConfigError> {
        if let Some(storage) = overrides.storage {
            self.storage.apply_overrides(storage)?;
        }
        if let Some(limits) = overrides.limits {
            self.limits.apply_overrides(limits)?;
        }
        if let Some(timeout) = overrides.connection_verification_timeout_ms {
            self.connection_verification_timeout_ms = timeout;
        }
        self.validate()
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if self.storage.default_profile.trim().is_empty() {
            return Err(invalid_config(
                "resource.storage.defaultProfile is required",
            ));
        }
        if !self
            .storage
            .profiles
            .contains_key(self.storage.default_profile.as_str())
        {
            return Err(invalid_config(
                "resource.storage.defaultProfile must reference a configured local profile",
            ));
        }
        if self
            .storage
            .profiles
            .values()
            .any(|profile| profile.root.trim().is_empty())
        {
            return Err(invalid_config("resource storage profile root is required"));
        }
        if self.limits.schema_max_bytes == 0
            || self.limits.attributes_max_bytes == 0
            || self.limits.max_depth == 0
            || self.limits.max_validation_errors == 0
            || self.limits.max_binaries == 0
            || self.limits.max_binary_bytes == 0
            || self.limits.max_commit_bytes == 0
            || self.limits.max_binary_name_length == 0
        {
            return Err(invalid_config("resource limits must be greater than zero"));
        }
        if self.connection_verification_timeout_ms == 0 {
            return Err(invalid_config(
                "resource.connectionVerificationTimeoutMs must be greater than zero",
            ));
        }
        Ok(())
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            schema_max_bytes: DEFAULT_RESOURCE_SCHEMA_MAX_BYTES,
            attributes_max_bytes: DEFAULT_RESOURCE_ATTRIBUTES_MAX_BYTES,
            max_depth: DEFAULT_RESOURCE_MAX_DEPTH,
            max_validation_errors: DEFAULT_RESOURCE_MAX_VALIDATION_ERRORS,
            max_binaries: DEFAULT_RESOURCE_MAX_BINARIES,
            max_binary_bytes: DEFAULT_RESOURCE_MAX_BINARY_BYTES,
            max_commit_bytes: DEFAULT_RESOURCE_MAX_COMMIT_BYTES,
            max_binary_name_length: DEFAULT_RESOURCE_MAX_BINARY_NAME_LENGTH,
        }
    }
}

impl ResourceStorageConfig {
    fn apply_overrides(&mut self, overrides: ResourceStorageFileConfig) -> Result<(), ConfigError> {
        if let Some(default_profile) = overrides.default_profile {
            self.default_profile = default_profile.trim().to_string();
        }
        if let Some(profiles) = overrides.profiles {
            let mut parsed = BTreeMap::new();
            for (code, profile) in profiles {
                let code = code.trim().to_string();
                if code.is_empty() {
                    return Err(invalid_config("resource storage profile code is required"));
                }
                if profile.storage_type.as_deref() != Some("local") {
                    return Err(invalid_config(
                        "only local resource storage profiles are supported in the first delivery",
                    ));
                }
                let root = profile
                    .root
                    .ok_or_else(|| invalid_config("resource storage profile root is required"))?;
                parsed.insert(
                    code,
                    LocalResourceStorageProfile {
                        root: normalize_storage_path(root),
                    },
                );
            }
            self.profiles = parsed;
        }
        Ok(())
    }
}

impl ResourceLimits {
    fn apply_overrides(&mut self, overrides: ResourceLimitsFileConfig) -> Result<(), ConfigError> {
        if let Some(value) = overrides.schema_max_bytes {
            self.schema_max_bytes = value;
        }
        if let Some(value) = overrides.attributes_max_bytes {
            self.attributes_max_bytes = value;
        }
        if let Some(value) = overrides.max_depth {
            self.max_depth = value;
        }
        if let Some(value) = overrides.max_validation_errors {
            self.max_validation_errors = value;
        }
        if let Some(value) = overrides.max_binaries {
            self.max_binaries = value;
        }
        if let Some(value) = overrides.max_binary_bytes {
            self.max_binary_bytes = value;
        }
        if let Some(value) = overrides.max_commit_bytes {
            self.max_commit_bytes = value;
        }
        if let Some(value) = overrides.max_binary_name_length {
            self.max_binary_name_length = value;
        }
        Ok(())
    }
}

fn invalid_config(message: impl Into<String>) -> ConfigError {
    ConfigError::InvalidResourceConfig {
        message: message.into(),
    }
}

#[derive(Debug, serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResourceFileConfig {
    storage: Option<ResourceStorageFileConfig>,
    limits: Option<ResourceLimitsFileConfig>,
    connection_verification_timeout_ms: Option<u64>,
}

#[derive(Debug, serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ResourceStorageFileConfig {
    default_profile: Option<String>,
    profiles: Option<BTreeMap<String, ResourceStorageProfileFileConfig>>,
}

#[derive(Debug, serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ResourceStorageProfileFileConfig {
    #[serde(rename = "type")]
    storage_type: Option<String>,
    root: Option<String>,
}

#[derive(Debug, serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ResourceLimitsFileConfig {
    schema_max_bytes: Option<usize>,
    attributes_max_bytes: Option<usize>,
    max_depth: Option<usize>,
    max_validation_errors: Option<usize>,
    max_binaries: Option<usize>,
    max_binary_bytes: Option<u64>,
    max_commit_bytes: Option<u64>,
    max_binary_name_length: Option<usize>,
}
