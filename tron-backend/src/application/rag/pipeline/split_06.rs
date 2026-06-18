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
}
