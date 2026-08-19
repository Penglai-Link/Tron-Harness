use rusqlite::{Connection, params};
use serde_json::json;
use tempfile::tempdir;
use tron_backend::{
    adapters::outbound::persistence::SqliteResourceRepository,
    application::resource::ResourceRepository,
    domain::resource::{
        DataSourceConnectionRecord, ResourceError, ResourceFileRecord, ResourceKind,
        ServerAccountRecord, ServerAuthenticationType, ServerRecord,
    },
    shared::time::current_timestamp,
};

#[test]
fn reopening_the_repository_does_not_remove_simplified_records() {
    let directory = tempdir().expect("temporary directory should exist");
    let database = directory.path().join("resources.sqlite3");
    let database_url = database.display().to_string();
    let record = file_record("TRON_CORE", "survives-restart");

    let mut repository =
        SqliteResourceRepository::connect(&database_url).expect("repository should connect");
    repository
        .insert_file(&record)
        .expect("file should be inserted");
    drop(repository);

    let mut reopened =
        SqliteResourceRepository::connect(&database_url).expect("repository should reopen");
    let files = reopened
        .list_files()
        .expect("files should load after reopen");

    assert_eq!(files, vec![record]);
}

#[test]
fn legacy_platform_resource_is_migrated_without_removing_legacy_rows() {
    let directory = tempdir().expect("temporary directory should exist");
    let database = directory.path().join("legacy-resources.sqlite3");
    seed_legacy_platform_file(&database);
    let database_url = database.display().to_string();

    let mut repository =
        SqliteResourceRepository::connect(&database_url).expect("legacy database should migrate");
    let files = repository.list_files().expect("migrated files should load");

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].workspace_code, "PLATFORM");
    assert_eq!(files[0].resource_code, "legacy-platform-file");
    assert_eq!(
        files[0].storage_locator,
        "local://default/legacy-platform-file"
    );
    drop(repository);

    let connection = Connection::open(&database).expect("legacy database should remain readable");
    let legacy_count = connection
        .query_row("SELECT COUNT(*) FROM resources", [], |row| {
            row.get::<_, i64>(0)
        })
        .expect("legacy rows should remain");
    assert_eq!(legacy_count, 1);
}

#[test]
fn legacy_server_and_account_migrate_with_a_valid_parent_reference() {
    let directory = tempdir().expect("temporary directory should exist");
    let database = directory.path().join("legacy-server-bundle.sqlite3");
    seed_legacy_server_bundle(&database);
    let database_url = database.display().to_string();

    let mut repository =
        SqliteResourceRepository::connect(&database_url).expect("legacy database should migrate");
    assert_eq!(
        repository
            .list_servers()
            .expect("server should migrate")
            .len(),
        1,
    );
    let accounts = repository
        .list_server_accounts()
        .expect("account should migrate");
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].server_workspace_code, "TRON_CORE");
    assert_eq!(accounts[0].server_code, "z-server");
}

#[test]
fn resource_code_registry_prevents_cross_kind_collisions() {
    let directory = tempdir().expect("temporary directory should exist");
    let database_url = directory
        .path()
        .join("resources.sqlite3")
        .display()
        .to_string();
    let mut repository =
        SqliteResourceRepository::connect(&database_url).expect("repository should connect");

    repository
        .insert_file(&file_record("TRON_CORE", "shared-code"))
        .expect("first kind should be inserted");
    let conflict =
        repository.insert_data_source_connection(&data_source_record("TRON_CORE", "shared-code"));

    assert_eq!(conflict, Err(ResourceError::Conflict));
    assert_eq!(
        repository
            .list_data_source_connections()
            .expect("data sources should load")
            .len(),
        0,
    );
}

#[test]
fn stale_resource_update_is_reported_as_not_updated() {
    let directory = tempdir().expect("temporary directory should exist");
    let database_url = directory
        .path()
        .join("resources.sqlite3")
        .display()
        .to_string();
    let mut repository =
        SqliteResourceRepository::connect(&database_url).expect("repository should connect");
    let original = file_record("TRON_CORE", "revision-checked");
    repository
        .insert_file(&original)
        .expect("file should be inserted");

    let mut current = original.clone();
    current.name = "Current writer".to_string();
    current.updated_at = current_timestamp();
    assert!(
        repository
            .update_file(&current)
            .expect("first update should succeed")
    );
    assert!(
        !repository
            .update_file(&original)
            .expect("stale update should be rejected without an error")
    );

    let persisted = repository
        .list_files()
        .expect("files should load")
        .into_iter()
        .next()
        .expect("file should remain");
    assert_eq!(persisted.name, "Current writer");
    assert_eq!(persisted.revision, 1);
}

