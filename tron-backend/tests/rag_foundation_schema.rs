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
        "rag_project_doc_text_units_fts".to_string(),
        "rag_project_doc_text_units_vec_d4096".to_string(),
        "rag_diagnostics".to_string(),
    ]);

    assert_eq!(repository.rag_table_names().unwrap(), expected_tables);
    assert_eq!(
        repository.seeded_corpus_statuses().unwrap(),
        BTreeMap::from([
            ("code-graph".to_string(), "reserved".to_string()),
            ("memory-card".to_string(), "enabled".to_string()),
            ("project-doc".to_string(), "enabled".to_string()),
            ("skill".to_string(), "enabled".to_string()),
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
        ]),
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
fn rag_memory_embedded_source_migration_backfills_legacy_source_rows() {
    let database_path = temp_database_path("rag-memory-embedded-source-backfill");
    let database_url = database_path.to_string_lossy().to_string();
    let connection = Connection::open(&database_url).expect("legacy database should open");
    connection
        .execute_batch(include_str!(
            "../migrations/20260530_0022_create_rag_foundation.sql"
        ))
        .expect("legacy RAG foundation should initialize");
    connection
        .execute_batch("ALTER TABLE rag_memory_sources ADD COLUMN memory_type TEXT;")
        .expect("legacy memory domain column should be added");
    connection
        .execute(
            "INSERT INTO rag_memory_sources (
                source_code, source_version, source_type, scope_type, scope_code, status,
                title, content_hash, metadata, created_at, updated_at, memory_type
            )
            VALUES ('legacy-memory', 'v1', 'PLAIN_TEXT', 'SPACE', 'TRON_CORE', 'ACTIVE',
                'Legacy Memory', 'source-hash', '{\"legacy\":true}', 11, 12, 'RULE')",
            [],
        )
        .expect("legacy source should insert");
    connection
        .execute(
            "INSERT INTO rag_memory_text_units (
                text_unit_code, source_code, source_version, scope_type, scope_code, unit_type,
                text, heading_path, ordinal, token_count, content_hash, status,
                metadata, created_at, updated_at
            )
            VALUES ('legacy-memory:v1:0', 'legacy-memory', 'v1', 'SPACE', 'TRON_CORE', 'LEAF',
                'Legacy memory text.', '', 0, 3, 'unit-hash',
                'ACTIVE', '{}', 21, 22)",
            [],
        )
        .expect("legacy text unit should insert");
    drop(connection);

    let repository =
        SqliteRagRepository::connect(&database_url).expect("new RAG migration should succeed");
    assert!(
        !repository
            .rag_table_names()
            .unwrap()
            .contains("rag_memory_sources"),
        "legacy memory source table should be removed after backfill",
    );
    let connection = Connection::open(&database_url).expect("database should reopen");
    assert_eq!(
        query_string(
            &connection,
            "SELECT source_status FROM rag_memory_text_units WHERE source_code = 'legacy-memory'"
        ),
        "ACTIVE"
    );
    assert_eq!(
        query_string(
            &connection,
            "SELECT title FROM rag_memory_text_units WHERE source_code = 'legacy-memory'"
        ),
        "Legacy Memory"
    );
    assert_eq!(
        query_string(
            &connection,
            "SELECT memory_type FROM rag_memory_text_units WHERE source_code = 'legacy-memory'"
        ),
        "RULE"
    );

    let _ = fs::remove_file(database_path);
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

fn query_string(connection: &Connection, sql: &str) -> String {
    connection.query_row(sql, [], |row| row.get(0)).unwrap()
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
