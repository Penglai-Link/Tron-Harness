use crate::domain::navigation::{LocalizedText, MenuType, NavigationMenuChildDisplay};

#[derive(Debug, Clone)]
pub struct CreateDomainCommand {
    pub name: LocalizedText,
    pub description: LocalizedText,
    pub enabled: bool,
    pub is_default: bool,
}

#[derive(Debug, Clone)]
pub struct UpdateDomainCommand {
    pub name: LocalizedText,
    pub description: LocalizedText,
    pub enabled: bool,
    pub is_default: bool,
    pub position: i64,
}

#[derive(Debug, Clone)]
pub struct CreateMenuCommand {
    pub menu_code: String,
    pub menu_type: MenuType,
    pub translations: LocalizedText,
    pub description: LocalizedText,
    pub page_code: Option<String>,
    pub url: Option<String>,
    pub icon: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct UpdateMenuCommand {
    pub menu_type: MenuType,
    pub translations: LocalizedText,
    pub description: LocalizedText,
    pub page_code: Option<String>,
    pub url: Option<String>,
    pub icon: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct AddMenuItemCommand {
    pub menu_code: String,
    pub parent_menu_code: Option<String>,
    pub position: i64,
}

#[derive(Debug, Clone)]
pub struct UpdateMenuItemCommand {
    pub parent_menu_code: Option<String>,
    pub position: i64,
    pub child_display: Option<NavigationMenuChildDisplay>,
}

#[derive(Debug, Clone)]
pub struct CopyMenuTreeCommand {
    pub source_domain_code: String,
    pub source_menu_code: Option<String>,
    pub target_parent_menu_code: Option<String>,
    pub position: i64,
}
