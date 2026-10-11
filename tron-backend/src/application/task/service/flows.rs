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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        adapters::outbound::persistence::{
            SqliteAppRepository, SqliteTaskRepository, SqliteWorkspaceRepository,
        },
        application::{
            app::AppService,
            task::CreateTaskCommand,
            workspace::{CreateWorkspaceCommand, WorkspaceService},
        },
        domain::{
            auth::AuthenticatedUser,
            task::{
                CLOSED_STATUS_CODE, NOT_STARTED_STATUS_CODE, TaskFlowDraft, TaskFlowStatus,
                TaskPriority, TaskSourceType, TaskType,
            },
        },
    };

    #[test]
    fn normalizes_flow_and_enforces_required_statuses() {
        let flow = normalized_task_flow(TaskFlowDraft {
            flow_code: " release-flow ".into(),
            name: "发布流程".into(),
            statuses: vec![TaskFlowStatus {
                status_code: "deploying".into(),
                name: "发布中".into(),
                position: 0,
                required: false,
            }],
        })
        .expect("flow is valid");

        assert_eq!(flow.flow_code, "RELEASE-FLOW");
        assert_eq!(flow.statuses[0].status_code, NOT_STARTED_STATUS_CODE);
        assert_eq!(flow.statuses[1].status_code, "DEPLOYING");
        assert_eq!(flow.statuses[2].status_code, CLOSED_STATUS_CODE);
        assert!(flow.statuses[0].required);
        assert!(flow.statuses[2].required);
    }

    #[test]
    fn rejects_duplicate_or_invalid_custom_status_codes() {
        let draft = TaskFlowDraft {
            flow_code: "RELEASE".into(),
            name: "发布流程".into(),
            statuses: vec![
                TaskFlowStatus {
                    status_code: "DEPLOY".into(),
                    name: "发布".into(),
                    position: 0,
                    required: false,
                },
                TaskFlowStatus {
                    status_code: "deploy".into(),
                    name: "再次发布".into(),
                    position: 1,
                    required: false,
                },
            ],
        };

        assert_eq!(normalized_task_flow(draft), Err(TaskError::InvalidRequest));
    }

    #[test]
    fn workspace_flow_switch_requires_closed_tasks_and_applies_to_new_tasks() {
        let workspaces = WorkspaceService::new(
            SqliteWorkspaceRepository::connect(":memory:").expect("workspace repository connects"),
        );
        let workspace = workspaces
            .create_workspace(
                &AuthenticatedUser {
                    avatar_url: None,
                    display_name: "Tester".into(),
                    email: String::new(),
                    user_code: "USR-TEST".into(),
                    username: "tester".into(),
                },
                CreateWorkspaceCommand {
                    name: "Task flow test".into(),
                    description: None,
                    parent_workspace_code: None,
                },
            )
            .expect("workspace is created");
        let apps = AppService::new(
            SqliteAppRepository::connect(":memory:").expect("app repository connects"),
            workspaces.clone(),
        );
        let tasks = TaskService::new(
            SqliteTaskRepository::connect(":memory:").expect("task repository connects"),
            workspaces,
            apps,
        );
        tasks
            .create_task_flow(TaskFlowDraft {
                flow_code: "RELEASE".into(),
                name: "发布流程".into(),
                statuses: vec![TaskFlowStatus {
                    status_code: "DEPLOYING".into(),
                    name: "发布中".into(),
                    position: 0,
                    required: false,
                }],
            })
            .expect("custom flow is created");
        let create_task = || CreateTaskCommand {
            workspace_code: Some(workspace.workspace_code.clone()),
            owner_dude_code: None,
            due_at: None,
            parent_task_code: None,
            title: "Release candidate".into(),
            description: None,
            priority: TaskPriority::Medium,
            task_type: TaskType::Feature,
            source_type: TaskSourceType::Internal,
            source_url: None,
            agent_code: None,
            model_code: None,
            assignee_user_codes: vec![],
            agent_codes: vec![],
            app_codes: vec![],
        };
        let open_task = tasks
            .create_task(create_task())
            .expect("open task is created");

        assert_eq!(
            tasks.assign_workspace_task_flow(&workspace.workspace_code, "RELEASE"),
            Err(TaskError::WorkspaceFlowHasOpenTasks),
        );

        tasks
            .update_status(
                &open_task.task_code,
                UpdateTaskStatusCommand {
                    status: CLOSED_STATUS_CODE.into(),
                    progress: None,
                },
            )
            .expect("task is closed");
        tasks
            .assign_workspace_task_flow(&workspace.workspace_code, "RELEASE")
            .expect("workspace flow switches");
        let new_task = tasks
            .create_task(create_task())
            .expect("new task is created");

        assert_eq!(new_task.flow_code, "RELEASE");
        assert_eq!(new_task.status, NOT_STARTED_STATUS_CODE);
    }
}
