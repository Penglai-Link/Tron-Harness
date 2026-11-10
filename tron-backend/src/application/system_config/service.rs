use std::{path::Path, sync::Arc};

use super::{
    SystemConfigUpdateResult,
    helpers::{
        changed_leaf_paths, classify_paths, config_items, effective_context_value, leaf_paths,
        parse_yaml, remove_removed_config_sections, set_value_at_path, value_at_path,
    },
    types::{
        SystemConfigChange, SystemConfigDocument, SystemConfigError, SystemConfigService,
        SystemConfigStore,
    },
};

impl SystemConfigService {
    pub fn new(store: Arc<dyn SystemConfigStore>) -> Self {
        Self {
            store,
            runtime_config: None,
            shortcut_defaults: None,
        }
    }

    pub fn with_runtime_config(mut self, runtime: super::BackendRuntimeConfig) -> Self {
        self.runtime_config = Some(runtime);
        self
    }

    pub fn with_shortcut_defaults(mut self, store: Arc<dyn super::ShortcutDefaultsStore>) -> Self {
        self.shortcut_defaults = Some(store);
        self
    }

    pub fn shortcut_defaults(&self) -> Result<std::collections::BTreeMap<String, super::ShortcutBinding>, SystemConfigError> {
        self.shortcut_defaults.as_ref().ok_or(SystemConfigError::ReadFailed)?.load()
    }

    pub fn get(&self) -> Result<SystemConfigDocument, SystemConfigError> {
        let (stored, value) = self.read_sanitized()?;
        let effective = effective_context_value(&stored.content, Path::new(&stored.path), value)?;
        Ok(SystemConfigDocument {
            revision: stored.revision,
            path: stored.path,
            items: config_items(&effective, self.runtime_config.is_some()),
        })
    }

    pub fn variable_crypto_key_base64(&self) -> Result<Option<String>, SystemConfigError> {
        let (_stored, value) = self.read_sanitized()?;
        Ok(value_at_path(&value, "variableCrypto.keyBase64")
            .and_then(serde_yaml::Value::as_str)
            .map(str::to_owned))
    }

    pub fn update(
        &self,
        expected_revision: &str,
        changes: Vec<SystemConfigChange>,
        confirm_migration: bool,
    ) -> Result<SystemConfigUpdateResult, SystemConfigError> {
        let (stored, mut next_value) = self.read_sanitized()?;
        if stored.revision != expected_revision {
            return Err(SystemConfigError::RevisionConflict);
        }
        let path = Path::new(&stored.path);
        let effective = effective_context_value(&stored.content, path, next_value.clone())?;
        let mut available_paths = leaf_paths(&effective)
            .into_iter()
            .filter(|path| !super::helpers::is_removed_config_path(path))
            .collect::<Vec<_>>();
        const SHARES_PATH: &str = "shortTermMemory.context.providers.shares";
        available_paths.push(SHARES_PATH.to_owned());
        let saving_shares = changes.iter().any(|change| change.path == SHARES_PATH);
        let mut selected_paths = std::collections::BTreeSet::new();
        for change in changes {
            if !selected_paths.insert(change.path.clone())
                || !available_paths.contains(&change.path)
            {
                return Err(SystemConfigError::InvalidField(change.path));
            }
            if value_at_path(&effective, &change.path)
                .and_then(|value| serde_json::to_value(value).ok())
                .as_ref()
                != Some(&change.value)
            {
                set_value_at_path(&mut next_value, &change.path, change.value)?;
            }
        }

        if saving_shares {
            if let Some(context) = next_value.get_mut("shortTermMemory").and_then(|memory| memory.get_mut("context")).and_then(serde_yaml::Value::as_mapping_mut) {
                context.remove(serde_yaml::Value::String("blocks".into()));
                if let Some(providers) = context.get_mut(serde_yaml::Value::String("providers".into())).and_then(serde_yaml::Value::as_mapping_mut) {
                    providers.remove(serde_yaml::Value::String("pluginOverrides".into()));
                    if let Some(blocks) = providers.get_mut(serde_yaml::Value::String("dataBlocks".into())).and_then(serde_yaml::Value::as_mapping_mut) {
                        for block in blocks.values_mut().filter_map(serde_yaml::Value::as_mapping_mut) {
                            block.remove(serde_yaml::Value::String("maxTokens".into()));
                        }
                    }
                }
            }
        }
        let next_yaml = serde_yaml::to_string(&next_value)
            .map_err(|error| SystemConfigError::InvalidYaml(error.to_string()))?;
        parse_yaml(&next_yaml, path)?;
        let current_value = parse_yaml(&stored.content, path)?;
        let changed_paths = changed_leaf_paths(&current_value, &next_value);
        let (hot_update_paths, restart_paths, migration_paths) =
            classify_paths(changed_paths.clone(), self.runtime_config.is_some());
        if !confirm_migration && !migration_paths.is_empty() {
            return Err(SystemConfigError::MigrationConfirmationRequired(
                migration_paths,
            ));
        }
        if changed_paths.is_empty() {
            return Ok(SystemConfigUpdateResult {
                revision: stored.revision,
                changed_paths,
                hot_update_paths,
                restart_paths,
                migration_paths,
            });
        }

        let persisted = if let Some(runtime) = &self.runtime_config {
            let next = crate::config::AppConfig::effective_config(&next_yaml, path)
                .map_err(|error| SystemConfigError::InvalidYaml(error.to_string()))?;
            // Serialize persistence and publication so concurrent saves cannot
            // publish an older revision after a newer one. Failed writes leave
            // the runtime untouched. Boot-time resources stay at their active values.
            let mut current = runtime
                .0
                .lock()
                .map_err(|_| SystemConfigError::WriteFailed)?;
            let persisted = self
                .store
                .replace(expected_revision, &next_yaml)
                .map_err(SystemConfigError::from_store)?;
            super::runtime::apply_hot_settings(&mut current, next, &hot_update_paths);
            persisted
        } else {
            self.store
                .replace(expected_revision, &next_yaml)
                .map_err(SystemConfigError::from_store)?
        };
        Ok(SystemConfigUpdateResult {
            revision: persisted.revision,
            changed_paths,
            hot_update_paths,
            restart_paths,
            migration_paths,
        })
    }

    fn read_sanitized(
        &self,
    ) -> Result<(super::StoredSystemConfig, serde_yaml::Value), SystemConfigError> {
        let mut stored = self.store.read().map_err(SystemConfigError::from_store)?;
        let path = Path::new(&stored.path);
        let mut value = parse_yaml(&stored.content, path)?;
        if remove_removed_config_sections(&mut value) {
            let next_yaml = serde_yaml::to_string(&value)
                .map_err(|error| SystemConfigError::InvalidYaml(error.to_string()))?;
            stored = self
                .store
                .replace(&stored.revision, &next_yaml)
                .map_err(SystemConfigError::from_store)?;
            value = parse_yaml(&stored.content, Path::new(&stored.path))?;
        }
        Ok((stored, value))
    }
}
