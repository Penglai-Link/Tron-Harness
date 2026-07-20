use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    config::ResourceStorageConfig,
    domain::resource::{DEFAULT_BINARY_STORAGE_PROFILE, ResourceError},
};

use super::super::ObjectStorage;

#[derive(Clone)]
pub struct StorageRouter {
    default_profile: String,
    profiles: BTreeMap<String, Arc<dyn ObjectStorage>>,
    roots: Vec<PathBuf>,
}

impl StorageRouter {
    pub fn new(
        config: &ResourceStorageConfig,
        profiles: BTreeMap<String, Arc<dyn ObjectStorage>>,
    ) -> Result<Self, ResourceError> {
        if config.default_profile.trim().is_empty()
            || !profiles.contains_key(config.default_profile.as_str())
        {
            return Err(ResourceError::StorageProfileNotFound);
        }

        let mut roots = Vec::new();
        for (profile_code, profile) in &config.profiles {
            if !profiles.contains_key(profile_code) {
                return Err(ResourceError::StorageProfileNotFound);
            }
            let root = PathBuf::from(&profile.root)
                .canonicalize()
                .map_err(|_| ResourceError::StorageFailure)?;
            roots.push(root);
        }

        Ok(Self {
            default_profile: config.default_profile.clone(),
            profiles,
            roots,
        })
    }

    pub fn storage_roots(&self) -> Vec<PathBuf> {
        self.roots.clone()
    }

    pub fn has_profile(&self, configured_profile: &str) -> bool {
        self.resolve_profile(configured_profile).is_ok()
    }

    pub fn put_file(
        &self,
        configured_profile: &str,
        object_key: &str,
        source: &Path,
    ) -> Result<(String, u64), ResourceError> {
        let profile = self.resolve_profile(configured_profile)?;
        let bytes = self.storage(profile)?.put_file(object_key, source)?;
        Ok((self.locator(profile, object_key), bytes))
    }

    pub fn put_reader(
        &self,
        configured_profile: &str,
        object_key: &str,
        source: &mut dyn Read,
    ) -> Result<(String, u64), ResourceError> {
        let profile = self.resolve_profile(configured_profile)?;
        let bytes = self.storage(profile)?.put_reader(object_key, source)?;
        Ok((self.locator(profile, object_key), bytes))
    }

    pub fn get_file(&self, locator: &str, target: &Path) -> Result<u64, ResourceError> {
        let (profile, key) = self.parse_locator(locator)?;
        self.storage(&profile)?.get_file(&key, target)
    }

    pub fn get_writer(&self, locator: &str, target: &mut dyn Write) -> Result<u64, ResourceError> {
        let (profile, key) = self.parse_locator(locator)?;
        self.storage(&profile)?.get_writer(&key, target)
    }

    pub fn delete(&self, locator: &str) -> Result<(), ResourceError> {
        let (profile, key) = self.parse_locator(locator)?;
        self.storage(&profile)?.delete(&key)
    }

    pub fn exists(&self, locator: &str) -> Result<bool, ResourceError> {
        let (profile, key) = self.parse_locator(locator)?;
        self.storage(&profile)?.exists(&key)
    }

    fn resolve_profile<'a>(
        &'a self,
        configured_profile: &'a str,
    ) -> Result<&'a str, ResourceError> {
        let profile = if configured_profile == DEFAULT_BINARY_STORAGE_PROFILE {
            self.default_profile.as_str()
        } else {
            configured_profile.trim()
        };
        if profile.is_empty() || !self.profiles.contains_key(profile) {
            return Err(ResourceError::StorageProfileNotFound);
        }
        Ok(profile)
    }

    fn storage(&self, profile: &str) -> Result<&Arc<dyn ObjectStorage>, ResourceError> {
        self.profiles
            .get(profile)
            .ok_or(ResourceError::StorageProfileNotFound)
    }

    fn locator(&self, profile: &str, object_key: &str) -> String {
        format!("local://{profile}/{object_key}")
    }

    fn parse_locator(&self, locator: &str) -> Result<(String, String), ResourceError> {
        let value = locator
            .strip_prefix("local://")
            .ok_or(ResourceError::StorageLocatorInvalid)?;
        let (profile, key) = value
            .split_once('/')
            .ok_or(ResourceError::StorageLocatorInvalid)?;
        if profile.is_empty()
            || key.is_empty()
            || !self.profiles.contains_key(profile)
            || key.starts_with('/')
            || key
                .split('/')
                .any(|component| component == "." || component == "..")
        {
            return Err(ResourceError::StorageLocatorInvalid);
        }
        Ok((profile.to_string(), key.to_string()))
    }
}
