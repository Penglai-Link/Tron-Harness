mod common;
use axum::{Router, body::Body, http::Request};
use serde_json::Value;
use tower::ServiceExt;

async fn read(router: &Router, url: &str, locale: &str) -> Value {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(url)
                .header("accept-language", locale)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    common::response_json(response).await
}
fn tool<'a>(value: &'a Value, name: &str) -> &'a Value {
    value["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| tool["toolName"] == name)
        .unwrap()
}
#[tokio::test]
async fn plugin_resources_reach_tools_and_pack_details_without_locale_leaking_through_cache() {
    let router = common::app();
    for _ in 0..3 {
        let (zh, en) = tokio::join!(
            read(&router, "/api/v1/tools", "zh-CN"),
            read(&router, "/api/v1/tools", "en-US")
        );
        let chinese = tool(&zh, "fs_read_text");
        let english = tool(&en, "fs_read_text");
        assert_eq!(chinese["name"], "读取文件");
        assert_eq!(english["name"], "File Read Text");
        assert_eq!(
            chinese["inputSchema"]["properties"]["path"]["description"],
            "文件路径"
        );
        assert_eq!(
            english["inputSchema"]["properties"]["path"]["description"],
            "File path"
        );
        assert_eq!(tool(&zh, "web_read")["name"], "读取网页");
        assert!(
            tool(&en, "web_search")["description"]
                .as_str()
                .unwrap()
                .is_ascii()
        );
        let zh_description = chinese["description"].clone();
        let en_description = english["description"].clone();
        let (zh, en) = tokio::join!(
            read(&router, "/api/v1/capabilities/packs", "zh-CN"),
            read(&router, "/api/v1/capabilities/packs", "en-US")
        );
        for (value, expected, description) in [
            (&zh, "读取文件", zh_description),
            (&en, "File Read Text", en_description),
        ] {
            let pack = value["data"]
                .as_array()
                .unwrap()
                .iter()
                .find(|pack| pack["id"] == "framework.local.scoped")
                .unwrap();
            let item = pack["contents"]
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["name"] == "fs_read_text")
                .unwrap();
            assert_eq!(item["displayName"], expected);
            assert_eq!(item["description"], description);
        }
    }
}
