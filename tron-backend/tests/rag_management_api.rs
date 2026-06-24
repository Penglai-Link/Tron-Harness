mod common;

use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::http::{Method, StatusCode};
use rusqlite::Connection;
use serde_json::{Value, json};
use tron_backend::config::AppConfig;

use common::{app_with_config, authenticated_request, request, response_json};

#[tokio::test]
async fn corpus_management_routes_require_auth_and_expose_targeted_projection() {
    let database_url = temp_database_path("rag-management-corpus-detail")
        .to_string_lossy()
        .to_string();
    let router = app_with_database(&database_url);

    let unauthorized = request(&router, Method::GET, "/api/v1/rag/corpora", None, None).await;
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let list = authenticated_request(&router, Method::GET, "/api/v1/rag/corpora", None).await;
    assert_eq!(list.status(), StatusCode::OK);
    let list_body = response_json(list).await;
    assert_eq!(list_body["code"], json!("SUCCESS"));
    assert!(
        list_body["data"]
            .as_array()
            .is_some_and(|records| !records.is_empty())
    );
    let memory_card = list_body["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|record| record["corpusId"] == json!("memory-card"))
        .cloned()
        .expect("memory-card should appear in corpus management list");
    assert_eq!(
        memory_card["ingestProfileId"],
        json!("memory-card-ingest-v1")
    );
    assert!(memory_card["retrievalProfileId"].as_str().is_some());
    assert_eq!(memory_card["metadataGroups"][0]["name"], json!("memory"));

    let detail = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/rag/corpora/memory-card",
        None,
    )
    .await;
    assert_eq!(detail.status(), StatusCode::OK);
    let detail_body = response_json(detail).await;
    assert_eq!(detail_body["code"], json!("SUCCESS"));
    assert_eq!(detail_body["data"]["corpusId"], json!("memory-card"));
    assert_eq!(
        detail_body["data"]["storageBinding"]["mode"],
        json!("TARGETED")
    );
    assert_eq!(
        detail_body["data"]["fieldMapping"]["source"]["sourceCode"],
        json!("source_code")
    );
    assert_eq!(
        detail_body["data"]["runtimeProjection"]["storageBinding"]["bindings"]["textUnit"],
        json!("memory-store")
    );
    assert_eq!(
        target_by_id(
            &detail_body["data"]["runtimeProjection"]["storageBinding"],
            "memory-store"
        )["table"],
        json!("rag_memory_text_units")
    );
    assert_eq!(
        detail_body["data"]["runtimeProjection"]["compatible"],
        json!(true)
    );

    let catalog =
        authenticated_request(&router, Method::GET, "/api/v1/rag/corpora/catalog", None).await;
    assert_eq!(catalog.status(), StatusCode::OK);
    let catalog_body = response_json(catalog).await;
    assert_eq!(
        catalog_body["data"]["storageBackends"],
        json!(["SQLITE", "GRAPH"])
    );
    assert_eq!(
        catalog_body["data"]["contractFields"]["source"][0]["logicalName"],
        json!("sourceCode")
    );
    assert!(
        catalog_body["data"]["capabilityRules"]
            .as_array()
            .is_some_and(|rules| rules
                .iter()
                .any(|rule| { rule["code"] == json!("RAG_STORAGE_GRAPH_BINDING_REQUIRED") }))
    );

    let invalid_verify = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/corpora/verify",
        Some(json!({
            "corpusId": "memory-card",
            "displayName": "Memory Card",
            "description": "Invalid graph toggle",
            "status": "enabled",
            "capabilities": {
                "fulltext": true,
                "vector": true,
                "graph": true,
                "reserved": false
            },
            "fieldMapping": {
                "source": {
                    "sourceCode": "source_code",
                    "sourceVersion": "source_version",
                    "scopeType": "scope_type",
                    "scopeCode": "scope_code",
                    "text": "text"
                },
                "textUnit": {
                    "sourceCode": "source_code",
                    "sourceVersion": "source_version",
                    "scopeType": "scope_type",
                    "scopeCode": "scope_code",
                    "text": "text"
                }
            },
            "filterSchema": {
                "fields": {
                    "scopeType": {
                        "column": "scope_type",
                        "operators": ["eq", "in"],
                        "type": "string"
                    }
                }
            },
            "storageBinding": {
                "mode": "TARGETED",
                "targets": [
                    {
                        "id": "sqlite-store",
                        "backend": "SQLITE",
                        "layout": "TABLE",
                        "table": "rag_memory_text_units"
                    }
                ],
                "bindings": {
                    "source": "sqlite-store",
                    "textUnit": "sqlite-store",
                    "vector": "sqlite-store"
                }
            }
        })),
    )
    .await;
    assert_eq!(invalid_verify.status(), StatusCode::OK);
    let invalid_verify_body = response_json(invalid_verify).await;
    assert_eq!(invalid_verify_body["data"]["valid"], json!(false));
    assert!(
        invalid_verify_body["data"]["errors"]
            .as_array()
            .is_some_and(|errors| errors
                .iter()
                .any(|error| { error["code"] == json!("RAG_STORAGE_GRAPH_BINDING_REQUIRED") }))
    );
}

#[tokio::test]
async fn corpus_management_create_and_update_persist_targeted_binding_and_runtime_projection() {
    let database_url = temp_database_path("rag-management-corpus-save")
        .to_string_lossy()
        .to_string();
    let router = app_with_database(&database_url);

    let create = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/corpora",
        Some(team_playbook_sqlite_request()),
    )
    .await;
    assert_eq!(create.status(), StatusCode::CREATED);
    let create_body = response_json(create).await;
    assert_eq!(create_body["code"], json!("SUCCESS"));
    assert_eq!(create_body["data"]["corpusId"], json!("team-playbook"));
    assert_eq!(
        create_body["data"]["runtimeProjection"]["storageBinding"]["mode"],
        json!("TARGETED")
    );
    assert_eq!(
        create_body["data"]["runtimeProjection"]["storageBinding"]["bindings"]["vector"],
        json!("playbook-vector-store")
    );

    let connection = Connection::open(&database_url).expect("database should open");
    let saved_runtime_binding = query_json_value(
        &connection,
        "SELECT storage_binding_json FROM rag_corpora WHERE corpus_id = 'team-playbook'",
    );
    assert_eq!(saved_runtime_binding["mode"], json!("TARGETED"));
    assert_eq!(
        saved_runtime_binding["bindings"]["source"],
        json!("playbook-source-store")
    );
    assert_eq!(
        saved_runtime_binding["bindings"]["textUnit"],
        json!("playbook-text-store")
    );
    assert_eq!(
        saved_runtime_binding["bindings"]["vector"],
        json!("playbook-vector-store")
    );
    assert_table_columns_include(
        &connection,
        "rag_team_playbook_sources",
        &[
            "id",
            "source_code",
            "source_version",
            "source_type",
            "scope_type",
            "scope_code",
            "status",
            "title",
            "metadata",
            "document_code",
            "created_at",
            "updated_at",
        ],
    );
    assert_table_columns_include(
        &connection,
        "rag_team_playbook_text_units",
        &[
            "id",
            "text_unit_code",
            "source_code",
            "source_version",
            "scope_type",
            "scope_code",
            "ordinal",
            "status",
            "text",
            "document_code",
            "created_at",
            "updated_at",
        ],
    );
    assert_table_columns_include(
        &connection,
        "rag_team_playbook_vectors",
        &["text_unit_code", "source_code", "source_version", "status"],
    );
    assert_table_columns_include(
        &connection,
        "rag_team_playbook_diagnostics",
        &[
            "id",
            "source_code",
            "source_version",
            "scope_type",
            "scope_code",
            "stage",
            "status",
        ],
    );

    let update = authenticated_request(
        &router,
        Method::PUT,
        "/api/v1/rag/corpora/team-playbook",
        Some(team_playbook_graph_request()),
    )
    .await;
    assert_eq!(update.status(), StatusCode::OK);
    let update_body = response_json(update).await;
    assert_eq!(
        target_by_id(
            &update_body["data"]["runtimeProjection"]["storageBinding"],
            "playbook-graph"
        )["backend"],
        json!("GRAPH")
    );
    assert_eq!(
        update_body["data"]["metadataGroups"][0]["name"],
        json!("document")
    );

    let updated_runtime_binding = query_json_value(
        &connection,
        "SELECT storage_binding_json FROM rag_corpora WHERE corpus_id = 'team-playbook'",
    );
    assert_eq!(
        target_by_id(&updated_runtime_binding, "playbook-graph")["namespace"],
        json!("rag_team_playbook")
    );
    let updated_capabilities = query_json_value(
        &connection,
        "SELECT capabilities_json FROM rag_corpora WHERE corpus_id = 'team-playbook'",
    );
    assert_eq!(
        updated_capabilities["metadataGroups"][0]["name"],
        json!("document")
    );
}

