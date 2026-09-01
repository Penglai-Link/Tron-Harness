#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeNavigationTree {
    pub domain_code: String,
    pub locale: String,
    pub max_depth: usize,
    pub menus: Vec<RuntimeMenu>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeMenu {
    pub menu_code: String,
    pub label: String,
    pub menu_type: MenuType,
    pub icon: Option<String>,
    pub page_code: Option<String>,
    pub url: Option<String>,
    pub children: Vec<RuntimeMenu>,
}

fn visible_item(
    item: NavigationTreeItem,
    role_codes: &[String],
    locale: &str,
) -> Option<RuntimeMenu> {
    if !item.menu.enabled || !roles_match(&item.visible_roles, role_codes) {
        return None;
    }
    let children = item
        .children
        .into_iter()
        .filter_map(|child| visible_item(child, role_codes, locale))
        .collect::<Vec<_>>();
    if item.menu.menu_type == MenuType::Group && children.is_empty() {
        return None;
    }
    Some(RuntimeMenu {
        menu_code: item.menu.menu_code,
        label: item.menu.translations.value(locale).to_string(),
        menu_type: item.menu.menu_type,
        icon: item.menu.icon,
        page_code: item.menu.page_code,
        url: item.menu.url,
        children,
    })
}

fn roles_match(required: &[String], actual: &[String]) -> bool {
    required.is_empty()
        || required
            .iter()
            .any(|role| actual.iter().any(|value| value == role))
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn build_tree(items: Vec<NavigationTreeItem>) -> Vec<NavigationTreeItem> {
    fn children(parent: Option<&str>, items: &[NavigationTreeItem]) -> Vec<NavigationTreeItem> {
        let mut result = items
            .iter()
            .filter(|item| item.parent_menu_code.as_deref() == parent)
            .cloned()
            .collect::<Vec<_>>();
        result.sort_by_key(|item| item.position);
        for item in &mut result {
            item.children = children(Some(&item.menu.menu_code), items);
        }
        result
    }
    children(None, &items)
}

fn validate_flat_tree(
    items: &[NavigationTreeItem],
    max_depth: usize,
) -> Result<(), NavigationError> {
    let codes = items
        .iter()
        .map(|item| item.menu.menu_code.as_str())
        .collect::<BTreeSet<_>>();
    if codes.len() != items.len() {
        return Err(NavigationError::DuplicateDomainItem);
    }
    for item in items {
        if let Some(parent) = item.parent_menu_code.as_deref() {
            if parent == item.menu.menu_code || !codes.contains(parent) {
                return Err(NavigationError::InvalidParent);
            }
        }
        if item.menu.menu_type != MenuType::Group
            && items.iter().any(|child| {
                child.parent_menu_code.as_deref() == Some(item.menu.menu_code.as_str())
            })
        {
            return Err(NavigationError::InvalidHierarchy);
        }
        if item.position < 0 {
            return Err(NavigationError::InvalidHierarchy);
        }
        let _ = depth_of(items, &item.menu.menu_code)?;
        if depth_of(items, &item.menu.menu_code)? > max_depth {
            return Err(NavigationError::MaxDepthExceeded);
        }
    }
    Ok(())
}

fn depth_of(items: &[NavigationTreeItem], code: &str) -> Result<usize, NavigationError> {
    let mut depth = 1;
    let mut current = code;
    let mut visited = BTreeSet::new();
    while let Some(item) = items.iter().find(|item| item.menu.menu_code == current) {
        if !visited.insert(current) {
            return Err(NavigationError::InvalidHierarchy);
        }
        if let Some(parent) = item.parent_menu_code.as_deref() {
            depth += 1;
            current = parent;
        } else {
            break;
        }
    }
    Ok(depth)
}

fn validate_parent(
    items: &[NavigationTreeItem],
    parent: Option<&str>,
    _child_type: MenuType,
) -> Result<(), NavigationError> {
    let Some(parent) = parent else {
        return Ok(());
    };
    let parent_item = items
        .iter()
        .find(|item| item.menu.menu_code == parent)
        .ok_or(NavigationError::InvalidParent)?;
    if parent_item.menu.menu_type != MenuType::Group {
        return Err(NavigationError::InvalidParent);
    }
    Ok(())
}

fn proposed_depth(
    items: &[NavigationTreeItem],
    _code: &str,
    parent: Option<&str>,
) -> Result<usize, NavigationError> {
    parent
        .map(|value| depth_of(items, value).map(|depth| depth + 1))
        .unwrap_or(Ok(1))
}

fn subtree_height(items: &[NavigationTreeItem], root: &str) -> usize {
    items
        .iter()
        .filter(|item| item.parent_menu_code.as_deref() == Some(root))
        .map(|item| subtree_height(items, &item.menu.menu_code))
        .max()
        .unwrap_or(0)
        + 1
}

fn selected_subtree(
    items: &[NavigationTreeItem],
    root: Option<&str>,
) -> Result<Vec<String>, NavigationError> {
    let mut result = Vec::new();
    fn visit(items: &[NavigationTreeItem], code: &str, result: &mut Vec<String>) {
        result.push(code.to_string());
        let mut children = items
            .iter()
            .filter(|item| item.parent_menu_code.as_deref() == Some(code))
            .collect::<Vec<_>>();
        children.sort_by_key(|item| item.position);
        for child in children {
            visit(items, &child.menu.menu_code, result);
        }
    }
    if let Some(root) = root {
        if !items.iter().any(|item| item.menu.menu_code == root) {
            return Err(NavigationError::ItemNotFound);
        }
        visit(items, root, &mut result);
    } else {
        items
            .iter()
            .filter(|item| item.parent_menu_code.is_none())
            .collect::<Vec<_>>()
            .into_iter()
            .for_each(|item| visit(items, &item.menu.menu_code, &mut result));
    }
    Ok(result)
}
