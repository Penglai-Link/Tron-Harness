use super::*;

impl VariableService {
    pub fn get_variable(
        &self,
        workspace_code: &str,
        variable_key: &str,
    ) -> Result<EffectiveVariable, VariableError> {
        let key = normalized_key(variable_key.to_owned())?;
        self.effective_variable(workspace_code, &key)
    }

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

    pub fn list_platform_variables(&self) -> Result<Vec<PlatformVariableDisplay>, VariableError> {
        Ok(self
            .all_platform_variables()?
            .into_iter()
            .map(display_platform_variable)
            .collect())
    }

    pub fn create_platform_variable(
        &self,
        command: CreateVariableCommand,
    ) -> Result<PlatformVariableDisplay, VariableError> {
        let key = normalized_key(command.key)?;
        if self
            .all_platform_variables()?
            .iter()
            .any(|variable| variable.key == key)
        {
            return Err(VariableError::DuplicateKey);
        }

        let now = current_timestamp();
        let value = build_stored_payload(
            self.cipher.as_ref(),
            PlatformVariableIdentity::new(&key).aad().as_bytes(),
            command.value,
            command.is_sensitive.unwrap_or(false),
        )?;
        let id = self
            .repository
            .lock()
            .expect("variable repository mutex poisoned")
            .next_id()?;
        let variable = PlatformVariable {
            id: SnowflakeId::new(id),
            key,
            value: value.value,
            description: optional_trimmed(command.description),
            is_sensitive: value.is_sensitive,
            value_nonce: value.value_nonce,
            value_tag: value.value_tag,
            value_algorithm: value.value_algorithm,
            value_key_version: value.value_key_version,
            created_at: now.clone(),
            updated_at: now,
        };
        self.repository
            .lock()
            .expect("variable repository mutex poisoned")
            .create_platform_variable(&variable)?;
        Ok(display_platform_variable(variable))
    }

    pub fn update_platform_variable(
        &self,
        variable_key: &str,
        command: UpdateVariableCommand,
    ) -> Result<PlatformVariableDisplay, VariableError> {
        let key = normalized_key(variable_key.to_owned())?;
        let Some(existing) = self
            .all_platform_variables()?
            .into_iter()
            .find(|variable| variable.key == key)
        else {
            return Err(VariableError::NotFound);
        };

        let updated_payload = updated_platform_payload(self.cipher.as_ref(), &existing, &command)?;
        let mut updated = existing;
        updated.value = updated_payload.value;
        updated.is_sensitive = updated_payload.is_sensitive;
        updated.value_nonce = updated_payload.value_nonce;
        updated.value_tag = updated_payload.value_tag;
        updated.value_algorithm = updated_payload.value_algorithm;
        updated.value_key_version = updated_payload.value_key_version;
        updated.description = optional_trimmed(command.description);
        updated.updated_at = current_timestamp();

        self.repository
            .lock()
            .expect("variable repository mutex poisoned")
            .update_platform_variable(&updated)?;
        Ok(display_platform_variable(updated))
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
        let variables = self.all_variables()?;

        if find_local_variable(&variables, &workspace.workspace_code, &key).is_some() {
            return Err(VariableError::DuplicateKey);
        }
        let inherited = self.inherited_variable_in_lineage(&workspace, &variables, &key);
        let requested_sensitive = command.is_sensitive.unwrap_or(false);
        if inherited
            .as_ref()
            .is_some_and(|variable| variable.is_sensitive && !requested_sensitive)
        {
            return Err(VariableError::SensitiveOverrideRequired);
        }

        let now = current_timestamp();
        let value = build_stored_payload(
            self.cipher.as_ref(),
            WorkspaceVariableIdentity::new(&workspace.workspace_code, &key)
                .aad()
                .as_bytes(),
            command.value,
            requested_sensitive,
        )?;
        let id = self
            .repository
            .lock()
            .expect("variable repository mutex poisoned")
            .next_id()?;
        let variable = Variable {
            id: SnowflakeId::new(id),
            workspace_code: workspace.workspace_code.clone(),
            key: key.clone(),
            value: value.value,
            description,
            is_sensitive: value.is_sensitive,
            value_nonce: value.value_nonce,
            value_tag: value.value_tag,
            value_algorithm: value.value_algorithm,
            value_key_version: value.value_key_version,
            created_at: now.clone(),
            updated_at: now,
        };
        self.repository
            .lock()
            .expect("variable repository mutex poisoned")
            .create_variable(&variable)?;
        self.effective_display_variable(&workspace.workspace_code, &key)
    }