#[tokio::test]
async fn corpus_management_fulltext_binding_projects_independent_sqlite_table() {
    let database_url = temp_database_path("rag-management-fulltext-split")
        .to_string_lossy()
        .to_string();
    let router = app_with_database(&database_url);

    let create = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/corpora",
        Some(team_playbook_sqlite_with_fulltext_request()),
    )
    .await;
    assert_eq!(create.status(), StatusCode::CREATED);
    let create_body = response_json(create).await;
    assert_eq!(
        create_body["data"]["storageBinding"]["bindings"]["fulltext"],
        json!("playbook-fulltext-store")
    );
    assert_eq!(
        create_body["data"]["runtimeProjection"]["storageBinding"]["bindings"]["fulltext"],
        json!("playbook-fulltext-store")
    );
    assert_eq!(
        create_body["data"]["runtimeProjection"]["storageBinding"]["bindings"]["textUnit"],
        json!("playbook-text-store")
    );

    let connection = Connection::open(&database_url).expect("database should open");
    let saved_runtime_binding = query_json_value(
        &connection,
        "SELECT storage_binding_json FROM rag_corpora WHERE corpus_id = 'team-playbook'",
    );
    assert_eq!(
        saved_runtime_binding["bindings"]["fulltext"],
        json!("playbook-fulltext-store")
    );
    assert_table_columns_include(
        &connection,
        "rag_team_playbook_fulltext",
        &[
            "text_unit_code",
            "source_code",
            "source_version",
            "scope_type",
            "scope_code",
            "status",
            "search_text",
        ],
    );
}

#[tokio::test]
async fn corpus_management_accepts_sqlite_retrieval_carriers_without_sqlite_text_units() {
    let database_url = temp_database_path("rag-management-sqlite-carrier-with-graph-text")
        .to_string_lossy()
        .to_string();
    let router = app_with_database(&database_url);

    let create = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/corpora",
        Some(team_playbook_text_graph_with_sqlite_retrieval_request()),
    )
    .await;
    assert_eq!(create.status(), StatusCode::CREATED);
    let create_body = response_json(create).await;
    assert_eq!(
        create_body["data"]["runtimeProjection"]["storageBinding"]["bindings"]["textUnit"],
        json!("playbook-text-graph")
    );
    assert_eq!(
        create_body["data"]["runtimeProjection"]["storageBinding"]["bindings"]["fulltext"],
        json!("playbook-fulltext-store")
    );
    assert_eq!(
        create_body["data"]["runtimeProjection"]["storageBinding"]["bindings"]["vector"],
        json!("playbook-vector-store")
    );

    let connection = Connection::open(&database_url).expect("database should open");
    assert_table_columns_include(
        &connection,
        "rag_team_playbook_fulltext",
        &["text_unit_code", "text", "search_text", "metadata"],
    );
    assert_table_columns_include(
        &connection,
        "rag_team_playbook_vectors",
        &["text_unit_code", "text", "search_text", "metadata"],
    );
}

#[tokio::test]
async fn corpus_management_accepts_targeted_binding_without_fulltext_segment() {
    let database_url = temp_database_path("rag-management-fulltext-legacy")
        .to_string_lossy()
        .to_string();
    let router = app_with_database(&database_url);

    let verify = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/corpora/verify",
        Some(team_playbook_sqlite_request()),
    )
    .await;
    assert_eq!(verify.status(), StatusCode::OK);
    let verify_body = response_json(verify).await;
    assert_eq!(verify_body["data"]["valid"], json!(true));
    assert_eq!(
        verify_body["data"]["runtimeProjection"]["storageBinding"]["bindings"]["textUnit"],
        json!("playbook-text-store")
    );
    assert!(
        verify_body["data"]["runtimeProjection"]["storageBinding"]["bindings"]["fulltext"]
            .is_null(),
        "runtime projection should not synthesize a fulltext binding into the stored TARGETED payload"
    );
}

#[tokio::test]
async fn corpus_management_fulltext_graph_binding_satisfies_graph_closure() {
    let database_url = temp_database_path("rag-management-fulltext-graph-closure")
        .to_string_lossy()
        .to_string();
    let router = app_with_database(&database_url);

    let verify = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/corpora/verify",
        Some(team_playbook_fulltext_graph_request()),
    )
    .await;
    assert_eq!(verify.status(), StatusCode::OK);
    let verify_body = response_json(verify).await;
    assert_eq!(verify_body["data"]["valid"], json!(true));
    assert!(
        verify_body["data"]["errors"]
            .as_array()
            .is_some_and(Vec::is_empty),
        "a non-diagnostics fulltext GRAPH binding should satisfy graph closure without requiring source/textUnit/vector to all be GRAPH"
    );
    assert_eq!(
        verify_body["data"]["runtimeProjection"]["storageBinding"]["bindings"]["fulltext"],
        json!("playbook-fulltext-graph")
    );
    assert_eq!(
        target_by_id(
            &verify_body["data"]["runtimeProjection"]["storageBinding"],
            "playbook-fulltext-graph"
        )["namespace"],
        json!("rag_team_playbook_fulltext")
    );
}

#[tokio::test]
async fn corpus_management_tree_graph_layout_is_valid_for_tree_segment() {
    let database_url = temp_database_path("rag-management-tree-graph-layout")
        .to_string_lossy()
        .to_string();
    let router = app_with_database(&database_url);

    let verify = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/corpora/verify",
        Some(team_playbook_tree_graph_request()),
    )
    .await;
    assert_eq!(verify.status(), StatusCode::OK);
    let verify_body = response_json(verify).await;

    assert_eq!(verify_body["data"]["valid"], json!(true));
    assert_eq!(
        target_by_id(
            &verify_body["data"]["runtimeProjection"]["storageBinding"],
            "playbook-tree-graph"
        )["layout"],
        json!("GRAPH_TREE")
    );
    assert_eq!(
        verify_body["data"]["runtimeProjection"]["storageBinding"]["bindings"]["tree"],
        json!("playbook-tree-graph")
    );
}

#[tokio::test]
async fn profile_catalog_exposes_graph_tree_routes_and_storage_summary() {
    let database_url = temp_database_path("rag-management-tree-graph-catalog")
        .to_string_lossy()
        .to_string();
    let router = app_with_database(&database_url);

    let create = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/corpora",
        Some(team_playbook_tree_graph_request()),
    )
    .await;
    assert_eq!(create.status(), StatusCode::CREATED);

    let list = authenticated_request(&router, Method::GET, "/api/v1/rag/corpora", None).await;
    let list_body = response_json(list).await;
    let team_playbook = list_body["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|record| record["corpusId"] == json!("team-playbook"))
        .cloned()
        .expect("team-playbook should be returned");
    assert!(
        team_playbook["storageSummary"]
            .as_str()
            .is_some_and(|summary| summary
                .contains("tree=graph.rag_team_playbook_tree/TeamPlaybookTreeNode")),
        "{team_playbook}"
    );

    let catalog = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/rag/profiles/catalog?profileType=RETRIEVAL",
        None,
    )
    .await;
    assert_eq!(catalog.status(), StatusCode::OK);
    let catalog_body = response_json(catalog).await;
    assert!(
        catalog_body["data"]["retrievalRouteTypes"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| {
                row["corpusId"] == json!("team-playbook")
                    && row["routeTypes"].as_array().is_some_and(|route_types| {
                        route_types.contains(&json!("TREE_FULLTEXT"))
                            && route_types.contains(&json!("TREE_VECTOR"))
                    })
            })),
        "{catalog_body}"
    );
    assert!(
        catalog_body["data"]["retrievalStrategies"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| {
                row["corpusId"] == json!("team-playbook")
                    && row["routeType"] == json!("TREE_FULLTEXT")
                    && row["strategyIds"] == json!(["TREE_FULLTEXT_V1"])
            })),
        "{catalog_body}"
    );
    assert!(
        catalog_body["data"]["retrievalStrategies"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| {
                row["corpusId"] == json!("team-playbook")
                    && row["routeType"] == json!("TREE_VECTOR")
                    && row["strategyIds"] == json!(["TREE_VECTOR_V1"])
            })),
        "{catalog_body}"
    );
}

