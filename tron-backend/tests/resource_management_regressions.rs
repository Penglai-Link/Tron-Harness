mod common;

use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use tempfile::tempfile;

use serde_json::json;
use tron_backend::{
    app::AppState,
    application::resource::{
        DataSourceCreateCommand, ResourceCommitBinary, ResourceConnectionTarget,
        ResourceConnectionVerificationFuture, ResourceConnectionVerifier, ResourceCreateCommand,
        ResourcePageQuery, ResourceUpdateCommand,
    },
    config::{AppConfig, ResourceConfig},
    domain::resource::{ResourceError, ResourceKind, ResourceSourceKind, ServerAuthenticationType},
};

use common::prepared_config;

const ACTOR: &str = "USR-AUTH-0001";
const WORKSPACE: &str = "TRON_CORE";
const PARENT_WORKSPACE: &str = "PLATFORM";
const PARENT_ACTOR: &str = "USR-LARGE-ID";

#[derive(Clone)]
struct FakeConnectionVerifier;

impl ResourceConnectionVerifier for FakeConnectionVerifier {
    fn verify(
        &self,
        _target: ResourceConnectionTarget,
    ) -> ResourceConnectionVerificationFuture<'_> {
        Box::pin(async move { Ok(()) })
    }
}

#[test]
fn file_updates_never_change_kind_or_storage_identity() {
    let state = resource_state("update-identity");
    create_workspace_file(&state, "stable-file", b"first");
    state
        .resources
        .update_resource(
            ACTOR,
            ResourceUpdateCommand {
                workspace_code: WORKSPACE.to_string(),
                resource_code: "stable-file".to_string(),
                name: Some("Renamed".to_string()),
                description: Some("new description".to_string()),
                allow_inheritance: Some(false),
            },
        )
        .expect("update should succeed");
    let detail = get_workspace_file(&state, "stable-file");
    assert_eq!(detail.kind, ResourceKind::File);
    assert_eq!(detail.name, "Renamed");
    assert_eq!(
        detail.file.as_ref().expect("file summary").file_name,
        "notes.txt"
    );
}

#[test]
fn workspace_override_shadows_platform_resource_and_delete_restores_it() {
    let state = resource_state("override-restore");
    state
        .resources
        .create_file_resource(
            PARENT_ACTOR,
            ResourceCreateCommand {
                workspace_code: PARENT_WORKSPACE.to_string(),
                resource_code: "shared".to_string(),
                kind: ResourceKind::File,
                name: "Shared".to_string(),
                description: String::new(),
                allow_inheritance: true,
            },
            Some(ResourceCommitBinary {
                source: temp_file(b"platform"),
                file_name: "shared.txt".to_string(),
            }),
        )
        .expect("platform resource should exist");

    state
        .resources
        .create_file_resource(
            ACTOR,
            ResourceCreateCommand {
                workspace_code: WORKSPACE.to_string(),
                resource_code: "shared".to_string(),
                kind: ResourceKind::File,
                name: "Shared override".to_string(),
                description: String::new(),
                allow_inheritance: false,
            },
            Some(ResourceCommitBinary {
                source: temp_file(b"workspace"),
                file_name: "shared-local.txt".to_string(),
            }),
        )
        .expect("workspace may override an inherited resource code");

    let page = workspace_page(&state);
    assert_eq!(page.total, 1);
    assert_eq!(page.records[0].source_kind, ResourceSourceKind::Override);
    assert_eq!(page.records[0].name, "Shared override");

    let batch = state
        .resources
        .download_binary_records(ACTOR, WORKSPACE, &["shared".into(), "shared".into()])
        .unwrap();
    assert_eq!(batch.len(), 2);
    assert!(
        batch
            .iter()
            .all(|record| record.workspace_code == WORKSPACE)
    );
    let mut body = Vec::new();
    state
        .resources
        .download_binary_storage(&batch[0], &mut body)
        .unwrap();
    assert_eq!(body, b"workspace");

    state
        .resources
        .delete_resource(
            ACTOR,
            tron_backend::application::resource::ResourceDeleteCommand {
                workspace_code: WORKSPACE.to_string(),
                resource_code: "shared".to_string(),
            },
        )
        .expect("override should be deletable");
    let page = workspace_page(&state);
    assert_eq!(page.total, 1);
    assert_eq!(page.records[0].source_kind, ResourceSourceKind::Inherited);
    let inherited = state
        .resources
        .download_binary_records(ACTOR, WORKSPACE, &["shared".into()])
        .unwrap();
    assert_eq!(inherited[0].workspace_code, PARENT_WORKSPACE);
}