#[test]
fn server_bundle_rolls_back_when_initial_account_cannot_be_registered() {
    let directory = tempdir().expect("temporary directory should exist");
    let database_url = directory
        .path()
        .join("resources.sqlite3")
        .display()
        .to_string();
    let mut repository =
        SqliteResourceRepository::connect(&database_url).expect("repository should connect");
    repository
        .insert_file(&file_record("TRON_CORE", "taken-account-code"))
        .expect("conflicting code should exist");

    let timestamp = current_timestamp();
    let server = ServerRecord {
        workspace_code: "TRON_CORE".to_string(),
        resource_code: "server-with-conflicting-account".to_string(),
        name: "Server".to_string(),
        description: String::new(),
        host: "server.internal".to_string(),
        port: 22,
        allow_inheritance: false,
        revision: 0,
        created_at: timestamp.clone(),
        updated_at: timestamp.clone(),
    };
    let account = ServerAccountRecord {
        workspace_code: "TRON_CORE".to_string(),
        resource_code: "taken-account-code".to_string(),
        server_workspace_code: server.workspace_code.clone(),
        server_code: server.resource_code.clone(),
        name: "Account".to_string(),
        description: String::new(),
        username: "root".to_string(),
        authentication_type: ServerAuthenticationType::Password,
        password_variable_key: Some("SSH_PASSWORD".to_string()),
        private_key_variable_key: None,
        private_key_passphrase_variable_key: None,
        allow_inheritance: false,
        revision: 0,
        created_at: timestamp.clone(),
        updated_at: timestamp,
    };

    assert_eq!(
        repository.insert_server_bundle(&server, &account),
        Err(ResourceError::Conflict),
    );
    assert!(
        repository
            .list_servers()
            .expect("servers should load")
            .is_empty()
    );
}

#[test]
fn server_account_keeps_an_inherited_server_reference_and_prevents_parent_deletion() {
    let directory = tempdir().expect("temporary directory should exist");
    let database_url = directory
        .path()
        .join("resources.sqlite3")
        .display()
        .to_string();
    let mut repository =
        SqliteResourceRepository::connect(&database_url).expect("repository should connect");
    let timestamp = current_timestamp();
    let server = ServerRecord {
        workspace_code: "PLATFORM".to_string(),
        resource_code: "shared-server".to_string(),
        name: "Shared server".to_string(),
        description: String::new(),
        host: "server.internal".to_string(),
        port: 22,
        allow_inheritance: true,
        revision: 0,
        created_at: timestamp.clone(),
        updated_at: timestamp.clone(),
    };
    repository
        .insert_server(&server)
        .expect("parent server should be inserted");
    let account = ServerAccountRecord {
        workspace_code: "TRON_CORE".to_string(),
        resource_code: "shared-server-deploy".to_string(),
        server_workspace_code: "PLATFORM".to_string(),
        server_code: server.resource_code.clone(),
        name: "Deploy".to_string(),
        description: String::new(),
        username: "deploy".to_string(),
        authentication_type: ServerAuthenticationType::Password,
        password_variable_key: Some("SSH_PASSWORD".to_string()),
        private_key_variable_key: None,
        private_key_passphrase_variable_key: None,
        allow_inheritance: false,
        revision: 0,
        created_at: timestamp.clone(),
        updated_at: timestamp,
    };
    repository
        .insert_server_account(&account)
        .expect("an account may reference an inherited server");
    let mut orphan = account.clone();
    orphan.resource_code = "orphan-account".to_string();
    orphan.server_code = "missing-server".to_string();
    assert_eq!(
        repository.insert_server_account(&orphan),
        Err(ResourceError::Conflict),
    );

    assert_eq!(
        repository.delete_server_if_no_accounts("PLATFORM", "shared-server", 0),
        Err(ResourceError::ServerHasAccounts),
    );
    assert_eq!(
        repository
            .list_server_accounts()
            .expect("account should remain"),
        vec![account],
    );
}

