mod common;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode, header},
};
use serde_json::{Value, json};
use tower::ServiceExt;

use common::{app, authenticated_bearer_token, response_json};

async fn request_json(
    app: &axum::Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
    token: Option<&str>,
    locale: Option<&str>,
) -> axum::response::Response {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    if let Some(locale) = locale {
        builder = builder.header(header::ACCEPT_LANGUAGE, locale);
    }
    let request_body = if let Some(body) = body {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        Body::from(body.to_string())
    } else {
        Body::empty()
    };
    app.clone()
        .oneshot(
            builder
                .body(request_body)
                .expect("navigation request should build"),
        )
        .await
        .expect("navigation router should return a response")
}

#[tokio::test]
async fn runtime_navigation_is_authenticated_localized_and_separate_from_domain_list() {
    let router = app();
    let anonymous = request_json(
        &router,
        Method::GET,
        "/api/v1/navigation/domains",
        None,
        None,
        None,
    )
    .await;
    assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);

    let token = authenticated_bearer_token(&router).await;
    let domains = request_json(
        &router,
        Method::GET,
        "/api/v1/navigation/domains",
        None,
        Some(&token),
        None,
    )
    .await;
    assert_eq!(domains.status(), StatusCode::OK);
    let domains_payload = response_json(domains).await;
    assert_eq!(domains_payload["code"], "0");
    assert_eq!(domains_payload["data"][0]["domainCode"], "ADVANCED");
    assert_eq!(domains_payload["data"][0]["name"], "高级模式");
    assert!(domains_payload["data"][0].get("menus").is_none());

    let english_domains = request_json(
        &router,
        Method::GET,
        "/api/v1/navigation/domains",
        None,
        Some(&token),
        Some("en-US"),
    )
    .await;
    let english_payload = response_json(english_domains).await;
    assert_eq!(english_payload["data"][0]["name"], "Advanced");

    let tree = request_json(
        &router,
        Method::GET,
        "/api/v1/navigation/domains/ADVANCED/menu-tree",
        None,
        Some(&token),
        None,
    )
    .await;
    let tree_status = tree.status();
    let tree_payload = response_json(tree).await;
    assert_eq!(
        tree_status,
        StatusCode::OK,
        "runtime tree response: {tree_payload}"
    );
    assert_eq!(tree_payload["code"], "0");
    assert_eq!(tree_payload["data"]["domainCode"], "ADVANCED");
    assert_eq!(tree_payload["data"]["locale"], "zh-CN");
    assert_eq!(tree_payload["data"]["maxDepth"], 3);
    assert_eq!(tree_payload["data"]["menus"][0]["menuCode"], "CORE");
    assert_eq!(tree_payload["data"]["menus"][0]["label"], "核心");
    assert_eq!(
        tree_payload["data"]["menus"][0]["children"][0]["pageCode"],
        "apps.index"
    );
}