    pub fn upsert_variable(
        &self,
        workspace_code: &str,
        command: CreateVariableCommand,
    ) -> Result<EffectiveVariable, VariableError> {
        let workspace = self.active_workspace_detail(workspace_code)?;
        let key = normalized_key(command.key)?;
        let current = self
            .list_variables(
                &workspace.workspace_code,
                VariableCatalogQuery { search: None },
            )?
            .into_iter()
            .find(|record| record.key == key);

        match current {
            Some(record) if record.source_workspace_code == workspace.workspace_code => self
                .update_variable(
                    &workspace.workspace_code,
                    &key,
                    UpdateVariableCommand {
                        value: command.value,
                        description: command.description,
                    },
                ),
            _ => self.create_variable(
                &workspace.workspace_code,
                CreateVariableCommand {
                    key,
                    value: command.value,
                    description: command.description,
                },
            ),
        }
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

        let updated_payload = updated_workspace_payload(
            self.cipher.as_ref(),
            &existing,
            &workspace.workspace_code,
            &command,
        )?;
        let mut updated = existing.clone();
        updated.value = updated_payload.value;
        updated.is_sensitive = updated_payload.is_sensitive;
        updated.value_nonce = updated_payload.value_nonce;
        updated.value_tag = updated_payload.value_tag;
        updated.value_algorithm = updated_payload.value_algorithm;
        updated.value_key_version = updated_payload.value_key_version;
        updated.description = optional_trimmed(command.description);
        updated.updated_at = current_timestamp();

        self.repository
            .lock()
            .expect("variable repository mutex poisoned")
            .update_variable(&updated)?;
        self.effective_display_variable(&workspace.workspace_code, &key)
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

    pub fn reveal_platform_variable(&self, variable_key: &str) -> Result<String, VariableError> {
        let key = normalized_key(variable_key.to_owned())?;
        let variable = self
            .all_platform_variables()?
            .into_iter()
            .find(|variable| variable.key == key)
            .ok_or(VariableError::NotFound)?;
        reveal_stored_value(
            self.cipher.as_ref(),
            &platform_to_variable(variable),
            PlatformVariableIdentity::new(&key).aad().as_bytes(),
        )
    }

    pub fn reveal_workspace_variable(
        &self,
        workspace_code: &str,
        variable_key: &str,
    ) -> Result<String, VariableError> {
        let workspace = self.active_workspace_detail(workspace_code)?;
        let key = normalized_key(variable_key.to_owned())?;
        let variable = self
            .resolve_raw_variables(&workspace)?
            .into_iter()
            .find(|record| record.variable.key == key)
            .map(|record| record.variable)
            .ok_or(VariableError::NotFound)?;
        let aad = if variable.workspace_code == "PLATFORM" {
            PlatformVariableIdentity::new(&key).aad()
        } else {
            WorkspaceVariableIdentity::new(&variable.workspace_code, &key).aad()
        };
        reveal_stored_value(self.cipher.as_ref(), &variable, aad.as_bytes())
    }

    pub fn runtime_variables(
        &self,
        workspace_code: &str,
    ) -> Result<Vec<RuntimeVariable>, VariableError> {
        let workspace = self.active_workspace_detail(workspace_code)?;
        self.resolve_raw_variables(&workspace)?
            .into_iter()
            .map(|record| {
                let aad = if record.variable.workspace_code == "PLATFORM" {
                    PlatformVariableIdentity::new(&record.variable.key).aad()
                } else {
                    WorkspaceVariableIdentity::new(
                        &record.variable.workspace_code,
                        &record.variable.key,
                    )
                    .aad()
                };
                let value =
                    reveal_stored_value(self.cipher.as_ref(), &record.variable, aad.as_bytes())?;
                Ok(RuntimeVariable {
                    key: record.variable.key,
                    value,
                    source: record.source_label,
                    is_sensitive: record.variable.is_sensitive,
                })
            })
            .collect()
    }

    fn resolve_raw_variables(
        &self,
        workspace: &WorkspaceDetail,
    ) -> Result<Vec<ResolvedVariable>, VariableError> {
        let current_workspace_code = workspace.workspace_code.clone();
        let lineage = workspace_lineage(workspace);
        let lineage_map: HashMap<_, _> = lineage
            .iter()
            .map(|entry| (entry.code.as_str(), entry.clone()))
            .collect();
        let mut variables_by_workspace: BTreeMap<String, Vec<Variable>> = BTreeMap::new();
        for variable in self.all_variables()? {
            if lineage_map.contains_key(variable.workspace_code.as_str()) {
                variables_by_workspace
                    .entry(variable.workspace_code.clone())
                    .or_default()
                    .push(variable);
            }
        }

        let mut resolved_by_key = BTreeMap::<String, ResolvedVariable>::new();
        for variable in self.all_platform_variables()? {
            resolved_by_key.insert(
                variable.key.clone(),
                ResolvedVariable {
                    source_kind: VariableSourceKind::Inherited,
                    source_label: "Platform Configuration".to_string(),
                    source_workspace_name: "Platform Configuration".to_string(),
                    shadowed_label: None,
                    shadowed_workspace_code: None,
                    shadowed_workspace_name: None,
                    variable: platform_to_variable(variable),
                },
            );
        }
        for entry in &lineage {
            let Some(workspace_variables) = variables_by_workspace.get_mut(&entry.code) else {
                continue;
            };
            workspace_variables.sort_by(|left, right| left.key.cmp(&right.key));
            for variable in workspace_variables.iter().cloned() {
                let previous = resolved_by_key.remove(&variable.key);
                let source_kind = match previous {
                    Some(_) if entry.code == current_workspace_code => VariableSourceKind::Override,
                    Some(_) => VariableSourceKind::Inherited,
                    None if entry.code == current_workspace_code => VariableSourceKind::Local,
                    None => VariableSourceKind::Inherited,
                };
                let (shadowed_workspace_code, shadowed_workspace_name, shadowed_label) =
                    if entry.code == current_workspace_code {
                        match previous.as_ref() {
                            Some(previous) => (
                                Some(previous.variable.workspace_code.clone()),
                                Some(previous.source_workspace_name.clone()),
                                Some(previous.source_label.clone()),
                            ),
                            None => (None, None, None),
                        }
                    } else {
                        (None, None, None)
                    };

                resolved_by_key.insert(
                    variable.key.clone(),
                    ResolvedVariable {
                        source_kind,
                        source_label: entry.label.clone(),
                        source_workspace_name: entry.name.clone(),
                        shadowed_label,
                        shadowed_workspace_code,
                        shadowed_workspace_name,
                        variable,
                    },
                );
            }
        }

        Ok(resolved_by_key.into_values().collect())
    }
}

struct StoredPayload {
    value: String,
    is_sensitive: bool,
    value_nonce: Option<String>,
    value_tag: Option<String>,
    value_algorithm: Option<String>,
    value_key_version: Option<String>,
}

struct PlatformVariableIdentity<'a> {
    key: &'a str,
}

impl<'a> PlatformVariableIdentity<'a> {
    fn new(key: &'a str) -> Self {
        Self { key }
    }

    fn aad(&self) -> String {
        format!("platform:{}", self.key)
    }
}

struct WorkspaceVariableIdentity<'a> {
    workspace_code: &'a str,
    key: &'a str,
}

impl<'a> WorkspaceVariableIdentity<'a> {
    fn new(workspace_code: &'a str, key: &'a str) -> Self {
        Self {
            workspace_code,
            key,
        }
    }

    fn aad(&self) -> String {
        format!("workspace:{}:{}", self.workspace_code, self.key)
    }
}

fn build_stored_payload(
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

fn updated_platform_payload(
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

fn updated_workspace_payload(
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

fn reveal_stored_value(
    cipher: &dyn VariableCipher,
    variable: &Variable,
    aad: &[u8],
) -> Result<String, VariableError> {
    if !variable.is_sensitive {
        return Ok(variable.value.clone());
    }
    cipher.decrypt(&encrypted_value(variable)?, aad)
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
