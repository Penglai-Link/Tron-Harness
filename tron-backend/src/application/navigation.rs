mod commands;
mod default_tree;
mod repository;

use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};

use crate::{
    domain::navigation::{
        MenuQuery, MenuSource, MenuType, NavigationDomain, NavigationError, NavigationMenu,
        NavigationPage, NavigationTreeItem, PANEL_DOMAIN_CODES, validate_domain_code,
        validate_menu_code, validate_menu_target, validate_roles, validate_translations,
    },
    shared::time::current_timestamp,
};

pub use commands::*;
pub use default_tree::{
    DEFAULT_NAVIGATION_MENU_TREE_DOMAINS, DEFAULT_NAVIGATION_MENU_TREE_ITEMS,
    DefaultNavigationMenuTreeItem,
};
pub use repository::NavigationRepository;

include!("navigation/service_impl.rs");
include!("navigation/tree.rs");
