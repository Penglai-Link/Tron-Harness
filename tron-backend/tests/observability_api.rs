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
    assert!(
        tree["data"]["menus"]
            .as_array()
            .unwrap()
            .iter()
            .any(|menu| menu["pageCode"] == "observability.index")
    );
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
fn observability_menu_survives_repeated_startup_without_duplicate_assignments() {
    use tron_backend::adapters::outbound::persistence::sqlite::SqliteNavigationRepository;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("navigation.sqlite3");
    for _ in 0..2 {
        drop(SqliteNavigationRepository::connect(path.to_str().unwrap()).unwrap());
    }
    let db = rusqlite::Connection::open(path).unwrap();
    let count:i64=db.query_row("SELECT COUNT(*) FROM navigation_menu_tree_items i JOIN navigation_menus m ON m.id=i.menu_id WHERE m.menu_code='OBSERVABILITY' AND i.parent_item_id IS NULL",[],|r|r.get(0)).unwrap();
    assert_eq!(count, 1);
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
