mod tests {
    use super::*;
    use crate::application::rag::{RagIngestAccessContext, RagIngestScope, RagIngestSource};

    #[test]
    fn prepares_text_contract_payload_from_text_and_domain_fields() {
        let request = skill_request(json!({
            "source": {
                "code": "skill:workspace-alpha:reviewer",
                "version": "v1",
                "type": "SKILL"
            },
            "title": "reviewer",
            "text": "Skill: reviewer\nDescription: Reviews code changes before release.",
            "metadata": {
                "source": "registry"
            },
            "domainFields": {
                "skillName": "reviewer",
                "skillDescription": "Reviews code changes before release."
            }
        }));
        let bundle = ArtifactBundle::from_request(&request);

        let prepared = prepare_payload(&request, &skill_profile(), &bundle).unwrap();

        assert_eq!(prepared.title.as_deref(), Some("reviewer"));
        assert_eq!(
            prepared.domain_fields.get("skillName"),
            Some(&json!("reviewer"))
        );
        assert_eq!(
            prepared.domain_fields.get("skillDescription"),
            Some(&json!("Reviews code changes before release."))
        );
        assert_eq!(prepared.units.len(), 1);
        assert_eq!(prepared.units[0].heading_path, "reviewer");
        assert_eq!(
            prepared.units[0].text,
            "Skill: reviewer\nDescription: Reviews code changes before release."
        );
    }

    #[test]
    fn rejects_text_contract_payload_without_text() {
        let request = skill_request(json!({
            "source": {
                "code": "skill:workspace-alpha:reviewer",
                "version": "v1",
                "type": "SKILL"
            },
            "title": "reviewer",
            "domainFields": {
                "skillName": "reviewer"
            }
        }));
        let bundle = ArtifactBundle::from_request(&request);

        let error = prepare_payload(&request, &skill_profile(), &bundle).unwrap_err();

        assert_eq!(error, RagError::IngestRequestInvalid);
    }

    fn skill_request(payload: Value) -> RagIngestRequest {
        RagIngestRequest {
            corpus_id: "skill".into(),
            ingest_profile_id: "skill-ingest-v1".into(),
            workspace_code: "workspace-alpha".into(),
            access_context: RagIngestAccessContext {
                user_code: "user-alpha".into(),
                agent_code: "agent-alpha".into(),
                workspace_code: "workspace-alpha".into(),
            },
            target_scope: RagIngestScope {
                scope_type: "SPACE".into(),
                scope_code: "workspace-alpha".into(),
            },
            source: RagIngestSource {
                code: "skill:workspace-alpha:reviewer".into(),
                version: "v1".into(),
                source_type: "SKILL".into(),
            },
            payload,
        }
    }

    fn skill_profile() -> CompiledRagIngestProfile {
        CompiledRagIngestProfile {
            profile_id: "skill-ingest-v1".into(),
            corpus_id: "skill".into(),
            stage_order: vec![],
            pipeline: None,
            stage_metadata: Default::default(),
            payload_strategy_id: "RAG_TEXT_CONTRACT_PAYLOAD_V1".into(),
            split_strategy_id: "RAG_SPLIT_TEXT".into(),
            supported_source_types: vec!["SKILL".into()],
            source_resolver_strategy_id: None,
            document_parser_strategy_id: None,
            source_resolver_max_bytes: 1024,
            record_resolver_info: false,
            record_parser_info: false,
            markdown_heading_depth: 6,
            include_heading_in_index_text: false,
            document_format: "MARKDOWN".into(),
            document_formats: vec!["MARKDOWN".into()],
            degrade_policy: "SKIP_VECTOR_WITH_WARNING".into(),
            embedding_model_code: None,
            embedding_version: "v1".into(),
            graph_write_policy: serde_json::Value::Null,
        }
    }
}
