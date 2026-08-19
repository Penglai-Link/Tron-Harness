mod common;

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::json;
use tron_backend::{
    app::AppState,
    application::resource::{
        DataSourceCreateCommand, ResourceConnectionTarget, ResourceConnectionVerificationFuture,
        ResourceConnectionVerifier, ResourceDeleteCommand, ResourcePageQuery,
        ServerAccountCreateCommand, ServerAccountCreateInput, ServerCreateCommand,
    },
    config::{AppConfig, ResourceConfig},
    domain::resource::{ResourceError, ResourceKind, ServerAuthenticationType},
};

use common::prepared_config;

const ACTOR: &str = "USR-AUTH-0001";
const WORKSPACE: &str = "TRON_CORE";
const SECRET_VARIABLE: &str = "OPENAI_API_KEY";

#[derive(Clone)]
struct FakeConnectionVerifier {
    calls: Arc<AtomicUsize>,
    fail: bool,
}

impl ResourceConnectionVerifier for FakeConnectionVerifier {
    fn verify(
        &self,
        _target: ResourceConnectionTarget,
    ) -> ResourceConnectionVerificationFuture<'_> {
        let calls = self.calls.clone();
        let fail = self.fail;
        Box::pin(async move {
            calls.fetch_add(1, Ordering::SeqCst);
            if fail {
                Err(ResourceError::ConnectionVerificationFailed(
                    "test probe failed".to_string(),
                ))
            } else {
                Ok(())
            }
        })
    }
}

#[tokio::test]
async fn data_source_create_verifies_before_persisting() {
    let (state, calls) = resource_state("data-source", false);
    let created = state
        .resources
        .create_data_source_for_http(
            ACTOR,
            DataSourceCreateCommand {
                workspace_code: WORKSPACE.to_string(),
                resource_code: "MYSQL_PRIMARY".to_string(),
                kind: ResourceKind::MysqlConnection,
                name: "Primary MySQL".to_string(),
                description: String::new(),
                attributes: mysql_attributes(),
                allow_inheritance: false,
            },
        )
        .await
        .expect("verified data source should be persisted");
    assert_eq!(created.kind, ResourceKind::MysqlConnection);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(resources(&state, Some("MYSQL_PRIMARY")).total, 1);
}

