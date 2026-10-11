mod common;
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use common::{request_with_headers, response_json};
use serde_json::json;
use tower::ServiceExt;
use tracing_subscriber::prelude::*;
#[tokio::test]
async fn logs_api_filters_real_events_and_validates_ids_and_pagination() {
    use tron_backend::adapters::outbound::observability::logs::{daily_appender, log_directory};
    let temp = tempfile::tempdir().unwrap();
    let config = tron_backend::config::AppConfig {
        application_config_path: temp.path().join("application.yaml").display().to_string(),
        ..Default::default()
    };
    let directory = log_directory(&config);
    let app = common::app_with_config(config);
    let trace = "1234567890abcdef1234567890abcdef";
    let span = "1234567890abcdef";
    tracing::subscriber::with_default(
        tracing_subscriber::registry().with(
            tracing_subscriber::fmt::layer()
                .json()
                .flatten_event(true)
                .with_writer(daily_appender(&directory, 30).unwrap()),
        ),
        || {
            tracing::info!(
                trace_id = trace,
                span_id = span,
                operation = "log-api-test",
                workspace_code = "TRON_CORE",
                "Log API correlated event"
            );
            tracing::warn!(
                trace_id = trace,
                span_id = "ffffffffffffffff",
                "Another event"
            );
            tracing::info!("Log API no context event");
        },
    );
    let response=request_with_headers(&app,Method::POST,"/api/v1/observability/logs/query",Some(json!({"traceId":trace.to_uppercase(),"spanId":span,"level":"Info","keyword":"CORRELATED","current":1,"size":15})),None,None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["code"], "0");
    assert_eq!(body["data"]["total"], 1);
    assert_eq!(body["data"]["records"][0]["traceId"], trace);
    assert_eq!(body["data"]["records"][0]["spanId"], span);
    assert_eq!(
        body["data"]["records"][0]["attributes"]["operation"],
        "log-api-test"
    );
    let scoped = request_with_headers(&app, Method::POST, "/api/v1/observability/logs/query", Some(json!({"workspaceCode":"TRON_CORE","current":1,"size":15})), None, None).await;
    assert_eq!(scoped.status(), StatusCode::OK);
    assert_eq!(response_json(scoped).await["data"]["total"], 1);
    let platform = request_with_headers(&app, Method::POST, "/api/v1/observability/logs/query", Some(json!({"workspaceCode":null,"current":1,"size":15})), None, None).await;
    assert_eq!(response_json(platform).await["data"]["total"], 3);
    let older = request_with_headers(&app, Method::POST, "/api/v1/observability/logs/query",
        Some(json!({"beforeLogId": body["data"]["records"][0]["logId"], "current":1,"size":100})), None, None).await;
    assert_eq!(older.status(), StatusCode::OK);
    assert_eq!(response_json(older).await["data"]["total"], 0);
    let empty = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/observability/logs/query",
        Some(json!({"traceId":trace,"spanId":"dddddddddddddddd","current":1,"size":15})),
        None,
        None,
    )
    .await;
    assert_eq!(response_json(empty).await["data"]["records"], json!([]));
    let no_context = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/observability/logs/query",
        Some(json!({"keyword":"Log API no context event","current":1,"size":15})),
        None,
        None,
    )
    .await;
    let record = response_json(no_context).await;
    assert_eq!(record["data"]["records"][0]["traceId"], json!(null));
    assert_eq!(record["data"]["records"][0]["spanId"], json!(null));
    for extra in [
        json!({"traceId":"bad"}),
        json!({"beforeLogId":"invalid"}),
        json!({"date":"../../outside"}),
        json!({"date":"2026-02-30"}),
        json!({"timeZone":"Invalid/Zone"}),
        json!({"spanId":"bad"}),
        json!({"spanId":"0000000000000000"}),
        json!({"level":"Unknown"}),
        json!({"current":0}),
        json!({"size":101}),
        json!({"keyword":"x".repeat(257)}),
    ] {
        let mut q = json!({"current":1,"size":15});
        q.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/observability/logs/query")
                    .header("content-type", "application/json")
                    .header("accept-language", "en-US")
                    .body(Body::from(q.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = response_json(response).await;
        assert_eq!(body["code"], "101040004");
        assert_eq!(body["data"], json!({}));
        assert!(
            body["message"]
                .as_str()
                .unwrap()
                .starts_with("Invalid log query")
        );
    }
}

#[tokio::test]
async fn profile_time_zone_api_saves_preference_and_bootstrap_returns_it() {
    let app = common::app_with_config(Default::default());
    let response = request_with_headers(
        &app,
        Method::PATCH,
        "/api/v1/profiles/active/preferences",
        Some(json!({"timeZone":"America/New_York"})),
        None,
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await["data"]["timeZone"],
        "America/New_York"
    );
    let response = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/bootstrap/status?refreshAccount=false",
        None,
        None,
        None,
    )
    .await;
    assert_eq!(
        response_json(response).await["data"]["timeZone"],
        "America/New_York"
    );
    let response = request_with_headers(
        &app,
        Method::PATCH,
        "/api/v1/profiles/active/preferences",
        Some(json!({"timeZone":"Invalid/Zone"})),
        None,
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let response = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/bootstrap/status?refreshAccount=false",
        None,
        None,
        None,
    )
    .await;
    assert_eq!(
        response_json(response).await["data"]["timeZone"],
        "America/New_York"
    );
}
