#[cfg(test)]
mod template_contract_tests {
    use super::*;

    #[test]
    fn document_reference_template_splits_only_after_document_parse() {
        let ingest = pipeline_catalog(RagProfileType::Ingest);
        let template = ingest
            .templates
            .iter()
            .find(|template| template.template_id == "document-reference")
            .expect("document reference template should be exposed");
        let edges = template
            .pipeline
            .edges
            .iter()
            .map(|edge| (edge.from.as_str(), edge.to.as_str()))
            .collect::<Vec<_>>();

        assert_eq!(
            edges,
            vec![
                ("validate-source", "prepare-payload"),
                ("prepare-payload", "resolve-source"),
                ("resolve-source", "parse-document"),
                ("parse-document", "split-text"),
                ("split-text", "fulltext-index"),
                ("split-text", "vector-index"),
                ("fulltext-index", "persist-source"),
                ("vector-index", "persist-source"),
            ],
            "document-reference ingest must resolve and parse the document before splitText consumes parsed content"
        );
    }

    #[test]
    fn mineru_document_reference_template_declares_mineru_parser() {
        let ingest = pipeline_catalog(RagProfileType::Ingest);
        let template = ingest
            .templates
            .iter()
            .find(|template| template.template_id == "mineru-document-reference")
            .expect("MinerU document reference template should be exposed");
        let parse_node = template
            .pipeline
            .nodes
            .iter()
            .find(|node| node.node_id == "parse-document")
            .expect("parse document node should exist");

        assert_eq!(parse_node.strategy_id, "RAG_PARSE_MINERU_DOCUMENT");
    }

    #[test]
    fn parse_document_stage_catalog_orders_before_split_text() {
        let ingest = pipeline_catalog(RagProfileType::Ingest);
        let parse_stage = ingest
            .stages
            .iter()
            .find(|stage| stage.stage_type == "PARSE_DOCUMENT")
            .expect("parse document stage should be exposed");

        assert!(
            parse_stage
                .orchestration
                .must_run_before_stage_types
                .contains(&"SPLIT_TEXT".to_string()),
            "frontend DAG construction must know parsed document output is required before splitText"
        );
    }

    #[test]
    fn document_parse_pipeline_without_parse_to_split_edge_is_invalid() {
        let ingest = pipeline_catalog(RagProfileType::Ingest);
        let mut pipeline = ingest
            .templates
            .iter()
            .find(|template| template.template_id == "mineru-document-reference")
            .expect("MinerU document reference template should be exposed")
            .pipeline
            .clone();
        pipeline
            .edges
            .retain(|edge| !(edge.from == "parse-document" && edge.to == "split-text"));

        let errors = validate_pipeline(&pipeline, RagProfileType::Ingest)
            .expect_err("missing parse-to-split dependency should be invalid");

        assert!(
            errors
                .iter()
                .any(|error| error.code == "RAG_PIPELINE_CONTROL_DEPENDENCY_MISSING"),
            "parseDocument must remain a control dependency of splitText; actual errors: {errors:?}"
        );
    }
}
