fn ingest_pipeline_templates() -> Vec<RagPipelineTemplateView> {
    vec![
        RagPipelineTemplateView {
            template_id: "text-standard".into(),
            display_name: "Text Standard Ingest".into(),
            description: "Prepare text payloads, split text units, build fulltext and vector indexes, then persist the source.".into(),
            pipeline: RagPipelineInput {
                mode: "DAG".into(),
                nodes: vec![
                    template_node("validate-source", "VALIDATE_SOURCE", "RAG_VALIDATE_SOURCE"),
                    template_node(
                        "prepare-payload",
                        "PREPARE_PAYLOAD",
                        "RAG_TEXT_CONTRACT_PAYLOAD_V1",
                    ),
                    template_node("split-text", "SPLIT_TEXT", "RAG_SPLIT_PLAIN_TEXT_V1"),
                    template_node("fulltext-index", "FULLTEXT_INDEX", "RAG_FULLTEXT_INDEX"),
                    template_node("vector-index", "VECTOR_INDEX", "RAG_VECTOR_INDEX"),
                    template_node("persist-source", "PERSIST_SOURCE", "RAG_PERSIST_SOURCE"),
                ],
                edges: vec![
                    template_edge("validate-source", "prepare-payload"),
                    template_edge("prepare-payload", "split-text"),
                    template_edge("split-text", "fulltext-index"),
                    template_edge("split-text", "vector-index"),
                    template_edge("fulltext-index", "persist-source"),
                    template_edge("vector-index", "persist-source"),
                ],
            },
        },
        RagPipelineTemplateView {
            template_id: "document-reference".into(),
            display_name: "Document Reference Ingest".into(),
            description: "Validate document source references while preparing indexable text payloads for fulltext and vector persistence.".into(),
            pipeline: RagPipelineInput {
                mode: "DAG".into(),
                nodes: vec![
                    template_node("validate-source", "VALIDATE_SOURCE", "RAG_VALIDATE_SOURCE"),
                    template_node(
                        "prepare-payload",
                        "PREPARE_PAYLOAD",
                        "RAG_DOCUMENT_REF_CONTRACT_PAYLOAD_V1",
                    ),
                    template_node("resolve-source", "RESOLVE_SOURCE", "PROJECT_DOC_RESOLVE_SOURCE"),
                    template_node("parse-document", "PARSE_DOCUMENT", "RAG_PARSE_MARKDOWN_DOCUMENT"),
                    template_node(
                        "split-text",
                        "SPLIT_TEXT",
                        "RAG_SPLIT_MARKDOWN_HEADING_V1",
                    ),
                    template_node("fulltext-index", "FULLTEXT_INDEX", "RAG_FULLTEXT_INDEX"),
                    template_node("vector-index", "VECTOR_INDEX", "RAG_VECTOR_INDEX"),
                    template_node("persist-source", "PERSIST_SOURCE", "RAG_PERSIST_SOURCE"),
                ],
                edges: vec![
                    template_edge("validate-source", "prepare-payload"),
                    template_edge("prepare-payload", "resolve-source"),
                    template_edge("resolve-source", "parse-document"),
                    template_edge("parse-document", "split-text"),
                    template_edge("split-text", "fulltext-index"),
                    template_edge("split-text", "vector-index"),
                    template_edge("fulltext-index", "persist-source"),
                    template_edge("vector-index", "persist-source"),
                ],
            },
        },
        RagPipelineTemplateView {
            template_id: "mineru-document-reference".into(),
            display_name: "MinerU Document Reference Ingest".into(),
            description: "Resolve PDF or DOCX project document references, parse them through MinerU into Markdown, then build fulltext and vector indexes.".into(),
            pipeline: RagPipelineInput {
                mode: "DAG".into(),
                nodes: vec![
                    template_node("validate-source", "VALIDATE_SOURCE", "RAG_VALIDATE_SOURCE"),
                    template_node(
                        "prepare-payload",
                        "PREPARE_PAYLOAD",
                        "RAG_DOCUMENT_REF_CONTRACT_PAYLOAD_V1",
                    ),
                    template_node("resolve-source", "RESOLVE_SOURCE", "PROJECT_DOC_RESOLVE_SOURCE"),
                    template_node("parse-document", "PARSE_DOCUMENT", "RAG_PARSE_MINERU_DOCUMENT"),
                    template_node(
                        "split-text",
                        "SPLIT_TEXT",
                        "RAG_SPLIT_MARKDOWN_HEADING_V1",
                    ),
                    template_node("fulltext-index", "FULLTEXT_INDEX", "RAG_FULLTEXT_INDEX"),
                    template_node("vector-index", "VECTOR_INDEX", "RAG_VECTOR_INDEX"),
                    template_node("persist-source", "PERSIST_SOURCE", "RAG_PERSIST_SOURCE"),
                ],
                edges: vec![
                    template_edge("validate-source", "prepare-payload"),
                    template_edge("prepare-payload", "resolve-source"),
                    template_edge("resolve-source", "parse-document"),
                    template_edge("parse-document", "split-text"),
                    template_edge("split-text", "fulltext-index"),
                    template_edge("split-text", "vector-index"),
                    template_edge("fulltext-index", "persist-source"),
                    template_edge("vector-index", "persist-source"),
                ],
            },
        },
        RagPipelineTemplateView {
            template_id: "graph-tree-enhanced".into(),
            display_name: "Graph Tree Enhanced Ingest".into(),
            description: "Build fulltext and vector indexes while extracting and persisting a graph tree branch for graph-enabled corpora.".into(),
            pipeline: RagPipelineInput {
                mode: "DAG".into(),
                nodes: vec![
                    template_node("validate-source", "VALIDATE_SOURCE", "RAG_VALIDATE_SOURCE"),
                    template_node(
                        "prepare-payload",
                        "PREPARE_PAYLOAD",
                        "RAG_TEXT_CONTRACT_PAYLOAD_V1",
                    ),
                    template_node("split-text", "SPLIT_TEXT", "RAG_SPLIT_PLAIN_TEXT_V1"),
                    template_node("fulltext-index", "FULLTEXT_INDEX", "RAG_FULLTEXT_INDEX"),
                    template_node("vector-index", "VECTOR_INDEX", "RAG_VECTOR_INDEX"),
                    template_node("extract-tree", "EXTRACT_TREE", "RAG_MARKDOWN_TREE_EXTRACT_V1"),
                    template_node(
                        "summarize-tree",
                        "SUMMARIZE_TREE",
                        "RAG_TREE_EXTRACTIVE_SUMMARY_V1",
                    ),
                    template_node(
                        "tree-fulltext-index",
                        "TREE_FULLTEXT_INDEX",
                        "RAG_TREE_FULLTEXT_INDEX",
                    ),
                    template_node(
                        "tree-vector-index",
                        "TREE_VECTOR_INDEX",
                        "RAG_TREE_VECTOR_INDEX",
                    ),
                    template_node("persist-tree", "PERSIST_TREE", "RAG_PERSIST_TREE_GRAPH_V1"),
                    template_node("persist-source", "PERSIST_SOURCE", "RAG_PERSIST_SOURCE"),
                ],
                edges: vec![
                    template_edge("validate-source", "prepare-payload"),
                    template_edge("prepare-payload", "split-text"),
                    template_edge("split-text", "fulltext-index"),
                    template_edge("split-text", "vector-index"),
                    template_edge("split-text", "extract-tree"),
                    template_edge("extract-tree", "summarize-tree"),
                    template_edge("summarize-tree", "tree-fulltext-index"),
                    template_edge("summarize-tree", "tree-vector-index"),
                    template_edge("tree-fulltext-index", "persist-tree"),
                    template_edge("tree-vector-index", "persist-tree"),
                    template_edge("fulltext-index", "persist-source"),
                    template_edge("vector-index", "persist-source"),
                    template_edge("persist-tree", "persist-source"),
                ],
            },
        },
    ]
}

fn template_node(node_id: &str, stage_type: &str, strategy_id: &str) -> RagPipelineNodeInput {
    RagPipelineNodeInput {
        node_id: node_id.into(),
        stage_type: stage_type.into(),
        strategy_id: strategy_id.into(),
        enabled: true,
        join_policy: None,
        failure_policy: None,
        config: None,
    }
}

fn template_edge(from: &str, to: &str) -> RagPipelineEdgeInput {
    RagPipelineEdgeInput {
        from: from.into(),
        to: to.into(),
    }
}
