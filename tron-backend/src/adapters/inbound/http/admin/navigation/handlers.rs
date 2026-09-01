pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/admin/navigation/domains/query",
            post(query_domains),
        )
        .route("/api/v1/admin/navigation/domains", post(create_domain))
        .route(
            "/api/v1/admin/navigation/domains/{domain_code}",
            get(get_domain).put(update_domain).delete(delete_domain),
        )
        .route("/api/v1/admin/navigation/menus/query", post(query_menus))
        .route("/api/v1/admin/navigation/menus", post(create_menu))
        .route(
            "/api/v1/admin/navigation/menus/{menu_code}",
            get(get_menu).put(update_menu).delete(delete_menu),
        )
        .route(
            "/api/v1/admin/navigation/domains/{domain_code}/menu-tree",
            get(admin_tree),
        )
        .route(
            "/api/v1/admin/navigation/domains/{domain_code}/menu-items",
            post(add_item),
        )
        .route(
            "/api/v1/admin/navigation/domains/{domain_code}/menu-items/{menu_code}",
            put(update_item).delete(remove_item),
        )
        .route(
            "/api/v1/admin/navigation/domains/{target_domain_code}/menu-tree/copy",
            post(copy_tree),
        )
}

async fn query_domains(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Json(request): Json<DomainQueryRequest>,
) -> Response {
    if let Err(response) = ensure_permission(&state, &auth, "R") {
        return response;
    }
    if request.current < 1 || request.size < 1 || request.size > 200 {
        return navigation_error_response(NavigationError::InvalidHierarchy, &auth.locale);
    }
    match state.navigation.list_domains() {
        Ok(items) => {
            let keyword = request.keyword.to_ascii_lowercase();
            let filtered = items
                .into_iter()
                .filter(|item| {
                    request
                        .enabled
                        .is_none_or(|enabled| item.enabled == enabled)
                        && (keyword.is_empty()
                            || item.domain_code.to_ascii_lowercase().contains(&keyword)
                            || item.name.zh_cn.contains(&request.keyword)
                            || item.name.en_us.to_ascii_lowercase().contains(&keyword))
                })
                .collect::<Vec<_>>();
            let total = filtered.len() as i64;
            let start = ((request.current.max(1) - 1) * request.size.max(1)) as usize;
            let records = filtered
                .into_iter()
                .skip(start)
                .take(request.size.max(1) as usize)
                .map(domain_vo)
                .collect::<Vec<_>>();
            Json(ApiResponse::protocol_success(serde_json::json!({ "current": request.current, "size": request.size, "total": total, "records": records }))).into_response()
        }
        Err(error) => navigation_error_response(error, &auth.locale),
    }
}

async fn create_domain(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Json(request): Json<CreateDomainRequest>,
) -> Response {
    if let Err(response) = ensure_permission(&state, &auth, "W") {
        return response;
    }
    let command = CreateDomainCommand {
        domain_code: request.domain_code,
        name: request_to_localized(request.name),
        description: request_to_localized(request.description),
        visible_roles: request.visible_roles,
        enabled: request.enabled,
        is_default: request.is_default,
        position: request.position,
    };
    match state.navigation.create_domain(command) {
        Ok(item) => Json(ApiResponse::protocol_success(domain_vo(item))).into_response(),
        Err(error) => navigation_error_response(error, &auth.locale),
    }
}

async fn get_domain(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(domain_code): Path<String>,
) -> Response {
    if let Err(response) = ensure_permission(&state, &auth, "R") {
        return response;
    }
    match state.navigation.get_domain(&domain_code) {
        Ok(item) => Json(ApiResponse::protocol_success(domain_vo(item))).into_response(),
        Err(error) => navigation_error_response(error, &auth.locale),
    }
}

async fn update_domain(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(domain_code): Path<String>,
    Json(request): Json<UpdateDomainRequest>,
) -> Response {
    if let Err(response) = ensure_permission(&state, &auth, "W") {
        return response;
    }
    let command = UpdateDomainCommand {
        name: request_to_localized(request.name),
        description: request_to_localized(request.description),
        visible_roles: request.visible_roles,
        enabled: request.enabled,
        is_default: request.is_default,
        position: request.position,
    };
    match state.navigation.update_domain(&domain_code, command) {
        Ok(item) => Json(ApiResponse::protocol_success(domain_vo(item))).into_response(),
        Err(error) => navigation_error_response(error, &auth.locale),
    }
}