#[tokio::test]
async fn corpus_management_saves_segmented_graph_targets_for_runtime() {
    let database_url = temp_database_path("rag-management-segmented-graph-runtime")
        .to_string_lossy()
        .to_string();
    let router = app_with_database(&database_url);

    let create = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/corpora",
        Some(team_playbook_segmented_graph_request()),
    )
    .await;
    assert_eq!(create.status(), StatusCode::CREATED);
    let create_body = response_json(create).await;
    assert_eq!(
        target_by_id(
            &create_body["data"]["runtimeProjection"]["storageBinding"],
            "playbook-source-graph"
        )["backend"],
        json!("GRAPH")
    );

    let connection = Connection::open(&database_url).expect("database should open");
    let runtime_binding = query_json_value(
        &connection,
        "SELECT storage_binding_json FROM rag_corpora WHERE corpus_id = 'team-playbook'",
    );
    assert_eq!(
        runtime_binding["bindings"]["source"],
        json!("playbook-source-graph")
    );
    assert_eq!(
        runtime_binding["bindings"]["textUnit"],
        json!("playbook-text-unit-graph")
    );
    assert_eq!(
        runtime_binding["bindings"]["fulltext"],
        json!("playbook-fulltext-graph")
    );
    assert_eq!(
        runtime_binding["bindings"]["vector"],
        json!("playbook-vector-graph")
    );
    assert!(
        runtime_binding["targets"]
            .as_array()
            .is_some_and(|targets| targets
                .iter()
                .any(|target| target["nodeLabel"] == json!("TeamPlaybookVector"))),
        "runtime binding should preserve role-specific graph target labels"
    );
}

#[tokio::test]
async fn corpus_management_fulltext_graph_target_still_requires_graph_shape_fields() {
    let database_url = temp_database_path("rag-management-fulltext-graph-shape")
        .to_string_lossy()
        .to_string();
    let router = app_with_database(&database_url);

    let mut payload = team_playbook_fulltext_graph_request();
    payload["storageBinding"]["targets"][3]["layout"] = json!("TABLE");
    payload["storageBinding"]["targets"][3]["namespace"] = json!("");
    payload["storageBinding"]["targets"][3]["nodeLabel"] = json!("");

    let verify = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/corpora/verify",
        Some(payload),
    )
    .await;
    assert_eq!(verify.status(), StatusCode::OK);
    let verify_body = response_json(verify).await;
    assert_eq!(verify_body["data"]["valid"], json!(false));
    assert!(
        verify_body["data"]["errors"]
            .as_array()
            .is_some_and(|errors| errors.iter().any(|error| {
                error["code"] == json!("RAG_STORAGE_TOPOLOGY_UNSUPPORTED")
                    && error["message"]
                        .as_str()
                        .is_some_and(|message| message.contains("GRAPH targets"))
            })),
        "GRAPH targets must still provide GRAPH_SINGLE_NODE layout, namespace, and nodeLabel"
    );
    assert!(
        verify_body["data"]["errors"]
            .as_array()
            .is_some_and(|errors| errors
                .iter()
                .all(|error| { error["code"] != json!("RAG_STORAGE_GRAPH_BINDING_REQUIRED") })),
        "shape validation should not be masked by the graph-closure rule once fulltext is bound to GRAPH"
    );
}

#[tokio::test]
async fn profile_management_list_detail_catalog_validate_and_save_runtime_readable_profiles() {
    let database_url = temp_database_path("rag-management-profile-save")
        .to_string_lossy()
        .to_string();
    let router = app_with_database(&database_url);

    let list = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/rag/profiles?profileType=RETRIEVAL",
        None,
    )
    .await;
    assert_eq!(list.status(), StatusCode::OK);
    let list_body = response_json(list).await;
    assert!(list_body["data"].as_array().is_some_and(|profiles| {
        profiles
            .iter()
            .any(|profile| profile["profileId"] == json!("memory-project-doc-fusion-v1"))
    }));

    let ingest_detail = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/rag/profiles/memory-card-ingest-v1",
        None,
    )
    .await;
    assert_eq!(ingest_detail.status(), StatusCode::OK);
    let ingest_body = response_json(ingest_detail).await;
    assert_eq!(ingest_body["data"]["profileType"], json!("INGEST"));
    assert!(
        ingest_body["data"]["pipelineStages"]
            .as_array()
            .is_some_and(|stages| stages.iter().any(|stage| {
                stage["id"] == json!("preparePayload")
                    && stage["strategyId"] == json!("RAG_TEXT_CONTRACT_PAYLOAD_V1")
            }))
    );

    let retrieval_detail = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/rag/profiles/memory-project-doc-fusion-v1",
        None,
    )
    .await;
    assert_eq!(retrieval_detail.status(), StatusCode::OK);
    let retrieval_body = response_json(retrieval_detail).await;
    assert_eq!(retrieval_body["data"]["profileType"], json!("RETRIEVAL"));
    assert_eq!(
        retrieval_body["data"]["corpusIds"],
        json!(["memory-card", "project-doc"])
    );
    assert_eq!(
        retrieval_body["data"]["routes"][0]["routeType"],
        json!("FULLTEXT")
    );

    let catalog = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/rag/profiles/catalog?profileType=RETRIEVAL",
        None,
    )
    .await;
    assert_eq!(catalog.status(), StatusCode::OK);
    let catalog_body = response_json(catalog).await;
    assert!(
        catalog_body["data"]["corpora"]
            .as_array()
            .is_some_and(|corpora| corpora
                .iter()
                .any(|corpus| { corpus["corpusId"] == json!("memory-card") }))
    );
    assert!(
        catalog_body["data"]["retrievalRouteTypes"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| {
                row["corpusId"] == json!("memory-card")
                    && row["routeTypes"] == json!(["FULLTEXT", "VECTOR", "STRUCTURED_FILTER"])
            }))
    );
    assert!(
        catalog_body["data"]["retrievalRouteTypes"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| {
                row["corpusId"] == json!("skill-graph")
                    && row["routeTypes"] == json!(["FULLTEXT", "VECTOR"])
            }))
    );
    assert!(
        catalog_body["data"]["retrievalStrategies"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| {
                row["corpusId"] == json!("memory-card")
                    && row["routeType"] == json!("VECTOR")
                    && row["strategyIds"] == json!(["VECTOR_SQLITE_VEC0_COSINE_V1"])
            }))
    );
    assert!(
        catalog_body["data"]["retrievalStrategies"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| {
                row["corpusId"] == json!("skill-graph")
                    && row["routeType"] == json!("VECTOR")
                    && row["strategyIds"] == json!(["VECTOR_GRAPH_SINGLE_NODE_V1"])
            }))
    );

    let ingest_catalog = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/rag/profiles/catalog?profileType=INGEST",
        None,
    )
    .await;
    assert_eq!(ingest_catalog.status(), StatusCode::OK);
    let ingest_catalog_body = response_json(ingest_catalog).await;
    assert_eq!(
        ingest_catalog_body["data"]["ingestStageStrategies"]["splitText"],
        json!([
            "RAG_SPLIT_TEXT",
            "RAG_SPLIT_PLAIN_TEXT_V1",
            "RAG_SPLIT_MARKDOWN_HEADING_V1"
        ])
    );

    let validate_filters = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/filters/validate",
        Some(json!({
            "corpusId": "memory-card",
            "filters": {
                "memoryType": {
                    "op": "eq",
                    "value": "RULE"
                }
            }
        })),
    )
    .await;
    assert_eq!(validate_filters.status(), StatusCode::OK);
    let filter_body = response_json(validate_filters).await;
    assert_eq!(filter_body["data"]["valid"], json!(true));
    assert_eq!(
        filter_body["data"]["normalizedFilters"]["memoryType"]["op"],
        json!("eq")
    );

    let invalid_verify = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/profiles/verify",
        Some(json!({
            "profileId": "invalid-routes-v1",
            "profileType": "RETRIEVAL",
            "status": "enabled",
            "description": "invalid retrieval profile",
            "primaryCorpusId": "memory-card",
            "corpusIds": ["memory-card"],
            "requiredSourceVersion": "v1",
            "routes": [
                {
                    "routeId": "bad-graph",
                    "corpusId": "memory-card",
                    "enabled": true,
                    "routeType": "GRAPH",
                    "strategyId": "VECTOR_GRAPH_SINGLE_NODE_V1"
                }
            ],
            "filters": [
                {
                    "field": "unknownField",
                    "operator": "eq",
                    "value": "x"
                }
            ],
            "fusionPolicy": "RRF",
            "dedupPolicy": "TEXT_UNIT",
            "topicPolicy": "NONE",
            "graphExpansionPolicy": "OUTGOING_DEPTH_1",
            "contextStrategies": ["RESULT_LIST"],
            "rerankEnabled": false,
            "rerankModelCode": "",
            "degradePolicy": "SKIP_VECTOR_ON_UNAVAILABLE"
        })),
    )
    .await;
    assert_eq!(invalid_verify.status(), StatusCode::OK);
    let invalid_verify_body = response_json(invalid_verify).await;
    assert_eq!(invalid_verify_body["data"]["valid"], json!(false));
    assert!(
        invalid_verify_body["data"]["errors"]
            .as_array()
            .is_some_and(|errors| errors
                .iter()
                .any(|error| { error["code"] == json!("RAG_PROFILE_STRATEGY_INVALID") }))
    );
    assert!(
        invalid_verify_body["data"]["errors"]
            .as_array()
            .is_some_and(|errors| errors
                .iter()
                .any(|error| { error["code"] == json!("RAG_FILTER_FIELD_INVALID") }))
    );

    let create = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/profiles",
        Some(retrieval_profile_request(
            "docs-memory-fusion-v1",
            "Docs and memory fusion",
            false,
        )),
    )
    .await;
    assert_eq!(create.status(), StatusCode::CREATED);
    let create_body = response_json(create).await;
    assert_eq!(
        create_body["data"]["profileId"],
        json!("docs-memory-fusion-v1")
    );
    assert_eq!(
        create_body["data"]["routes"][1]["routeType"],
        json!("VECTOR")
    );

    let update = authenticated_request(
        &router,
        Method::PUT,
        "/api/v1/rag/profiles/docs-memory-fusion-v1",
        Some(retrieval_profile_request(
            "docs-memory-fusion-v1",
            "Docs and memory fusion v2",
            true,
        )),
    )
    .await;
    assert_eq!(update.status(), StatusCode::OK);
    let update_body = response_json(update).await;
    assert_eq!(
        update_body["data"]["description"],
        json!("Docs and memory fusion v2")
    );
    assert_eq!(update_body["data"]["rerankEnabled"], json!(true));

    let connection = Connection::open(&database_url).expect("database should open");
    let saved_profile_config = query_json_value(
        &connection,
        "SELECT config_json FROM rag_profiles WHERE profile_id = 'docs-memory-fusion-v1'",
    );
    assert_eq!(
        saved_profile_config["routes"][0]["retriever"],
        json!("FULLTEXT_SQLITE_FTS5_SIMPLE_V1")
    );
    assert_eq!(saved_profile_config["rerankPolicy"]["enabled"], json!(true));
    assert_eq!(
        saved_profile_config["description"],
        json!("Docs and memory fusion v2")
    );
    let saved_profile_filters = query_json_value(
        &connection,
        "SELECT filters_json FROM rag_profiles WHERE profile_id = 'docs-memory-fusion-v1'",
    );
    assert_eq!(saved_profile_filters["memoryType"]["op"], json!("eq"));

    let search = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/search",
        Some(json!({
            "profileId": "docs-memory-fusion-v1",
            "query": "memory rule",
            "accessContext": {
                "userCode": "USR-AUTH-0001",
                "agentCode": "AGENT_CODEX",
                "workspaceCode": "TRON_CORE"
            },
            "filters": {},
            "limit": 3,
            "tokenBudget": 100,
            "context": { "strategy": "RESULT_LIST" }
        })),
    )
    .await;
    assert_eq!(search.status(), StatusCode::OK);
    let search_body = response_json(search).await;
    assert_eq!(
        search_body["data"]["profileId"],
        json!("docs-memory-fusion-v1")
    );
}

