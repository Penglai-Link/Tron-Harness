#[derive(Debug, Clone, Copy)]
pub struct DefaultNavigationMenuTreeItem {
    pub domain_code: &'static str,
    pub menu_code: &'static str,
    pub parent_menu_code: Option<&'static str>,
    pub position: i64,
}

pub const DEFAULT_NAVIGATION_MENU_TREE_CODES: &[&str] = &["ADVANCED", "CHAT_PANEL"];

// Keep this in the same parent-before-child order as the built-in navigation seed.
// It provides a runtime recovery path when an administrator removes all assignments.
pub const DEFAULT_NAVIGATION_MENU_TREE_ITEMS: &[DefaultNavigationMenuTreeItem] = &[
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "CORE",
        parent_menu_code: None,
        position: 0,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "APPS",
        parent_menu_code: Some("CORE"),
        position: 0,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "TASKS",
        parent_menu_code: Some("CORE"),
        position: 1,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "AUTOMATIONS",
        parent_menu_code: Some("CORE"),
        position: 2,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "AGENT_OPERATIONS",
        parent_menu_code: None,
        position: 1,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "AGENTS",
        parent_menu_code: Some("AGENT_OPERATIONS"),
        position: 0,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "INTERNAL_AGENTS",
        parent_menu_code: Some("AGENTS"),
        position: 0,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "SYSTEM_AGENTS",
        parent_menu_code: Some("AGENTS"),
        position: 1,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "MCP_SERVERS",
        parent_menu_code: Some("AGENT_OPERATIONS"),
        position: 1,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "PROMPTS",
        parent_menu_code: Some("AGENT_OPERATIONS"),
        position: 2,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "MODELS",
        parent_menu_code: Some("AGENT_OPERATIONS"),
        position: 3,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "SKILLS",
        parent_menu_code: Some("AGENT_OPERATIONS"),
        position: 4,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "DATA",
        parent_menu_code: None,
        position: 2,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "MEMORY",
        parent_menu_code: Some("DATA"),
        position: 0,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "KNOWLEDGE_GRAPH",
        parent_menu_code: Some("DATA"),
        position: 1,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "KNOWLEDGE_BASE",
        parent_menu_code: Some("DATA"),
        position: 2,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "RAG",
        parent_menu_code: Some("DATA"),
        position: 3,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "RESOURCE_MANAGEMENT",
        parent_menu_code: Some("DATA"),
        position: 4,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "FILE_MANAGEMENT",
        parent_menu_code: Some("RESOURCE_MANAGEMENT"),
        position: 0,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "DATA_SOURCE_MANAGEMENT",
        parent_menu_code: Some("RESOURCE_MANAGEMENT"),
        position: 1,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "SERVER_MANAGEMENT",
        parent_menu_code: Some("RESOURCE_MANAGEMENT"),
        position: 2,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "SERVER_ACCOUNT_MANAGEMENT",
        parent_menu_code: Some("RESOURCE_MANAGEMENT"),
        position: 3,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "ADVANCED",
        menu_code: "VARIABLES",
        parent_menu_code: Some("DATA"),
        position: 5,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "CHAT_PANEL",
        menu_code: "CHAT_SESSIONS",
        parent_menu_code: None,
        position: 0,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "CHAT_PANEL",
        menu_code: "CHAT_NEW_SESSION",
        parent_menu_code: Some("CHAT_SESSIONS"),
        position: 0,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "CHAT_PANEL",
        menu_code: "CHAT_PROJECTS",
        parent_menu_code: Some("CHAT_SESSIONS"),
        position: 1,
    },
    DefaultNavigationMenuTreeItem {
        domain_code: "CHAT_PANEL",
        menu_code: "CHAT_RECENTS",
        parent_menu_code: Some("CHAT_SESSIONS"),
        position: 2,
    },
];
