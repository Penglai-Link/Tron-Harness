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
    shared::{id::SnowflakeId, time::current_timestamp},
};

const GENERATED_APP_CODE_PREFIX: &str = "APP-";

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
    fn next_id(&mut self) -> Result<i64, AppError>;
    fn create_app(&mut self, app: &App) -> Result<(), AppError>;
    fn update_app(&mut self, app: &App) -> Result<(), AppError>;
    fn replace_app_tags(
        &mut self,
        app_code: &str,
        tags: &[AppTag],
    ) -> Result<Vec<AppTag>, AppError>;
}

#[derive(Clone)]
pub struct AppService {
    repository: Arc<Mutex<Box<dyn AppRepository>>>,
    workspaces: WorkspaceService,
}

impl AppService {
    pub fn new(repository: impl AppRepository + 'static, workspaces: WorkspaceService) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
            workspaces,
        }
    }

    pub fn list_apps(
        &self,
        workspace_code: &str,
        query: AppListQuery,
    ) -> Result<PagedAppRecords<AppListItem>, AppError> {
        let workspace = self.active_workspace_detail(workspace_code)?;
        let current = query.current.unwrap_or(1).max(1);
        let size = query.size.unwrap_or(15).clamp(1, 100);
        let mut records = self.filtered_apps(&workspace.workspace_code, &query)?;
        records.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then(left.app_code.cmp(&right.app_code))
        });
        let total = records.len();
        let start = (current - 1) * size;

        Ok(PagedAppRecords {
            current,
            size,
            total,
            records: records.into_iter().skip(start).take(size).collect(),
        })
    }

    pub fn list_app_options(
        &self,
        workspace_code: &str,
        query: AppOptionQuery,
    ) -> Result<Vec<AppOption>, AppError> {
        let workspace = self.active_workspace_detail(workspace_code)?;
        let search = normalized_search(query.search);
        let tags_by_code = self.tags_by_code()?;
        let mut options = self
            .all_apps()?
            .into_iter()
            .filter(|app| app.workspace_code == workspace.workspace_code)
            .filter(|app| app.status == AppStatus::Active)
            .filter(|app| {
                query
                    .runtime_role
                    .map_or(true, |role| app.runtime_role == role)
            })
            .filter(|app| matches_search(app, &tags_by_code, search.as_deref()))
            .map(|app| AppOption {
                app_code: app.app_code,
                name: app.name,
                description: app.description,
                runtime_role: app.runtime_role,
            })
            .collect::<Vec<_>>();
        options.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then(left.app_code.cmp(&right.app_code))
        });
        Ok(options)
    }

    pub fn get_app(&self, app_code: &str) -> Result<AppDetail, AppError> {
        let app = self.get_app_record(app_code)?;
        self.detail_from_app(app)
    }

    pub fn create_app(
        &self,
        workspace_code: &str,
        command: CreateAppCommand,
    ) -> Result<AppDetail, AppError> {
        let workspace = self.active_workspace_detail(workspace_code)?;
        let name = normalized_name(command.name)?;
        let description = optional_trimmed(command.description);
        let local_directory = required_local_directory(command.local_directory)?;
        let repository_url = optional_trimmed(command.repository_url);
        let default_branch = optional_trimmed(command.default_branch);
        let tags = normalized_tags(command.tags);
        let apps = self.all_apps()?;

        if apps.iter().any(|app| {
            app.workspace_code == workspace.workspace_code && app.name.eq_ignore_ascii_case(&name)
        }) {
            return Err(AppError::DuplicateName);
        }

        let id = self
            .repository
            .lock()
            .expect("app repository mutex poisoned")
            .next_id()?;
        let app_code = format!("{GENERATED_APP_CODE_PREFIX}{id}");
        let app = App {
            id: SnowflakeId::new(id),
            app_code: app_code.clone(),
            workspace_code: workspace.workspace_code,
            name,
            description,
            local_directory,
            runtime_role: command.runtime_role,
            repository_url,
            default_branch,
            status: AppStatus::Active,
            created_at: current_timestamp(),
            updated_at: current_timestamp(),
            archived_at: None,
        };
        let tag_rows = tag_rows(&app_code, &tags);

        {
            let mut repository = self
                .repository
                .lock()
                .expect("app repository mutex poisoned");
            repository.create_app(&app)?;
            repository.replace_app_tags(&app_code, &tag_rows)?;
        }

        self.get_app(&app_code)
    }

    pub fn update_app(
        &self,
        app_code: &str,
        command: UpdateAppCommand,
    ) -> Result<AppDetail, AppError> {
        let mut app = self.get_app_record(app_code)?;
        if app.status == AppStatus::Archived {
            return Err(AppError::ArchivedMutationDenied);
        }

        let name = normalized_name(command.name)?;
        let apps = self.all_apps()?;
        if apps.iter().any(|candidate| {
            candidate.workspace_code == app.workspace_code
                && candidate.app_code != app.app_code
                && candidate.name.eq_ignore_ascii_case(&name)
        }) {
            return Err(AppError::DuplicateName);
        }

        app.name = name;
        app.description = optional_trimmed(command.description);
        app.local_directory = required_local_directory(command.local_directory)?;
        app.runtime_role = command.runtime_role;
        app.repository_url = optional_trimmed(command.repository_url);
        app.default_branch = optional_trimmed(command.default_branch);
        app.updated_at = current_timestamp();
        let tags = normalized_tags(command.tags);

        {
            let mut repository = self
                .repository
                .lock()
                .expect("app repository mutex poisoned");
            repository.update_app(&app)?;
            repository.replace_app_tags(&app.app_code, &tag_rows(&app.app_code, &tags))?;
        }

        self.get_app(&app.app_code)
    }

    pub fn archive_app(&self, app_code: &str) -> Result<AppLifecycleResult, AppError> {
        let mut app = self.get_app_record(app_code)?;
        if app.status != AppStatus::Archived {
            app.status = AppStatus::Archived;
            app.archived_at = Some(current_timestamp());
            app.updated_at = current_timestamp();
            self.repository
                .lock()
                .expect("app repository mutex poisoned")
                .update_app(&app)?;
        }
        Ok(AppLifecycleResult {
            app_code: app.app_code,
            status: app.status,
            archived_at: app.archived_at,
        })
    }

    pub fn restore_app(&self, app_code: &str) -> Result<AppLifecycleResult, AppError> {
        let mut app = self.get_app_record(app_code)?;
        if app.status == AppStatus::Archived {
            self.active_workspace_detail(&app.workspace_code)?;
            app.status = AppStatus::Active;
            app.archived_at = None;
            app.updated_at = current_timestamp();
            self.repository
                .lock()
                .expect("app repository mutex poisoned")
                .update_app(&app)?;
        }
        Ok(AppLifecycleResult {
            app_code: app.app_code,
            status: app.status,
            archived_at: app.archived_at,
        })
    }

    pub fn assignable_app_codes(&self, workspace_code: &str) -> Result<BTreeSet<String>, AppError> {
        let workspace = self.active_workspace_detail(workspace_code)?;
        Ok(self
            .all_apps()?
            .into_iter()
            .filter(|app| app.workspace_code == workspace.workspace_code)
            .filter(|app| app.status == AppStatus::Active)
            .map(|app| app.app_code)
            .collect())
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
        let tags_by_code = self.tags_by_code()?;
        Ok(AppDetail {
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
        })
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
        self.all_apps()?
            .into_iter()
            .find(|app| app.app_code == app_code.trim())
            .ok_or(AppError::NotFound)
    }
}

