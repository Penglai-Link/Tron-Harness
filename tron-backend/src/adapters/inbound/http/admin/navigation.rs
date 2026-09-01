use axum::{
    Json, Router,
    extract::{Path, State},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use serde::{Deserialize, Serialize};

use crate::{
    adapters::inbound::http::{
        auth::RequestAuthContext,
        navigation::{menu_type_name, navigation_error_response},
    },
    app::AppState,
    application::navigation::{
        AddMenuItemCommand, CopyMenuTreeCommand, CreateDomainCommand, CreateMenuCommand,
        UpdateDomainCommand, UpdateMenuCommand, UpdateMenuItemCommand,
    },
    domain::navigation::{
        LocalizedText, MenuQuery, MenuSource, MenuType, NavigationDomain, NavigationError,
        NavigationMenu, NavigationTreeItem,
    },
    response::ApiResponse,
    shared::time::timestamp_to_millis,
};

include!("navigation/requests.rs");
include!("navigation/responses.rs");
include!("navigation/handlers.rs");
