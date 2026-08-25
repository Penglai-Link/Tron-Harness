use std::{path::Path, sync::Arc};

use super::{
    helpers::{
        changed_leaf_paths, classify_paths, config_items, leaf_paths, parse_yaml, set_value_at_path,
    },
    types::{
        SystemConfigChange, SystemConfigDocument, SystemConfigError, SystemConfigService,
        SystemConfigStore,
    },
    SystemConfigUpdateResult,
};

impl SystemConfigService {
    pub fn new(store: Arc<dyn SystemConfigStore>) -> Self {
        Self { store }
    }

    pub fn get(&self) -> Result<SystemConfigDocument, SystemConfigError> {
        let stored = self.store.read().map_err(SystemConfigError::from_store)?;
        let value = parse_yaml(&stored.content, Path::new(&stored.path))?;
        Ok(SystemConfigDocument {
            revision: stored.revision,
            path: stored.path,
            items: config_items(&value),
        })
    }

    pub fn update(
        &self,
        expected_revision: &str,
        changes: Vec<SystemConfigChange>,
        confirm_migration: bool,
    ) -> Result<SystemConfigUpdateResult, SystemConfigError> {
        let stored = self.store.read().map_err(SystemConfigError::from_store)?;
        if stored.revision != expected_revision {
            return Err(SystemConfigError::RevisionConflict);
        }
        let path = Path::new(&stored.path);
        let mut next_value = parse_yaml(&stored.content, path)?;
        let available_paths = leaf_paths(&next_value);
        for change in changes {
            if !available_paths.contains(&change.path) {
                return Err(SystemConfigError::InvalidField(change.path));
            }
            set_value_at_path(&mut next_value, &change.path, change.value)?;
        }

        let next_yaml = serde_yaml::to_string(&next_value)
            .map_err(|error| SystemConfigError::InvalidYaml(error.to_string()))?;
        parse_yaml(&next_yaml, path)?;
        let current_value = parse_yaml(&stored.content, path)?;
        let changed_paths = changed_leaf_paths(&current_value, &next_value);
        let (hot_update_paths, restart_paths, migration_paths) =
            classify_paths(changed_paths.clone());
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

        let persisted = self
            .store
            .replace(expected_revision, &next_yaml)
            .map_err(SystemConfigError::from_store)?;
        Ok(SystemConfigUpdateResult {
            revision: persisted.revision,
            changed_paths,
            hot_update_paths,
            restart_paths,
            migration_paths,
        })
    }
}