fn normalized_name(value: String) -> Result<String, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::NameRequired);
    }
    Ok(trimmed.to_string())
}

fn optional_trimmed(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn required_local_directory(value: String) -> Result<String, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::LocalDirectoryRequired);
    }
    Ok(trimmed.to_string())
}

fn normalized_tags(tags: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    tags.into_iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .filter(|tag| seen.insert(tag.to_ascii_lowercase()))
        .collect()
}

fn tag_rows(app_code: &str, tags: &[String]) -> Vec<AppTag> {
    tags.iter()
        .map(|tag| AppTag {
            app_code: app_code.to_string(),
            tag: tag.clone(),
            linked_at: current_timestamp(),
        })
        .collect()
}

fn normalized_search(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_lowercase())
        .filter(|item| !item.is_empty())
}

fn matches_search(
    app: &App,
    tags_by_code: &BTreeMap<String, Vec<String>>,
    search: Option<&str>,
) -> bool {
    let Some(search) = search else {
        return true;
    };
    let search_blob = [
        app.app_code.as_str(),
        app.name.as_str(),
        app.description.as_deref().unwrap_or_default(),
        app.local_directory.as_str(),
        match app.runtime_role {
            AppRuntimeRole::Frontend => "frontend",
            AppRuntimeRole::Backend => "backend",
        },
        app.repository_url.as_deref().unwrap_or_default(),
        app.default_branch.as_deref().unwrap_or_default(),
        match app.status {
            AppStatus::Active => "active",
            AppStatus::Archived => "archived",
        },
        &tags_by_code
            .get(&app.app_code)
            .cloned()
            .unwrap_or_default()
            .join(" "),
    ]
    .join(" ")
    .to_lowercase();

    search_blob.contains(search)
}

fn map_workspace_error(error: WorkspaceError) -> AppError {
    match error {
        WorkspaceError::NotFound | WorkspaceError::ArchivedMutationDenied => {
            AppError::InvalidWorkspace
        }
        _ => AppError::StorageFailure,
    }
}