#[tokio::test]
async fn custom_menu_lifecycle_and_copy_conflicts_follow_navigation_rules() {
    let router = app();
    let token = authenticated_bearer_token(&router).await;

    let create_domain = request_json(
        &router,
        Method::POST,
        "/api/v1/admin/navigation/domains",
        Some(json!({
            "domainCode": "COMPLIANCE",
            "name": { "zh-CN": "合规", "en-US": "Compliance" },
            "description": { "zh-CN": "合规领域", "en-US": "Compliance domain" },
            "visibleRoles": [],
            "enabled": true,
            "isDefault": false,
            "position": 1
        })),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(create_domain.status(), StatusCode::OK);
    assert_eq!(response_json(create_domain).await["code"], "0");

    let create_menu = request_json(
        &router,
        Method::POST,
        "/api/v1/admin/navigation/menus",
        Some(json!({
            "menuCode": "COMPLIANCE_PORTAL",
            "menuType": "ExternalLink",
            "translations": { "zh-CN": "合规门户", "en-US": "Compliance Portal" },
            "pageCode": null,
            "url": "https://example.com/compliance",
            "icon": "Globe",
            "enabled": true
        })),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(create_menu.status(), StatusCode::OK);
    assert_eq!(response_json(create_menu).await["data"]["source"], "Custom");

    let advanced_before_add = request_json(
        &router,
        Method::GET,
        "/api/v1/navigation/domains/ADVANCED/menu-tree",
        None,
        Some(&token),
        None,
    )
    .await;
    let before_payload = response_json(advanced_before_add).await;
    assert!(!before_payload.to_string().contains("COMPLIANCE_PORTAL"));

    let add_menu = request_json(
        &router,
        Method::POST,
        "/api/v1/admin/navigation/domains/ADVANCED/menu-items",
        Some(json!({
            "menuCode": "COMPLIANCE_PORTAL",
            "parentMenuCode": null,
            "position": 0,
            "visibleRoles": ["VIEWER"]
        })),
        Some(&token),
        None,
    )
    .await;
    let add_menu_status = add_menu.status();
    let add_menu_payload = response_json(add_menu).await;
    assert_eq!(
        add_menu_status,
        StatusCode::OK,
        "add menu response: {add_menu_payload}"
    );
    assert_eq!(add_menu_payload["data"][0]["menuCode"], "COMPLIANCE_PORTAL");

    let runtime_after_add = request_json(
        &router,
        Method::GET,
        "/api/v1/navigation/domains/ADVANCED/menu-tree",
        None,
        Some(&token),
        None,
    )
    .await;
    let runtime_after_add_payload = response_json(runtime_after_add).await;
    assert!(
        !runtime_after_add_payload
            .to_string()
            .contains("COMPLIANCE_PORTAL")
    );

    let delete_in_use = request_json(
        &router,
        Method::DELETE,
        "/api/v1/admin/navigation/menus/COMPLIANCE_PORTAL",
        None,
        Some(&token),
        None,
    )
    .await;
    assert_eq!(delete_in_use.status(), StatusCode::CONFLICT);

    let remove_item = request_json(
        &router,
        Method::DELETE,
        "/api/v1/admin/navigation/domains/ADVANCED/menu-items/COMPLIANCE_PORTAL",
        None,
        Some(&token),
        None,
    )
    .await;
    assert_eq!(remove_item.status(), StatusCode::OK);

    let delete_menu = request_json(
        &router,
        Method::DELETE,
        "/api/v1/admin/navigation/menus/COMPLIANCE_PORTAL",
        None,
        Some(&token),
        None,
    )
    .await;
    assert_eq!(delete_menu.status(), StatusCode::OK);

    let protected_menu = request_json(
        &router,
        Method::DELETE,
        "/api/v1/admin/navigation/menus/CORE",
        None,
        Some(&token),
        None,
    )
    .await;
    assert_eq!(protected_menu.status(), StatusCode::CONFLICT);

    let copy = request_json(
        &router,
        Method::POST,
        "/api/v1/admin/navigation/domains/COMPLIANCE/menu-tree/copy",
        Some(json!({
            "sourceDomainCode": "ADVANCED",
            "sourceMenuCode": null,
            "targetParentMenuCode": null,
            "position": 0
        })),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(copy.status(), StatusCode::OK);

    let duplicate_copy = request_json(
        &router,
        Method::POST,
        "/api/v1/admin/navigation/domains/COMPLIANCE/menu-tree/copy",
        Some(json!({
            "sourceDomainCode": "ADVANCED",
            "sourceMenuCode": null,
            "targetParentMenuCode": null,
            "position": 0
        })),
        Some(&token),
        Some("en-US"),
    )
    .await;
    assert_eq!(duplicate_copy.status(), StatusCode::CONFLICT);
    let duplicate_payload = response_json(duplicate_copy).await;
    let duplicate_message = duplicate_payload["message"].as_str().unwrap_or_default();
    assert!(duplicate_message.contains("CORE"));
    assert!(duplicate_message.contains("RESOURCE_MANAGEMENT"));

    let advanced_tree_after_failed_copy = request_json(
        &router,
        Method::GET,
        "/api/v1/admin/navigation/domains/COMPLIANCE/menu-tree",
        None,
        Some(&token),
        None,
    )
    .await;
    let tree_payload = response_json(advanced_tree_after_failed_copy).await;
    assert_eq!(tree_payload["data"].as_array().map(Vec::len), Some(3));
}

#[tokio::test]
async fn adding_and_moving_items_under_a_group_preserves_root_items() {
    let router = app();
    let token = authenticated_bearer_token(&router).await;

    let create_domain = request_json(
        &router,
        Method::POST,
        "/api/v1/admin/navigation/domains",
        Some(json!({
            "domainCode": "ARRANGEMENT",
            "name": { "zh-CN": "编排测试", "en-US": "Arrangement test" },
            "description": { "zh-CN": "编排测试领域", "en-US": "Arrangement test domain" },
            "visibleRoles": [],
            "enabled": true,
            "isDefault": false,
            "position": 1
        })),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(create_domain.status(), StatusCode::OK);

    for (menu_code, menu_type, translations, page_code) in [
        (
            "ARRANGEMENT_GROUP",
            "Group",
            json!({ "zh-CN": "编排分组", "en-US": "Arrangement group" }),
            Value::Null,
        ),
        (
            "ARRANGEMENT_PAGE",
            "Internal",
            json!({ "zh-CN": "编排页面", "en-US": "Arrangement page" }),
            json!("arrangement.index"),
        ),
    ] {
        let create_menu = request_json(
            &router,
            Method::POST,
            "/api/v1/admin/navigation/menus",
            Some(json!({
                "menuCode": menu_code,
                "menuType": menu_type,
                "translations": translations,
                "pageCode": page_code,
                "url": null,
                "icon": "Folder",
                "enabled": true
            })),
            Some(&token),
            None,
        )
        .await;
        assert_eq!(create_menu.status(), StatusCode::OK, "create {menu_code}");
    }

    let add_group = request_json(
        &router,
        Method::POST,
        "/api/v1/admin/navigation/domains/ARRANGEMENT/menu-items",
        Some(json!({
            "menuCode": "ARRANGEMENT_GROUP",
            "parentMenuCode": null,
            "position": 0,
            "visibleRoles": []
        })),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(add_group.status(), StatusCode::OK);

    let add_page = request_json(
        &router,
        Method::POST,
        "/api/v1/admin/navigation/domains/ARRANGEMENT/menu-items",
        Some(json!({
            "menuCode": "ARRANGEMENT_PAGE",
            "parentMenuCode": null,
            "position": 1,
            "visibleRoles": []
        })),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(add_page.status(), StatusCode::OK);

    let move_page = request_json(
        &router,
        Method::PUT,
        "/api/v1/admin/navigation/domains/ARRANGEMENT/menu-items/ARRANGEMENT_PAGE",
        Some(json!({
            "parentMenuCode": "ARRANGEMENT_GROUP",
            "position": 0,
            "visibleRoles": []
        })),
        Some(&token),
        None,
    )
    .await;
    let move_page_status = move_page.status();
    let move_page_payload = response_json(move_page).await;
    assert_eq!(
        move_page_status,
        StatusCode::OK,
        "move item response: {move_page_payload}"
    );
    assert_eq!(
        move_page_payload["data"][0]["menuCode"],
        "ARRANGEMENT_GROUP"
    );
    assert_eq!(
        move_page_payload["data"][0]["children"][0]["menuCode"],
        "ARRANGEMENT_PAGE"
    );
}
