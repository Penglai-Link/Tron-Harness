fn strategy_config_schema(strategy_id: &str) -> Value {
    match strategy_id {
        "PROJECT_DOC_RESOLVE_SOURCE" => json!({
            "type": "object",
            "properties": {
                "allowedUriSchemes": {
                    "type": "array",
                    "items": { "type": "string" },
                    "default": ["file", "http", "https"]
                },
                "maxBytes": {
                    "type": "integer",
                    "minimum": 1,
                    "default": 1048576
                }
            }
        }),
        "RAG_PARSE_MARKDOWN_DOCUMENT" => json!({
            "type": "object",
            "properties": {
                "format": {
                    "type": "string",
                    "enum": ["MARKDOWN"],
                    "default": "MARKDOWN"
                },
                "unsupportedFormatPolicy": {
                    "type": "string",
                    "enum": ["FAIL_FAST", "SKIP"],
                    "default": "FAIL_FAST"
                }
            }
        }),
        "RAG_PARSE_MINERU_DOCUMENT" => json!({
            "type": "object",
            "properties": {
                "supportedFormats": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "enum": ["PDF", "DOCX"]
                    },
                    "default": ["PDF", "DOCX"]
                },
                "unsupportedFormatPolicy": {
                    "type": "string",
                    "enum": ["FAIL_FAST", "SKIP"],
                    "default": "FAIL_FAST"
                }
            }
        }),
        "RAG_SPLIT_TEXT" => json!({
            "type": "object",
            "properties": {
                "splitter": {
                    "type": "object",
                    "properties": {
                        "type": {
                            "type": "string",
                            "enum": ["plainText", "markdownHeading"],
                            "default": "plainText"
                        },
                        "maxTokens": {
                            "type": "integer",
                            "minimum": 1,
                            "default": 800
                        },
                        "overlapTokens": {
                            "type": "integer",
                            "minimum": 0,
                            "default": 80
                        },
                        "headingDepth": {
                            "type": "integer",
                            "minimum": 1,
                            "default": 6
                        }
                    }
                }
            }
        }),
        _ => json!({
            "type": "object",
            "additionalProperties": false
        }),
    }
}