async fn delete_domain(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(domain_code): Path<String>,
) -> Response {
    if let Err(response) = ensure_permission(&state, &auth, "D") {
        return response;
    }
    match state.navigation.delete_domain(&domain_code) {
        Ok(()) => Json(ApiResponse::protocol_success(serde_json::json!({}))).into_response(),
        Err(error) => navigation_error_response(error, &auth.locale),
    }
}

async fn query_menus(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Json(request): Json<MenuQueryRequest>,
) -> Response {
    if let Err(response) = ensure_permission(&state, &auth, "R") {
        return response;
    }
    let query = MenuQuery {
        current: request.current,
        size: request.size,
        keyword: request.keyword,
        source: request.source.map(Into::into),
        menu_type: request.menu_type.map(Into::into),
        enabled: request.enabled,
    };
    match state.navigation.list_menus(query) { Ok(page) => Json(ApiResponse::protocol_success(serde_json::json!({ "current": page.current, "size": page.size, "total": page.total, "records": page.records.into_iter().map(menu_vo).collect::<Vec<_>>() }))).into_response(), Err(error) => navigation_error_response(error, &auth.locale) }
}

async fn create_menu(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Json(request): Json<CreateMenuRequest>,
) -> Response {
    if let Err(response) = ensure_permission(&state, &auth, "W") {
        return response;
    }
    let command = CreateMenuCommand {
        menu_code: request.menu_code,
        menu_type: request.menu_type.into(),
        translations: request_to_localized(request.translations),
        page_code: request.page_code,
        url: request.url,
        icon: request.icon,
        enabled: request.enabled,
    };
    match state.navigation.create_menu(command) {
        Ok(item) => Json(ApiResponse::protocol_success(menu_vo(item))).into_response(),
        Err(error) => navigation_error_response(error, &auth.locale),
    }
}

async fn get_menu(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(menu_code): Path<String>,
) -> Response {
    if let Err(response) = ensure_permission(&state, &auth, "R") {
        return response;
    }
    match state.navigation.get_menu(&menu_code) {
        Ok(item) => Json(ApiResponse::protocol_success(menu_vo(item))).into_response(),
        Err(error) => navigation_error_response(error, &auth.locale),
    }
}

async fn update_menu(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(menu_code): Path<String>,
    Json(request): Json<UpdateMenuRequest>,
) -> Response {
    if let Err(response) = ensure_permission(&state, &auth, "W") {
        return response;
    }
    let command = UpdateMenuCommand {
        menu_type: request.menu_type.into(),
        translations: request_to_localized(request.translations),
        page_code: request.page_code,
        url: request.url,
        icon: request.icon,
        enabled: request.enabled,
    };
    match state.navigation.update_menu(&menu_code, command) {
        Ok(item) => Json(ApiResponse::protocol_success(menu_vo(item))).into_response(),
        Err(error) => navigation_error_response(error, &auth.locale),
    }
}

async fn delete_menu(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(menu_code): Path<String>,
) -> Response {
    if let Err(response) = ensure_permission(&state, &auth, "D") {
        return response;
    }
    match state.navigation.delete_menu(&menu_code) {
        Ok(()) => Json(ApiResponse::protocol_success(serde_json::json!({}))).into_response(),
        Err(error) => navigation_error_response(error, &auth.locale),
    }
}

async fn admin_tree(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(domain_code): Path<String>,
) -> Response {
    if let Err(response) = ensure_permission(&state, &auth, "R") {
        return response;
    }
    match state.navigation.admin_tree(&domain_code) {
        Ok(items) => Json(ApiResponse::protocol_success(
            items.into_iter().map(tree_item_vo).collect::<Vec<_>>(),
        ))
        .into_response(),
        Err(error) => navigation_error_response(error, &auth.locale),
    }
}

async fn add_item(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(domain_code): Path<String>,
    Json(request): Json<AddMenuItemRequest>,
) -> Response {
    if let Err(response) = ensure_permission(&state, &auth, "W") {
        return response;
    }
    let command = AddMenuItemCommand {
        menu_code: request.menu_code,
        parent_menu_code: request.parent_menu_code,
        position: request.position,
        visible_roles: request.visible_roles,
    };
    match state.navigation.add_item(&domain_code, command) {
        Ok(items) => Json(ApiResponse::protocol_success(
            items.into_iter().map(tree_item_vo).collect::<Vec<_>>(),
        ))
        .into_response(),
        Err(error) => navigation_error_response(error, &auth.locale),
    }
}