#[tokio::test]
async fn pipeline_catalog_and_profile_management_accept_new_dag_payload() {
    let database_url = temp_database_path("rag-pipeline-dag-management")
        .to_string_lossy()
        .to_string();
    let router = app_with_database(&database_url);

    let catalog = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/rag/pipeline/catalog?profileType=INGEST",
        None,
    )
    .await;
    assert_eq!(catalog.status(), StatusCode::OK);
    let catalog_body = response_json(catalog).await;
    assert_eq!(catalog_body["code"], json!("SUCCESS"));
    assert_eq!(catalog_body["data"]["profileType"], json!("INGEST"));
    assert_eq!(
        catalog_body["data"]["joinPolicies"],
        json!(["ALL_SUCCESS", "ANY_SUCCESS", "BEST_EFFORT"])
    );
    assert!(
        catalog_body["data"]["stages"]
            .as_array()
            .is_some_and(|stages| stages.iter().any(|stage| {
                stage["stageType"] == json!("PREPARE_PAYLOAD")
                    && stage["inputArtifactKeys"] == json!(["request.payload"])
                    && stage["outputArtifactKeys"] == json!(["payload.prepared"])
                    && stage["strategies"].as_array().is_some_and(|strategies| {
                        strategies.iter().any(|strategy| {
                            strategy["strategyId"] == json!("RAG_TEXT_CONTRACT_PAYLOAD_V1")
                        })
                    })
            }))
    );
    assert!(
        catalog_body["data"]["stages"]
            .as_array()
            .is_some_and(|stages| stages.iter().any(|stage| {
                stage["stageType"] == json!("SPLIT_TEXT")
                    && stage["strategies"].as_array().is_some_and(|strategies| {
                        strategies
                            .iter()
                            .map(|strategy| strategy["strategyId"].clone())
                            .collect::<Vec<_>>()
                            == vec![
                                json!("RAG_SPLIT_TEXT"),
                                json!("RAG_SPLIT_PLAIN_TEXT_V1"),
                                json!("RAG_SPLIT_MARKDOWN_HEADING_V1"),
                            ]
                    })
                    && stage["strategies"].as_array().is_some_and(|strategies| {
                        strategies.iter().any(|strategy| {
                            strategy["strategyId"] == json!("RAG_SPLIT_MARKDOWN_HEADING_V1")
                                && strategy["configSchema"]["properties"]["splitter"]["properties"]
                                    ["includeHeadingInIndexText"]["default"]
                                    == json!(true)
                        })
                    })
            }))
    );

    let mut invalid = ingest_profile_pipeline_request("memory-card-dag-ingest-v1");
    invalid["pipeline"]["edges"]
        .as_array_mut()
        .expect("edges should be an array")
        .push(json!({"from": "persist", "to": "prepare"}));
    let invalid_verify = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/profiles/verify",
        Some(invalid),
    )
    .await;
    assert_eq!(invalid_verify.status(), StatusCode::OK);
    let invalid_body = response_json(invalid_verify).await;
    assert_eq!(invalid_body["data"]["valid"], json!(false));
    assert!(
        invalid_body["data"]["errors"]
            .as_array()
            .is_some_and(|errors| errors
                .iter()
                .any(|error| error["code"] == json!("RAG_PIPELINE_CYCLE")))
    );

    let create = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/profiles",
        Some(ingest_profile_pipeline_request("memory-card-dag-ingest-v1")),
    )
    .await;
    assert_eq!(create.status(), StatusCode::CREATED);
    let create_body = response_json(create).await;
    assert_eq!(
        create_body["data"]["pipeline"]["nodes"][1]["stageType"],
        json!("PREPARE_PAYLOAD")
    );

    let connection = Connection::open(&database_url).expect("database should open");
    let saved_profile_config = query_json_value(
        &connection,
        "SELECT config_json FROM rag_profiles WHERE profile_id = 'memory-card-dag-ingest-v1'",
    );
    assert_eq!(saved_profile_config["pipeline"]["mode"], json!("DAG"));
    assert_eq!(
        saved_profile_config["pipeline"]["nodes"][1]["nodeId"],
        json!("prepare")
    );
    assert_eq!(
        saved_profile_config["stages"]["preparePayload"]["strategyId"],
        json!("RAG_TEXT_CONTRACT_PAYLOAD_V1")
    );
}

