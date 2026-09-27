use std::sync::{Arc, Mutex};

use super::ApplicationDirectoryInspector;

use crate::{
    application::{app::AppService, workspace::WorkspaceService},
    domain::{
        app::{AppDetail, AppStatus},
        project::{
            Project, ProjectAppLink, ProjectError, ProjectStatus, ProjectView,
            validate_project_code,
        },
        workspace::WorkspaceStatus,
    },
    shared::{
        id::{IdGenerator, SnowflakedSnowflakeIdProvider},
        time::current_timestamp,
    },
};

#[derive(Debug, Clone)]
pub struct SaveProjectCommand {
    pub project_code: Option<String>,
    pub workspace_code: String,
    pub name: String,
    pub app_codes: Vec<String>,
    pub primary_app_code: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProjectExecutionContext {
    pub project_code: String,
    pub primary_app: Option<AppDetail>,
    pub linked_apps: Vec<AppDetail>,
}

pub trait ProjectRepository: Send {
    fn next_id(&mut self) -> Result<i64, ProjectError>;
    fn list_projects(&mut self, workspace_code: &str) -> Result<Vec<Project>, ProjectError>;
    fn get_project(&mut self, project_code: &str) -> Result<Project, ProjectError>;
    fn list_links(&mut self, project_code: &str) -> Result<Vec<ProjectAppLink>, ProjectError>;
    fn save_project(
        &mut self,
        project: &Project,
        links: &[ProjectAppLink],
    ) -> Result<(), ProjectError>;
    fn archive_project(
        &mut self,
        project_code: &str,
        archived_at: &str,
    ) -> Result<(), ProjectError>;
}

#[derive(Clone)]
pub struct ProjectService {
    pub(super) directory_inspector: Option<Arc<dyn ApplicationDirectoryInspector>>,
    apps: AppService,
    id_generator: Arc<dyn IdGenerator>,
    repository: Arc<Mutex<Box<dyn ProjectRepository>>>,
    workspaces: WorkspaceService,
}

impl ProjectService {
    pub fn new(
        repository: impl ProjectRepository + 'static,
        workspaces: WorkspaceService,
        apps: AppService,
    ) -> Self {
        Self {
            directory_inspector: None,
            apps,
            workspaces,
            id_generator: Arc::new(SnowflakedSnowflakeIdProvider::default()),
            repository: Arc::new(Mutex::new(Box::new(repository))),
        }
    }

    pub fn list(&self, workspace_code: &str) -> Result<Vec<ProjectView>, ProjectError> {
        self.active_workspace(workspace_code)?;
        // `view` reads each project's links from the same repository.  Keep
        // this guard scoped to the initial query so the follow-up reads do
        // not attempt to acquire the mutex recursively.
        let projects = {
            let mut repository = self
                .repository
                .lock()
                .expect("project repository mutex poisoned");
            repository.list_projects(workspace_code)?
        };
        projects
            .into_iter()
            .filter(|project| project.status == ProjectStatus::Active)
            .map(|project| self.view(project))
            .collect()
    }

    pub fn get(&self, project_code: &str) -> Result<ProjectView, ProjectError> {
        validate_project_code(project_code)?;
        let project = {
            let mut repository = self
                .repository
                .lock()
                .expect("project repository mutex poisoned");
            repository.get_project(project_code)?
        };
        self.active_workspace(&project.workspace_code)?;
        if project.status != ProjectStatus::Active {
            return Err(ProjectError::Archived);
        }
        self.view(project)
    }

