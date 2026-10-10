use std::{
    collections::{BTreeMap, BTreeSet},
};

use rusqlite::{Connection, ErrorCode, OptionalExtension, params};

use crate::{
    application::navigation::{
        CopyMenuTreeCommand, DefaultNavigationMenuTreeItem, NavigationRepository,
        UpdateMenuItemCommand,
    },
    domain::navigation::{
        LocalizedText, MenuQuery, MenuSource, MenuType, NavigationDomain, NavigationError,
        NavigationMenu, NavigationMenuChildDisplay, NavigationMenuTree, NavigationMenuTreeKind,
        NavigationPage, NavigationTreeItem,
    },
    shared::id::SnowflakeId,
};

const POSITION_WORK_OFFSET: i64 = 1_000_000_000;

struct SqliteNavigationRepositorySql<'a> {
    connection: &'a mut Connection,
}
pub struct SqliteNavigationRepository {
    connection: crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase,
}

include!("navigation_repository/storage.rs");
include!("navigation_repository/implementation.rs");
