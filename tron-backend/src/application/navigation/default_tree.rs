use crate::domain::navigation::NavigationMenuChildDisplay;

#[derive(Debug, Clone, Copy)]
pub struct DefaultNavigationMenuTreeItem {
    pub domain_code: &'static str,
    pub menu_code: &'static str,
    pub parent_menu_code: Option<&'static str>,
    pub position: i64,
    pub child_display: NavigationMenuChildDisplay,
}

pub const DEFAULT_NAVIGATION_MENU_TREE_CODES: &[&str] = &["ADVANCED"];

// Daily Work defaults mirror its curated menu arrangement. Keep parents before children.
pub const DEFAULT_NAVIGATION_MENU_TREE_ITEMS: &[DefaultNavigationMenuTreeItem] = &[
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "ASSISTANTS",
        parent_menu_code: None,
        position: 0,
        child_display: NavigationMenuChildDisplay::Inline,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "TASKS",
        parent_menu_code: None,
        position: 1,
        child_display: NavigationMenuChildDisplay::Inline,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "AUTOMATIONS",
        parent_menu_code: None,
        position: 2,
        child_display: NavigationMenuChildDisplay::Inline,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "EXPLORE_MORE",
        parent_menu_code: None,
        position: 3,
        child_display: NavigationMenuChildDisplay::Flyout,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "APPS",
        parent_menu_code: Some("EXPLORE_MORE"),
        position: 0,
        child_display: NavigationMenuChildDisplay::Inline,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "NOTIFICATIONS",
        parent_menu_code: Some("EXPLORE_MORE"),
        position: 1,
        child_display: NavigationMenuChildDisplay::Inline,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "VARIABLES",
        parent_menu_code: Some("EXPLORE_MORE"),
        position: 2,
        child_display: NavigationMenuChildDisplay::Inline,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "MCP_SERVERS",
        parent_menu_code: Some("EXPLORE_MORE"),
        position: 3,
        child_display: NavigationMenuChildDisplay::Inline,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "FILE_MANAGEMENT",
        parent_menu_code: Some("EXPLORE_MORE"),
        position: 4,
        child_display: NavigationMenuChildDisplay::Inline,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "CHAT_PROJECTS",
        parent_menu_code: None,
        position: 4,
        child_display: NavigationMenuChildDisplay::Inline,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "CHAT_DIRECTORIES",
        parent_menu_code: None,
        position: 5,
        child_display: NavigationMenuChildDisplay::Inline,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "CHAT_RECENTS",
        parent_menu_code: None,
        position: 6,
        child_display: NavigationMenuChildDisplay::Inline,
    },
];
