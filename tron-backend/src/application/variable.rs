use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Mutex},
};

use crate::{
    application::workspace::WorkspaceService,
    domain::{
        variable::{
            EffectiveVariable, Variable, VariableCatalogQuery, VariableError,
            VariableMutationResult, VariableSourceKind, validate_variable_key,
        },
        workspace::{WorkspaceDetail, WorkspaceError, WorkspaceStatus},
    },
    shared::{id::SnowflakeId, time::current_timestamp},
};

#[derive(Debug, Clone)]
pub struct CreateVariableCommand {
    pub key: String,
    pub value: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateVariableCommand {
    pub value: String,
    pub description: Option<String>,
}

pub trait VariableRepository: Send {
    fn list_variables(&mut self) -> Result<Vec<Variable>, VariableError>;
    fn next_id(&mut self) -> Result<i64, VariableError>;
    fn create_variable(&mut self, variable: &Variable) -> Result<(), VariableError>;
    fn update_variable(&mut self, variable: &Variable) -> Result<(), VariableError>;
    fn delete_variable(&mut self, workspace_code: &str, key: &str) -> Result<bool, VariableError>;
}

#[derive(Clone)]
pub struct VariableService {
    repository: Arc<Mutex<Box<dyn VariableRepository>>>,
    workspaces: WorkspaceService,
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
    ) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
            workspaces,
        }
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

    fn effective_variable(
        &self,
        workspace_code: &str,
        key: &str,
    ) -> Result<EffectiveVariable, VariableError> {
        self.list_variables(workspace_code, VariableCatalogQuery { search: None })?
            .into_iter()
            .find(|record| record.key == key)
            .ok_or(VariableError::NotFound)
    }

    fn key_exists_in_lineage(
        &self,
        workspace: &WorkspaceDetail,
        variables: &[Variable],
        key: &str,
    ) -> bool {
        let lineage_codes: HashMap<_, _> = workspace
            .lineage
            .iter()
            .map(|item| (item.workspace_code.as_str(), ()))
            .collect();
        variables.iter().any(|variable| {
            variable.key == key && lineage_codes.contains_key(variable.workspace_code.as_str())
        })
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
                key: record.variable.key,
                value: record.variable.value,
                description: record.variable.description,
                source_kind: record.source_kind,
                source_workspace_code: record.variable.workspace_code,
                source_workspace_name: record.source_workspace_name,
                source_label: record.source_label,
                shadowed_workspace_code: record.shadowed_workspace_code,
                shadowed_workspace_name: record.shadowed_workspace_name,
                shadowed_label: record.shadowed_label,
                updated_at: record.variable.updated_at,
            })
            .collect())
    }
}

fn find_local_variable(
    variables: &[Variable],
    workspace_code: &str,
    key: &str,
) -> Option<Variable> {
    variables
        .iter()
        .find(|variable| variable.workspace_code == workspace_code && variable.key == key)
        .cloned()
}

fn map_workspace_error(error: WorkspaceError) -> VariableError {
    match error {
        WorkspaceError::StorageFailure => VariableError::StorageFailure,
        WorkspaceError::NotFound => VariableError::InvalidWorkspace,
        _ => VariableError::InvalidWorkspace,
    }
}

fn matches_search(record: &EffectiveVariable, search: &str) -> bool {
    [
        record.key.as_str(),
        record.value.as_str(),
        record.description.as_deref().unwrap_or(""),
        record.source_workspace_code.as_str(),
        record.source_workspace_name.as_str(),
        record.source_label.as_str(),
    ]
    .join(" ")
    .to_lowercase()
    .contains(search)
}

fn normalized_key(value: String) -> Result<String, VariableError> {
    validate_variable_key(&value)?;
    Ok(value.trim().to_string())
}

fn normalized_search(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_lowercase())
        .filter(|item| !item.is_empty())
}

fn optional_trimmed(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn workspace_lineage(workspace: &WorkspaceDetail) -> Vec<LineageEntry> {
    let mut labels = Vec::new();
    workspace
        .lineage
        .iter()
        .map(|item| {
            labels.push(item.name.clone());
            LineageEntry {
                code: item.workspace_code.clone(),
                label: labels.join(" / "),
                name: item.name.clone(),
            }
        })
        .collect()
}