#[tokio::test]
async fn profile_catalog_exposes_route_strategies_by_segment_backend() {
    let database_url = temp_database_path("rag-management-profile-mixed-catalog")
        .to_string_lossy()
        .to_string();
    let router = app_with_database(&database_url);

    let create = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/corpora",
        Some(team_playbook_vector_graph_request()),
    )
    .await;
    assert_eq!(create.status(), StatusCode::CREATED);

    let catalog = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/rag/profiles/catalog?profileType=RETRIEVAL",
        None,
    )
    .await;
    assert_eq!(catalog.status(), StatusCode::OK);
    let catalog_body = response_json(catalog).await;
    assert!(
        catalog_body["data"]["retrievalRouteTypes"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| {
                row["corpusId"] == json!("team-playbook")
                    && row["routeTypes"] == json!(["FULLTEXT", "VECTOR", "STRUCTURED_FILTER"])
            })),
        "mixed storage should expose logical retrieval routes rather than a single GRAPH umbrella"
    );
    assert!(
        catalog_body["data"]["retrievalStrategies"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| {
                row["corpusId"] == json!("team-playbook")
                    && row["routeType"] == json!("FULLTEXT")
                    && row["strategyIds"] == json!(["FULLTEXT_SQLITE_FTS5_SIMPLE_V1"])
            }))
    );
    assert!(
        catalog_body["data"]["retrievalStrategies"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| {
                row["corpusId"] == json!("team-playbook")
                    && row["routeType"] == json!("VECTOR")
                    && row["strategyIds"] == json!(["VECTOR_GRAPH_SINGLE_NODE_V1"])
            }))
    );
}

#[tokio::test]
async fn profile_management_saved_ingest_profile_is_loaded_by_ingest_api() {
    let database_url = temp_database_path("rag-management-ingest-profile-runtime")
        .to_string_lossy()
        .to_string();
    let router = app_with_database(&database_url);

    let create_profile = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/profiles",
        Some(ingest_profile_request("memory-card-runtime-ingest-v1")),
    )
    .await;
    assert_eq!(create_profile.status(), StatusCode::CREATED);

    let ingest = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/ingest",
        Some(json!({
            "corpusId": "memory-card",
            "ingestProfileId": "memory-card-runtime-ingest-v1",
            "accessContext": {
                "userCode": "USR-AUTH-0001",
                "agentCode": "AGENT_CODEX",
                "workspaceCode": "TRON_CORE"
            },
            "targetScope": {
                "scopeType": "SPACE",
                "scopeCode": "TRON_CORE"
            },
            "payload": {
                "source": {
                    "code": "runtime-ingest-profile",
                    "version": "v1",
                    "type": "MEMORY_CARD"
                },
                "title": "Runtime ingest profile",
                "text": "Saved ingest profiles must compile inside the ingest API."
            }
        })),
    )
    .await;
    let ingest_status = ingest.status();
    let ingest_body = response_json(ingest).await;
    assert_eq!(ingest_status, StatusCode::OK, "{ingest_body}");
    assert_eq!(
        ingest_body["data"]["profileId"],
        json!("memory-card-runtime-ingest-v1")
    );
}

#[tokio::test]
async fn ui_driven_tree_corpus_profiles_ingest_and_search_api_flow() {
    let database_url = temp_database_path("rag-ui-tree-e2e")
        .to_string_lossy()
        .to_string();
    let router = app_with_database(&database_url);

    let corpus = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/corpora",
        Some(ui_tree_corpus_request()),
    )
    .await;
    assert_eq!(corpus.status(), StatusCode::CREATED);
    let corpus_body = response_json(corpus).await;
    assert_eq!(corpus_body["data"]["capabilities"]["tree"], json!(true));
    assert_eq!(
        corpus_body["data"]["runtimeProjection"]["storageBinding"]["bindings"]["tree"],
        json!("ui-tree-store")
    );

    let detail = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/rag/corpora/ui-tree-memory",
        None,
    )
    .await;
    let detail_body = response_json(detail).await;
    assert_eq!(detail_body["data"]["capabilities"]["tree"], json!(true));

    let list = authenticated_request(&router, Method::GET, "/api/v1/rag/corpora", None).await;
    let list_body = response_json(list).await;
    let records = list_body["data"]
        .as_array()
        .or_else(|| list_body["data"]["records"].as_array())
        .expect("corpus list should return records");
    assert!(
        records
            .iter()
            .any(|record| record["corpusId"] == json!("ui-tree-memory")
                && record["capabilities"]["tree"] == json!(true)),
        "{list_body}"
    );

    let profile_catalog = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/rag/profiles/catalog?profileType=RETRIEVAL",
        None,
    )
    .await;
    assert_eq!(profile_catalog.status(), StatusCode::OK);
    let profile_catalog_body = response_json(profile_catalog).await;
    assert!(
        profile_catalog_body["data"]["retrievalRouteTypes"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| {
                row["corpusId"] == json!("ui-tree-memory")
                    && row["routeTypes"].as_array().is_some_and(|route_types| {
                        route_types.contains(&json!("TREE_FULLTEXT"))
                            && route_types.contains(&json!("TREE_VECTOR"))
                    })
            })),
        "{profile_catalog_body}"
    );
    assert!(
        profile_catalog_body["data"]["retrievalStrategies"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| {
                row["corpusId"] == json!("ui-tree-memory")
                    && row["routeType"] == json!("TREE_FULLTEXT")
                    && row["strategyIds"] == json!(["TREE_FULLTEXT_V1"])
            })),
        "{profile_catalog_body}"
    );
    assert!(
        profile_catalog_body["data"]["retrievalStrategies"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| {
                row["corpusId"] == json!("ui-tree-memory")
                    && row["routeType"] == json!("TREE_VECTOR")
                    && row["strategyIds"] == json!(["TREE_VECTOR_V1"])
            })),
        "{profile_catalog_body}"
    );

    let ingest_profile = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/profiles",
        Some(ui_tree_ingest_profile_request("ui-tree-ingest-v1")),
    )
    .await;
    assert_eq!(ingest_profile.status(), StatusCode::CREATED);
    let ingest_profile_body = response_json(ingest_profile).await;
    assert!(
        ingest_profile_body["data"]["pipeline"]["nodes"]
            .as_array()
            .is_some_and(|nodes| nodes.iter().any(|node| {
                node["stageType"] == json!("TREE_VECTOR_INDEX")
                    && node["strategyId"] == json!("RAG_TREE_VECTOR_INDEX")
            }))
    );

    let retrieval_profile = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/profiles",
        Some(ui_tree_retrieval_profile_request("ui-tree-search-v1")),
    )
    .await;
    let retrieval_profile_status = retrieval_profile.status();
    let retrieval_profile_body = response_json(retrieval_profile).await;
    assert_eq!(
        retrieval_profile_status,
        StatusCode::CREATED,
        "{retrieval_profile_body}"
    );
    assert!(
        retrieval_profile_body["data"]["routes"]
            .as_array()
            .is_some_and(|routes| routes.iter().any(|route| {
                route["routeType"] == json!("TREE_VECTOR")
                    && route["strategyId"] == json!("TREE_VECTOR_V1")
            }))
    );

    let ingest = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/ingest",
        Some(json!({
            "corpusId": "ui-tree-memory",
            "ingestProfileId": "ui-tree-ingest-v1",
            "accessContext": {
                "userCode": "USR-AUTH-0001",
                "agentCode": "AGENT_CODEX",
                "workspaceCode": "TRON_CORE"
            },
            "targetScope": {
                "scopeType": "SPACE",
                "scopeCode": "TRON_CORE"
            },
            "payload": {
                "source": {
                    "code": "ui-tree-source",
                    "version": "v1",
                    "type": "MEMORY_CARD"
                },
                "title": "UI Tree Runtime",
                "text": "UI tree summaries should make retrieval find this source from a summary node."
            }
        })),
    )
    .await;
    let ingest_status = ingest.status();
    let ingest_body = response_json(ingest).await;
    assert_eq!(ingest_status, StatusCode::OK, "{ingest_body}");
    assert_eq!(ingest_body["data"]["profileId"], json!("ui-tree-ingest-v1"));

    let search = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/search",
        Some(json!({
            "profileId": "ui-tree-search-v1",
            "query": "tree summaries retrieval",
            "accessContext": {
                "userCode": "USR-AUTH-0001",
                "agentCode": "AGENT_CODEX",
                "workspaceCode": "TRON_CORE"
            },
            "filters": {},
            "limit": 3,
            "tokenBudget": 100,
            "context": { "strategy": "RESULT_LIST" }
        })),
    )
    .await;
    let search_status = search.status();
    let search_body = response_json(search).await;
    assert_eq!(search_status, StatusCode::OK, "{search_body}");
    assert_eq!(search_body["data"]["profileId"], json!("ui-tree-search-v1"));
    assert_eq!(
        search_body["data"]["hits"][0]["fields"]["matchedBy"],
        json!("TREE_FULLTEXT_V1")
    );

    let connection = Connection::open(&database_url).expect("database should open");
    let tree_rows: i64 = connection
        .query_row("SELECT COUNT(*) FROM rag_ui_tree_nodes", [], |row| {
            row.get(0)
        })
        .expect("tree rows should be queryable");
    assert!(tree_rows >= 1);
}