#[tokio::test]
async fn non_data_source_kind_is_rejected_by_connection_endpoint() {
    let (state, calls) = resource_state("data-source-kind-guard", false);
    let result = state
        .resources
        .create_data_source_for_http(
            ACTOR,
            DataSourceCreateCommand {
                workspace_code: WORKSPACE.to_string(),
                resource_code: "FILE_AS_CONNECTION".to_string(),
                kind: ResourceKind::File,
                name: "Not a connection".to_string(),
                description: String::new(),
                attributes: json!({}),
                allow_inheritance: false,
            },
        )
        .await;
    assert_eq!(result, Err(ResourceError::InvalidInput));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn failed_data_source_probe_does_not_persist_the_resource() {
    let (state, calls) = resource_state("data-source-failure", true);
    let error = state
        .resources
        .create_data_source_for_http(
            ACTOR,
            DataSourceCreateCommand {
                workspace_code: WORKSPACE.to_string(),
                resource_code: "MYSQL_UNREACHABLE".to_string(),
                kind: ResourceKind::MysqlConnection,
                name: "Unreachable MySQL".to_string(),
                description: String::new(),
                attributes: mysql_attributes(),
                allow_inheritance: false,
            },
        )
        .await
        .expect_err("failed connectivity must reject creation");
    assert!(matches!(
        error,
        ResourceError::ConnectionVerificationFailed(_)
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(resources(&state, Some("MYSQL_UNREACHABLE")).total, 0);
}

#[tokio::test]
async fn data_source_password_accepts_direct_input_or_a_variable_reference() {
    let (state, calls) = resource_state("data-source-password-source", false);
    for (resource_code, attributes) in [
        ("MYSQL_DIRECT_PASSWORD", mysql_attributes()),
        (
            "MYSQL_VARIABLE_PASSWORD",
            mysql_attributes_with_variable_password(),
        ),
    ] {
        state
            .resources
            .create_data_source_for_http(
                ACTOR,
                DataSourceCreateCommand {
                    workspace_code: WORKSPACE.to_string(),
                    resource_code: resource_code.to_string(),
                    kind: ResourceKind::MysqlConnection,
                    name: resource_code.to_string(),
                    description: String::new(),
                    attributes,
                    allow_inheritance: false,
                },
            )
            .await
            .expect("both password sources should be verified and persisted");
    }
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    let direct = resources(&state, Some("MYSQL_DIRECT_PASSWORD"));
    assert_eq!(
        direct.records[0]
            .attributes
            .as_ref()
            .and_then(|attributes| attributes.pointer("/credentials/password/value"))
            .and_then(serde_json::Value::as_str),
        Some("database-password")
    );
    let variable = resources(&state, Some("MYSQL_VARIABLE_PASSWORD"));
    assert_eq!(
        variable.records[0]
            .attributes
            .as_ref()
            .and_then(|attributes| attributes.pointer("/credentials/password/variableKey"))
            .and_then(serde_json::Value::as_str),
        Some(SECRET_VARIABLE)
    );
}

#[tokio::test]
async fn server_creation_is_atomic_and_one_server_accepts_multiple_accounts() {
    let (state, calls) = resource_state("server-accounts", false);
    let created = state
        .resources
        .create_server_for_http(
            ACTOR,
            ServerCreateCommand {
                workspace_code: WORKSPACE.to_string(),
                server_code: "SERVER_PROD_1".to_string(),
                name: "Production server".to_string(),
                description: String::new(),
                host: "server.internal".to_string(),
                port: 22,
                allow_inheritance: false,
                initial_account: password_account("SERVER_PROD_1_ROOT", "root"),
            },
        )
        .await
        .expect("verified server and first account should persist atomically");
    assert!(created.connectivity_verified);
    assert_eq!(created.server.kind, ResourceKind::ServerConnection);
    assert_eq!(created.initial_account.kind, ResourceKind::ServerAccount);
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    let second = state
        .resources
        .create_server_account_for_http(
            ACTOR,
            ServerAccountCreateCommand {
                workspace_code: WORKSPACE.to_string(),
                server_code: "SERVER_PROD_1".to_string(),
                account: password_account("SERVER_PROD_1_DEPLOY", "deploy"),
            },
        )
        .expect("a server should accept an additional account");
    assert_eq!(second.kind, ResourceKind::ServerAccount);
    state
        .resources
        .create_server_account_for_http(
            ACTOR,
            ServerAccountCreateCommand {
                workspace_code: WORKSPACE.to_string(),
                server_code: "SERVER_PROD_1".to_string(),
                account: private_key_account("SERVER_PROD_1_AUTOMATION", "automation"),
            },
        )
        .expect("private-key authentication should be modeled independently");
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "later account creation is not a health probe"
    );

    for account_code in [
        "SERVER_PROD_1_ROOT",
        "SERVER_PROD_1_DEPLOY",
        "SERVER_PROD_1_AUTOMATION",
    ] {
        let account = resources(&state, Some(account_code))
            .records
            .into_iter()
            .next()
            .expect("account should be queryable");
        assert_eq!(account.kind, ResourceKind::ServerAccount);
        assert_eq!(
            account
                .attributes
                .as_ref()
                .and_then(|value| value.get("serverCode"))
                .and_then(serde_json::Value::as_str),
            Some("SERVER_PROD_1")
        );
    }
    let private_key_account = resources(&state, Some("SERVER_PROD_1_AUTOMATION"))
        .records
        .into_iter()
        .next()
        .expect("private-key account should be queryable");
    assert_eq!(
        private_key_account
            .attributes
            .as_ref()
            .and_then(|value| value.get("authenticationType"))
            .and_then(serde_json::Value::as_str),
        Some("PrivateKey")
    );
}

#[tokio::test]
async fn server_with_accounts_cannot_be_deleted() {
    let (state, _) = resource_state("server-delete-guard", false);
    state
        .resources
        .create_server_for_http(
            ACTOR,
            ServerCreateCommand {
                workspace_code: WORKSPACE.to_string(),
                server_code: "SERVER_GUARDED".to_string(),
                name: "Guarded server".to_string(),
                description: String::new(),
                host: "server.internal".to_string(),
                port: 22,
                allow_inheritance: false,
                initial_account: password_account("SERVER_GUARDED_ROOT", "root"),
            },
        )
        .await
        .expect("server should be created");
    let delete = state.resources.delete_resource(
        ACTOR,
        ResourceDeleteCommand {
            workspace_code: WORKSPACE.to_string(),
            resource_code: "SERVER_GUARDED".to_string(),
        },
    );
    assert_eq!(delete, Err(ResourceError::ServerHasAccounts));
    assert_eq!(
        resources(&state, Some("SERVER_GUARDED"))
            .records
            .iter()
            .filter(|record| record.resource_code == "SERVER_GUARDED")
            .count(),
        1
    );
}

#[tokio::test]
async fn failed_ssh_probe_persists_neither_server_nor_initial_account() {
    let (state, _) = resource_state("server-atomic-failure", true);
    let result = state
        .resources
        .create_server_for_http(
            ACTOR,
            ServerCreateCommand {
                workspace_code: WORKSPACE.to_string(),
                server_code: "SERVER_FAILED".to_string(),
                name: "Failed server".to_string(),
                description: String::new(),
                host: "server.internal".to_string(),
                port: 22,
                allow_inheritance: false,
                initial_account: password_account("SERVER_FAILED_ROOT", "root"),
            },
        )
        .await;
    assert!(matches!(
        result,
        Err(ResourceError::ConnectionVerificationFailed(_))
    ));
    assert_eq!(resources(&state, Some("SERVER_FAILED")).total, 0);
    assert_eq!(resources(&state, Some("SERVER_FAILED_ROOT")).total, 0);
}

fn mysql_attributes() -> serde_json::Value {
    json!({
        "connectionMode": "SINGLE",
        "endpoints": [{ "host": "mysql.internal", "port": 3306 }],
        "database": "tron",
        "credentials": {
            "username": "tron_user",
            "password": { "source": "DIRECT", "value": "database-password" }
        },
        "tls": { "mode": "DISABLED" }
    })
}

fn mysql_attributes_with_variable_password() -> serde_json::Value {
    json!({
        "connectionMode": "SINGLE",
        "endpoints": [{ "host": "mysql.internal", "port": 3306 }],
        "database": "tron",
        "credentials": {
            "username": "tron_user",
            "password": { "source": "VARIABLE", "variableKey": SECRET_VARIABLE }
        },
        "tls": { "mode": "DISABLED" }
    })
}

fn password_account(account_code: &str, username: &str) -> ServerAccountCreateInput {
    ServerAccountCreateInput {
        account_code: account_code.to_string(),
        name: format!("{username} account"),
        description: String::new(),
        username: username.to_string(),
        authentication_type: ServerAuthenticationType::Password,
        password_variable_key: Some(SECRET_VARIABLE.to_string()),
        private_key_variable_key: None,
        private_key_passphrase_variable_key: None,
        allow_inheritance: false,
    }
}

fn private_key_account(account_code: &str, username: &str) -> ServerAccountCreateInput {
    ServerAccountCreateInput {
        account_code: account_code.to_string(),
        name: format!("{username} account"),
        description: String::new(),
        username: username.to_string(),
        authentication_type: ServerAuthenticationType::PrivateKey,
        password_variable_key: None,
        private_key_variable_key: Some(SECRET_VARIABLE.to_string()),
        private_key_passphrase_variable_key: None,
        allow_inheritance: false,
    }
}

fn resource_state(test_name: &str, fail: bool) -> (AppState, Arc<AtomicUsize>) {
    let base = temp_directory(test_name);
    let mut config = AppConfig::default();
    config.database_url = base.join("tron.sqlite3").display().to_string();
    config.resource = ResourceConfig::with_default_root(base.join("objects").display().to_string());
    let mut state =
        AppState::new(prepared_config(config)).expect("resource test app should initialize");
    let calls = Arc::new(AtomicUsize::new(0));
    state.resources =
        state
            .resources
            .clone()
            .with_connection_verifier(Arc::new(FakeConnectionVerifier {
                calls: calls.clone(),
                fail,
            }));
    (state, calls)
}

fn resources(
    state: &AppState,
    resource_code: Option<&str>,
) -> tron_backend::application::resource::ResourcePage {
    state
        .resources
        .query_resources(
            ACTOR,
            ResourcePageQuery {
                workspace_code: WORKSPACE.to_string(),
                kind: None,
                search: resource_code.map(str::to_string),
                current: 1,
                size: 50,
            },
        )
        .expect("resource query should succeed")
}

fn temp_directory(test_name: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should advance")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("tron-resource-connection-{test_name}-{nanos}"));
    std::fs::create_dir_all(&path).expect("temporary directory should be created");
    path
}
