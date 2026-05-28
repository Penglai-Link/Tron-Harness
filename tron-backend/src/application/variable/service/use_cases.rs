use super::*;

impl VariableService {
    pub fn list_variables(
        &self,
        workspace_code: &str,
        query: VariableCatalogQuery,
    ) -> Result<Vec<EffectiveVariable>, VariableError> {
        let workspace = self.active_workspace_detail(workspace_code)?;
        let mut records = self.resolve_variables(&workspace)?;
        if let Some(search) = normalized_search(query.search) {
            records.retain(|record| matches_search(record, &search));
        }
        Ok(records)
    }

    pub fn list_platform_variables(&self) -> Result<Vec<PlatformVariable>, VariableError> {
        self.all_platform_variables()
    }

    pub fn create_platform_variable(
        &self,
        command: CreateVariableCommand,
    ) -> Result<PlatformVariable, VariableError> {
        let key = normalized_key(command.key)?;
        let platform_variables = self.all_platform_variables()?;
        if platform_variables
            .iter()
            .any(|variable| variable.key == key)
        {
            return Err(VariableError::DuplicateKey);
        }

        let id = self
            .repository
            .lock()
            .expect("variable repository mutex poisoned")
            .next_id()?;
        let variable = PlatformVariable {
            id: SnowflakeId::new(id),
            key,
            value: command.value,
            description: optional_trimmed(command.description),
            created_at: current_timestamp(),
            updated_at: current_timestamp(),
        };
        self.repository
            .lock()
            .expect("variable repository mutex poisoned")
            .create_platform_variable(&variable)?;
        Ok(variable)
    }

    pub fn update_platform_variable(
        &self,
        variable_key: &str,
        command: UpdateVariableCommand,
    ) -> Result<PlatformVariable, VariableError> {
        let key = normalized_key(variable_key.to_owned())?;
        let Some(existing) = self
            .all_platform_variables()?
            .into_iter()
            .find(|variable| variable.key == key)
        else {
            return Err(VariableError::NotFound);
        };

        let mut updated = existing;
        updated.value = command.value;
        updated.description = optional_trimmed(command.description);
        updated.updated_at = current_timestamp();

        self.repository
            .lock()
            .expect("variable repository mutex poisoned")
            .update_platform_variable(&updated)?;
        Ok(updated)
    }

    pub fn delete_platform_variable(
        &self,
        variable_key: &str,
    ) -> Result<VariableMutationResult, VariableError> {
        let key = normalized_key(variable_key.to_owned())?;
        let deleted = self
            .repository
            .lock()
            .expect("variable repository mutex poisoned")
            .delete_platform_variable(&key)?;
        if !deleted {
            return Err(VariableError::NotFound);
        }

        Ok(VariableMutationResult {
            key,
            workspace_code: "PLATFORM".to_string(),
        })
    }

    pub fn create_variable(
        &self,
        workspace_code: &str,
        command: CreateVariableCommand,
    ) -> Result<EffectiveVariable, VariableError> {
        let workspace = self.active_workspace_detail(workspace_code)?;
        let key = normalized_key(command.key)?;
        let description = optional_trimmed(command.description);
        let value = command.value;
        let variables = self.all_variables()?;

        if find_local_variable(&variables, &workspace.workspace_code, &key).is_some() {
            return Err(VariableError::DuplicateKey);
        }

        let id = self
            .repository
            .lock()
            .expect("variable repository mutex poisoned")
            .next_id()?;
        let variable = Variable {
            id: SnowflakeId::new(id),
            workspace_code: workspace.workspace_code.clone(),
            key: key.clone(),
            value,
            description,
            created_at: current_timestamp(),
            updated_at: current_timestamp(),
        };
        self.repository
            .lock()
            .expect("variable repository mutex poisoned")
            .create_variable(&variable)?;
        self.effective_variable(&workspace.workspace_code, &key)
    }

    pub fn update_variable(
        &self,
        workspace_code: &str,
        variable_key: &str,
        command: UpdateVariableCommand,
    ) -> Result<EffectiveVariable, VariableError> {
        let workspace = self.active_workspace_detail(workspace_code)?;
        let key = normalized_key(variable_key.to_owned())?;
        let variables = self.all_variables()?;
        let Some(existing) = find_local_variable(&variables, &workspace.workspace_code, &key)
        else {
            return if self.key_exists_in_lineage(&workspace, &variables, &key) {
                Err(VariableError::InheritedMutationDenied)
            } else {
                Err(VariableError::NotFound)
            };
        };

        let mut updated = existing.clone();
        updated.value = command.value;
        updated.description = optional_trimmed(command.description);
        updated.updated_at = current_timestamp();

        self.repository
            .lock()
            .expect("variable repository mutex poisoned")
            .update_variable(&updated)?;
        self.effective_variable(&workspace.workspace_code, &key)
    }

    pub fn delete_variable(
        &self,
        workspace_code: &str,
        variable_key: &str,
    ) -> Result<VariableMutationResult, VariableError> {
        let workspace = self.active_workspace_detail(workspace_code)?;
        let key = normalized_key(variable_key.to_owned())?;
        let variables = self.all_variables()?;
        if find_local_variable(&variables, &workspace.workspace_code, &key).is_none() {
            return if self.key_exists_in_lineage(&workspace, &variables, &key) {
                Err(VariableError::InheritedMutationDenied)
            } else {
                Err(VariableError::NotFound)
            };
        }

        let deleted = self
            .repository
            .lock()
            .expect("variable repository mutex poisoned")
            .delete_variable(&workspace.workspace_code, &key)?;
        if !deleted {
            return Err(VariableError::NotFound);
        }

        Ok(VariableMutationResult {
            key,
            workspace_code: workspace.workspace_code,
        })
    }
}