fn stage_definitions() -> Vec<StageDefinition> {
    vec![
        StageDefinition {
            stage_type: "VALIDATE_SOURCE",
            display_name: "Validate Source",
            profile_types: &[RagProfileType::Ingest],
            input_artifact_keys: &["request.payload", "request.accessContext"],
            output_artifact_keys: &["source.validated"],
            repeatable: false,
            parallel: false,
            default_join_policy: "ALL_SUCCESS",
            default_failure_policy: "FAIL_PIPELINE",
            strategies: &[StrategyDefinition {
                strategy_id: "RAG_VALIDATE_SOURCE",
                display_name: "Validate RAG Source",
                required_capabilities: &[],
                required_storage_segments: &[],
            }],
            execution_stage_id: "validateSource",
        },
        StageDefinition {
            stage_type: "PREPARE_PAYLOAD",
            display_name: "Prepare Payload",
            profile_types: &[RagProfileType::Ingest],
            input_artifact_keys: &["request.payload"],
            output_artifact_keys: &["payload.prepared"],
            repeatable: false,
            parallel: false,
            default_join_policy: "ALL_SUCCESS",
            default_failure_policy: "FAIL_PIPELINE",
            strategies: &[
                StrategyDefinition {
                    strategy_id: "RAG_TEXT_CONTRACT_PAYLOAD_V1",
                    display_name: "Text Contract Payload",
                    required_capabilities: &["fulltext"],
                    required_storage_segments: &["textUnit"],
                },
                StrategyDefinition {
                    strategy_id: "RAG_DOCUMENT_REF_CONTRACT_PAYLOAD_V1",
                    display_name: "Document Reference Payload",
                    required_capabilities: &["fulltext"],
                    required_storage_segments: &["textUnit"],
                },
            ],
            execution_stage_id: "preparePayload",
        },
        StageDefinition {
            stage_type: "RESOLVE_SOURCE",
            display_name: "Resolve Source",
            profile_types: &[RagProfileType::Ingest],
            input_artifact_keys: &["source.validated"],
            output_artifact_keys: &["document.resolved"],
            repeatable: false,
            parallel: false,
            default_join_policy: "ALL_SUCCESS",
            default_failure_policy: "FAIL_PIPELINE",
            strategies: &[StrategyDefinition {
                strategy_id: "PROJECT_DOC_RESOLVE_SOURCE",
                display_name: "Project Document Resolver",
                required_capabilities: &["fulltext"],
                required_storage_segments: &["source"],
            }],
            execution_stage_id: "resolveSource",
        },
        StageDefinition {
            stage_type: "PARSE_DOCUMENT",
            display_name: "Parse Document",
            profile_types: &[RagProfileType::Ingest],
            input_artifact_keys: &["document.resolved"],
            output_artifact_keys: &["document.parsed"],
            repeatable: false,
            parallel: false,
            default_join_policy: "ALL_SUCCESS",
            default_failure_policy: "FAIL_PIPELINE",
            strategies: &[
                StrategyDefinition {
                    strategy_id: "RAG_PARSE_MARKDOWN_DOCUMENT",
                    display_name: "Markdown Parser",
                    required_capabilities: &["fulltext"],
                    required_storage_segments: &["source"],
                },
                StrategyDefinition {
                    strategy_id: "RAG_PARSE_MINERU_DOCUMENT",
                    display_name: "MinerU Document Parser",
                    required_capabilities: &["fulltext"],
                    required_storage_segments: &["source"],
                },
            ],
            execution_stage_id: "parseDocument",
        },
        StageDefinition {
            stage_type: "SPLIT_TEXT",
            display_name: "Split Text",
            profile_types: &[RagProfileType::Ingest],
            input_artifact_keys: &["payload.prepared"],
            output_artifact_keys: &["textUnits.generated"],
            repeatable: false,
            parallel: false,
            default_join_policy: "ALL_SUCCESS",
            default_failure_policy: "FAIL_PIPELINE",
            strategies: &[StrategyDefinition {
                strategy_id: "RAG_SPLIT_TEXT",
                display_name: "RAG Split Text",
                required_capabilities: &["fulltext"],
                required_storage_segments: &["textUnit"],
            }],
            execution_stage_id: "splitText",
        },
        StageDefinition {
            stage_type: "FULLTEXT_INDEX",
            display_name: "Fulltext Index",
            profile_types: &[RagProfileType::Ingest],
            input_artifact_keys: &["textUnits.generated"],
            output_artifact_keys: &["index.fulltext"],
            repeatable: false,
            parallel: true,
            default_join_policy: "ALL_SUCCESS",
            default_failure_policy: "CONTINUE_WITH_WARNING",
            strategies: &[StrategyDefinition {
                strategy_id: "RAG_FULLTEXT_INDEX",
                display_name: "Fulltext Index",
                required_capabilities: &["fulltext"],
                required_storage_segments: &["fulltext"],
            }],
            execution_stage_id: "fulltextIndex",
        },
        StageDefinition {
            stage_type: "EXTRACT_TREE",
            display_name: "Extract Tree",
            profile_types: &[RagProfileType::Ingest],
            input_artifact_keys: &["textUnits.generated"],
            output_artifact_keys: &["tree.extracted", "graph.related"],
            repeatable: false,
            parallel: false,
            default_join_policy: "ALL_SUCCESS",
            default_failure_policy: "FAIL_PIPELINE",
            strategies: &[StrategyDefinition {
                strategy_id: "RAG_MARKDOWN_TREE_EXTRACT_V1",
                display_name: "Markdown Tree Extract",
                required_capabilities: &["graph"],
                required_storage_segments: &["textUnit"],
            }],
            execution_stage_id: "extractTree",
        },
        StageDefinition {
            stage_type: "VECTOR_INDEX",
            display_name: "Vector Index",
            profile_types: &[RagProfileType::Ingest],
            input_artifact_keys: &["textUnits.generated"],
            output_artifact_keys: &["index.vector"],
            repeatable: false,
            parallel: true,
            default_join_policy: "ALL_SUCCESS",
            default_failure_policy: "CONTINUE_WITH_WARNING",
            strategies: &[StrategyDefinition {
                strategy_id: "RAG_VECTOR_INDEX",
                display_name: "Vector Index",
                required_capabilities: &["vector"],
                required_storage_segments: &["vector"],
            }],
            execution_stage_id: "vectorIndex",
        },
        StageDefinition {
            stage_type: "PERSIST_TREE",
            display_name: "Persist Tree",
            profile_types: &[RagProfileType::Ingest],
            input_artifact_keys: &["graph.related"],
            output_artifact_keys: &["tree.persisted"],
            repeatable: false,
            parallel: false,
            default_join_policy: "ALL_SUCCESS",
            default_failure_policy: "FAIL_PIPELINE",
            strategies: &[StrategyDefinition {
                strategy_id: "RAG_PERSIST_TREE_GRAPH_V1",
                display_name: "Persist Tree Graph",
                required_capabilities: &["graph"],
                required_storage_segments: &["source"],
            }],
            execution_stage_id: "persistTree",
        },
        StageDefinition {
            stage_type: "PERSIST_SOURCE",
            display_name: "Persist Source",
            profile_types: &[RagProfileType::Ingest],
            input_artifact_keys: &["index.fulltext", "index.vector"],
            output_artifact_keys: &["source.persisted"],
            repeatable: false,
            parallel: false,
            default_join_policy: "ALL_SUCCESS",
            default_failure_policy: "FAIL_PIPELINE",
            strategies: &[StrategyDefinition {
                strategy_id: "RAG_PERSIST_SOURCE",
                display_name: "Persist Source",
                required_capabilities: &[],
                required_storage_segments: &["source", "textUnit"],
            }],
            execution_stage_id: "persistSource",
        },
        StageDefinition {
            stage_type: "QUESTION_GENERALIZATION",
            display_name: "Question Generalization",
            profile_types: &[RagProfileType::Retrieval],
            input_artifact_keys: &["query.original"],
            output_artifact_keys: &["query.generalized", "query.effective"],
            repeatable: false,
            parallel: false,
            default_join_policy: "ALL_SUCCESS",
            default_failure_policy: "FAIL_PIPELINE",
            strategies: &[StrategyDefinition {
                strategy_id: "RAG_QUESTION_GENERALIZATION_SIMPLE_V1",
                display_name: "Simple Question Generalization",
                required_capabilities: &[],
                required_storage_segments: &[],
            }],
            execution_stage_id: "questionGeneralization",
        },
        StageDefinition {
            stage_type: "QUERY_REWRITE",
            display_name: "Query Rewrite",
            profile_types: &[RagProfileType::Retrieval],
            input_artifact_keys: &["query.effective"],
            output_artifact_keys: &["query.rewritten", "query.effective"],
            repeatable: false,
            parallel: false,
            default_join_policy: "ALL_SUCCESS",
            default_failure_policy: "FAIL_PIPELINE",
            strategies: &[StrategyDefinition {
                strategy_id: "RAG_QUERY_REWRITE_TEMPLATE_V1",
                display_name: "Template Query Rewrite",
                required_capabilities: &[],
                required_storage_segments: &[],
            }],
            execution_stage_id: "queryRewrite",
        },
        StageDefinition {
            stage_type: "ROUTE_SEARCH",
            display_name: "Route Search",
            profile_types: &[RagProfileType::Retrieval],
            input_artifact_keys: &["query.effective"],
            output_artifact_keys: &["hits.route"],
            repeatable: true,
            parallel: true,
            default_join_policy: "ALL_SUCCESS",
            default_failure_policy: "CONTINUE_WITH_WARNING",
            strategies: &[
                StrategyDefinition {
                    strategy_id: "FULLTEXT",
                    display_name: "Legacy Fulltext Alias",
                    required_capabilities: &["fulltext"],
                    required_storage_segments: &["fulltext"],
                },
                StrategyDefinition {
                    strategy_id: "VECTOR",
                    display_name: "Legacy Vector Alias",
                    required_capabilities: &["vector"],
                    required_storage_segments: &["vector"],
                },
                StrategyDefinition {
                    strategy_id: "FULLTEXT_SQLITE_FTS5_SIMPLE_V1",
                    display_name: "SQLite FTS5 Fulltext",
                    required_capabilities: &["fulltext"],
                    required_storage_segments: &["fulltext"],
                },
                StrategyDefinition {
                    strategy_id: "VECTOR_SQLITE_VEC0_COSINE_V1",
                    display_name: "SQLite Vec0 Vector",
                    required_capabilities: &["vector"],
                    required_storage_segments: &["vector"],
                },
                StrategyDefinition {
                    strategy_id: "FULLTEXT_GRAPH_SINGLE_NODE_V1",
                    display_name: "Graph Fulltext",
                    required_capabilities: &["graph"],
                    required_storage_segments: &["fulltext"],
                },
                StrategyDefinition {
                    strategy_id: "VECTOR_GRAPH_SINGLE_NODE_V1",
                    display_name: "Graph Vector",
                    required_capabilities: &["graph"],
                    required_storage_segments: &["vector"],
                },
                StrategyDefinition {
                    strategy_id: "STRUCTURED_FILTER_SQLITE_V1",
                    display_name: "SQLite Structured Filter",
                    required_capabilities: &["fulltext"],
                    required_storage_segments: &["textUnit"],
                },
            ],
            execution_stage_id: "routeSearch",
        },
        StageDefinition {
            stage_type: "FUSE_RESULTS",
            display_name: "Fuse Results",
            profile_types: &[RagProfileType::Retrieval],
            input_artifact_keys: &["hits.route"],
            output_artifact_keys: &["hits.fused"],
            repeatable: false,
            parallel: false,
            default_join_policy: "ALL_SUCCESS",
            default_failure_policy: "FAIL_PIPELINE",
            strategies: &[
                StrategyDefinition {
                    strategy_id: "SUM_SCORE",
                    display_name: "Sum Score",
                    required_capabilities: &[],
                    required_storage_segments: &[],
                },
                StrategyDefinition {
                    strategy_id: "RRF",
                    display_name: "Reciprocal Rank Fusion",
                    required_capabilities: &[],
                    required_storage_segments: &[],
                },
            ],
            execution_stage_id: "fuseResults",
        },
        StageDefinition {
            stage_type: "DEDUP_RESULTS",
            display_name: "Deduplicate Results",
            profile_types: &[RagProfileType::Retrieval],
            input_artifact_keys: &["hits.fused"],
            output_artifact_keys: &["hits.deduped"],
            repeatable: false,
            parallel: false,
            default_join_policy: "ALL_SUCCESS",
            default_failure_policy: "FAIL_PIPELINE",
            strategies: &[
                StrategyDefinition {
                    strategy_id: "TEXT_UNIT",
                    display_name: "Text Unit Dedup",
                    required_capabilities: &[],
                    required_storage_segments: &[],
                },
                StrategyDefinition {
                    strategy_id: "CANONICAL_TEXT_UNIT",
                    display_name: "Canonical Text Unit Dedup",
                    required_capabilities: &[],
                    required_storage_segments: &[],
                },
            ],
            execution_stage_id: "dedupResults",
        },
        StageDefinition {
            stage_type: "BUILD_CONTEXT",
            display_name: "Build Context",
            profile_types: &[RagProfileType::Retrieval],
            input_artifact_keys: &["hits.deduped"],
            output_artifact_keys: &["context.pack"],
            repeatable: false,
            parallel: false,
            default_join_policy: "ALL_SUCCESS",
            default_failure_policy: "FAIL_PIPELINE",
            strategies: &[
                StrategyDefinition {
                    strategy_id: "RESULT_LIST",
                    display_name: "Result List",
                    required_capabilities: &[],
                    required_storage_segments: &[],
                },
                StrategyDefinition {
                    strategy_id: "TOKEN_BUDGET_PACK",
                    display_name: "Token Budget Pack",
                    required_capabilities: &[],
                    required_storage_segments: &[],
                },
                StrategyDefinition {
                    strategy_id: "MEMORY_CONTEXT_PACK",
                    display_name: "Memory Context Pack",
                    required_capabilities: &[],
                    required_storage_segments: &[],
                },
                StrategyDefinition {
                    strategy_id: "DOC_QA_CONTEXT_PACK",
                    display_name: "Document QA Context Pack",
                    required_capabilities: &[],
                    required_storage_segments: &[],
                },
            ],
            execution_stage_id: "buildContext",
        },
    ]
}

fn pipeline_templates(profile_type: RagProfileType) -> Vec<RagPipelineTemplateView> {
    match profile_type {
        RagProfileType::Ingest => ingest_pipeline_templates(),
        RagProfileType::Retrieval => Vec::new(),
    }
}
