mod common;
use axum::{
    body::Body,
    http::{Method, Request, StatusCode, header},
};
use common::{app, request_with_headers, response_json};
use serde_json::json;
use tower::ServiceExt;
#[tokio::test]
async fn observability_api_validates_scope_pagination_and_localized_errors() {
    let app = app();
    let tree = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/navigation/domains/ADVANCED/menu-tree",
        None,
        None,
        None,
    )
    .await;
    assert_eq!(tree.status(), StatusCode::OK);
    let tree = response_json(tree).await;
    assert!(!tree["data"]["menus"].as_array().unwrap().iter().any(|menu| menu["menuCode"] == "OBSERVABILITY"));
    let all = request_with_headers(&app, Method::POST, "/api/v1/observability/traces/query", Some(json!({"workspaceCode":null,"current":1,"size":15})), None, None).await;
    assert_eq!(all.status(), StatusCode::OK);
    let response=request_with_headers(&app,Method::POST,"/api/v1/observability/traces/query",Some(json!({"workspaceCode":"TRON_CORE","sessionCode":null,"traceId":null,"current":1,"size":15})),None,None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let page = response_json(response).await;
    assert_eq!(page["code"], "0");
    assert_eq!(page["data"]["total"], 0);
    assert_eq!(page["data"]["records"], json!([]));
    for (workspace, trace, current, size, status) in [
        ("TRON_CORE", "invalid", 1, 15, StatusCode::BAD_REQUEST),
        ("TRON_CORE", "", 0, 15, StatusCode::BAD_REQUEST),
        ("TRON_CORE", "", 1, 101, StatusCode::BAD_REQUEST),
        ("missing", "", 1, 15, StatusCode::NOT_FOUND),
    ] {
        let response=request_with_headers(&app,Method::POST,"/api/v1/observability/traces/query",Some(json!({"workspaceCode":workspace,"sessionCode":null,"traceId":trace,"current":current,"size":size})),None,None).await;
        assert_eq!(response.status(), status);
        let body = response_json(response).await;
        assert_eq!(body["data"], json!({}));
        assert_ne!(body["code"], "0");
    }
    let missing = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/observability/traces/0123456789abcdef0123456789abcdef?workspaceCode=TRON_CORE",
        None,
        None,
        None,
    )
    .await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    let metrics = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/observability/metrics?workspaceCode=TRON_CORE",
        None,
        None,
        None,
    )
    .await;
    assert_eq!(metrics.status(), StatusCode::OK);
    let metrics = response_json(metrics).await;
    assert_eq!(metrics["data"]["traceCount"], 0);
    assert_eq!(metrics["data"]["inputTokens"], 0);
    let runtime = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/observability/runtime",
        None,
        None,
        None,
    )
    .await;
    assert_eq!(runtime.status(), StatusCode::OK);
    assert_eq!(response_json(runtime).await["data"]["metricPointCount"], 0);
    let points = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/observability/metric-points/query",
        Some(json!({"name":"external.gauge","current":1,"size":15})),
        None,
        None,
    )
    .await;
    assert_eq!(points.status(), StatusCode::OK);
    assert_eq!(response_json(points).await["data"]["records"], json!([]));
    for (current, size) in [(0, 15), (1, 101)] {
        let invalid = request_with_headers(
            &app,
            Method::POST,
            "/api/v1/observability/metric-points/query",
            Some(json!({"current":current,"size":size})),
            None,
            None,
        )
        .await;
        assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
        assert_ne!(response_json(invalid).await["code"], "0");
    }
    let config = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/observability/collector/export-config",
        None,
        None,
        None,
    )
    .await;
    assert_eq!(config.status(), StatusCode::OK);
    assert_eq!(config.headers()[header::CACHE_CONTROL], "no-store");
    let config = response_json(config).await;
    assert!(
        config["data"]["endpoint"]
            .as_str()
            .unwrap()
            .starts_with("http://127.0.0.1:")
    );
    assert!(
        config["data"]["authorization"]
            .as_str()
            .unwrap()
            .starts_with("Bearer ")
    );
    assert_eq!(config["data"]["traceRetentionDays"], 15);
    assert_eq!(config["data"]["metricRetentionDays"], 5);
    assert_eq!(config["data"]["maxTraceRecords"], 50_000);
    let localized = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/observability/metrics?workspaceCode=TRON_CORE&traceId=bad")
                .header(header::ACCEPT_LANGUAGE, "en-US")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(localized.status(), StatusCode::BAD_REQUEST);
    assert!(
        response_json(localized).await["message"]
            .as_str()
            .unwrap()
            .starts_with("Invalid query")
    );
}

