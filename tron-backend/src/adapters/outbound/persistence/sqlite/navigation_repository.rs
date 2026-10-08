use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
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
    shared::time::current_timestamp,
};

use super::migrations::{
    CHAT_PANEL_NAVIGATION_MIGRATION, NAVIGATION_AGENTS_SPLIT_MIGRATION,
    NAVIGATION_MENU_TREES_MIGRATION, NAVIGATION_MIGRATION,
    REMOVE_CHATS_NAVIGATION_MIGRATION, REMOVE_OBSERVABILITY_NAVIGATION_MIGRATION,
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