    pub fn save(&self, command: SaveProjectCommand) -> Result<ProjectView, ProjectError> {
        self.active_workspace(&command.workspace_code)?;
        let name = command.name.trim();
        if name.is_empty() {
            return Err(ProjectError::NameRequired);
        }
        let mut app_codes = Vec::new();
        for app_code in command.app_codes {
            let app_code = app_code.trim().to_string();
            if app_code.is_empty() || app_codes.iter().any(|item: &String| item == &app_code) {
                return Err(ProjectError::DuplicateApp);
            }
            let app = self
                .apps
                .get_app(&app_code)
                .map_err(|_| ProjectError::AppInvalid)?;
            if app.workspace_code != command.workspace_code || app.status != AppStatus::Active {
                return Err(ProjectError::AppInvalid);
            }
            app_codes.push(app_code);
        }
        let primary_app_code = match (app_codes.first(), command.primary_app_code) {
            (None, None) => None,
            (None, Some(_)) => return Err(ProjectError::PrimaryAppInvalid),
            (Some(first), requested) => {
                let code = requested.unwrap_or_else(|| first.clone());
                if !app_codes.iter().any(|app_code| app_code == &code) {
                    return Err(ProjectError::PrimaryAppInvalid);
                }
                Some(code)
            }
        };
        let now = current_timestamp();
        let (id, project_code, created_at) = match command.project_code {
            Some(code) => {
                validate_project_code(&code)?;
                let current = {
                    let mut repository = self
                        .repository
                        .lock()
                        .expect("project repository mutex poisoned");
                    repository.get_project(&code)?
                };
                if current.workspace_code != command.workspace_code {
                    return Err(ProjectError::WorkspaceInvalid);
                }
                if current.status != ProjectStatus::Active {
                    return Err(ProjectError::Archived);
                }
                (current.id, current.project_code, current.created_at)
            }
            None => {
                let id = self
                    .id_generator
                    .next_id()
                    .map_err(|_| ProjectError::StorageFailure)?
                    .value();
                (id, format!("PROJECT-{id}"), now.clone())
            }
        };
        let project = Project {
            id,
            project_code: project_code.clone(),
            workspace_code: command.workspace_code,
            name: name.to_string(),
            status: ProjectStatus::Active,
            created_at,
            updated_at: now,
            archived_at: None,
        };
        let mut links = Vec::with_capacity(app_codes.len());
        for (position, app_code) in app_codes.iter().enumerate() {
            let id = self
                .id_generator
                .next_id()
                .map_err(|_| ProjectError::StorageFailure)?
                .value();
            links.push(ProjectAppLink {
                id,
                project_code: project_code.clone(),
                app_code: app_code.clone(),
                position,
                is_primary: primary_app_code.as_deref() == Some(app_code.as_str()),
            });
        }
        {
            let mut repository = self
                .repository
                .lock()
                .expect("project repository mutex poisoned");
            repository.save_project(&project, &links)?;
        }
        self.view(project)
    }

    pub fn archive(&self, project_code: &str) -> Result<(), ProjectError> {
        let project = {
            let mut repository = self
                .repository
                .lock()
                .expect("project repository mutex poisoned");
            repository.get_project(project_code)?
        };
        self.active_workspace(&project.workspace_code)?;
        let mut repository = self
            .repository
            .lock()
            .expect("project repository mutex poisoned");
        repository.archive_project(project_code, &current_timestamp())
    }

    /// Resolve linked apps at invocation time so project edits affect future runs.
    pub fn resolve_execution_context(
        &self,
        workspace_code: &str,
        project_code: &str,
    ) -> Result<ProjectExecutionContext, ProjectError> {
        let project = self.get(project_code)?;
        if project.workspace_code != workspace_code {
            return Err(ProjectError::WorkspaceInvalid);
        }
        let mut linked_apps = Vec::with_capacity(project.app_codes.len());
        for app_code in &project.app_codes {
            let app = self
                .apps
                .get_app(app_code)
                .map_err(|_| ProjectError::AppInvalid)?;
            if app.workspace_code != workspace_code || app.status != AppStatus::Active {
                return Err(ProjectError::AppInvalid);
            }
            linked_apps.push(app);
        }
        let primary_app = linked_apps
            .iter()
            .find(|app| Some(app.app_code.as_str()) == project.primary_app_code.as_deref())
            .cloned();
        Ok(ProjectExecutionContext {
            project_code: project.project_code,
            primary_app,
            linked_apps,
        })
    }

    fn active_workspace(&self, workspace_code: &str) -> Result<(), ProjectError> {
        let workspace = self
            .workspaces
            .get_workspace(workspace_code.trim())
            .map_err(|_| ProjectError::WorkspaceInvalid)?;
        if workspace.status != WorkspaceStatus::Active {
            return Err(ProjectError::WorkspaceInvalid);
        }
        Ok(())
    }

    fn view(&self, project: Project) -> Result<ProjectView, ProjectError> {
        let links = self
            .repository
            .lock()
            .expect("project repository mutex poisoned")
            .list_links(&project.project_code)?;
        let primary_app_code = links
            .iter()
            .find(|link| link.is_primary)
            .map(|link| link.app_code.clone());
        Ok(ProjectView {
            project_code: project.project_code,
            workspace_code: project.workspace_code,
            name: project.name,
            status: project.status,
            app_codes: links.into_iter().map(|link| link.app_code).collect(),
            primary_app_code,
            created_at: project.created_at,
            updated_at: project.updated_at,
            archived_at: project.archived_at,
        })
    }
}
