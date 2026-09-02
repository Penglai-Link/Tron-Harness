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
        NavigationMenu, NavigationPage, NavigationTreeItem,
    },
    shared::id::SnowflakeId,
};

use super::migrations::{NAVIGATION_AGENTS_SPLIT_MIGRATION, NAVIGATION_MIGRATION};

const POSITION_WORK_OFFSET: i64 = 1_000_000_000;

pub struct SqliteNavigationRepository {
    connection: Connection,
}

include!("navigation_repository/storage.rs");
include!("navigation_repository/implementation.rs");
