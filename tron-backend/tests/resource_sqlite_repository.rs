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
