use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
};

use crate::{
    application::workspace::WorkspaceService,
    domain::{
        app::{
            App, AppDetail, AppError, AppLifecycleResult, AppListItem, AppListQuery, AppOption,
            AppOptionQuery, AppRuntimeRole, AppStatus, AppTag, PagedAppRecords, validate_app_code,
        },
        workspace::{WorkspaceDetail, WorkspaceError, WorkspaceStatus},
    },
    shared::{
        id::{IdGenerator, SnowflakedSnowflakeIdProvider},
        time::current_timestamp,
    },
};

const GENERATED_APP_CODE_PREFIX: &str = "APP-";

mod projection;
mod use_cases;

use self::projection::*;

#[derive(Debug, Clone)]
pub struct CreateAppCommand {
    pub name: String,
    pub description: Option<String>,
    pub local_directory: String,
    pub runtime_role: AppRuntimeRole,
    pub repository_url: Option<String>,
    pub default_branch: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateAppCommand {
    pub name: String,
    pub description: Option<String>,
    pub local_directory: String,
    pub runtime_role: AppRuntimeRole,
    pub repository_url: Option<String>,
    pub default_branch: Option<String>,
    pub tags: Vec<String>,
}

pub trait AppRepository: Send {
    fn list_apps(&mut self) -> Result<Vec<App>, AppError>;
    fn list_app_tags(&mut self) -> Result<Vec<AppTag>, AppError>;
    fn apps_by_codes(&mut self, codes: &[String]) -> Result<Vec<App>, AppError>;
    fn tags_for_apps(&mut self, codes: &[String]) -> Result<Vec<AppTag>, AppError>;
    fn next_id(&mut self) -> Result<i64, AppError>;
    fn create_app(&mut self, app: &App) -> Result<(), AppError>;
    fn delete_app(&mut self, app_code: &str) -> Result<(), AppError>;
    fn update_app(&mut self, app: &App) -> Result<(), AppError>;
    fn replace_app_tags(
        &mut self,
        app_code: &str,
        tags: &[AppTag],
    ) -> Result<Vec<AppTag>, AppError>;
}

#[derive(Clone)]
pub struct AppService {
    id_generator: Arc<dyn IdGenerator>,
    repository: Arc<Mutex<Box<dyn AppRepository>>>,
    workspaces: WorkspaceService,
}

impl AppService {
    pub fn delete_app(&self, app_code: &str) -> Result<(), AppError> {
        validate_app_code(app_code)?;
        self.repository.lock().expect("app repository mutex poisoned").delete_app(app_code.trim())
    }

    pub fn new(repository: impl AppRepository + 'static, workspaces: WorkspaceService) -> Self {
        Self::with_id_generator(
            repository,
            workspaces,
            Arc::new(SnowflakedSnowflakeIdProvider::default()),
        )
    }

    pub fn with_id_generator(
        repository: impl AppRepository + 'static,
        workspaces: WorkspaceService,
        id_generator: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            id_generator,
            repository: Arc::new(Mutex::new(Box::new(repository))),
            workspaces,
        }
    }

    /// Resolve a set of applications and their tags once, in caller order.
    pub fn get_apps(&self, codes: &[String]) -> Result<Vec<AppDetail>, AppError> {
        if codes.is_empty() {
            return Ok(Vec::new());
        }
        for code in codes {
            validate_app_code(code)?;
        }
        let codes = codes
            .iter()
            .map(|code| code.trim().to_owned())
            .collect::<Vec<_>>();
        let mut repository = self
            .repository
            .lock()
            .expect("app repository mutex poisoned");
        let apps = repository
            .apps_by_codes(&codes)?
            .into_iter()
            .map(|app| (app.app_code.clone(), app))
            .collect::<BTreeMap<_, _>>();
        let mut tags = BTreeMap::<String, Vec<String>>::new();
        for tag in repository.tags_for_apps(&codes)? {
            tags.entry(tag.app_code).or_default().push(tag.tag);
        }
        codes
            .iter()
            .map(|code| {
                let app = apps.get(code).cloned().ok_or(AppError::NotFound)?;
                Ok(Self::detail_with_tags(app, &tags))
            })
            .collect()
    }
    fn filtered_apps(
        &self,
        workspace_code: &str,
        query: &AppListQuery,
    ) -> Result<Vec<AppListItem>, AppError> {
        let search = normalized_search(query.search.clone());
        let tags_by_code = self.tags_by_code()?;

        self.all_apps()?
            .into_iter()
            .filter(|app| app.workspace_code == workspace_code)
            .filter(|app| match query.status {
                Some(status) => app.status == status,
                None if query.include_archived.unwrap_or(false) => true,
                None => app.status == AppStatus::Active,
            })
            .filter(|app| {
                query
                    .runtime_role
                    .map_or(true, |role| app.runtime_role == role)
            })
            .filter(|app| matches_search(app, &tags_by_code, search.as_deref()))
            .map(|app| self.list_item_from_app(app, &tags_by_code))
            .collect()
    }