#[test]
fn non_inheritable_parent_resources_are_hidden_from_child_workspaces() {
    let state = resource_state("inheritance-flag");
    state
        .resources
        .create_file_resource(
            PARENT_ACTOR,
            ResourceCreateCommand {
                workspace_code: PARENT_WORKSPACE.to_string(),
                resource_code: "parent-only".to_string(),
                kind: ResourceKind::Pdf,
                name: "Parent only".to_string(),
                description: String::new(),
                allow_inheritance: false,
            },
            Some(ResourceCommitBinary {
                source: temp_file(b"%PDF"),
                file_name: "private.pdf".to_string(),
            }),
        )
        .expect("parent resource should exist");
    let page = workspace_page(&state);
    assert_eq!(page.total, 0);
    assert!(matches!(
        state
            .resources
            .download_binary_records(ACTOR, WORKSPACE, &["parent-only".into()]),
        Err(ResourceError::NotFound)
    ));
}

#[test]
fn binary_batch_keeps_order_and_checks_every_resource_privacy() {
    struct DenySecond;
    impl tron_backend::application::resource::ResourcePrivacyPolicy for DenySecond {
        fn can_read(&self, _: &str, _: &str, resource: &str) -> bool {
            resource != "second"
        }
        fn is_protected(&self, _: &str, resource: &str) -> bool {
            resource == "second"
        }
    }
    let state = resource_state("binary-batch-privacy");
    create_workspace_file(&state, "first", b"one");
    create_workspace_file(&state, "second", b"two");
    let ordered = state
        .resources
        .download_binary_records(
            ACTOR,
            WORKSPACE,
            &["second".into(), "first".into(), "second".into()],
        )
        .unwrap();
    assert_eq!(
        ordered
            .iter()
            .map(|r| r.resource_code.as_str())
            .collect::<Vec<_>>(),
        ["second", "first", "second"]
    );
    let scoped = state
        .resources
        .clone()
        .with_privacy_policy(std::sync::Arc::new(DenySecond));
    assert!(
        scoped
            .download_binary_records(ACTOR, WORKSPACE, &["first".into()])
            .is_ok()
    );
    assert!(
        scoped
            .download_binary_records(ACTOR, WORKSPACE, &["first".into(), "second".into()])
            .is_err()
    );
    assert!(matches!(
        scoped.download_binary_records(ACTOR, "not-a-workspace", &["first".into()]),
        Err(ResourceError::InvalidWorkspace)
    ));
}

#[test]
fn connection_kinds_cannot_be_created_through_the_file_endpoint() {
    let state = resource_state("kind-guard");
    for kind in [
        ResourceKind::MysqlConnection,
        ResourceKind::ServerConnection,
        ResourceKind::ServerAccount,
    ] {
        let result = state.resources.create_file_resource(
            ACTOR,
            ResourceCreateCommand {
                workspace_code: WORKSPACE.to_string(),
                resource_code: format!("guarded-{}", kind.as_str()),
                kind,
                name: "Guarded".to_string(),
                description: String::new(),
                allow_inheritance: false,
            },
            Some(ResourceCommitBinary {
                source: temp_file(b"payload"),
                file_name: "payload.txt".to_string(),
            }),
        );
        assert_eq!(result.err(), Some(ResourceError::InvalidInput));
    }
}

#[test]
fn data_source_update_reverifies_attributes_before_persisting() {
    let state = resource_state("update-reverify");
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime should build");
    rt.block_on(async {
        state
            .resources
            .create_data_source_for_http(
                ACTOR,
                DataSourceCreateCommand {
                    workspace_code: WORKSPACE.to_string(),
                    resource_code: "MYSQL_REVERIFY".to_string(),
                    kind: ResourceKind::MysqlConnection,
                    name: "MySQL".to_string(),
                    description: String::new(),
                    attributes: mysql_attributes("mysql.internal"),
                    allow_inheritance: false,
                },
            )
            .await
            .expect("data source should be created");
        state
            .resources
            .update_data_source_for_http(
                ACTOR,
                tron_backend::application::resource::DataSourceUpdateCommand {
                    workspace_code: WORKSPACE.to_string(),
                    resource_code: "MYSQL_REVERIFY".to_string(),
                    name: None,
                    description: None,
                    attributes: Some(mysql_attributes("mysql-relocated.internal")),
                    allow_inheritance: None,
                },
            )
            .await
            .expect("data source attributes should update");
    });
    let page = workspace_page(&state);
    let record = page
        .records
        .iter()
        .find(|record| record.resource_code == "MYSQL_REVERIFY")
        .expect("data source should remain queryable");
    assert_eq!(
        record.attributes, None,
        "list responses must not expose attributes"
    );
    let record = state
        .resources
        .get_resource(
            ACTOR,
            tron_backend::application::resource::ResourceGetQuery {
                workspace_code: WORKSPACE.to_string(),
                resource_code: "MYSQL_REVERIFY".to_string(),
            },
        )
        .expect("data source detail should remain queryable");
    assert_eq!(
        record
            .attributes
            .as_ref()
            .and_then(|value| value.pointer("/endpoints/0/host"))
            .and_then(serde_json::Value::as_str),
        Some("mysql-relocated.internal")
    );
}

