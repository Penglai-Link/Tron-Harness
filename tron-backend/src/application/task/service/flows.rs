use std::collections::HashSet;

use crate::domain::task::TaskFlowStatus;

use super::*;

impl TaskService {
    pub fn list_task_flows(&self) -> Result<Vec<TaskFlow>, TaskError> {
        self.repository
            .lock()
            .expect("task repository mutex poisoned")
            .list_task_flows()
    }

    pub fn get_task_flow(&self, flow_code: &str) -> Result<TaskFlow, TaskError> {
        let flow_code = normalized_flow_code(flow_code).ok_or(TaskError::FlowNotFound)?;
        self.repository
            .lock()
            .expect("task repository mutex poisoned")
            .get_task_flow(&flow_code)?
            .ok_or(TaskError::FlowNotFound)
    }

    pub fn create_task_flow(&self, draft: TaskFlowDraft) -> Result<TaskFlow, TaskError> {
        let flow = normalized_task_flow(draft)?;
        let mut repository = self
            .repository
            .lock()
            .expect("task repository mutex poisoned");
        if repository.get_task_flow(&flow.flow_code)?.is_some() {
            return Err(TaskError::FlowCodeConflict);
        }
        repository.create_task_flow(&flow)?;
        Ok(flow)
    }

    pub fn update_task_flow(
        &self,
        flow_code: &str,
        draft: TaskFlowDraft,
    ) -> Result<TaskFlow, TaskError> {
        let flow_code = normalized_flow_code(flow_code).ok_or(TaskError::FlowNotFound)?;
        let flow = normalized_task_flow(draft)?;
        if flow.flow_code != flow_code {
            return Err(TaskError::InvalidRequest);
        }
        let mut repository = self
            .repository
            .lock()
            .expect("task repository mutex poisoned");
        if repository.get_task_flow(&flow_code)?.is_none() {
            return Err(TaskError::FlowNotFound);
        }
        repository.update_task_flow(&flow)?;
        Ok(flow)
    }

    pub fn delete_task_flow(&self, flow_code: &str) -> Result<(), TaskError> {
        let flow_code = normalized_flow_code(flow_code).ok_or(TaskError::FlowNotFound)?;
        if flow_code == crate::domain::task::DEFAULT_TASK_FLOW_CODE {
            return Err(TaskError::InvalidRequest);
        }
        let mut repository = self
            .repository
            .lock()
            .expect("task repository mutex poisoned");
        if repository.get_task_flow(&flow_code)?.is_none() {
            return Err(TaskError::FlowNotFound);
        }
        if repository
            .list_tasks()?
            .iter()
            .any(|task| task.flow_code == flow_code)
        {
            return Err(TaskError::FlowInUse);
        }
        repository.delete_task_flow(&flow_code)
    }

    pub fn workspace_task_flow(
        &self,
        workspace_code: &str,
    ) -> Result<WorkspaceTaskFlow, TaskError> {
        let workspace_code =
            normalized_required(workspace_code.to_owned()).ok_or(TaskError::WorkspaceRequired)?;
        self.ensure_active_workspace(&workspace_code)?;
        let flow_code = self
            .repository
            .lock()
            .expect("task repository mutex poisoned")
            .workspace_task_flow_code(&workspace_code)?
            .unwrap_or_else(|| crate::domain::task::DEFAULT_TASK_FLOW_CODE.to_string());
        Ok(WorkspaceTaskFlow {
            workspace_code,
            flow: self.get_task_flow(&flow_code)?,
        })
    }

    pub fn assign_workspace_task_flow(
        &self,
        workspace_code: &str,
        flow_code: &str,
    ) -> Result<WorkspaceTaskFlow, TaskError> {
        let workspace_code =
            normalized_required(workspace_code.to_owned()).ok_or(TaskError::WorkspaceRequired)?;
        let flow_code = normalized_flow_code(flow_code).ok_or(TaskError::FlowNotFound)?;
        self.ensure_active_workspace(&workspace_code)?;
        let flow = self.get_task_flow(&flow_code)?;
        if self
            .all_tasks()?
            .iter()
            .filter(|task| task.workspace_code.as_deref() == Some(workspace_code.as_str()))
            .any(|task| task.status != crate::domain::task::CLOSED_STATUS_CODE)
        {
            return Err(TaskError::WorkspaceFlowHasOpenTasks);
        }
        let mut repository = self
            .repository
            .lock()
            .expect("task repository mutex poisoned");
        repository.assign_workspace_task_flow(&workspace_code, &flow_code)?;
        repository.update_workspace_task_flow(&workspace_code, &flow_code)?;
        Ok(WorkspaceTaskFlow {
            workspace_code,
            flow,
        })
    }

    pub(super) fn task_flow_for_workspace(
        &self,
        workspace_code: &str,
    ) -> Result<TaskFlow, TaskError> {
        Ok(self.workspace_task_flow(workspace_code)?.flow)
    }

    pub(super) fn flow_contains_status(flow: &TaskFlow, status: &str) -> bool {
        flow.statuses
            .iter()
            .any(|candidate| candidate.status_code == status)
    }
}

fn normalized_task_flow(draft: TaskFlowDraft) -> Result<TaskFlow, TaskError> {
    let flow_code = normalized_flow_code(&draft.flow_code).ok_or(TaskError::InvalidRequest)?;
    let name = normalized_required(draft.name).ok_or(TaskError::InvalidRequest)?;
    let mut seen = HashSet::new();
    let mut custom_statuses = Vec::new();
    for status in draft.statuses {
        let status_code =
            normalized_flow_code(&status.status_code).ok_or(TaskError::InvalidRequest)?;
        let name = normalized_required(status.name).ok_or(TaskError::InvalidRequest)?;
        if !seen.insert(status_code.clone()) {
            return Err(TaskError::InvalidRequest);
        }
        if status_code != crate::domain::task::NOT_STARTED_STATUS_CODE
            && status_code != crate::domain::task::CLOSED_STATUS_CODE
        {
            custom_statuses.push(TaskFlowStatus {
                status_code,
                name,
                position: custom_statuses.len() + 1,
                required: false,
            });
        }
    }
    let mut statuses = vec![TaskFlowStatus {
        status_code: crate::domain::task::NOT_STARTED_STATUS_CODE.to_string(),
        name: "未开始".to_string(),
        position: 0,
        required: true,
    }];
    statuses.append(&mut custom_statuses);
    statuses.push(TaskFlowStatus {
        status_code: crate::domain::task::CLOSED_STATUS_CODE.to_string(),
        name: "关闭".to_string(),
        position: statuses.len(),
        required: true,
    });
    Ok(TaskFlow {
        flow_code,
        name,
        statuses,
    })
}

pub(super) fn normalized_flow_code(value: &str) -> Option<String> {
    let code = value.trim().to_ascii_uppercase();
    let mut chars = code.chars();
    let first = chars.next()?;
    if !first.is_ascii_alphabetic()
        || !chars.all(|character| {
            character.is_ascii_alphanumeric() || character == '_' || character == '-'
        })
        || code.len() > 64
    {
        return None;
    }
    Some(code)
}
