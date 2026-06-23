fn retrieval_phase3_pipeline() -> RagPipelineInput {
    RagPipelineInput {
        mode: "DAG".into(),
        nodes: vec![
            RagPipelineNodeInput {
                node_id: "generalize-question".into(),
                stage_type: "QUESTION_GENERALIZATION".into(),
                strategy_id: "RAG_QUESTION_GENERALIZATION_SIMPLE_V1".into(),
                enabled: true,
                join_policy: None,
                failure_policy: None,
                config: Some(json!({})),
            },
            RagPipelineNodeInput {
                node_id: "rewrite-query".into(),
                stage_type: "QUERY_REWRITE".into(),
                strategy_id: "RAG_QUERY_REWRITE_TEMPLATE_V1".into(),
                enabled: true,
                join_policy: None,
                failure_policy: None,
                config: Some(json!({})),
            },
            RagPipelineNodeInput {
                node_id: "fulltext-route".into(),
                stage_type: "ROUTE_SEARCH".into(),
                strategy_id: "FULLTEXT_SQLITE_FTS5_SIMPLE_V1".into(),
                enabled: true,
                join_policy: None,
                failure_policy: None,
                config: Some(json!({})),
            },
            RagPipelineNodeInput {
                node_id: "fuse-results".into(),
                stage_type: "FUSE_RESULTS".into(),
                strategy_id: "SUM_SCORE".into(),
                enabled: true,
                join_policy: None,
                failure_policy: None,
                config: Some(json!({})),
            },
            RagPipelineNodeInput {
                node_id: "dedup-results".into(),
                stage_type: "DEDUP_RESULTS".into(),
                strategy_id: "TEXT_UNIT".into(),
                enabled: true,
                join_policy: None,
                failure_policy: None,
                config: Some(json!({})),
            },
            RagPipelineNodeInput {
                node_id: "build-context".into(),
                stage_type: "BUILD_CONTEXT".into(),
                strategy_id: "RESULT_LIST".into(),
                enabled: true,
                join_policy: None,
                failure_policy: None,
                config: Some(json!({})),
            },
        ],
        edges: vec![
            RagPipelineEdgeInput {
                from: "generalize-question".into(),
                to: "rewrite-query".into(),
            },
            RagPipelineEdgeInput {
                from: "rewrite-query".into(),
                to: "fulltext-route".into(),
            },
            RagPipelineEdgeInput {
                from: "fulltext-route".into(),
                to: "fuse-results".into(),
            },
            RagPipelineEdgeInput {
                from: "fuse-results".into(),
                to: "dedup-results".into(),
            },
            RagPipelineEdgeInput {
                from: "dedup-results".into(),
                to: "build-context".into(),
            },
        ],
    }
}
