mod commands;
mod default_tree;
mod repository;

use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};

use crate::{
    domain::navigation::{
        LocalizedText, MenuQuery, MenuSource, MenuType, NavigationDomain,
        NavigationError, NavigationMenu, NavigationMenuChildDisplay, NavigationMenuTree,
        NavigationPage, NavigationTreeItem, validate_menu_code,
        validate_menu_target, validate_translations,
    },
    shared::time::current_timestamp,
};

pub use commands::*;
pub use default_tree::{
    DEFAULT_NAVIGATION_MENU_TREE_CODES, DEFAULT_NAVIGATION_MENU_TREE_ITEMS,
    DefaultNavigationMenuTreeItem,
};
pub use repository::NavigationRepository;

include!("navigation/service_impl.rs");
include!("navigation/tree.rs");
