use std::collections::{BTreeMap, BTreeSet};

use tron_backend::adapters::outbound::persistence::sqlite::SqliteRagRepository;
use tron_backend::domain::rag::RagScopeFields;

#[test]
fn rag_foundation_migration_creates_approved_tables_and_seeds_corpora_idempotently() {
    let repository =
        SqliteRagRepository::connect(":memory:").expect("RAG repository should connect");
    let expected_tables = BTreeSet::from([
        "rag_corpora".to_string(),
        "rag_profiles".to_string(),
        "rag_memory_sources".to_string(),
        "rag_project_doc_sources".to_string(),
        "rag_memory_text_units".to_string(),
        "rag_project_doc_text_units".to_string(),
        "rag_diagnostics".to_string(),
    ]);

    assert_eq!(repository.rag_table_names().unwrap(), expected_tables);
    assert_eq!(
        repository.seeded_corpus_statuses().unwrap(),
        BTreeMap::from([
            ("code-graph".to_string(), "reserved".to_string()),
            ("memory-card".to_string(), "enabled".to_string()),
            ("project-doc".to_string(), "enabled".to_string()),
        ]),
    );

    repository
        .apply_rag_foundation_migration()
        .expect("RAG migration should be idempotent");

    assert_eq!(
        repository.seeded_corpus_statuses().unwrap(),
        BTreeMap::from([
            ("code-graph".to_string(), "reserved".to_string()),
            ("memory-card".to_string(), "enabled".to_string()),
            ("project-doc".to_string(), "enabled".to_string()),
        ]),
    );
}

#[test]
fn rag_foundation_keeps_source_and_text_unit_storage_split_by_corpus() {
    let repository =
        SqliteRagRepository::connect(":memory:").expect("RAG repository should connect");
    let forbidden_tables = BTreeSet::from([
        "rag_sources".to_string(),
        "rag_text_units".to_string(),
        "rag_fulltext".to_string(),
        "rag_vectors".to_string(),
        "rag_memory_fulltext".to_string(),
        "rag_memory_vectors".to_string(),
        "rag_project_doc_fulltext".to_string(),
        "rag_project_doc_vectors".to_string(),
    ]);

    assert!(
        repository
            .rag_table_names()
            .unwrap()
            .is_disjoint(&forbidden_tables),
        "RAG storage must stay split by Corpus and keep fulltext/vector fields in TextUnit tables",
    );

    for table in ["rag_memory_text_units", "rag_project_doc_text_units"] {
        let columns = repository.table_columns(table).unwrap();
        for required in [
            "search_text",
            "fulltext_status",
            "embedding_model_code",
            "embedding_version",
            "embedding_text_hash",
            "embedding_dimension",
            "embedding",
            "embedding_status",
        ] {
            assert!(
                columns.contains(required),
                "{table} should own {required} without separate fulltext/vector tables",
            );
        }
    }
}

#[test]
fn rag_foundation_preserves_scope_isolation_columns_without_permission_columns() {
    let repository =
        SqliteRagRepository::connect(":memory:").expect("RAG repository should connect");
    let row_scope = RagScopeFields {
        scope_type: "SPACE".to_string(),
        scope_code: "space-demo".to_string(),
    };

    assert_eq!(row_scope.scope_type, "SPACE");
    assert_eq!(row_scope.scope_code, "space-demo");

    for table in [
        "rag_memory_sources",
        "rag_project_doc_sources",
        "rag_memory_text_units",
        "rag_project_doc_text_units",
    ] {
        let columns = repository.table_columns(table).unwrap();
        assert!(columns.contains("scope_type"), "{table} keeps scope_type");
        assert!(columns.contains("scope_code"), "{table} keeps scope_code");
        for forbidden in [
            "permission",
            "permissions",
            "role_code",
            "principal_code",
            "field_permissions",
        ] {
            assert!(
                !columns.contains(forbidden),
                "{table} should not implement permission columns in the foundation schema",
            );
        }
    }
}
