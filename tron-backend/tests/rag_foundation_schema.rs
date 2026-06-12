use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::Connection;
use tron_backend::adapters::outbound::persistence::sqlite::SqliteRagRepository;
use tron_backend::domain::rag::RagScopeFields;

#[test]
fn rag_foundation_migration_creates_approved_tables_and_seeds_corpora_idempotently() {
    let repository =
        SqliteRagRepository::connect(":memory:").expect("RAG repository should connect");
    let expected_tables = BTreeSet::from([
        "rag_corpora".to_string(),
        "rag_profiles".to_string(),
        "rag_project_doc_sources".to_string(),
        "rag_memory_text_units".to_string(),
        "rag_memory_text_units_fts".to_string(),
        "rag_memory_text_units_vec_d4096".to_string(),
        "rag_project_doc_text_units".to_string(),
        "rag_skill_text_units".to_string(),
        "rag_skill_text_units_fts".to_string(),
        "rag_skill_text_units_vec_d4096".to_string(),
        "rag_project_doc_text_units_fts".to_string(),
        "rag_project_doc_text_units_vec_d4096".to_string(),
        "rag_diagnostics".to_string(),
    ]);

    assert_eq!(repository.rag_table_names().unwrap(), expected_tables);
    assert!(
        !repository
            .rag_table_names()
            .unwrap()
            .contains("rag_skill_graph_text_units"),
        "skill-graph stores skill payloads in graph nodes, not SQLite text-unit tables",
    );
    assert_eq!(
        repository.seeded_corpus_statuses().unwrap(),
        BTreeMap::from([
            ("code-graph".to_string(), "reserved".to_string()),
            ("memory-card".to_string(), "enabled".to_string()),
            ("project-doc".to_string(), "enabled".to_string()),
            ("skill".to_string(), "enabled".to_string()),
            ("skill-graph".to_string(), "enabled".to_string()),
        ]),
    );

    repository
        .apply_rag_migrations()
        .expect("RAG migrations should be idempotent");

    assert_eq!(
        repository.seeded_corpus_statuses().unwrap(),
        BTreeMap::from([
            ("code-graph".to_string(), "reserved".to_string()),
            ("memory-card".to_string(), "enabled".to_string()),
            ("project-doc".to_string(), "enabled".to_string()),
            ("skill".to_string(), "enabled".to_string()),
            ("skill-graph".to_string(), "enabled".to_string()),
        ]),
    );
}

#[test]
fn extension_indexes_use_configured_enabled_corpus_text_unit_tables() {
    let database_path = temp_database_path("rag-configured-extension-indexes");
    let database_url = database_path.to_string_lossy().to_string();
    let repository =
        SqliteRagRepository::connect(&database_url).expect("RAG repository should connect");

    let connection = Connection::open(&database_url).expect("database should open");
    connection
        .execute_batch(
            "CREATE TABLE rag_custom_text_units (
                id INTEGER PRIMARY KEY,
                text_unit_code TEXT NOT NULL UNIQUE,
                source_code TEXT NOT NULL,
                source_version TEXT NOT NULL,
                scope_type TEXT NOT NULL,
                scope_code TEXT NOT NULL,
                text TEXT NOT NULL,
                heading_path TEXT NOT NULL,
                ordinal INTEGER NOT NULL,
                status TEXT NOT NULL,
                metadata TEXT NOT NULL DEFAULT '{}'
             );
             INSERT INTO rag_corpora (
                corpus_id, display_name, status, description, capabilities_json,
                filter_schema_json, field_mapping_json, storage_binding_json, created_at, updated_at
             )
             VALUES (
                'custom-corpus', 'Custom Corpus', 'enabled', 'Custom test corpus.',
                '{\"fulltext\":true,\"vector\":true,\"reserved\":false}',
                '{\"fields\":{\"scopeType\":{\"type\":\"string\",\"column\":\"scope_type\"}}}',
                '{\"source\":{\"sourceCode\":\"source_code\",\"sourceVersion\":\"source_version\",\"scopeType\":\"scope_type\",\"scopeCode\":\"scope_code\"},\"textUnit\":{\"text\":\"text\"}}',
                '{\"layout\":\"EMBEDDED_SOURCE\",\"table\":\"rag_custom_text_units\",\"diagnosticsTable\":\"rag_diagnostics\"}',
                1780339200000, 1780339200000
             );",
        )
        .expect("custom corpus should seed");
    drop(connection);

    repository
        .apply_rag_sqlite_extension_index_migration()
        .expect("extension migration should use configured corpus tables");
    let connection = Connection::open(&database_url).expect("database should reopen");

    assert_eq!(
        schema_object_count(&connection, "rag_custom_text_units_fts"),
        1
    );
    assert_eq!(
        schema_object_count(&connection, "rag_custom_text_units_vec_d4096"),
        1
    );
}

