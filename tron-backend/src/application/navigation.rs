mod commands;
mod repository;

use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};

use crate::{
    domain::navigation::{
        MenuQuery, MenuSource, MenuType, NavigationDomain, NavigationError,
        NavigationMenu, NavigationPage, NavigationTreeItem, validate_domain_code,
        validate_menu_code, validate_menu_target, validate_roles, validate_translations,
    },
    shared::time::current_timestamp,
};

pub use commands::*;
pub use repository::NavigationRepository;

include!("navigation/service_impl.rs");
include!("navigation/tree.rs");
