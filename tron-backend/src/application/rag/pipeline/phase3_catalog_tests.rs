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
        vec!["tree.hints", "textUnits.generated"]
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
    assert_eq!(persist_tree.input_artifact_keys, vec!["tree.summarized"]);
    assert_eq!(
        persist_tree.output_artifact_keys,
        vec!["tree.writePayload", "tree.persisted"]
    );

    let summarize_tree = ingest
        .stages
        .iter()
        .find(|stage| stage.stage_type == "SUMMARIZE_TREE")
        .expect("ingest catalog should expose SUMMARIZE_TREE");
    assert_eq!(summarize_tree.input_artifact_keys, vec!["tree.extracted"]);
    assert_eq!(summarize_tree.output_artifact_keys, vec!["tree.summarized"]);
    let llm_summary = summarize_tree
        .strategies
        .iter()
        .find(|strategy| strategy.strategy_id == "RAG_TREE_LLM_SUMMARY_V1")
        .expect("SUMMARIZE_TREE should expose the LLM summary strategy");
    assert_eq!(
        llm_summary.config_schema,
        json!({
            "type": "object",
            "properties": {
                "modelCode": {
                    "type": "string",
                    "minLength": 1
                },
                "timeoutSeconds": {
                    "type": "integer",
                    "minimum": 1
                },
                "maxPromptChars": {
                    "type": "integer",
                    "minimum": 1,
                    "default": 12000
                }
            },
            "additionalProperties": false
        })
    );
    let tree_vector = ingest
        .stages
        .iter()
        .find(|stage| stage.stage_type == "TREE_VECTOR_INDEX")
        .expect("ingest catalog should expose TREE_VECTOR_INDEX");
    assert_eq!(tree_vector.input_artifact_keys, vec!["tree.summarized"]);
    assert_eq!(tree_vector.output_artifact_keys, vec!["index.treeVector"]);

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
        vec!["tree.summarized"]
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
