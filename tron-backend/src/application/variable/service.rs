use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Mutex},
};

use crate::{
    application::workspace::WorkspaceService,
    domain::{
        variable::{
            EffectiveVariable, EncryptedVariableValue, PlatformVariable, PlatformVariableDisplay,
            RuntimeVariable, VARIABLE_MASK, Variable, VariableCatalogQuery, VariableError,
            VariableMutationResult, VariableSourceKind, validate_variable_key,
        },
        workspace::{WorkspaceDetail, WorkspaceError, WorkspaceStatus},
    },
    shared::{id::SnowflakeId, time::current_timestamp},
};

mod projection;
mod use_cases;

use self::projection::*;

#[derive(Debug, Clone)]
pub struct CreateVariableCommand {
    pub key: String,
    pub value: String,
    pub description: Option<String>,
    pub is_sensitive: Option<bool>,
    pub value_mode: Option<VariableValueMode>,
}

#[derive(Debug, Clone)]
pub struct UpdateVariableCommand {
    pub value: Option<String>,
    pub description: Option<String>,
    pub is_sensitive: Option<bool>,
    pub value_mode: Option<VariableValueMode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableValueMode {
    Replace,
    Preserve,
}

pub trait VariableCipher: Send + Sync {
    fn encrypt(&self, plaintext: &str, aad: &[u8])
    -> Result<EncryptedVariableValue, VariableError>;
    fn decrypt(
        &self,
        encrypted: &EncryptedVariableValue,
        aad: &[u8],
    ) -> Result<String, VariableError>;
}

pub trait VariableRepository: Send {
    fn list_variables(&mut self) -> Result<Vec<Variable>, VariableError>;
    fn list_platform_variables(&mut self) -> Result<Vec<PlatformVariable>, VariableError>;
    fn next_id(&mut self) -> Result<i64, VariableError>;
    fn create_variable(&mut self, variable: &Variable) -> Result<(), VariableError>;
    fn create_platform_variable(
        &mut self,
        variable: &PlatformVariable,
    ) -> Result<(), VariableError>;
    fn update_variable(&mut self, variable: &Variable) -> Result<(), VariableError>;
    fn update_platform_variable(
        &mut self,
        variable: &PlatformVariable,
    ) -> Result<(), VariableError>;
    fn delete_variable(&mut self, workspace_code: &str, key: &str) -> Result<bool, VariableError>;
    fn delete_platform_variable(&mut self, key: &str) -> Result<bool, VariableError>;
}

#[derive(Clone)]
pub struct VariableService {
    repository: Arc<Mutex<Box<dyn VariableRepository>>>,
    workspaces: WorkspaceService,
    cipher: Arc<dyn VariableCipher>,
}

#[derive(Clone)]
struct LineageEntry {
    code: String,
    label: String,
    name: String,
}

#[derive(Clone)]
struct ResolvedVariable {
    source_kind: VariableSourceKind,
    source_label: String,
    source_workspace_name: String,
    shadowed_label: Option<String>,
    shadowed_workspace_code: Option<String>,
    shadowed_workspace_name: Option<String>,
    variable: Variable,
}

impl VariableService {
    pub fn new(
        repository: impl VariableRepository + 'static,
        workspaces: WorkspaceService,
        cipher: impl VariableCipher + 'static,
    ) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
            workspaces,
            cipher: Arc::new(cipher),
        }
    }

    fn active_workspace_detail(
        &self,
        workspace_code: &str,
    ) -> Result<WorkspaceDetail, VariableError> {
        let code = workspace_code.trim();
        if code.is_empty() {
            return Err(VariableError::WorkspaceRequired);
        }

        let workspace = self
            .workspaces
            .get_workspace(code)
            .map_err(map_workspace_error)?;
        if workspace.status != WorkspaceStatus::Active {
            return Err(VariableError::InvalidWorkspace);
        }

        Ok(workspace)
    }

    fn all_variables(&self) -> Result<Vec<Variable>, VariableError> {
        self.repository
            .lock()
            .expect("variable repository mutex poisoned")
            .list_variables()
    }

    fn all_platform_variables(&self) -> Result<Vec<PlatformVariable>, VariableError> {
        self.repository
            .lock()
            .expect("variable repository mutex poisoned")
            .list_platform_variables()
    }

    fn effective_display_variable(
        &self,
        workspace_code: &str,
        key: &str,
    ) -> Result<EffectiveVariable, VariableError> {
        self.list_variables(workspace_code, VariableCatalogQuery { search: None })?
            .into_iter()
            .find(|record| record.key == key)
            .ok_or(VariableError::NotFound)
    }

    fn inherited_variable_in_lineage(
        &self,
        workspace: &WorkspaceDetail,
        variables: &[Variable],
        key: &str,
    ) -> Option<Variable> {
        let lineage_codes: HashMap<_, _> = workspace
            .path
            .iter()
            .map(|item| (item.workspace_code.as_str(), ()))
            .collect();
        variables
            .iter()
            .find(|variable| {
                variable.key == key
                    && variable.workspace_code != workspace.workspace_code
                    && lineage_codes.contains_key(variable.workspace_code.as_str())
            })
            .cloned()
            .or_else(|| {
                self.all_platform_variables()
                    .ok()?
                    .into_iter()
                    .find(|variable| variable.key == key)
                    .map(platform_to_variable)
            })
    }

    fn key_exists_in_lineage(
        &self,
        workspace: &WorkspaceDetail,
        variables: &[Variable],
        key: &str,
    ) -> bool {
        self.inherited_variable_in_lineage(workspace, variables, key)
            .is_some()
            || variables.iter().any(|variable| {
                variable.key == key && variable.workspace_code == workspace.workspace_code
            })
            || self
                .all_platform_variables()
                .map(|variables| variables.iter().any(|variable| variable.key == key))
                .unwrap_or(false)
    }

    fn resolve_variables(
        &self,
        workspace: &WorkspaceDetail,
    ) -> Result<Vec<EffectiveVariable>, VariableError> {
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

        Ok(resolved_by_key
            .into_values()
            .map(|record| EffectiveVariable {
                key: record.variable.key.clone(),
                value: display_value(&record.variable),
                description: record.variable.description.clone(),
                source_kind: record.source_kind,
                source_workspace_code: record.variable.workspace_code.clone(),
                source_workspace_name: record.source_workspace_name,
                source_label: record.source_label,
                shadowed_workspace_code: record.shadowed_workspace_code,
                shadowed_workspace_name: record.shadowed_workspace_name,
                shadowed_label: record.shadowed_label,
                is_sensitive: record.variable.is_sensitive,
                value_masked: record.variable.is_sensitive,
                has_value: !record.variable.value.is_empty(),
                updated_at: record.variable.updated_at,
            })
            .collect())
    }
}

fn platform_to_variable(variable: PlatformVariable) -> Variable {
    Variable {
        id: variable.id,
        workspace_code: "PLATFORM".to_string(),
        key: variable.key,
        value: variable.value,
        description: variable.description,
        is_sensitive: variable.is_sensitive,
        value_nonce: variable.value_nonce,
        value_tag: variable.value_tag,
        value_algorithm: variable.value_algorithm,
        value_key_version: variable.value_key_version,
        created_at: variable.created_at,
        updated_at: variable.updated_at,
    }
}

fn display_value(variable: &Variable) -> String {
    if variable.is_sensitive {
        VARIABLE_MASK.into()
    } else {
        variable.value.clone()
    }
}

fn display_platform_variable(variable: PlatformVariable) -> PlatformVariableDisplay {
    let has_value = !variable.value.is_empty();
    PlatformVariableDisplay {
        key: variable.key,
        value: if variable.is_sensitive {
            VARIABLE_MASK.into()
        } else {
            variable.value
        },
        description: variable.description,
        is_sensitive: variable.is_sensitive,
        value_masked: variable.is_sensitive,
        has_value,
        updated_at: variable.updated_at,
    }
}