#[test]
fn server_account_authentication_constraints_are_enforced() {
    let state = resource_state("account-constraints");
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime should build");
    let result = rt.block_on(async {
        state
            .resources
            .create_server_for_http(
                ACTOR,
                tron_backend::application::resource::ServerCreateCommand {
                    workspace_code: WORKSPACE.to_string(),
                    server_code: "SERVER_INVALID".to_string(),
                    name: "Invalid".to_string(),
                    description: String::new(),
                    host: "server.internal".to_string(),
                    port: 22,
                    allow_inheritance: false,
                    initial_account: Some(
                        tron_backend::application::resource::ServerAccountCreateInput {
                            account_code: "SERVER_INVALID_ACCOUNT".to_string(),
                            name: "Broken".to_string(),
                            description: String::new(),
                            username: "root".to_string(),
                            authentication_type: ServerAuthenticationType::Password,
                            password_variable_key: None,
                            private_key_variable_key: None,
                            private_key_passphrase_variable_key: None,
                            allow_inheritance: false,
                        },
                    ),
                },
            )
            .await
    });
    assert_eq!(result.err(), Some(ResourceError::InvalidInput));
    assert_eq!(workspace_page(&state).total, 0);
}

fn mysql_attributes(host: &str) -> serde_json::Value {
    json!({
        "connectionMode": "SINGLE",
        "endpoints": [{ "host": host, "port": 3306 }],
        "database": "tron",
        "credentials": {
            "username": "tron_user",
            "password": { "source": "DIRECT", "value": "database-password" }
        },
        "tls": { "mode": "DISABLED" }
    })
}

fn create_workspace_file(state: &AppState, resource_code: &str, payload: &[u8]) {
    state
        .resources
        .create_file_resource(
            ACTOR,
            ResourceCreateCommand {
                workspace_code: WORKSPACE.to_string(),
                resource_code: resource_code.to_string(),
                kind: ResourceKind::File,
                name: resource_code.to_string(),
                description: String::new(),
                allow_inheritance: false,
            },
            Some(ResourceCommitBinary {
                source: temp_file(payload),
                file_name: "notes.txt".to_string(),
            }),
        )
        .expect("file resource should be created");
}

fn get_workspace_file(
    state: &AppState,
    resource_code: &str,
) -> tron_backend::application::resource::ResourceRecord {
    state
        .resources
        .get_resource(
            ACTOR,
            tron_backend::application::resource::ResourceGetQuery {
                workspace_code: WORKSPACE.to_string(),
                resource_code: resource_code.to_string(),
            },
        )
        .expect("resource should exist")
}

fn workspace_page(state: &AppState) -> tron_backend::application::resource::ResourcePage {
    state
        .resources
        .query_resources(
            ACTOR,
            ResourcePageQuery {
                workspace_code: WORKSPACE.to_string(),
                kinds: Vec::new(),
                source_kind: None,
                search: None,
                current: 1,
                size: 50,
            },
        )
        .expect("workspace page should succeed")
}

fn resource_state(test_name: &str) -> AppState {
    let mut config = resource_config(test_name);
    config.resource = ResourceConfig::with_default_root(
        temp_directory(&format!("{test_name}-objects"))
            .display()
            .to_string(),
    );
    let mut state =
        AppState::new(prepared_config(config)).expect("resource test app should initialize");
    state.resources = state
        .resources
        .clone()
        .with_connection_verifier(std::sync::Arc::new(FakeConnectionVerifier));
    state
}

fn resource_config(test_name: &str) -> AppConfig {
    let base = temp_directory(test_name);
    let mut config = AppConfig::default();
    config.database_url = base.join("tron.sqlite3").display().to_string();
    config
}

fn temp_directory(test_name: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should advance")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("tron-resource-regressions-{test_name}-{nanos}"));
    fs::create_dir_all(&path).expect("temporary directory should be created");
    path
}

fn temp_file(payload: &[u8]) -> std::fs::File {
    let mut file = tempfile().expect("tempfile should create");
    std::io::Write::write_all(&mut file, payload).expect("payload should write");
    std::io::Seek::seek(&mut file, std::io::SeekFrom::Start(0)).expect("file should rewind");
    file
}
