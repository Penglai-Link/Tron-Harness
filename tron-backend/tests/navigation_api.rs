mod common;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode, header},
};
use serde_json::{Value, json};
use tower::ServiceExt;

use common::{app, authenticated_bearer_token, response_json};
use tron_backend::application::navigation::DEFAULT_NAVIGATION_MENU_TREE_ITEMS;

fn menu<'a>(menus: &'a Value, code: &str) -> &'a Value {
    fn find<'a>(menus: &'a Value, code: &str) -> Option<&'a Value> {
        for item in menus.as_array()? {
            if item["menuCode"] == code {
                return Some(item);
            }
            if let Some(found) = find(&item["children"], code) {
                return Some(found);
            }
        }
        None
    }
    find(menus, code).unwrap_or_else(|| panic!("missing menu {code} in {menus}"))
}

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
async fn runtime_navigation_requires_profile_and_is_localized_and_separate_from_domain_list() {
    let router = app();
    let anonymous = request_json(
        &common::app_without_profile(),
        Method::GET,
        "/api/v1/navigation/domains",
        None,
        None,
        None,
    )
    .await;
    assert_eq!(anonymous.status(), StatusCode::CONFLICT);
    assert_eq!(
        response_json(anonymous).await["code"],
        "PERSONAI_PROFILE_REQUIRED"
    );

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
    assert!(!tree_payload["data"].to_string().contains("\"CHATS\""));
    let menu_definitions = request_json(
        &router,
        Method::POST,
        "/api/v1/admin/navigation/menus/query",
        Some(json!({ "current": 1, "size": 200, "keyword": "" })),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(menu_definitions.status(), StatusCode::OK);
    let definitions = response_json(menu_definitions).await;
    assert!(!definitions["data"].to_string().contains("\"CHATS\""));
    assert_eq!(tree_payload["data"]["maxDepth"], 16);
    let menus = &tree_payload["data"]["menus"];
    assert_eq!(
        menus
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["menuCode"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "DUDES",
            "CHAT_NEW_SESSION",
            "EXPLORE_MORE",
            "CHAT_PROJECTS",
            "CHAT_RECENTS"
        ]
    );
    let core = menu(menus, "CORE");
    assert_eq!(core["label"], "核心");
    assert_eq!(core["children"][0]["pageCode"], "apps.index");
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
            "name": { "zh-CN": "合规", "en-US": "Compliance" },
            "description": { "zh-CN": "合规领域", "en-US": "Compliance domain" },
            "enabled": true,
            "isDefault": false,
            "position": 1
        })),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(create_domain.status(), StatusCode::OK);
    let domain = response_json(create_domain).await;
    assert_eq!(domain["code"], "0");
    let domain_code = domain["data"]["domainCode"].as_str().unwrap();

    let create_menu = request_json(
        &router,
        Method::POST,
        "/api/v1/admin/navigation/menus",
        Some(json!({
            "menuCode": "COMPLIANCE_PORTAL",
            "menuType": "ExternalLink",
            "translations": { "zh-CN": "合规门户", "en-US": "Compliance Portal" },
            "description": { "zh-CN": "合规信息入口", "en-US": "Entry point for compliance information" },
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
        runtime_after_add_payload
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
        &format!("/api/v1/admin/navigation/domains/{domain_code}/menu-tree/copy"),
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
        &format!("/api/v1/admin/navigation/domains/{domain_code}/menu-tree/copy"),
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
        &format!("/api/v1/admin/navigation/domains/{domain_code}/menu-tree"),
        None,
        Some(&token),
        None,
    )
    .await;
    let tree_payload = response_json(advanced_tree_after_failed_copy).await;
    assert_eq!(tree_payload["data"].as_array().map(Vec::len), Some(5));
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
            "name": { "zh-CN": "编排测试", "en-US": "Arrangement test" },
            "description": { "zh-CN": "编排测试领域", "en-US": "Arrangement test domain" },
            "enabled": true,
            "isDefault": false,
            "position": 1
        })),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(create_domain.status(), StatusCode::OK);

    let domain = response_json(create_domain).await;
    let domain_code = domain["data"]["domainCode"].as_str().unwrap();

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
                "description": { "zh-CN": "编排测试菜单", "en-US": "Arrangement test menu" },
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
        &format!("/api/v1/admin/navigation/domains/{domain_code}/menu-items"),
        Some(json!({
            "menuCode": "ARRANGEMENT_GROUP",
            "parentMenuCode": null,
            "position": 0,
        })),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(add_group.status(), StatusCode::OK);

    let add_page = request_json(
        &router,
        Method::POST,
        &format!("/api/v1/admin/navigation/domains/{domain_code}/menu-items"),
        Some(json!({
            "menuCode": "ARRANGEMENT_PAGE",
            "parentMenuCode": null,
            "position": 1,
        })),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(add_page.status(), StatusCode::OK);

    let move_page = request_json(
        &router,
        Method::PUT,
        &format!("/api/v1/admin/navigation/domains/{domain_code}/menu-items/ARRANGEMENT_PAGE"),
        Some(json!({
            "parentMenuCode": "ARRANGEMENT_GROUP",
            "position": 0,
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

#[tokio::test]
async fn default_menu_tree_can_be_restored_after_all_assignments_are_removed() {
    let router = app();
    let token = authenticated_bearer_token(&router).await;

    // Parents restrict deletion while children exist. The defaults are parent-first.
    for item in DEFAULT_NAVIGATION_MENU_TREE_ITEMS.iter().rev() {
        let menu_code = item.menu_code;
        let response = request_json(
            &router,
            Method::DELETE,
            &format!("/api/v1/admin/navigation/domains/ADVANCED/menu-items/{menu_code}"),
            None,
            Some(&token),
            None,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK, "remove {menu_code}");
    }

    let empty_tree = request_json(
        &router,
        Method::GET,
        "/api/v1/admin/navigation/domains/ADVANCED/menu-tree",
        None,
        Some(&token),
        None,
    )
    .await;
    assert_eq!(response_json(empty_tree).await["data"], json!([]));

    let restore = request_json(
        &router,
        Method::POST,
        "/api/v1/admin/navigation/menu-trees/default",
        None,
        Some(&token),
        None,
    )
    .await;
    assert_eq!(restore.status(), StatusCode::OK);
    let restore_payload = response_json(restore).await;
    assert_eq!(restore_payload["code"], "0");
    assert_eq!(
        restore_payload["data"]["restoredDomainCodes"],
        json!(["ADVANCED"])
    );
    assert_eq!(
        restore_payload["data"]["restoredItemCount"],
        DEFAULT_NAVIGATION_MENU_TREE_ITEMS.len()
    );

    let restored_tree = request_json(
        &router,
        Method::GET,
        "/api/v1/admin/navigation/domains/ADVANCED/menu-tree",
        None,
        Some(&token),
        None,
    )
    .await;
    let restored_payload = response_json(restored_tree).await;
    let menus = &restored_payload["data"];
    assert_eq!(
        menus
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["menuCode"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "DUDES",
            "CHAT_NEW_SESSION",
            "EXPLORE_MORE",
            "CHAT_PROJECTS",
            "CHAT_RECENTS"
        ]
    );
    assert_eq!(menu(menus, "EXPLORE_MORE")["childDisplay"], "Flyout");
    assert_eq!(menu(menus, "CORE")["childDisplay"], "Inline");
    assert_eq!(
        menu(menus, "AGENT_OPERATIONS")["children"][0]["menuCode"],
        "MCP_SERVERS"
    );
    for expected in DEFAULT_NAVIGATION_MENU_TREE_ITEMS {
        let siblings = expected
            .parent_menu_code
            .map_or(menus, |parent| &menu(menus, parent)["children"]);
        assert_eq!(
            siblings[expected.position as usize]["menuCode"],
            expected.menu_code
        );
    }
    assert!(
        !menus
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["menuCode"] == "OBSERVABILITY")
    );
    let tree_json = restored_payload["data"].to_string();
    for removed in [
        "INTERNAL_AGENTS",
        "SYSTEM_AGENTS",
        "KNOWLEDGE_BASE",
        "KNOWLEDGE_GRAPH",
        "MEMORY",
        "RAG",
    ] {
        assert!(
            !tree_json.contains(removed),
            "removed menu {removed} was restored"
        );
    }

    let second_restore = request_json(
        &router,
        Method::POST,
        "/api/v1/admin/navigation/menu-trees/default",
        None,
        Some(&token),
        None,
    )
    .await;
    assert_eq!(second_restore.status(), StatusCode::OK);
    assert_eq!(
        response_json(second_restore).await["data"]["restoredItemCount"],
        0
    );
}

#[tokio::test]
async fn whole_group_can_move_into_another_group_with_descendants_preserved() {
    let router = app();
    let token = authenticated_bearer_token(&router).await;
    let path = "/api/v1/admin/navigation/menu-trees/ADVANCED/menu-items/DATA";
    let moved = request_json(&router, Method::PUT, path,
        Some(json!({ "parentMenuCode": "EXPLORE_MORE", "position": 0, "childDisplay": "Inline" })),
        Some(&token), None).await;
    let status = moved.status();
    let payload = response_json(moved).await;
    assert_eq!(status, StatusCode::OK, "move entire group: {payload}");

    let tree = request_json(&router, Method::GET,
        "/api/v1/navigation/domains/ADVANCED/menu-tree", None, Some(&token), None).await;
    let tree = response_json(tree).await;
    let roots = tree["data"]["menus"].as_array().unwrap();
    assert!(!roots.iter().any(|item| item["menuCode"] == "DATA"));
    let explore = roots.iter().find(|item| item["menuCode"] == "EXPLORE_MORE").unwrap();
    let data = &explore["children"][0];
    assert_eq!(data["menuCode"], "DATA");
    let resources = data["children"].as_array().unwrap().iter()
        .find(|item| item["menuCode"] == "RESOURCE_MANAGEMENT").unwrap();
    assert!(!resources["children"].as_array().unwrap().is_empty());

    let cycle = request_json(&router, Method::PUT, path,
        Some(json!({ "parentMenuCode": "RESOURCE_MANAGEMENT", "position": 0 })),
        Some(&token), None).await;
    assert_eq!(cycle.status(), StatusCode::BAD_REQUEST);

    let restored = request_json(&router, Method::PUT, path,
        Some(json!({ "parentMenuCode": null, "position": 0 })), Some(&token), None).await;
    assert_eq!(restored.status(), StatusCode::OK);
    let restored = response_json(restored).await;
    assert_eq!(restored["data"][0]["menuCode"], "DATA");
    assert!(!restored["data"][0]["children"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn widgets_and_pages_share_the_domain_tree_and_explore_group() {
    let router = app();
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
    let domains_payload = response_json(domains).await;
    let domain_codes = domains_payload["data"]
        .as_array()
        .expect("domain list should be an array")
        .iter()
        .map(|domain| domain["domainCode"].as_str().unwrap_or(""))
        .collect::<Vec<_>>();
    assert!(
        !domain_codes.contains(&"CHAT"),
        "panel domain must not appear in the primary domain list: {domain_codes:?}"
    );

    let menu_trees = request_json(
        &router,
        Method::GET,
        "/api/v1/admin/navigation/menu-trees",
        None,
        Some(&token),
        None,
    )
    .await;
    let menu_trees_payload = response_json(menu_trees).await;
    let trees = menu_trees_payload["data"].as_array().unwrap();
    assert_eq!(trees.len(), 1);
    assert_eq!(trees[0]["treeCode"], "ADVANCED");

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
        "chat panel tree response: {tree_payload}"
    );
    assert_eq!(tree_payload["data"]["domainCode"], "ADVANCED");
    let menus = tree_payload["data"]["menus"]
        .as_array()
        .expect("chat tree should have menus");
    assert_eq!(menus[0]["menuCode"], "DUDES");
    assert_eq!(menus[0]["menuType"], "Internal");
    assert!(!menus.iter().any(|item| item["menuCode"] == "CHAT_SESSIONS"));
    for (code, page) in [
        ("CHAT_NEW_SESSION", "chat.newSession"),
        ("CHAT_PROJECTS", "chat.projects"),
        ("CHAT_RECENTS", "chat.recents"),
    ] {
        let widget = menus.iter().find(|item| item["menuCode"] == code).unwrap();
        assert_eq!(widget["menuType"], "Widget");
        assert_eq!(widget["pageCode"], page);
        assert_eq!(widget["childDisplay"], "Inline");
        assert!(widget["url"].is_null());
        assert!(widget["children"].as_array().unwrap().is_empty());
    }

    let explore = menus
        .iter()
        .find(|item| item["menuCode"] == "EXPLORE_MORE")
        .unwrap();
    assert_eq!(explore["childDisplay"], "Flyout");
    for (code, position) in [("CHAT_PROJECTS", 0), ("APPS", 1)] {
        let moved = request_json(&router, Method::PUT,
            &format!("/api/v1/admin/navigation/menu-trees/ADVANCED/menu-items/{code}"),
            Some(json!({ "parentMenuCode": "EXPLORE_MORE", "position": position, "childDisplay": "Inline" })), Some(&token), None).await;
        assert_eq!(moved.status(), StatusCode::OK);
    }
    let updated = request_json(&router, Method::GET, "/api/v1/navigation/domains/ADVANCED/menu-tree", None, Some(&token), None).await;
    let updated = response_json(updated).await;
    let explore = updated["data"]["menus"].as_array().unwrap().iter().find(|item| item["menuCode"] == "EXPLORE_MORE").unwrap();
    assert_eq!(explore["children"][0]["menuType"], "Widget");
    assert_eq!(explore["children"][1]["menuCode"], "APPS");

    // Widget menus stay leaves: static children are rejected.
    let add_child = request_json(
        &router,
        Method::POST,
        "/api/v1/admin/navigation/menu-trees/ADVANCED/menu-items",
        Some(json!({
            "menuCode": "APPS",
            "parentMenuCode": "CHAT_PROJECTS",
            "position": 0,
        })),
        Some(&token),
        None,
    )
    .await;
    assert_ne!(add_child.status(), StatusCode::OK);
}