async fn update_item(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path((domain_code, menu_code)): Path<(String, String)>,
    Json(request): Json<UpdateMenuItemRequest>,
) -> Response {
    if let Err(response) = ensure_permission(&state, &auth, "W") {
        return response;
    }
    let command = UpdateMenuItemCommand {
        parent_menu_code: request.parent_menu_code,
        position: request.position,
        visible_roles: request.visible_roles,
    };
    match state
        .navigation
        .update_item(&domain_code, &menu_code, command)
    {
        Ok(items) => Json(ApiResponse::protocol_success(
            items.into_iter().map(tree_item_vo).collect::<Vec<_>>(),
        ))
        .into_response(),
        Err(error) => navigation_error_response(error, &auth.locale),
    }
}

async fn remove_item(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path((domain_code, menu_code)): Path<(String, String)>,
) -> Response {
    if let Err(response) = ensure_permission(&state, &auth, "D") {
        return response;
    }
    match state.navigation.remove_item(&domain_code, &menu_code) {
        Ok(items) => Json(ApiResponse::protocol_success(
            items.into_iter().map(tree_item_vo).collect::<Vec<_>>(),
        ))
        .into_response(),
        Err(error) => navigation_error_response(error, &auth.locale),
    }
}

async fn copy_tree(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(target_domain_code): Path<String>,
    Json(request): Json<CopyMenuTreeRequest>,
) -> Response {
    if let Err(response) = ensure_permission(&state, &auth, "A") {
        return response;
    }
    let command = CopyMenuTreeCommand {
        source_domain_code: request.source_domain_code,
        source_menu_code: request.source_menu_code,
        target_parent_menu_code: request.target_parent_menu_code,
        position: request.position,
    };
    match state.navigation.copy_tree(&target_domain_code, command) {
        Ok(items) => Json(ApiResponse::protocol_success(
            items.into_iter().map(tree_item_vo).collect::<Vec<_>>(),
        ))
        .into_response(),
        Err(error) => navigation_error_response(error, &auth.locale),
    }
}

fn ensure_permission(
    state: &AppState,
    auth: &RequestAuthContext,
    permission: &str,
) -> Result<(), Response> {
    if auth.user().role_codes.iter().any(|role| role == "ADMIN") {
        return Ok(());
    }
    match state
        .roles
        .has_permission(&auth.user().role_codes, "Navigation", permission)
    {
        Ok(true) => Ok(()),
        Ok(false) | Err(_) => Err(navigation_error_response(
            NavigationError::ManagementAccessDenied,
            &auth.locale,
        )),
    }
}

fn request_to_localized(value: LocalizedTextRequest) -> LocalizedText {
    LocalizedText {
        zh_cn: value.zh_cn,
        en_us: value.en_us,
    }
}

fn domain_vo(domain: NavigationDomain) -> DomainVo {
    DomainVo {
        domain_code: domain.domain_code,
        name: localized_vo(domain.name),
        description: localized_vo(domain.description),
        built_in: domain.built_in,
        enabled: domain.enabled,
        is_default: domain.is_default,
        position: domain.position,
        visible_roles: domain.visible_roles,
        created_at: timestamp_to_millis(&domain.created_at),
        updated_at: timestamp_to_millis(&domain.updated_at),
    }
}

fn menu_vo(menu: NavigationMenu) -> MenuVo {
    MenuVo {
        menu_code: menu.menu_code,
        source: source_name(menu.source).into(),
        menu_type: menu_type_name(menu.menu_type).into(),
        translations: localized_vo(menu.translations),
        page_code: menu.page_code,
        url: menu.url,
        icon: menu.icon,
        enabled: menu.enabled,
        created_at: timestamp_to_millis(&menu.created_at),
        updated_at: timestamp_to_millis(&menu.updated_at),
    }
}

fn tree_item_vo(item: NavigationTreeItem) -> MenuTreeItemVo {
    MenuTreeItemVo {
        menu_code: item.menu.menu_code,
        source: source_name(item.menu.source).into(),
        menu_type: menu_type_name(item.menu.menu_type).into(),
        translations: localized_vo(item.menu.translations),
        page_code: item.menu.page_code,
        url: item.menu.url,
        icon: item.menu.icon,
        enabled: item.menu.enabled,
        position: item.position,
        visible_roles: item.visible_roles,
        children: item.children.into_iter().map(tree_item_vo).collect(),
    }
}

fn localized_vo(value: LocalizedText) -> LocalizedTextVo {
    LocalizedTextVo {
        zh_cn: value.zh_cn,
        en_us: value.en_us,
    }
}

fn source_name(source: MenuSource) -> &'static str {
    match source {
        MenuSource::BuiltIn => "BuiltIn",
        MenuSource::Custom => "Custom",
    }
}
