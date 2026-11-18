use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Serialize;

use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    app::AppState,
    application::navigation::{RuntimeMenu, RuntimeNavigationTree},
    domain::navigation::{MenuType, NavigationDomain, NavigationError},
    response::{ApiResponse, EmptyData},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NavigationDomainOptionVo {
    domain_code: String,
    name: String,
    description: String,
    built_in: bool,
    is_default: bool,
    position: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeNavigationTreeVo {
    domain_code: String,
    locale: String,
    max_depth: usize,
    menus: Vec<RuntimeMenuVo>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeMenuVo {
    menu_code: String,
    label: String,
    menu_type: String,
    icon: Option<String>,
    page_code: Option<String>,
    url: Option<String>,
    child_display: String,
    children: Vec<RuntimeMenuVo>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/navigation/domains", get(list_domains))
        .route(
            "/api/v1/navigation/domains/{domain_code}/menu-tree",
            get(menu_tree),
        )
}

async fn list_domains(State(state): State<AppState>, auth: RequestAuthContext) -> Response {
    match state.navigation.list_visible_domains(&auth.locale) {
        Ok(domains) => Json(ApiResponse::protocol_success(
            domains
                .into_iter()
                .map(|domain| domain_option_vo(domain, &auth.locale))
                .collect::<Vec<_>>(),
        ))
        .into_response(),
        Err(error) => navigation_error_response(error, &auth.locale),
    }
}

async fn menu_tree(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(domain_code): Path<String>,
) -> Response {
    match state.navigation.runtime_tree(&domain_code, &auth.locale) {
        Ok(tree) => Json(ApiResponse::protocol_success(runtime_tree_vo(tree))).into_response(),
        Err(error) => navigation_error_response(error, &auth.locale),
    }
}

fn domain_option_vo(domain: NavigationDomain, locale: &str) -> NavigationDomainOptionVo {
    NavigationDomainOptionVo {
        domain_code: domain.domain_code,
        name: domain.name.value(locale).to_string(),
        description: domain.description.value(locale).to_string(),
        built_in: domain.built_in,
        is_default: domain.is_default,
        position: domain.position,
    }
}

fn runtime_tree_vo(tree: RuntimeNavigationTree) -> RuntimeNavigationTreeVo {
    RuntimeNavigationTreeVo {
        domain_code: tree.domain_code,
        locale: tree.locale,
        max_depth: tree.max_depth,
        menus: tree.menus.into_iter().map(runtime_menu_vo).collect(),
    }
}

fn runtime_menu_vo(menu: RuntimeMenu) -> RuntimeMenuVo {
    RuntimeMenuVo {
        menu_code: menu.menu_code,
        label: menu.label,
        menu_type: menu_type_name(menu.menu_type).into(),
        icon: menu.icon,
        page_code: menu.page_code,
        url: menu.url,
        child_display: child_display_name(menu.child_display).into(),
        children: menu.children.into_iter().map(runtime_menu_vo).collect(),
    }
}

pub(crate) fn menu_type_name(menu_type: MenuType) -> &'static str {
    match menu_type {
        MenuType::Group => "Group",
        MenuType::Internal => "Internal",
        MenuType::PackPage => "PackPage",
        MenuType::ExternalLink => "ExternalLink",
        MenuType::Widget => "Widget",
    }
}

fn child_display_name(
    display: crate::domain::navigation::NavigationMenuChildDisplay,
) -> &'static str {
    match display {
        crate::domain::navigation::NavigationMenuChildDisplay::Inline => "Inline",
        crate::domain::navigation::NavigationMenuChildDisplay::Flyout => "Flyout",
    }
}

pub(crate) fn navigation_error_response(error: NavigationError, locale: &str) -> Response {
    if let NavigationError::MaxDepthExceeded { depth, max_depth } = &error {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<EmptyData>::protocol_error(
                "NAVIGATION_MAX_DEPTH_EXCEEDED",
                tron_i18n::t!(
                    "navigation.max_depth_exceeded",
                    locale = locale,
                    depth = depth,
                    max_depth = max_depth
                ),
            )),
        )
            .into_response();
    }
    if let NavigationError::DuplicateDomainItems(codes) = error {
        let codes = codes.join(", ");
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::<EmptyData>::protocol_error(
                "NAVIGATION_DUPLICATE",
                tron_i18n::t!("navigation.duplicate_items", locale = locale, codes = codes),
            )),
        )
            .into_response();
    }
    let (status, code, message_key) = match error {
        NavigationError::DomainNotFound
        | NavigationError::MenuNotFound
        | NavigationError::ItemNotFound => (
            StatusCode::NOT_FOUND,
            "NAVIGATION_NOT_FOUND",
            "navigation.not_found",
        ),
        NavigationError::DomainAccessDenied => (
            StatusCode::FORBIDDEN,
            "NAVIGATION_DOMAIN_FORBIDDEN",
            "navigation.domain_forbidden",
        ),
        NavigationError::ManagementAccessDenied => (
            StatusCode::FORBIDDEN,
            "NAVIGATION_MANAGEMENT_FORBIDDEN",
            "navigation.management_forbidden",
        ),
        NavigationError::DuplicateDomainCode
        | NavigationError::DuplicateMenuCode
        | NavigationError::DuplicateDomainItem
        | NavigationError::DuplicateDomainItems(_) => (
            StatusCode::CONFLICT,
            "NAVIGATION_DUPLICATE",
            "navigation.duplicate_reference",
        ),
        NavigationError::CapabilityOwned
        | NavigationError::BuiltInMenuProtected
        | NavigationError::BuiltInDomainProtected
        | NavigationError::MenuInUse
        | NavigationError::DomainNotEmpty
        | NavigationError::NonLeafItem => (
            StatusCode::CONFLICT,
            "NAVIGATION_PROTECTED",
            "navigation.protected",
        ),
        NavigationError::StorageFailure => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "NAVIGATION_STORAGE_FAILURE",
            "navigation.storage_failure",
        ),
        _ => (
            StatusCode::BAD_REQUEST,
            "NAVIGATION_REQUEST_INVALID",
            "navigation.request_invalid",
        ),
    };
    (
        status,
        Json(ApiResponse::<EmptyData>::protocol_error(
            code,
            tron_i18n::t!(message_key, locale = locale),
        )),
    )
        .into_response()
}