#[test]
fn rag_foundation_embeds_memory_source_and_keeps_project_doc_split() {
    let repository =
        SqliteRagRepository::connect(":memory:").expect("RAG repository should connect");
    let forbidden_tables = BTreeSet::from([
        "rag_sources".to_string(),
        "rag_text_units".to_string(),
        "rag_fulltext".to_string(),
        "rag_vectors".to_string(),
        "rag_memory_sources".to_string(),
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
        "Memory source data is embedded in TextUnit while Project Doc keeps separate source storage",
    );

    for table in [
        "rag_memory_text_units",
        "rag_project_doc_text_units",
        "rag_skill_text_units",
    ] {
        let columns = repository.table_columns(table).unwrap();
        for forbidden in [
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
                !columns.contains(forbidden),
                "{table} should keep index payload out of the business table",
            );
        }
    }
    for sidecar in [
        "rag_memory_text_units_fts",
        "rag_memory_text_units_vec_d4096",
        "rag_project_doc_text_units_fts",
        "rag_project_doc_text_units_vec_d4096",
        "rag_skill_text_units_fts",
        "rag_skill_text_units_vec_d4096",
    ] {
        assert!(
            repository.rag_table_names().unwrap().contains(sidecar),
            "{sidecar} should provide the reusable search index contract",
        );
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
        "rag_project_doc_sources",
        "rag_memory_text_units",
        "rag_project_doc_text_units",
        "rag_skill_text_units",
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

#[test]
fn rag_domain_fields_migration_adds_memory_governance_columns_and_indexes_idempotently() {
    let database_path = temp_database_path("rag-domain-fields-memory");
    let database_url = database_path.to_string_lossy().to_string();

    SqliteRagRepository::connect(&database_url).expect("first RAG migration should succeed");
    let repository = SqliteRagRepository::connect(&database_url)
        .expect("second RAG migration should be idempotent");

    let columns = repository.table_columns("rag_memory_text_units").unwrap();
    for required in [
        "source_type",
        "source_status",
        "title",
        "source_content_hash",
        "source_metadata",
        "source_created_at",
        "source_updated_at",
        "memory_type",
        "memory_status",
        "review_status",
        "version",
        "superseded_by",
        "conflict_group",
        "duplicate_group",
        "injectable",
        "importance_score",
        "confidence_score",
        "quality_score",
        "pin_level",
        "expires_at",
        "derived_source_type",
        "derived_source_code",
    ] {
        assert!(
            columns.contains(required),
            "rag_memory_text_units should expose {required}",
        );
    }

    let indexes = table_indexes(&database_url, "rag_memory_text_units");
    for required in [
        "idx_rag_memory_text_units_source_status",
        "idx_rag_memory_text_units_domain_status",
        "idx_rag_memory_text_units_domain_rank",
        "idx_rag_memory_text_units_expires",
    ] {
        assert!(
            indexes.contains(required),
            "rag_memory_text_units should expose {required}",
        );
    }

    let _ = fs::remove_file(database_path);
}

#[test]
fn rag_memory_fresh_schema_embeds_source_fields_without_legacy_source_table() {
    let repository =
        SqliteRagRepository::connect(":memory:").expect("RAG repository should connect");

    assert!(
        !repository
            .rag_table_names()
            .unwrap()
            .contains("rag_memory_sources"),
        "memory source fields should live on rag_memory_text_units in the fresh schema",
    );

    let columns = repository.table_columns("rag_memory_text_units").unwrap();
    for required in [
        "source_type",
        "source_status",
        "title",
        "source_content_hash",
        "source_metadata",
        "source_created_at",
        "source_updated_at",
        "memory_type",
    ] {
        assert!(
            columns.contains(required),
            "rag_memory_text_units should expose {required}",
        );
    }
}

#[test]
fn rag_domain_fields_migration_adds_project_doc_repo_module_columns_and_indexes_idempotently() {
    let database_path = temp_database_path("rag-domain-fields-project-doc");
    let database_url = database_path.to_string_lossy().to_string();

    SqliteRagRepository::connect(&database_url).expect("first RAG migration should succeed");
    let repository = SqliteRagRepository::connect(&database_url)
        .expect("second RAG migration should be idempotent");

    let columns = repository.table_columns("rag_project_doc_sources").unwrap();
    for required in ["repo_code", "module_code", "module_name"] {
        assert!(
            columns.contains(required),
            "rag_project_doc_sources should expose {required}",
        );
    }
    for reused in [
        "document_code",
        "document_type",
        "path",
        "title",
        "body_hash",
    ] {
        assert!(
            columns.contains(reused),
            "rag_project_doc_sources should keep existing {reused}",
        );
    }

    let indexes = table_indexes(&database_url, "rag_project_doc_sources");
    for required in [
        "idx_rag_project_doc_sources_repo_module",
        "idx_rag_project_doc_sources_path",
    ] {
        assert!(
            indexes.contains(required),
            "rag_project_doc_sources should expose {required}",
        );
    }

    let _ = fs::remove_file(database_path);
}

fn table_indexes(database_url: &str, table_name: &str) -> BTreeSet<String> {
    let connection = Connection::open(database_url).expect("database should be readable");
    let mut statement = connection
        .prepare(
            "SELECT name
             FROM sqlite_schema
             WHERE type = 'index'
               AND tbl_name = ?1
             ORDER BY name",
        )
        .expect("index query should prepare");
    statement
        .query_map([table_name], |row| row.get::<_, String>(0))
        .expect("index query should run")
        .collect::<Result<BTreeSet<_>, _>>()
        .expect("index names should collect")
}

fn schema_object_count(connection: &Connection, name: &str) -> i64 {
    connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_schema WHERE name = ?1",
            [name],
            |row| row.get(0),
        )
        .unwrap()
}

fn temp_database_path(test_name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "tron-{test_name}-{nanos}-{}.sqlite3",
        std::process::id()
    ))
}