fn app_with_database(database_url: &str) -> axum::Router {
    app_with_config(AppConfig {
        database_url: database_url.into(),
        ..AppConfig::default()
    })
}

fn query_json_array(connection: &Connection, sql: &str, column: &str) -> Vec<String> {
    let mut statement = connection.prepare(sql).expect("query should prepare");
    statement
        .query_map([], |row| row.get::<_, String>(column))
        .expect("query should map")
        .collect::<Result<Vec<_>, _>>()
        .expect("query rows should collect")
}

fn query_json_value(connection: &Connection, sql: &str) -> Value {
    let raw = connection
        .query_row(sql, [], |row| row.get::<_, String>(0))
        .expect("query should return one row");
    serde_json::from_str(&raw).expect("stored json should parse")
}

fn target_by_id<'a>(binding: &'a Value, target_id: &str) -> &'a Value {
    binding["targets"]
        .as_array()
        .and_then(|targets| {
            targets
                .iter()
                .find(|target| target["id"] == json!(target_id))
        })
        .expect("target id should be present")
}

fn assert_table_columns_include(connection: &Connection, table_name: &str, expected: &[&str]) {
    let columns = query_json_array(
        connection,
        &format!("PRAGMA table_info({table_name})"),
        "name",
    );
    for column in expected {
        assert!(
            columns.iter().any(|actual| actual == column),
            "{table_name} should include column {column}; actual columns: {columns:?}"
        );
    }
}

fn team_playbook_sqlite_request() -> Value {
    json!({
        "corpusId": "team-playbook",
        "displayName": "Team Playbook",
        "description": "Internal operational playbooks.",
        "status": "enabled",
        "capabilities": {
            "fulltext": true,
            "vector": true,
            "tree": true,
            "graph": false,
            "reserved": false
        },
        "metadataGroups": [
            {
                "name": "document",
                "fields": ["documentCode", "documentType"]
            }
        ],
        "filterSchema": {
            "fields": {
                "documentCode": {
                    "column": "document_code",
                    "operators": ["eq", "in"],
                    "type": "string"
                },
                "scopeType": {
                    "column": "scope_type",
                    "operators": ["eq", "in"],
                    "type": "string"
                }
            }
        },
        "fieldMapping": {
            "source": {
                "sourceCode": "source_code",
                "sourceVersion": "source_version",
                "sourceType": "source_type",
                "scopeType": "scope_type",
                "scopeCode": "scope_code",
                "status": "status",
                "title": "title",
                "metadata": "metadata",
                "documentCode": "document_code"
            },
            "textUnit": {
                "sourceCode": "source_code",
                "sourceVersion": "source_version",
                "scopeType": "scope_type",
                "scopeCode": "scope_code",
                "text": "text",
                "documentCode": "document_code"
            }
        },
        "storageBinding": {
            "mode": "TARGETED",
            "targets": [
                {
                    "id": "playbook-source-store",
                    "backend": "SQLITE",
                    "layout": "TABLE",
                    "table": "rag_team_playbook_sources"
                },
                {
                    "id": "playbook-text-store",
                    "backend": "SQLITE",
                    "layout": "TABLE",
                    "table": "rag_team_playbook_text_units"
                },
                {
                    "id": "playbook-vector-store",
                    "backend": "SQLITE",
                    "layout": "TABLE",
                    "table": "rag_team_playbook_vectors"
                },
                {
                    "id": "playbook-diagnostics",
                    "backend": "SQLITE",
                    "layout": "TABLE",
                    "table": "rag_team_playbook_diagnostics"
                }
            ],
            "bindings": {
                "source": "playbook-source-store",
                "textUnit": "playbook-text-store",
                "vector": "playbook-vector-store",
                "diagnostics": "playbook-diagnostics"
            }
        }
    })
}

fn team_playbook_sqlite_with_fulltext_request() -> Value {
    let mut payload = team_playbook_sqlite_request();
    payload["storageBinding"]["targets"]
        .as_array_mut()
        .expect("targets should be an array")
        .insert(
            3,
            json!({
                "id": "playbook-fulltext-store",
                "backend": "SQLITE",
                "layout": "TABLE",
                "table": "rag_team_playbook_fulltext"
            }),
        );
    payload["storageBinding"]["bindings"]["fulltext"] = json!("playbook-fulltext-store");
    payload
}

fn team_playbook_fulltext_graph_request() -> Value {
    let mut payload = team_playbook_sqlite_with_fulltext_request();
    payload["capabilities"]["graph"] = json!(true);
    payload["storageBinding"]["targets"][3] = json!({
        "id": "playbook-fulltext-graph",
        "backend": "GRAPH",
        "layout": "GRAPH_SINGLE_NODE",
        "namespace": "rag_team_playbook_fulltext",
        "nodeLabel": "TeamPlaybookFulltext",
        "codeStrategy": "CORPUS_SOURCE_VERSION_SCOPE_HASH"
    });
    payload["storageBinding"]["bindings"]["fulltext"] = json!("playbook-fulltext-graph");
    payload
}

fn team_playbook_tree_graph_request() -> Value {
    let mut payload = team_playbook_sqlite_with_fulltext_request();
    payload["capabilities"]["graph"] = json!(true);
    payload["storageBinding"]["targets"]
        .as_array_mut()
        .expect("targets should be an array")
        .push(json!({
            "id": "playbook-tree-graph",
            "backend": "GRAPH",
            "layout": "GRAPH_TREE",
            "namespace": "rag_team_playbook_tree",
            "nodeLabel": "TeamPlaybookTreeNode",
            "codeStrategy": "CORPUS_SOURCE_VERSION_SCOPE_HASH"
        }));
    payload["storageBinding"]["bindings"]["tree"] = json!("playbook-tree-graph");
    payload
}

fn team_playbook_vector_graph_request() -> Value {
    let mut payload = team_playbook_sqlite_with_fulltext_request();
    payload["capabilities"]["graph"] = json!(true);
    payload["storageBinding"]["targets"][2] = json!({
        "id": "playbook-vector-graph",
        "backend": "GRAPH",
        "layout": "GRAPH_SINGLE_NODE",
        "namespace": "rag_team_playbook_vectors",
        "nodeLabel": "TeamPlaybookVector",
        "codeStrategy": "CORPUS_SOURCE_VERSION_SCOPE_HASH"
    });
    payload["storageBinding"]["bindings"]["vector"] = json!("playbook-vector-graph");
    payload
}

fn team_playbook_text_graph_with_sqlite_retrieval_request() -> Value {
    let mut payload = team_playbook_sqlite_with_fulltext_request();
    payload["capabilities"]["graph"] = json!(true);
    payload["storageBinding"]["targets"][1] = json!({
        "id": "playbook-text-graph",
        "backend": "GRAPH",
        "layout": "GRAPH_SINGLE_NODE",
        "namespace": "rag_team_playbook_text_units",
        "nodeLabel": "TeamPlaybookTextUnit",
        "codeStrategy": "CORPUS_SOURCE_VERSION_SCOPE_HASH"
    });
    payload["storageBinding"]["bindings"]["textUnit"] = json!("playbook-text-graph");
    payload
}