    fn detail_from_app(&self, app: App) -> Result<AppDetail, AppError> {
        let mut tags_by_code = BTreeMap::<String, Vec<String>>::new();
        for tag in self
            .repository
            .lock()
            .expect("app repository mutex poisoned")
            .tags_for_apps(&[app.app_code.clone()])?
        {
            tags_by_code.entry(tag.app_code).or_default().push(tag.tag);
        }
        for tags in tags_by_code.values_mut() {
            tags.sort();
        }
        Ok(Self::detail_with_tags(app, &tags_by_code))
    }

    fn detail_with_tags(app: App, tags_by_code: &BTreeMap<String, Vec<String>>) -> AppDetail {
        AppDetail {
            id: app.id.as_decimal_string(),
            app_code: app.app_code.clone(),
            workspace_code: app.workspace_code,
            name: app.name,
            description: app.description,
            local_directory: app.local_directory,
            runtime_role: app.runtime_role,
            repository_url: app.repository_url,
            default_branch: app.default_branch,
            tags: tags_by_code.get(&app.app_code).cloned().unwrap_or_default(),
            status: app.status,
            created_at: app.created_at,
            updated_at: app.updated_at,
            archived_at: app.archived_at,
        }
    }

    fn list_item_from_app(
        &self,
        app: App,
        tags_by_code: &BTreeMap<String, Vec<String>>,
    ) -> Result<AppListItem, AppError> {
        Ok(AppListItem {
            app_code: app.app_code.clone(),
            workspace_code: app.workspace_code,
            name: app.name,
            description: app.description,
            local_directory: app.local_directory,
            runtime_role: app.runtime_role,
            repository_url: app.repository_url,
            default_branch: app.default_branch,
            tags: tags_by_code.get(&app.app_code).cloned().unwrap_or_default(),
            status: app.status,
            updated_at: app.updated_at,
            archived_at: app.archived_at,
        })
    }

    fn active_workspace_detail(&self, workspace_code: &str) -> Result<WorkspaceDetail, AppError> {
        let code = workspace_code.trim();
        if code.is_empty() {
            return Err(AppError::WorkspaceRequired);
        }

        let workspace = self
            .workspaces
            .get_workspace(code)
            .map_err(map_workspace_error)?;
        if workspace.status != WorkspaceStatus::Active {
            return Err(AppError::InvalidWorkspace);
        }

        Ok(workspace)
    }

    fn all_apps(&self) -> Result<Vec<App>, AppError> {
        self.repository
            .lock()
            .expect("app repository mutex poisoned")
            .list_apps()
    }

    fn tags_by_code(&self) -> Result<BTreeMap<String, Vec<String>>, AppError> {
        let mut tags_by_code = BTreeMap::<String, Vec<String>>::new();
        for tag in self
            .repository
            .lock()
            .expect("app repository mutex poisoned")
            .list_app_tags()?
        {
            tags_by_code.entry(tag.app_code).or_default().push(tag.tag);
        }
        for tags in tags_by_code.values_mut() {
            tags.sort();
        }
        Ok(tags_by_code)
    }

    fn get_app_record(&self, app_code: &str) -> Result<App, AppError> {
        validate_app_code(app_code)?;
        self.repository
            .lock()
            .expect("app repository mutex poisoned")
            .apps_by_codes(&[app_code.trim().to_owned()])?
            .into_iter()
            .next()
            .ok_or(AppError::NotFound)
    }
}
