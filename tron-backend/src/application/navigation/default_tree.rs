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

// Parent-before-child order matches the initialization seed.
pub const DEFAULT_NAVIGATION_MENU_TREE_ITEMS: &[DefaultNavigationMenuTreeItem] = &[
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "DUDES",
        parent_menu_code: None,
        position: 0,
        child_display: NavigationMenuChildDisplay::Inline,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "CHAT_NEW_SESSION",
        parent_menu_code: None,
        position: 1,
        child_display: NavigationMenuChildDisplay::Inline,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "EXPLORE_MORE",
        parent_menu_code: None,
        position: 2,
        child_display: NavigationMenuChildDisplay::Flyout,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "NOTIFICATIONS",
        parent_menu_code: Some("EXPLORE_MORE"),
        position: 0,
        child_display: NavigationMenuChildDisplay::Inline,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "CORE",
        parent_menu_code: Some("EXPLORE_MORE"),
        position: 1,
        child_display: NavigationMenuChildDisplay::Inline,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "APPS",
        parent_menu_code: Some("CORE"),
        position: 0,
        child_display: NavigationMenuChildDisplay::Inline,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "TASKS",
        parent_menu_code: Some("CORE"),
        position: 1,
        child_display: NavigationMenuChildDisplay::Inline,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "AUTOMATIONS",
        parent_menu_code: Some("CORE"),
        position: 2,
        child_display: NavigationMenuChildDisplay::Inline,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "VARIABLES",
        parent_menu_code: Some("CORE"),
        position: 3,
        child_display: NavigationMenuChildDisplay::Inline,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "AGENT_OPERATIONS",
        parent_menu_code: Some("EXPLORE_MORE"),
        position: 2,
        child_display: NavigationMenuChildDisplay::Inline,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "MCP_SERVERS",
        parent_menu_code: Some("AGENT_OPERATIONS"),
        position: 0,
        child_display: NavigationMenuChildDisplay::Inline,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "DATA",
        parent_menu_code: Some("EXPLORE_MORE"),
        position: 3,
        child_display: NavigationMenuChildDisplay::Inline,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "RESOURCE_MANAGEMENT",
        parent_menu_code: Some("DATA"),
        position: 0,
        child_display: NavigationMenuChildDisplay::Inline,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "FILE_MANAGEMENT",
        parent_menu_code: Some("RESOURCE_MANAGEMENT"),
        position: 0,
        child_display: NavigationMenuChildDisplay::Inline,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "DATA_SOURCE_MANAGEMENT",
        parent_menu_code: Some("RESOURCE_MANAGEMENT"),
        position: 1,
        child_display: NavigationMenuChildDisplay::Inline,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "SERVER_MANAGEMENT",
        parent_menu_code: Some("RESOURCE_MANAGEMENT"),
        position: 2,
        child_display: NavigationMenuChildDisplay::Inline,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "SERVER_ACCOUNT_MANAGEMENT",
        parent_menu_code: Some("RESOURCE_MANAGEMENT"),
        position: 3,
        child_display: NavigationMenuChildDisplay::Inline,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "CHAT_PROJECTS",
        parent_menu_code: None,
        position: 3,
        child_display: NavigationMenuChildDisplay::Inline,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "CHAT_RECENTS",
        parent_menu_code: None,
        position: 4,
        child_display: NavigationMenuChildDisplay::Inline,
    },
];