fn team_playbook_graph_request() -> Value {
    json!({
        "corpusId": "team-playbook",
        "displayName": "Team Playbook Graph",
        "description": "Internal operational playbooks stored in graph form.",
        "status": "enabled",
        "capabilities": {
            "fulltext": true,
            "vector": true,
            "graph": true,
            "reserved": false
        },
        "filterSchema": {
            "fields": {
                "documentCode": {
                    "column": "documentCode",
                    "operators": ["eq", "in"],
                    "type": "string"
                }
            }
        },
        "fieldMapping": {
            "source": {
                "sourceCode": "sourceCode",
                "sourceVersion": "sourceVersion",
                "sourceType": "sourceType",
                "scopeType": "scopeType",
                "scopeCode": "scopeCode",
                "status": "sourceStatus",
                "title": "title",
                "metadata": "sourceMetadata",
                "documentCode": "documentCode"
            },
            "textUnit": {
                "sourceCode": "sourceCode",
                "sourceVersion": "sourceVersion",
                "scopeType": "scopeType",
                "scopeCode": "scopeCode",
                "text": "text",
                "documentCode": "documentCode"
            }
        },
        "storageBinding": {
            "mode": "TARGETED",
            "targets": [
                {
                    "id": "playbook-graph",
                    "backend": "GRAPH",
                    "layout": "GRAPH_SINGLE_NODE",
                    "namespace": "rag_team_playbook",
                    "nodeLabel": "TeamPlaybookDoc",
                    "codeStrategy": "CORPUS_SOURCE_VERSION_SCOPE_HASH"
                }
            ],
            "bindings": {
                "source": "playbook-graph",
                "textUnit": "playbook-graph",
                "vector": "playbook-graph"
            }
        }
    })
}

fn team_playbook_segmented_graph_request() -> Value {
    let mut payload = team_playbook_graph_request();
    payload["storageBinding"]["targets"] = json!([
        {
            "id": "playbook-source-graph",
            "backend": "GRAPH",
            "layout": "GRAPH_SINGLE_NODE",
            "namespace": "rag_team_playbook_sources",
            "nodeLabel": "TeamPlaybookSource",
            "codeStrategy": "CORPUS_SOURCE_VERSION_SCOPE_HASH"
        },
        {
            "id": "playbook-text-unit-graph",
            "backend": "GRAPH",
            "layout": "GRAPH_SINGLE_NODE",
            "namespace": "rag_team_playbook_text_units",
            "nodeLabel": "TeamPlaybookTextUnit",
            "codeStrategy": "CORPUS_SOURCE_VERSION_SCOPE_HASH"
        },
        {
            "id": "playbook-fulltext-graph",
            "backend": "GRAPH",
            "layout": "GRAPH_SINGLE_NODE",
            "namespace": "rag_team_playbook_fulltext",
            "nodeLabel": "TeamPlaybookFulltext",
            "codeStrategy": "CORPUS_SOURCE_VERSION_SCOPE_HASH"
        },
        {
            "id": "playbook-vector-graph",
            "backend": "GRAPH",
            "layout": "GRAPH_SINGLE_NODE",
            "namespace": "rag_team_playbook_vectors",
            "nodeLabel": "TeamPlaybookVector",
            "codeStrategy": "CORPUS_SOURCE_VERSION_SCOPE_HASH"
        }
    ]);
    payload["storageBinding"]["bindings"] = json!({
        "source": "playbook-source-graph",
        "textUnit": "playbook-text-unit-graph",
        "fulltext": "playbook-fulltext-graph",
        "vector": "playbook-vector-graph"
    });
    payload
}

fn retrieval_profile_request(profile_id: &str, description: &str, rerank_enabled: bool) -> Value {
    json!({
        "profileId": profile_id,
        "profileType": "RETRIEVAL",
        "status": "enabled",
        "description": description,
        "primaryCorpusId": "memory-card",
        "corpusIds": ["memory-card", "project-doc"],
        "requiredSourceVersion": "v1",
        "routes": [
            {
                "routeId": "memory-fulltext",
                "corpusId": "memory-card",
                "enabled": true,
                "routeType": "FULLTEXT",
                "strategyId": "FULLTEXT_SQLITE_FTS5_SIMPLE_V1",
                "topK": 8,
                "weight": 1.0
            },
            {
                "routeId": "project-doc-vector",
                "corpusId": "project-doc",
                "enabled": true,
                "routeType": "VECTOR",
                "strategyId": "VECTOR_SQLITE_VEC0_COSINE_V1",
                "topK": 8,
                "weight": 0.8
            }
        ],
        "filters": [
            {
                "field": "memoryType",
                "operator": "eq",
                "value": "RULE"
            }
        ],
        "fusionPolicy": "RRF",
        "dedupPolicy": "TEXT_UNIT",
        "topicPolicy": "NONE",
        "graphExpansionPolicy": "",
        "contextStrategies": ["RESULT_LIST", "TOKEN_BUDGET_PACK"],
        "rerankEnabled": rerank_enabled,
        "rerankModelCode": "",
        "degradePolicy": "SKIP_VECTOR_ON_UNAVAILABLE"
    })
}

fn ingest_profile_request(profile_id: &str) -> Value {
    json!({
        "profileId": profile_id,
        "profileType": "INGEST",
        "status": "enabled",
        "description": "Runtime ingest profile",
        "primaryCorpusId": "memory-card",
        "corpusIds": ["memory-card"],
        "requiredSourceVersion": "v1",
        "pipelineStages": [
            {
                "id": "validateSource",
                "enabled": true,
                "strategyId": "RAG_VALIDATE_SOURCE",
                "config": {}
            },
            {
                "id": "preparePayload",
                "enabled": true,
                "strategyId": "RAG_TEXT_CONTRACT_PAYLOAD_V1",
                "config": {
                    "supportedSourceTypes": ["MEMORY_CARD"]
                }
            },
            {
                "id": "splitText",
                "enabled": true,
                "strategyId": "RAG_SPLIT_TEXT",
                "config": {
                    "splitter": {
                        "type": "plainText",
                        "maxTokens": 800,
                        "overlapTokens": 80
                    }
                }
            },
            {
                "id": "fulltextIndex",
                "enabled": true,
                "strategyId": "RAG_FULLTEXT_INDEX",
                "config": {}
            },
            {
                "id": "vectorIndex",
                "enabled": true,
                "strategyId": "RAG_VECTOR_INDEX",
                "config": {}
            },
            {
                "id": "persistSource",
                "enabled": true,
                "strategyId": "RAG_PERSIST_SOURCE",
                "config": {}
            }
        ],
        "filters": [],
        "embeddingModelCode": "",
        "embeddingVersion": "v1",
        "graphWritePolicy": {},
        "sourceInputPolicy": {},
        "degradePolicy": "SKIP_VECTOR_ON_UNAVAILABLE"
    })
}

