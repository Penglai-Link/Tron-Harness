#[test]
fn rag_pipeline_catalog_enforces_tree_orchestration_dependencies() {
    let ingest = pipeline_catalog(RagProfileType::Ingest);
    let orchestration = |stage_type: &str| {
        ingest
            .stages
            .iter()
            .find(|stage| stage.stage_type == stage_type)
            .expect("stage should be exposed")
            .orchestration
            .clone()
    };

    assert_eq!(
        orchestration("EXTRACT_TREE").must_run_after_stage_types,
        vec!["SPLIT_TEXT"]
    );
    assert_eq!(
        orchestration("EXTRACT_TREE").must_run_before_stage_types,
        vec!["SUMMARIZE_TREE"]
    );
    assert_eq!(
        orchestration("SUMMARIZE_TREE").must_run_before_stage_types,
        vec!["TREE_FULLTEXT_INDEX", "TREE_VECTOR_INDEX"]
    );
    assert_eq!(
        orchestration("TREE_FULLTEXT_INDEX").must_run_before_stage_types,
        vec!["PERSIST_TREE", "PERSIST_SOURCE"]
    );
    assert_eq!(
        orchestration("TREE_VECTOR_INDEX").must_run_before_stage_types,
        vec!["PERSIST_TREE", "PERSIST_SOURCE"]
    );
    assert_eq!(
        orchestration("PERSIST_TREE").must_run_before_stage_types,
        vec!["PERSIST_SOURCE"]
    );
    assert_eq!(
        orchestration("PERSIST_TREE").must_run_after_stage_types,
        vec!["SUMMARIZE_TREE", "TREE_FULLTEXT_INDEX", "TREE_VECTOR_INDEX"]
    );
    assert_eq!(
        orchestration("PERSIST_SOURCE").must_run_after_stage_types,
        vec!["FULLTEXT_INDEX", "VECTOR_INDEX", "PERSIST_TREE"]
    );

    let mut missing_persist_tree_to_source = ingest_phase3_pipeline();
    missing_persist_tree_to_source
        .edges
        .retain(|edge| !(edge.from == "persist-tree" && edge.to == "persist"));
    let compiled = validate_pipeline(&missing_persist_tree_to_source, RagProfileType::Ingest)
        .expect("legacy tree pipeline should complete persist-tree before persist-source");
    assert!(
        compiled
            .edges
            .iter()
            .any(|edge| edge.from == "persist-tree" && edge.to == "persist"),
        "missing persist-tree control edge should be completed; actual edges: {:?}",
        compiled.edges,
    );
}

#[test]
fn rag_pipeline_compiles_summary_tree_persist_without_optional_tree_indexes() {
    let compiled = validate_pipeline(
        &ingest_phase3_pipeline_without_tree_indexes(),
        RagProfileType::Ingest,
    )
    .expect("summary tree pipeline should compile without optional tree index stages");

    assert_eq!(
        compiled.execution_levels,
        vec![
            vec!["validate".to_string()],
            vec!["prepare".to_string()],
            vec!["split".to_string()],
            vec![
                "extract-tree".to_string(),
                "fulltext".to_string(),
                "vector".to_string()
            ],
            vec!["summarize-tree".to_string()],
            vec!["persist-tree".to_string()],
            vec!["persist".to_string()],
        ]
    );
    let persist_tree = compiled
        .nodes
        .iter()
        .find(|node| node.stage_type == "PERSIST_TREE")
        .expect("persist tree node should compile");
    assert_eq!(persist_tree.input_artifact_keys, vec!["tree.summarized"]);
}