#[test]
fn observability_is_not_reseeded_into_workspace_navigation() {
    use tron_backend::adapters::outbound::persistence::sqlite::SqliteNavigationRepository;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("navigation.sqlite3");
    for _ in 0..2 {
        drop(SqliteNavigationRepository::connect(path.to_str().unwrap()).unwrap());
    }
    let db = rusqlite::Connection::open(path).unwrap();
    let count:i64=db.query_row("SELECT COUNT(*) FROM navigation_menu_tree_items i JOIN navigation_menus m ON m.id=i.menu_id WHERE m.menu_code IN ('TRACES', 'METRICS', 'OBSERVABILITY_DASHBOARD', 'LOGS') AND i.parent_item_id IS NOT NULL",[],|r|r.get(0)).unwrap();
    assert_eq!(count, 0);
    let menus: i64 = db.query_row("SELECT COUNT(*) FROM navigation_menus WHERE menu_code IN ('OBSERVABILITY', 'TRACES', 'METRICS', 'OBSERVABILITY_DASHBOARD', 'LOGS')", [], |row| row.get(0)).unwrap();
    assert_eq!(menus, 0);
}

#[tokio::test]
async fn observability_collector_accepts_gzip_and_preserves_i64_points() {
    use opentelemetry_proto::tonic::{
        collector::metrics::v1::ExportMetricsServiceRequest,
        metrics::v1::{
            Gauge, Metric, NumberDataPoint, ResourceMetrics, ScopeMetrics, metric::Data,
            number_data_point,
        },
    };
    use prost::Message;
    use std::io::Write;
    let app = app();
    let config = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/observability/collector/export-config",
        None,
        None,
        None,
    )
    .await;
    let config = response_json(config).await;
    let endpoint = config["data"]["endpoint"].as_str().unwrap();
    let authorization = config["data"]["authorization"].as_str().unwrap();
    let request = ExportMetricsServiceRequest {
        resource_metrics: vec![ResourceMetrics {
            scope_metrics: vec![ScopeMetrics {
                metrics: vec![Metric {
                    name: "external.large_integer".into(),
                    data: Some(Data::Gauge(Gauge {
                        data_points: vec![NumberDataPoint {
                            time_unix_nano: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_nanos() as u64,
                            value: Some(number_data_point::Value::AsInt(i64::MAX)),
                            ..Default::default()
                        }],
                    })),
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        }],
    };
    let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    gzip.write_all(&request.encode_to_vec()).unwrap();
    let response = reqwest::Client::builder()
        .no_proxy()
        .build()
        .unwrap()
        .post(format!("{endpoint}/v1/metrics"))
        .header("authorization", authorization)
        .header("content-type", "application/x-protobuf")
        .header("content-encoding", "gzip")
        .body(gzip.finish().unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let points = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/observability/metric-points/query",
        Some(json!({"name":"external.large_integer","current":1,"size":15})),
        None,
        None,
    )
    .await;
    let points = response_json(points).await;
    assert_eq!(points["data"]["total"], 1);
    assert_eq!(
        points["data"]["records"][0]["data"]["asInt"],
        i64::MAX.to_string()
    );
}

#[tokio::test]
async fn trace_session_names_are_current_and_workspace_scoped() {
    let config = common::prepared_config(tron_backend::config::AppConfig::default());
    let telemetry_path =
        std::path::Path::new(&config.database_url).with_extension("telemetry.sqlite3");
    let app = common::app_with_config(config);
    let token = common::authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");
    let created = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({"workspaceCode":"TRON_CORE", "title":"Current session name"})),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created = response_json(created).await;
    let session_code = created["data"]["sessionCode"].as_str().unwrap();
    let runtime = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/observability/runtime",
        None,
        None,
        None,
    )
    .await;
    assert_eq!(runtime.status(), StatusCode::OK);
    let db = rusqlite::Connection::open(telemetry_path).unwrap();
    for (id, workspace, session) in [
        (
            "11111111111111111111111111111111",
            "TRON_CORE",
            session_code,
        ),
        (
            "22222222222222222222222222222222",
            "COMPLIANCE",
            session_code,
        ),
        (
            "33333333333333333333333333333333",
            "TRON_CORE",
            "missing-session",
        ),
    ] {
        db.execute("INSERT INTO ef_telemetry_traces(trace_id,started_at,ended_at,status_code,attributes,abandoned) VALUES (?1,1,2,0,?2,0)",
            rusqlite::params![id, json!({"tron.workspace.code":workspace,"tron.session.code":session}).to_string()]).unwrap();
    }
    for (id, workspace, expected) in [
        (
            "11111111111111111111111111111111",
            "TRON_CORE",
            json!("Current session name"),
        ),
        (
            "22222222222222222222222222222222",
            "COMPLIANCE",
            json!(null),
        ),
        ("33333333333333333333333333333333", "TRON_CORE", json!(null)),
    ] {
        let page = request_with_headers(
            &app,
            Method::POST,
            "/api/v1/observability/traces/query",
            Some(json!({"workspaceCode":workspace,"traceId":id,"current":1,"size":15})),
            None,
            None,
        )
        .await;
        assert_eq!(page.status(), StatusCode::OK);
        assert_eq!(
            response_json(page).await["data"]["records"][0]["sessionName"],
            expected
        );
        let all_detail = request_with_headers(&app, Method::GET, &format!("/api/v1/observability/traces/{id}"), None, None, None).await;
        assert_eq!(all_detail.status(), StatusCode::OK);
        let all_detail = response_json(all_detail).await;
        assert_eq!(all_detail["data"]["trace"]["sessionName"], expected);
        assert_eq!(all_detail["data"]["trace"]["workspaceCode"], workspace);
        let detail = request_with_headers(
            &app,
            Method::GET,
            &format!("/api/v1/observability/traces/{id}?workspaceCode={workspace}"),
            None,
            None,
            None,
        )
        .await;
        assert_eq!(detail.status(), StatusCode::OK);
        assert_eq!(
            response_json(detail).await["data"]["trace"]["sessionName"],
            expected
        );
    }
}

#[tokio::test]
async fn dashboard_counts_workspace_requests_and_recent_traffic_without_other_workspaces() {
    let config = common::prepared_config(tron_backend::config::AppConfig::default());
    let telemetry_path =
        std::path::Path::new(&config.database_url).with_extension("telemetry.sqlite3");
    let app = common::app_with_config(config);
    let runtime = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/observability/runtime",
        None,
        None,
        None,
    )
    .await;
    assert_eq!(runtime.status(), StatusCode::OK);
    let db = rusqlite::Connection::open(telemetry_path).unwrap();
    let at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    for (id, workspace, start, end, status) in [
        (
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "TRON_CORE",
            at - 3600000,
            Some(at - 3599000),
            0,
        ),
        (
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "TRON_CORE",
            at - 3000,
            Some(at - 1000),
            2,
        ),
        (
            "cccccccccccccccccccccccccccccccc",
            "TRON_CORE",
            at - 1000,
            None,
            0,
        ),
        (
            "dddddddddddddddddddddddddddddddd",
            "COMPLIANCE",
            at - 3000,
            Some(at - 1000),
            2,
        ),
    ] {
        db.execute("INSERT INTO ef_telemetry_traces(trace_id,started_at,ended_at,status_code,attributes,abandoned) VALUES (?1,?2,?3,?4,?5,0)",rusqlite::params![id,start,end,status,json!({"tron.workspace.code":workspace}).to_string()]).unwrap();
    }
    let response = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/observability/dashboard?workspaceCode=TRON_CORE",
        None,
        None,
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    let data = &body["data"];
    assert_eq!(data["traceCount"], 3);
    assert_eq!(data["failedTraceCount"], 1);
    assert_eq!(data["runningTraceCount"], 1);
    assert_eq!(data["averageDurationMs"], 1500.0);
    assert_eq!(data["p95DurationMs"], 2000.0);
    assert!(data["diskUsage"]["paths"].is_array());
    assert!(data["diskUsage"].as_object().unwrap().contains_key("bytes"));
    assert!(data["diskUsage"].as_object().unwrap().contains_key("sampledAt"));
    assert!(matches!(data["diskUsage"]["measurement"].as_str(), Some("Allocated" | "Logical")));
    let buckets = data["recentRequests"].as_array().unwrap();
    assert_eq!(buckets.len(), 15);
    assert_eq!(
        buckets
            .iter()
            .map(|b| b["requests"].as_u64().unwrap())
            .sum::<u64>(),
        2
    );
    assert_eq!(
        buckets
            .iter()
            .map(|b| b["errors"].as_u64().unwrap())
            .sum::<u64>(),
        1
    );
    assert!(
        data["processes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["pid"] == std::process::id() && p["parentPid"].is_null())
    );
    let all = request_with_headers(&app, Method::GET, "/api/v1/observability/dashboard", None, None, None).await;
    assert_eq!(all.status(), StatusCode::OK);
    let all = response_json(all).await;
    assert_eq!(all["data"]["traceCount"], 4);
    assert_eq!(all["data"]["failedTraceCount"], 2);
    let page = request_with_headers(&app, Method::POST, "/api/v1/observability/traces/query", Some(json!({"current":1,"size":15})), None, None).await;
    assert_eq!(page.status(), StatusCode::OK);
    let page = response_json(page).await;
    assert_eq!(page["data"]["total"], 4);
    assert!(page["data"]["records"].as_array().unwrap().iter().any(|record| record["workspaceCode"] == "COMPLIANCE"));
    let missing = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/observability/dashboard?workspaceCode=missing",
        None,
        None,
        None,
    )
    .await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}