fn ingest_profile_pipeline_request(profile_id: &str) -> Value {
    json!({
        "profileId": profile_id,
        "profileType": "INGEST",
        "status": "enabled",
        "description": "Runtime DAG ingest profile",
        "primaryCorpusId": "memory-card",
        "corpusIds": ["memory-card"],
        "requiredSourceVersion": "v1",
        "sourceInputPolicy": {
            "supportedSourceTypes": ["MEMORY_CARD"]
        },
        "pipeline": {
            "mode": "DAG",
            "nodes": [
                {
                    "nodeId": "validate",
                    "stageType": "VALIDATE_SOURCE",
                    "strategyId": "RAG_VALIDATE_SOURCE",
                    "enabled": true,
                    "joinPolicy": "ALL_SUCCESS",
                    "failurePolicy": "FAIL_PIPELINE",
                    "config": {}
                },
                {
                    "nodeId": "prepare",
                    "stageType": "PREPARE_PAYLOAD",
                    "strategyId": "RAG_TEXT_CONTRACT_PAYLOAD_V1",
                    "enabled": true,
                    "joinPolicy": "ALL_SUCCESS",
                    "failurePolicy": "FAIL_PIPELINE",
                    "config": {
                        "supportedSourceTypes": ["MEMORY_CARD"]
                    }
                },
                {
                    "nodeId": "split",
                    "stageType": "SPLIT_TEXT",
                    "strategyId": "RAG_SPLIT_TEXT",
                    "enabled": true,
                    "joinPolicy": "ALL_SUCCESS",
                    "failurePolicy": "FAIL_PIPELINE",
                    "config": {}
                },
                {
                    "nodeId": "fulltext",
                    "stageType": "FULLTEXT_INDEX",
                    "strategyId": "RAG_FULLTEXT_INDEX",
                    "enabled": true,
                    "joinPolicy": "ALL_SUCCESS",
                    "failurePolicy": "CONTINUE_WITH_WARNING",
                    "config": {}
                },
                {
                    "nodeId": "vector",
                    "stageType": "VECTOR_INDEX",
                    "strategyId": "RAG_VECTOR_INDEX",
                    "enabled": true,
                    "joinPolicy": "ALL_SUCCESS",
                    "failurePolicy": "CONTINUE_WITH_WARNING",
                    "config": {}
                },
                {
                    "nodeId": "persist",
                    "stageType": "PERSIST_SOURCE",
                    "strategyId": "RAG_PERSIST_SOURCE",
                    "enabled": true,
                    "joinPolicy": "ALL_SUCCESS",
                    "failurePolicy": "FAIL_PIPELINE",
                    "config": {}
                }
            ],
            "edges": [
                { "from": "validate", "to": "prepare" },
                { "from": "prepare", "to": "split" },
                { "from": "split", "to": "fulltext" },
                { "from": "split", "to": "vector" },
                { "from": "fulltext", "to": "persist" },
                { "from": "vector", "to": "persist" }
            ]
        },
        "embeddingVersion": "v1",
        "degradePolicy": "SKIP_VECTOR_ON_UNAVAILABLE"
    })
}

fn ui_tree_corpus_request() -> Value {
    json!({
        "corpusId": "ui-tree-memory",
        "displayName": "UI Tree Memory",
        "description": "UI-created corpus with summary tree storage.",
        "status": "enabled",
        "capabilities": {
            "fulltext": true,
            "vector": true,
            "tree": true,
            "graph": false,
            "reserved": false
        },
        "filterSchema": { "fields": {} },
        "fieldMapping": {
            "source": {
                "sourceCode": "source_code",
                "sourceVersion": "source_version",
                "sourceType": "source_type",
                "scopeType": "scope_type",
                "scopeCode": "scope_code",
                "status": "status",
                "title": "title",
                "metadata": "metadata"
            },
            "textUnit": {
                "sourceCode": "source_code",
                "sourceVersion": "source_version",
                "scopeType": "scope_type",
                "scopeCode": "scope_code",
                "text": "text"
            }
        },
        "storageBinding": {
            "mode": "TARGETED",
            "targets": [
                {
                    "id": "ui-source-store",
                    "backend": "SQLITE",
                    "layout": "TABLE",
                    "table": "rag_ui_sources"
                },
                {
                    "id": "ui-text-store",
                    "backend": "SQLITE",
                    "layout": "TABLE",
                    "table": "rag_ui_text_units"
                },
                {
                    "id": "ui-vector-store",
                    "backend": "SQLITE",
                    "layout": "TABLE",
                    "table": "rag_ui_vectors"
                },
                {
                    "id": "ui-tree-store",
                    "backend": "SQLITE",
                    "layout": "TABLE",
                    "table": "rag_ui_tree_nodes"
                },
                {
                    "id": "ui-diagnostics-store",
                    "backend": "SQLITE",
                    "layout": "TABLE",
                    "table": "rag_diagnostics"
                }
            ],
            "bindings": {
                "source": "ui-source-store",
                "textUnit": "ui-text-store",
                "vector": "ui-vector-store",
                "tree": "ui-tree-store",
                "diagnostics": "ui-diagnostics-store"
            }
        }
    })
}

fn ui_tree_ingest_profile_request(profile_id: &str) -> Value {
    let mut payload = ingest_profile_pipeline_request(profile_id);
    payload["primaryCorpusId"] = json!("ui-tree-memory");
    payload["corpusIds"] = json!(["ui-tree-memory"]);
    payload["sourceInputPolicy"] = json!({
        "supportedSourceTypes": ["MEMORY_CARD"]
    });
    let nodes = payload["pipeline"]["nodes"]
        .as_array_mut()
        .expect("pipeline nodes should be an array");
    nodes.insert(
        3,
        json!({
            "nodeId": "extract-tree",
            "stageType": "EXTRACT_TREE",
            "strategyId": "RAG_MARKDOWN_TREE_EXTRACT_V1",
            "enabled": true,
            "joinPolicy": "ALL_SUCCESS",
            "failurePolicy": "FAIL_PIPELINE",
            "config": {}
        }),
    );
    nodes.insert(
        6,
        json!({
            "nodeId": "summarize-tree",
            "stageType": "SUMMARIZE_TREE",
            "strategyId": "RAG_TREE_EXTRACTIVE_SUMMARY_V1",
            "enabled": true,
            "joinPolicy": "ALL_SUCCESS",
            "failurePolicy": "FAIL_PIPELINE",
            "config": {}
        }),
    );
    nodes.insert(
        7,
        json!({
            "nodeId": "tree-fulltext",
            "stageType": "TREE_FULLTEXT_INDEX",
            "strategyId": "RAG_TREE_FULLTEXT_INDEX",
            "enabled": true,
            "joinPolicy": "ALL_SUCCESS",
            "failurePolicy": "CONTINUE_WITH_WARNING",
            "config": {}
        }),
    );
    nodes.insert(
        8,
        json!({
            "nodeId": "tree-vector",
            "stageType": "TREE_VECTOR_INDEX",
            "strategyId": "RAG_TREE_VECTOR_INDEX",
            "enabled": true,
            "joinPolicy": "ALL_SUCCESS",
            "failurePolicy": "CONTINUE_WITH_WARNING",
            "config": {}
        }),
    );
    nodes.insert(
        9,
        json!({
            "nodeId": "persist-tree",
            "stageType": "PERSIST_TREE",
            "strategyId": "RAG_PERSIST_TREE_GRAPH_V1",
            "enabled": true,
            "joinPolicy": "ALL_SUCCESS",
            "failurePolicy": "FAIL_PIPELINE",
            "config": {}
        }),
    );
    payload["pipeline"]["edges"] = json!([
        { "from": "validate", "to": "prepare" },
        { "from": "prepare", "to": "split" },
        { "from": "split", "to": "extract-tree" },
        { "from": "extract-tree", "to": "summarize-tree" },
        { "from": "summarize-tree", "to": "tree-fulltext" },
        { "from": "tree-fulltext", "to": "tree-vector" },
        { "from": "tree-vector", "to": "persist-tree" },
        { "from": "split", "to": "fulltext" },
        { "from": "split", "to": "vector" },
        { "from": "fulltext", "to": "persist" },
        { "from": "vector", "to": "persist" },
        { "from": "persist-tree", "to": "persist" }
    ]);
    payload
}

fn ui_tree_retrieval_profile_request(profile_id: &str) -> Value {
    json!({
        "profileId": profile_id,
        "profileType": "RETRIEVAL",
        "status": "enabled",
        "description": "UI tree retrieval profile",
        "primaryCorpusId": "ui-tree-memory",
        "corpusIds": ["ui-tree-memory"],
        "requiredSourceVersion": "v1",
        "routes": [
            {
                "routeId": "ui-tree-fulltext",
                "corpusId": "ui-tree-memory",
                "enabled": true,
                "routeType": "TREE_FULLTEXT",
                "strategyId": "TREE_FULLTEXT_V1",
                "topK": 5,
                "weight": 1.0
            },
            {
                "routeId": "ui-tree-vector",
                "corpusId": "ui-tree-memory",
                "enabled": true,
                "routeType": "TREE_VECTOR",
                "strategyId": "TREE_VECTOR_V1",
                "topK": 5,
                "weight": 0.8
            }
        ],
        "filters": [],
        "fusionPolicy": "RRF",
        "dedupPolicy": "TEXT_UNIT",
        "topicPolicy": "NONE",
        "graphExpansionPolicy": "",
        "contextStrategies": ["RESULT_LIST"],
        "rerankEnabled": false,
        "rerankModelCode": "",
        "degradePolicy": "SKIP_VECTOR_ON_UNAVAILABLE"
    })
}

fn temp_database_path(test_name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "tron-{test_name}-{nanos}-{}.sqlite3",
        std::process::id()
    ))
}