fn file_record(workspace_code: &str, resource_code: &str) -> ResourceFileRecord {
    let timestamp = current_timestamp();
    ResourceFileRecord {
        workspace_code: workspace_code.to_string(),
        resource_code: resource_code.to_string(),
        kind: ResourceKind::File,
        name: resource_code.to_string(),
        description: String::new(),
        file_name: "notes.txt".to_string(),
        file_extension: "txt".to_string(),
        byte_size: 5,
        storage_locator: format!("local://default/{resource_code}"),
        allow_inheritance: false,
        revision: 0,
        created_at: timestamp.clone(),
        updated_at: timestamp,
    }
}

fn data_source_record(workspace_code: &str, resource_code: &str) -> DataSourceConnectionRecord {
    let timestamp = current_timestamp();
    DataSourceConnectionRecord {
        workspace_code: workspace_code.to_string(),
        resource_code: resource_code.to_string(),
        kind: ResourceKind::MysqlConnection,
        name: resource_code.to_string(),
        description: String::new(),
        attributes: json!({ "connectionMode": "SINGLE" }),
        allow_inheritance: false,
        revision: 0,
        created_at: timestamp.clone(),
        updated_at: timestamp,
    }
}

fn seed_legacy_platform_file(database: &std::path::Path) {
    let connection = Connection::open(database).expect("legacy database should open");
    connection
        .execute_batch(include_str!(
            "../migrations/20260716_0003_create_resource_management.sql"
        ))
        .expect("legacy schema should apply");
    connection
        .execute(
            "INSERT INTO resources (
                scope_kind, scope_code, resource_code, resource_version,
                definition_scope_kind, definition_scope_code, definition_code, definition_version,
                based_on_resource_version, name, description, attributes_json, allow_inheritance,
                draft_revision, validation_status, lifecycle_status, created_at, updated_at, published_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                ?14, ?15, ?16, ?17, ?18, ?19
            )",
            params![
                "PLATFORM",
                "PLATFORM",
                "legacy-platform-file",
                "v1",
                "PLATFORM",
                "PLATFORM",
                "FILE",
                "v1",
                Option::<String>::None,
                "Legacy platform file",
                "kept during migration",
                r#"{"fileName":"legacy.txt","fileExtension":"txt"}"#,
                1_i64,
                0_i64,
                "VALID",
                "ACTIVE",
                1_700_000_000_000_i64,
                1_700_000_000_000_i64,
                Some(1_700_000_000_000_i64),
            ],
        )
        .expect("legacy resource should insert");
    connection
        .execute(
            "INSERT INTO resource_binary_mappings (
                scope_kind, scope_code, resource_code, resource_version,
                binary_name, storage_locator, byte_size, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                "PLATFORM",
                "PLATFORM",
                "legacy-platform-file",
                "v1",
                "legacy.txt",
                "local://default/legacy-platform-file",
                5_i64,
                1_700_000_000_000_i64,
            ],
        )
        .expect("legacy binary mapping should insert");
}

fn seed_legacy_server_bundle(database: &std::path::Path) {
    let connection = Connection::open(database).expect("legacy database should open");
    connection
        .execute_batch(include_str!(
            "../migrations/20260716_0003_create_resource_management.sql"
        ))
        .expect("legacy schema should apply");
    for (resource_code, definition_code, attributes_json) in [
        (
            "a-account",
            "SERVER_ACCOUNT",
            r#"{"serverCode":"z-server","username":"deploy","authenticationType":"Password","passwordVariableKey":"SSH_PASSWORD"}"#,
        ),
        (
            "z-server",
            "SERVER_CONNECTION",
            r#"{"host":"server.internal","port":22,"protocol":"Ssh"}"#,
        ),
    ] {
        connection
            .execute(
                "INSERT INTO resources (
                    scope_kind, scope_code, resource_code, resource_version,
                    definition_scope_kind, definition_scope_code, definition_code, definition_version,
                    based_on_resource_version, name, description, attributes_json, allow_inheritance,
                    draft_revision, validation_status, lifecycle_status, created_at, updated_at, published_at
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                    ?14, ?15, ?16, ?17, ?18, ?19
                )",
                params![
                    "WORKSPACE",
                    "TRON_CORE",
                    resource_code,
                    "v1",
                    "PLATFORM",
                    "PLATFORM",
                    definition_code,
                    "v1",
                    Option::<String>::None,
                    resource_code,
                    "legacy bundle",
                    attributes_json,
                    0_i64,
                    0_i64,
                    "VALID",
                    "ACTIVE",
                    1_700_000_000_000_i64,
                    1_700_000_000_000_i64,
                    Some(1_700_000_000_000_i64),
                ],
            )
            .expect("legacy resource should insert");
    }
}
