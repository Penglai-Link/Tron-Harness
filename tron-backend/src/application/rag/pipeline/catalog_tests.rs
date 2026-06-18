#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn rag_pipeline_catalog_exposes_stage_strategy_and_artifact_contracts() {
        let ingest = pipeline_catalog(RagProfileType::Ingest);
        assert_eq!(ingest.profile_type, "INGEST");
        assert_eq!(
            ingest.join_policies,
            vec!["ALL_SUCCESS", "ANY_SUCCESS", "BEST_EFFORT"]
        );
        assert_eq!(
            ingest.failure_policies,
            vec!["FAIL_PIPELINE", "SKIP_DOWNSTREAM", "CONTINUE_WITH_WARNING"]
        );
        let prepare = ingest
            .stages
            .iter()
            .find(|stage| stage.stage_type == "PREPARE_PAYLOAD")
            .expect("ingest catalog should expose PREPARE_PAYLOAD");
        assert_eq!(prepare.input_artifact_keys, vec!["request.payload"]);
        assert_eq!(prepare.output_artifact_keys, vec!["payload.prepared"]);
        let validate = ingest
            .stages
            .iter()
            .find(|stage| stage.stage_type == "VALIDATE_SOURCE")
            .expect("ingest catalog should expose VALIDATE_SOURCE");
        assert!(validate.orchestration.required);
        assert!(validate.orchestration.acts_as_barrier);
        assert_eq!(
            validate.orchestration.must_run_before_stage_types,
            vec!["PREPARE_PAYLOAD", "RESOLVE_SOURCE"]
        );
        assert!(
            prepare
                .strategies
                .iter()
                .any(|strategy| strategy.strategy_id == "RAG_TEXT_CONTRACT_PAYLOAD_V1")
        );
        assert!(
            prepare
                .strategies
                .iter()
                .find(|strategy| strategy.strategy_id == "RAG_TEXT_CONTRACT_PAYLOAD_V1")
                .expect("text payload strategy should be exposed")
                .config_schema
                .is_object()
        );
        assert!(
            ingest
                .templates
                .iter()
                .any(|template| template.template_id == "text-standard")
        );

        let retrieval = pipeline_catalog(RagProfileType::Retrieval);
        assert_eq!(retrieval.profile_type, "RETRIEVAL");
        assert!(retrieval.templates.is_empty());
        let route_search = retrieval
            .stages
            .iter()
            .find(|stage| stage.stage_type == "ROUTE_SEARCH")
            .expect("retrieval catalog should expose ROUTE_SEARCH");
        assert_eq!(route_search.input_artifact_keys, vec!["query.effective"]);
        assert_eq!(route_search.output_artifact_keys, vec!["hits.route"]);
        assert!(
            route_search
                .strategies
                .iter()
                .any(|strategy| strategy.strategy_id == "FULLTEXT_SQLITE_FTS5_SIMPLE_V1")
        );
    }

    #[test]
    fn rag_pipeline_catalog_exposes_phase3_advanced_stage_contracts() {
        let ingest = pipeline_catalog(RagProfileType::Ingest);
        let extract_tree = ingest
            .stages
            .iter()
            .find(|stage| stage.stage_type == "EXTRACT_TREE")
            .expect("ingest catalog should expose EXTRACT_TREE");
        assert_eq!(
            extract_tree.input_artifact_keys,
            vec!["textUnits.generated"]
        );
        assert_eq!(
            extract_tree.output_artifact_keys,
            vec!["tree.extracted", "graph.related"]
        );
        assert!(
            extract_tree
                .strategies
                .iter()
                .any(|strategy| strategy.strategy_id == "RAG_MARKDOWN_TREE_EXTRACT_V1")
        );
        let persist_tree = ingest
            .stages
            .iter()
            .find(|stage| stage.stage_type == "PERSIST_TREE")
            .expect("ingest catalog should expose PERSIST_TREE");
        assert_eq!(persist_tree.input_artifact_keys, vec!["graph.related"]);
        assert_eq!(persist_tree.output_artifact_keys, vec!["tree.persisted"]);

        let retrieval = pipeline_catalog(RagProfileType::Retrieval);
        let question_generalization = retrieval
            .stages
            .iter()
            .find(|stage| stage.stage_type == "QUESTION_GENERALIZATION")
            .expect("retrieval catalog should expose QUESTION_GENERALIZATION");
        assert_eq!(
            question_generalization.input_artifact_keys,
            vec!["query.original"]
        );
        assert_eq!(
            question_generalization.output_artifact_keys,
            vec!["query.generalized", "query.effective"]
        );
        let query_rewrite = retrieval
            .stages
            .iter()
            .find(|stage| stage.stage_type == "QUERY_REWRITE")
            .expect("retrieval catalog should expose QUERY_REWRITE");
        assert_eq!(query_rewrite.input_artifact_keys, vec!["query.effective"]);
        assert_eq!(
            query_rewrite.output_artifact_keys,
            vec!["query.rewritten", "query.effective"]
        );

        let compiled_ingest = validate_pipeline(&ingest_phase3_pipeline(), RagProfileType::Ingest)
            .expect("advanced ingest tree pipeline should compile");
        assert_eq!(
            compiled_ingest
                .nodes
                .iter()
                .find(|node| node.stage_type == "PERSIST_TREE")
                .expect("persist tree node should compile")
                .input_artifact_keys,
            vec!["graph.related"]
        );

        let compiled_retrieval =
            validate_pipeline(&retrieval_phase3_pipeline(), RagProfileType::Retrieval)
                .expect("advanced retrieval query pipeline should compile");
        assert_eq!(
            compiled_retrieval
                .nodes
                .iter()
                .find(|node| node.stage_type == "ROUTE_SEARCH")
                .expect("route node should compile")
                .input_artifact_keys,
            vec!["query.effective"]
        );
    }

    #[test]
    fn rag_pipeline_catalog_templates_compile_against_declared_stage_contracts() {
        let ingest = pipeline_catalog(RagProfileType::Ingest);
        let templates = ingest
            .templates
            .iter()
            .map(|template| template.template_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            templates,
            vec![
                "text-standard",
                "document-reference",
                "mineru-document-reference",
                "graph-tree-enhanced"
            ]
        );

        for template in ingest.templates {
            validate_pipeline(&template.pipeline, RagProfileType::Ingest).unwrap_or_else(
                |errors| {
                    panic!(
                        "template {} should compile: {errors:?}",
                        template.template_id
                    )
                },
            );
        }
    }

    #[test]
    fn rag_pipeline_validation_rejects_invalid_dag_contracts() {
        let valid = ingest_branch_pipeline();
        assert!(validate_pipeline(&valid, RagProfileType::Ingest).is_ok());

        for (name, mut pipeline, expected_code) in [
            (
                "duplicate-node",
                ingest_branch_pipeline(),
                "RAG_PIPELINE_NODE_DUPLICATE",
            ),
            (
                "bad-edge",
                ingest_branch_pipeline(),
                "RAG_PIPELINE_EDGE_INVALID",
            ),
            ("cycle", ingest_branch_pipeline(), "RAG_PIPELINE_CYCLE"),
            (
                "bad-stage",
                ingest_branch_pipeline(),
                "RAG_PIPELINE_STAGE_UNSUPPORTED",
            ),
            (
                "bad-strategy",
                ingest_branch_pipeline(),
                "RAG_PIPELINE_STRATEGY_INVALID",
            ),
            (
                "bad-profile-type",
                ingest_branch_pipeline(),
                "RAG_PIPELINE_STAGE_UNSUPPORTED",
            ),
            (
                "bad-join",
                ingest_branch_pipeline(),
                "RAG_PIPELINE_JOIN_POLICY_INVALID",
            ),
            (
                "bad-failure",
                ingest_branch_pipeline(),
                "RAG_PIPELINE_FAILURE_POLICY_INVALID",
            ),
            (
                "artifact-override",
                ingest_branch_pipeline(),
                "RAG_PIPELINE_ARTIFACT_KEYS_OWNED_BY_STAGE",
            ),
            (
                "missing-required-stage",
                ingest_branch_pipeline(),
                "RAG_PIPELINE_REQUIRED_STAGE_MISSING",
            ),
            (
                "missing-control-edge",
                ingest_branch_pipeline(),
                "RAG_PIPELINE_CONTROL_DEPENDENCY_MISSING",
            ),
        ] {
            match name {
                "duplicate-node" => pipeline.nodes[1].node_id = "validate".into(),
                "bad-edge" => pipeline.edges[0].to = "missing".into(),
                "cycle" => pipeline.edges.push(RagPipelineEdgeInput {
                    from: "persist".into(),
                    to: "validate".into(),
                }),
                "bad-stage" => pipeline.nodes[0].stage_type = "UNKNOWN_STAGE".into(),
                "bad-strategy" => {
                    pipeline.nodes[1].strategy_id = "FULLTEXT_SQLITE_FTS5_SIMPLE_V1".into()
                }
                "bad-profile-type" => pipeline.nodes[0].stage_type = "ROUTE_SEARCH".into(),
                "bad-join" => pipeline.nodes[3].join_policy = Some("FIRST_SUCCESS".into()),
                "bad-failure" => pipeline.nodes[3].failure_policy = Some("WARN_ONLY".into()),
                "artifact-override" => {
                    pipeline.nodes[1].config = Some(json!({
                        "inputArtifactKeys": ["strategy-owned-input"],
                        "outputArtifactKeys": ["strategy-owned-output"]
                    }))
                }
                "missing-required-stage" => {
                    pipeline.nodes.retain(|node| node.node_id != "validate");
                    pipeline
                        .edges
                        .retain(|edge| edge.from != "validate" && edge.to != "validate");
                }
                "missing-control-edge" => {
                    pipeline
                        .edges
                        .retain(|edge| !(edge.from == "validate" && edge.to == "prepare"));
                }
                _ => unreachable!(),
            }
            let errors = validate_pipeline(&pipeline, RagProfileType::Ingest)
                .expect_err("invalid pipeline should fail validation");
            assert!(
                errors.iter().any(|error| error.code == expected_code),
                "{name} should include {expected_code}; actual errors: {errors:?}",
            );
        }
    }

    #[test]
    fn rag_pipeline_plan_preserves_branch_merge_levels_and_stage_owned_artifacts() {
        let compiled = validate_pipeline(&ingest_branch_pipeline(), RagProfileType::Ingest)
            .expect("branch pipeline should compile");

        assert_eq!(
            compiled.execution_levels,
            vec![
                vec!["validate".to_string()],
                vec!["prepare".to_string()],
                vec!["split".to_string()],
                vec!["fulltext".to_string(), "vector".to_string()],
                vec!["persist".to_string()],
            ]
        );

        let fulltext = compiled
            .nodes
            .iter()
            .find(|node| node.node_id == "fulltext")
            .expect("fulltext branch node should compile");
        assert_eq!(fulltext.input_artifact_keys, vec!["textUnits.generated"]);
        assert_eq!(fulltext.output_artifact_keys, vec!["index.fulltext"]);

        let persist = compiled
            .nodes
            .iter()
            .find(|node| node.node_id == "persist")
            .expect("merge node should compile");
        assert_eq!(persist.join_policy, "ALL_SUCCESS");
        assert_eq!(
            persist.input_artifact_keys,
            vec!["index.fulltext", "index.vector"]
        );
        assert_eq!(persist.output_artifact_keys, vec!["source.persisted"]);
    }

    #[test]
    fn rag_pipeline_compiler_rejects_missing_stage_inputs_and_repeatability_violations() {
        let mut missing_input = ingest_branch_pipeline();
        missing_input.edges.retain(|edge| {
            !(edge.from == "split" && edge.to == "fulltext")
                && !(edge.from == "fulltext" && edge.to == "persist")
        });
        let missing_input_errors = validate_pipeline(&missing_input, RagProfileType::Ingest)
            .expect_err("persist should not compile without index.fulltext availability");
        assert!(
            missing_input_errors
                .iter()
                .any(|error| error.code == "RAG_PIPELINE_INPUT_ARTIFACT_MISSING"),
            "missing input should be rejected; actual errors: {missing_input_errors:?}",
        );

        let mut repeated_non_repeatable = ingest_branch_pipeline();
        repeated_non_repeatable.nodes.push(RagPipelineNodeInput {
            node_id: "fulltext-copy".into(),
            stage_type: "FULLTEXT_INDEX".into(),
            strategy_id: "RAG_FULLTEXT_INDEX".into(),
            enabled: true,
            join_policy: None,
            failure_policy: None,
            config: Some(json!({})),
        });
        repeated_non_repeatable.edges.push(RagPipelineEdgeInput {
            from: "split".into(),
            to: "fulltext-copy".into(),
        });
        let repeat_errors = validate_pipeline(&repeated_non_repeatable, RagProfileType::Ingest)
            .expect_err("non-repeatable stage should not compile twice");
        assert!(
            repeat_errors
                .iter()
                .any(|error| error.code == "RAG_PIPELINE_STAGE_NOT_REPEATABLE"),
            "repeatability should be rejected; actual errors: {repeat_errors:?}",
        );
    }

    fn ingest_branch_pipeline() -> RagPipelineInput {
        RagPipelineInput {
            mode: "DAG".into(),
            nodes: vec![
                RagPipelineNodeInput {
                    node_id: "validate".into(),
                    stage_type: "VALIDATE_SOURCE".into(),
                    strategy_id: "RAG_VALIDATE_SOURCE".into(),
                    enabled: true,
                    join_policy: None,
                    failure_policy: None,
                    config: Some(json!({})),
                },
                RagPipelineNodeInput {
                    node_id: "prepare".into(),
                    stage_type: "PREPARE_PAYLOAD".into(),
                    strategy_id: "RAG_TEXT_CONTRACT_PAYLOAD_V1".into(),
                    enabled: true,
                    join_policy: None,
                    failure_policy: None,
                    config: Some(json!({"supportedSourceTypes": ["MEMORY_CARD"]})),
                },
                RagPipelineNodeInput {
                    node_id: "split".into(),
                    stage_type: "SPLIT_TEXT".into(),
                    strategy_id: "RAG_SPLIT_TEXT".into(),
                    enabled: true,
                    join_policy: None,
                    failure_policy: None,
                    config: Some(json!({})),
                },
                RagPipelineNodeInput {
                    node_id: "fulltext".into(),
                    stage_type: "FULLTEXT_INDEX".into(),
                    strategy_id: "RAG_FULLTEXT_INDEX".into(),
                    enabled: true,
                    join_policy: None,
                    failure_policy: None,
                    config: Some(json!({})),
                },
                RagPipelineNodeInput {
                    node_id: "vector".into(),
                    stage_type: "VECTOR_INDEX".into(),
                    strategy_id: "RAG_VECTOR_INDEX".into(),
                    enabled: true,
                    join_policy: None,
                    failure_policy: None,
                    config: Some(json!({})),
                },
                RagPipelineNodeInput {
                    node_id: "persist".into(),
                    stage_type: "PERSIST_SOURCE".into(),
                    strategy_id: "RAG_PERSIST_SOURCE".into(),
                    enabled: true,
                    join_policy: Some("ALL_SUCCESS".into()),
                    failure_policy: Some("FAIL_PIPELINE".into()),
                    config: Some(json!({})),
                },
            ],
            edges: vec![
                RagPipelineEdgeInput {
                    from: "validate".into(),
                    to: "prepare".into(),
                },
                RagPipelineEdgeInput {
                    from: "prepare".into(),
                    to: "split".into(),
                },
                RagPipelineEdgeInput {
                    from: "split".into(),
                    to: "fulltext".into(),
                },
                RagPipelineEdgeInput {
                    from: "split".into(),
                    to: "vector".into(),
                },
                RagPipelineEdgeInput {
                    from: "fulltext".into(),
                    to: "persist".into(),
                },
                RagPipelineEdgeInput {
                    from: "vector".into(),
                    to: "persist".into(),
                },
            ],
        }
    }

    fn ingest_phase3_pipeline() -> RagPipelineInput {
        let mut pipeline = ingest_branch_pipeline();
        pipeline.nodes.insert(
            3,
            RagPipelineNodeInput {
                node_id: "extract-tree".into(),
                stage_type: "EXTRACT_TREE".into(),
                strategy_id: "RAG_MARKDOWN_TREE_EXTRACT_V1".into(),
                enabled: true,
                join_policy: None,
                failure_policy: None,
                config: Some(json!({})),
            },
        );
        pipeline.nodes.insert(
            6,
            RagPipelineNodeInput {
                node_id: "persist-tree".into(),
                stage_type: "PERSIST_TREE".into(),
                strategy_id: "RAG_PERSIST_TREE_GRAPH_V1".into(),
                enabled: true,
                join_policy: None,
                failure_policy: None,
                config: Some(json!({})),
            },
        );
        pipeline.edges.extend([
            RagPipelineEdgeInput {
                from: "split".into(),
                to: "extract-tree".into(),
            },
            RagPipelineEdgeInput {
                from: "extract-tree".into(),
                to: "persist-tree".into(),
            },
            RagPipelineEdgeInput {
                from: "persist-tree".into(),
                to: "persist".into(),
            },
        ]);
        pipeline
    }

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
}
