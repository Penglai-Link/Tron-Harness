use super::*;

impl AppService {
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
}
