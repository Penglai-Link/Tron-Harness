mod common;
use axum::{
    Router,
    http::{Method, StatusCode},
};
use common::{authenticated_bearer_token, request_with_headers, response_json};
use serde_json::{Value, json};
use tron_backend::config::AppConfig;
async fn call(
    app: &Router,
    method: Method,
    path: &str,
    body: Option<Value>,
    status: StatusCode,
) -> Value {
    let token = format!("Bearer {}", authenticated_bearer_token(app).await);
    let response = request_with_headers(app, method, path, body, None, Some(&token)).await;
    let actual = response.status();
    let body = response_json(response).await;
    assert_eq!(actual, status, "{body}");
    body
}
fn design() -> Value {
    json!({"loopCode":"review-flow","name":"Review", "kind":"Workflow","nodes":[{"id":"draft","kind":"Model","instructions":"Draft","join":"Selected","condition":null},{"id":"review","kind":"Model","instructions":"Review","join":"Selected","condition":null}],"edges":[],"maxParallelNodes":1})
}
#[tokio::test]
async fn loop_crud_binding_inheritance_version_and_restart() {
    let config = common::prepared_config(AppConfig::default());
    let app = common::app_with_state_setup(config.clone(), |s| {
        s.knowledge_bases.stop_ingestion_workers()
    });
    let saved = call(
        &app,
        Method::POST,
        "/api/v1/agents/loops",
        Some(design()),
        StatusCode::OK,
    )
    .await;
    assert_eq!(saved["data"]["version"], 1);
    assert_eq!(saved["data"]["nodes"][0]["position"], Value::Null);
    assert!(saved["data"]["updatedAt"].as_i64().unwrap() > 0);
    let agent=call(&app,Method::POST,"/api/v1/agents/registry/internal",Some(json!({"agentCode":"loop-agent","name":"Loop agent","rolePrompt":"Role","loopCode":"review-flow"})),StatusCode::OK).await;
    assert_eq!(agent["data"]["loopCode"], "review-flow");
    call(
        &app,
        Method::PUT,
        "/api/v1/agents/registry/loop-agent/workspace-grants",
        Some(json!({"workspaceCodes":["TRON_CORE"]})),
        StatusCode::OK,
    )
    .await;
    let inherited = call(
        &app,
        Method::GET,
        "/api/v1/agents/workspaces/TRON_CORE/internal/loop-agent",
        None,
        StatusCode::OK,
    )
    .await;
    assert_eq!(inherited["data"]["loopCode"], "review-flow");
    call(
        &app,
        Method::DELETE,
        "/api/v1/agents/loops/review-flow",
        None,
        StatusCode::CONFLICT,
    )
    .await;
    call(
        &app,
        Method::PATCH,
        "/api/v1/agents/workspaces/TRON_CORE/internal/loop-agent/definition",
        Some(json!({"loopCode":"builtin.react-loop"})),
        StatusCode::OK,
    )
    .await;
    let mut update = design();
    update.as_object_mut().unwrap().remove("loopCode");
    update["expectedVersion"] = json!(1);
    update["nodes"][1]["instructions"] = json!("Review v2");
    update["nodes"][0]["position"] = json!({"x":-120.5,"y":640.25});
    let updated = call(
        &app,
        Method::PUT,
        "/api/v1/agents/loops/review-flow",
        Some(update.clone()),
        StatusCode::OK,
    )
    .await;
    assert_eq!(updated["data"]["version"], 2);
    assert_eq!(
        updated["data"]["nodes"][0]["position"],
        json!({"x":-120.5,"y":640.25})
    );
    call(
        &app,
        Method::PUT,
        "/api/v1/agents/loops/review-flow",
        Some(update),
        StatusCode::CONFLICT,
    )
    .await;
    let restarted =
        common::app_with_state_setup(config, |s| s.knowledge_bases.stop_ingestion_workers());
    let list = call(
        &restarted,
        Method::GET,
        "/api/v1/agents/loops",
        None,
        StatusCode::OK,
    )
    .await;
    assert_eq!(list["data"][1]["version"], 2);
    assert_eq!(
        list["data"][1]["nodes"][0]["position"],
        json!({"x":-120.5,"y":640.25})
    );
    let overridden = call(
        &restarted,
        Method::GET,
        "/api/v1/agents/workspaces/TRON_CORE/internal/loop-agent",
        None,
        StatusCode::OK,
    )
    .await;
    assert_eq!(overridden["data"]["loopCode"], "builtin.react-loop");
    let reset = call(
        &restarted,
        Method::POST,
        "/api/v1/agents/workspaces/TRON_CORE/internal/loop-agent/reset-policy",
        None,
        StatusCode::OK,
    )
    .await;
    assert_eq!(reset["data"]["loopCode"], "review-flow");
    call(
        &restarted,
        Method::PATCH,
        "/api/v1/agents/registry/internal/loop-agent/definition",
        Some(json!({"loopCode":"builtin.react-loop"})),
        StatusCode::OK,
    )
    .await;
    call(
        &restarted,
        Method::DELETE,
        "/api/v1/agents/loops/review-flow",
        None,
        StatusCode::OK,
    )
    .await;
    call(
        &restarted,
        Method::DELETE,
        "/api/v1/agents/loops/builtin.react-loop",
        None,
        StatusCode::CONFLICT,
    )
    .await;
}
#[tokio::test]
async fn loop_rejects_invalid_graphs_references_and_duplicate_identity() {
    let app = common::app_with_state_setup(common::prepared_config(AppConfig::default()), |s| {
        s.knowledge_bases.stop_ingestion_workers()
    });
    call(
        &app,
        Method::POST,
        "/api/v1/agents/loops",
        Some(design()),
        StatusCode::OK,
    )
    .await;
    call(
        &app,
        Method::POST,
        "/api/v1/agents/loops",
        Some(design()),
        StatusCode::CONFLICT,
    )
    .await;
    for body in [
        json!({"agentCode":"bad-loop","name":"Bad", "rolePrompt":"", "loopCode":"missing"}),
        json!({"agentCode":"bad-loop","name":"Bad", "rolePrompt":"", "loopCode":""}),
    ] {
        call(
            &app,
            Method::POST,
            "/api/v1/agents/registry/internal",
            Some(body),
            StatusCode::NOT_FOUND,
        )
        .await;
    }
    let mut invalid = design();
    invalid["loopCode"] = json!("bad-position");
    invalid["nodes"][0]["position"] = json!({"x":100001,"y":0});
    call(
        &app,
        Method::POST,
        "/api/v1/agents/loops",
        Some(invalid),
        StatusCode::BAD_REQUEST,
    )
    .await;
    let mut invalid = design();
    invalid["loopCode"] = json!("bad-dag");
    invalid["kind"] = json!("Dag");
    invalid["edges"] = json!([{"from":"draft","to":"review"},{"from":"review","to":"draft"}]);
    call(
        &app,
        Method::POST,
        "/api/v1/agents/loops",
        Some(invalid.clone()),
        StatusCode::BAD_REQUEST,
    )
    .await;
    invalid["edges"] = json!([{"from":"missing","to":"review"}]);
    call(
        &app,
        Method::POST,
        "/api/v1/agents/loops",
        Some(invalid.clone()),
        StatusCode::BAD_REQUEST,
    )
    .await;
    invalid["edges"] = json!([]);
    invalid["nodes"][1]["id"] = json!("draft");
    call(
        &app,
        Method::POST,
        "/api/v1/agents/loops",
        Some(invalid),
        StatusCode::BAD_REQUEST,
    )
    .await;
}
