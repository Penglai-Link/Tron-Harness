#[derive(Clone)]
pub struct NavigationService {
    repository: Arc<Mutex<Box<dyn NavigationRepository>>>,
    max_depth: usize,
}

impl NavigationService {
    pub fn new(repository: impl NavigationRepository + 'static, max_depth: usize) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
            max_depth: max_depth.max(1),
        }
    }

    pub fn max_depth(&self) -> usize {
        self.max_depth
    }

    pub fn list_visible_domains(
        &self,
        role_codes: &[String],
        locale: &str,
    ) -> Result<Vec<NavigationDomain>, NavigationError> {
        let domains = self.list_domains()?;
        let _ = locale;
        Ok(domains
            .into_iter()
            .filter(|domain| domain.enabled && roles_match(&domain.visible_roles, role_codes))
            .collect())
    }

    pub fn list_domains(&self) -> Result<Vec<NavigationDomain>, NavigationError> {
        let mut repository = self.lock_repository();
        repository.list_domains()
    }

    pub fn get_domain(&self, domain_code: &str) -> Result<NavigationDomain, NavigationError> {
        let mut repository = self.lock_repository();
        repository.get_domain(domain_code)
    }

    pub fn create_domain(
        &self,
        command: CreateDomainCommand,
    ) -> Result<NavigationDomain, NavigationError> {
        let domain_code = validate_domain_code(&command.domain_code)?;
        validate_translations(&command.name)?;
        validate_translations(&command.description)?;
        validate_roles(&command.visible_roles)?;
        if command.position < 0 {
            return Err(NavigationError::InvalidHierarchy);
        }
        let timestamp = current_timestamp();
        let domain = NavigationDomain {
            id: crate::shared::id::SnowflakeId::new(self.next_id()?),
            domain_code,
            name: command.name,
            description: command.description,
            built_in: false,
            enabled: command.enabled,
            is_default: command.is_default,
            position: command.position,
            visible_roles: command.visible_roles,
            created_at: timestamp.clone(),
            updated_at: timestamp,
        };
        let mut repository = self.lock_repository();
        repository.save_domain(&domain)?;
        Ok(domain)
    }

    pub fn update_domain(
        &self,
        domain_code: &str,
        command: UpdateDomainCommand,
    ) -> Result<NavigationDomain, NavigationError> {
        validate_translations(&command.name)?;
        validate_translations(&command.description)?;
        validate_roles(&command.visible_roles)?;
        if command.position < 0 {
            return Err(NavigationError::InvalidHierarchy);
        }
        let mut repository = self.lock_repository();
        let current = repository.get_domain(domain_code)?;
        if current.built_in && !command.enabled {
            return Err(NavigationError::BuiltInDomainProtected);
        }
        let updated = NavigationDomain {
            updated_at: current_timestamp(),
            name: command.name,
            description: command.description,
            visible_roles: command.visible_roles,
            enabled: command.enabled,
            is_default: command.is_default,
            position: command.position,
            ..current
        };
        repository.save_domain(&updated)?;
        Ok(updated)
    }

    pub fn delete_domain(&self, domain_code: &str) -> Result<(), NavigationError> {
        let mut repository = self.lock_repository();
        let domain = repository.get_domain(domain_code)?;
        if domain.built_in {
            return Err(NavigationError::BuiltInDomainProtected);
        }
        if !repository.list_items(domain_code)?.is_empty() {
            return Err(NavigationError::DomainNotEmpty);
        }
        repository.delete_domain(domain_code)
    }

    pub fn list_menus(
        &self,
        query: MenuQuery,
    ) -> Result<NavigationPage<NavigationMenu>, NavigationError> {
        if query.current < 1 || query.size < 1 || query.size > 200 {
            return Err(NavigationError::InvalidHierarchy);
        }
        let mut repository = self.lock_repository();
        repository.list_menus(&query)
    }

    pub fn create_menu(
        &self,
        command: CreateMenuCommand,
    ) -> Result<NavigationMenu, NavigationError> {
        let menu_code = validate_menu_code(&command.menu_code)?;
        validate_translations(&command.translations)?;
        validate_menu_target(
            command.menu_type,
            command.page_code.as_deref(),
            command.url.as_deref(),
        )?;
        let timestamp = current_timestamp();
        let menu = NavigationMenu {
            id: crate::shared::id::SnowflakeId::new(self.next_id()?),
            menu_code,
            source: MenuSource::Custom,
            menu_type: command.menu_type,
            translations: command.translations,
            page_code: command.page_code.map(|value| value.trim().to_string()),
            url: command.url.map(|value| value.trim().to_string()),
            icon: normalize_optional(command.icon),
            enabled: command.enabled,
            created_at: timestamp.clone(),
            updated_at: timestamp,
        };
        let mut repository = self.lock_repository();
        repository.save_menu(&menu)?;
        Ok(menu)
    }

    pub fn update_menu(
        &self,
        menu_code: &str,
        command: UpdateMenuCommand,
    ) -> Result<NavigationMenu, NavigationError> {
        validate_translations(&command.translations)?;
        validate_menu_target(
            command.menu_type,
            command.page_code.as_deref(),
            command.url.as_deref(),
        )?;
        let mut repository = self.lock_repository();
        let current = repository.get_menu(menu_code)?;
        if current.source == MenuSource::BuiltIn
            && (current.menu_type != command.menu_type
                || current.page_code != command.page_code
                || current.url != command.url
                || current.enabled != command.enabled)
        {
            return Err(NavigationError::BuiltInMenuProtected);
        }
        if current.menu_type != command.menu_type
            && command.menu_type != MenuType::Group
            && repository.menu_has_children(menu_code)?
        {
            return Err(NavigationError::InvalidHierarchy);
        }
        let updated = NavigationMenu {
            menu_type: command.menu_type,
            translations: command.translations,
            page_code: command.page_code.map(|value| value.trim().to_string()),
            url: command.url.map(|value| value.trim().to_string()),
            icon: normalize_optional(command.icon),
            enabled: command.enabled,
            updated_at: current_timestamp(),
            ..current
        };
        repository.save_menu(&updated)?;
        Ok(updated)
    }

    pub fn get_menu(&self, menu_code: &str) -> Result<NavigationMenu, NavigationError> {
        let mut repository = self.lock_repository();
        repository.get_menu(menu_code)
    }

    pub fn delete_menu(&self, menu_code: &str) -> Result<(), NavigationError> {
        let mut repository = self.lock_repository();
        let menu = repository.get_menu(menu_code)?;
        if menu.source == MenuSource::BuiltIn {
            return Err(NavigationError::BuiltInMenuProtected);
        }
        if repository.menu_usage_count(menu_code)? > 0 {
            return Err(NavigationError::MenuInUse);
        }
        repository.delete_menu(menu_code)
    }

    pub fn admin_tree(
        &self,
        domain_code: &str,
    ) -> Result<Vec<NavigationTreeItem>, NavigationError> {
        let mut repository = self.lock_repository();
        repository.get_domain(domain_code)?;
        let items = repository.list_items(domain_code)?;
        validate_flat_tree(&items, self.max_depth)?;
        Ok(build_tree(items))
    }

    pub fn runtime_tree(
        &self,
        domain_code: &str,
        role_codes: &[String],
        locale: &str,
    ) -> Result<RuntimeNavigationTree, NavigationError> {
        let mut repository = self.lock_repository();
        let domain = repository.get_domain(domain_code)?;
        if !domain.enabled || !roles_match(&domain.visible_roles, role_codes) {
            return Err(NavigationError::DomainAccessDenied);
        }
        let items = repository.list_items(domain_code)?;
        validate_flat_tree(&items, self.max_depth)?;
        let visible = build_tree(items)
            .into_iter()
            .filter_map(|item| visible_item(item, role_codes, locale))
            .collect();
        Ok(RuntimeNavigationTree {
            domain_code: domain.domain_code,
            locale: locale.to_string(),
            max_depth: self.max_depth,
            menus: visible,
        })
    }

    pub fn add_item(
        &self,
        domain_code: &str,
        command: AddMenuItemCommand,
    ) -> Result<Vec<NavigationTreeItem>, NavigationError> {
        validate_menu_code(&command.menu_code)?;
        validate_roles(&command.visible_roles)?;
        if command.position < 0 {
            return Err(NavigationError::InvalidHierarchy);
        }
        let mut repository = self.lock_repository();
        repository.get_domain(domain_code)?;
        let menu = repository.get_menu(&command.menu_code)?;
        let items = repository.list_items(domain_code)?;
        validate_flat_tree(&items, self.max_depth)?;
        if items
            .iter()
            .any(|item| item.menu.menu_code == menu.menu_code)
        {
            return Err(NavigationError::DuplicateDomainItem);
        }
        validate_parent(&items, command.parent_menu_code.as_deref(), menu.menu_type)?;
        let depth = proposed_depth(&items, &menu.menu_code, command.parent_menu_code.as_deref())?;
        if depth > self.max_depth {
            return Err(NavigationError::MaxDepthExceeded);
        }
        let item = NavigationTreeItem {
            id: crate::shared::id::SnowflakeId::new(repository.next_id()?),
            menu,
            parent_menu_code: command.parent_menu_code,
            position: command.position,
            visible_roles: command.visible_roles,
            children: Vec::new(),
        };
        repository.add_item(domain_code, &item)?;
        let next = repository.list_items(domain_code)?;
        validate_flat_tree(&next, self.max_depth)?;
        Ok(build_tree(next))
    }

    pub fn update_item(
        &self,
        domain_code: &str,
        menu_code: &str,
        command: UpdateMenuItemCommand,
    ) -> Result<Vec<NavigationTreeItem>, NavigationError> {
        validate_roles(&command.visible_roles)?;
        if command.position < 0 {
            return Err(NavigationError::InvalidHierarchy);
        }
        let mut repository = self.lock_repository();
        repository.get_domain(domain_code)?;
        let items = repository.list_items(domain_code)?;
        let item = items
            .iter()
            .find(|item| item.menu.menu_code == menu_code)
            .ok_or(NavigationError::ItemNotFound)?;
        validate_parent(
            &items,
            command.parent_menu_code.as_deref(),
            item.menu.menu_type,
        )?;
        let mut candidate = items.clone();
        if let Some(candidate_item) = candidate
            .iter_mut()
            .find(|value| value.menu.menu_code == menu_code)
        {
            candidate_item.parent_menu_code = command.parent_menu_code.clone();
        }
        validate_flat_tree(&candidate, self.max_depth)?;
        repository.update_item(domain_code, menu_code, &command)?;
        let next = repository.list_items(domain_code)?;
        Ok(build_tree(next))
    }

    pub fn remove_item(
        &self,
        domain_code: &str,
        menu_code: &str,
    ) -> Result<Vec<NavigationTreeItem>, NavigationError> {
        let mut repository = self.lock_repository();
        repository.get_domain(domain_code)?;
        let items = repository.list_items(domain_code)?;
        items
            .iter()
            .find(|item| item.menu.menu_code == menu_code)
            .ok_or(NavigationError::ItemNotFound)?;
        if items
            .iter()
            .any(|candidate| candidate.parent_menu_code.as_deref() == Some(menu_code))
        {
            return Err(NavigationError::NonLeafItem);
        }
        repository.remove_item(domain_code, menu_code)?;
        Ok(build_tree(repository.list_items(domain_code)?))
    }

    pub fn copy_tree(
        &self,
        target_domain_code: &str,
        command: CopyMenuTreeCommand,
    ) -> Result<Vec<NavigationTreeItem>, NavigationError> {
        if command.position < 0 {
            return Err(NavigationError::InvalidHierarchy);
        }
        let mut repository = self.lock_repository();
        repository.get_domain(target_domain_code)?;
        repository.get_domain(&command.source_domain_code)?;
        let source = repository.list_items(&command.source_domain_code)?;
        let target = repository.list_items(target_domain_code)?;
        validate_flat_tree(&source, self.max_depth)?;
        validate_flat_tree(&target, self.max_depth)?;
        let selected_codes = selected_subtree(&source, command.source_menu_code.as_deref())?;
        let conflicts = selected_codes
            .iter()
            .filter(|code| target.iter().any(|item| item.menu.menu_code == **code))
            .cloned()
            .collect::<Vec<_>>();
        if !conflicts.is_empty() {
            return Err(NavigationError::DuplicateDomainItems(conflicts));
        }
        validate_parent(
            &target,
            command.target_parent_menu_code.as_deref(),
            MenuType::Group,
        )?;
        let root_depth = proposed_depth(
            &target,
            "__COPY_ROOT__",
            command.target_parent_menu_code.as_deref(),
        )?;
        let source_height = selected_codes
            .iter()
            .filter_map(|code| source.iter().find(|item| item.menu.menu_code == *code))
            .map(|item| subtree_height(&source, &item.menu.menu_code))
            .max()
            .unwrap_or(0);
        if root_depth + source_height.saturating_sub(1) > self.max_depth {
            return Err(NavigationError::MaxDepthExceeded);
        }
        repository.copy_tree(target_domain_code, &command)?;
        Ok(build_tree(repository.list_items(target_domain_code)?))
    }

    pub fn restore_default_menu_trees(
        &self,
    ) -> Result<RestoreDefaultMenuTreesResult, NavigationError> {
        let mut repository = self.lock_repository();
        for domain_code in DEFAULT_NAVIGATION_MENU_TREE_DOMAINS {
            repository.get_domain(domain_code)?;
        }
        for item in DEFAULT_NAVIGATION_MENU_TREE_ITEMS {
            repository.get_menu(item.menu_code)?;
        }
        let restored_item_count =
            repository.restore_default_menu_trees(DEFAULT_NAVIGATION_MENU_TREE_ITEMS)?;
        for domain_code in DEFAULT_NAVIGATION_MENU_TREE_DOMAINS {
            validate_flat_tree(&repository.list_items(domain_code)?, self.max_depth)?;
        }
        Ok(RestoreDefaultMenuTreesResult {
            restored_domain_codes: DEFAULT_NAVIGATION_MENU_TREE_DOMAINS
                .iter()
                .map(|domain_code| (*domain_code).to_string())
                .collect(),
            restored_item_count,
        })
    }

    fn next_id(&self) -> Result<i64, NavigationError> {
        let mut repository = self.lock_repository();
        repository.next_id()
    }

    fn lock_repository(&self) -> std::sync::MutexGuard<'_, Box<dyn NavigationRepository>> {
        self.repository
            .lock()
            .expect("navigation repository mutex poisoned")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreDefaultMenuTreesResult {
    pub restored_domain_codes: Vec<String>,
    pub restored_item_count: i64,
}
