mod common;

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use tokio::sync::Notify;

use serde_json::json;
use tron_backend::{
    app::AppState,
    application::resource::{
        DataSourceCreateCommand, ResourceConnectionTarget, ResourceConnectionVerificationFuture,
        ResourceConnectionVerifier, ResourceDeleteCommand, ResourceGetQuery, ResourcePageQuery,
        ServerAccountCreateCommand, ServerAccountCreateInput, ServerCreateCommand,
    },
    config::{AppConfig, ResourceConfig},
    domain::resource::{ResourceError, ResourceKind, ServerAuthenticationType},
};

use common::prepared_config;

const ACTOR: &str = "USR-AUTH-0001";
const PLATFORM_ACTOR: &str = "USR-LARGE-ID";
const WORKSPACE: &str = "TRON_CORE";
const PLATFORM_WORKSPACE: &str = "PLATFORM";
const SECRET_VARIABLE: &str = "OPENAI_API_KEY";

#[derive(Clone)]
struct FakeConnectionVerifier {
    calls: Arc<AtomicUsize>,
    fail: bool,
}

#[derive(Clone)]
struct BlockingConnectionVerifier {
    release: Arc<Notify>,
    started: Arc<Notify>,
}

impl ResourceConnectionVerifier for BlockingConnectionVerifier {
    fn verify(
        &self,
        _target: ResourceConnectionTarget,
    ) -> ResourceConnectionVerificationFuture<'_> {
        let release = self.release.clone();
        let started = self.started.clone();
        Box::pin(async move {
            started.notify_one();
            release.notified().await;
            Ok(())
        })
    }
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
async fn data_source_attribute_validation_rejects_invalid_schema_values_before_verification() {
    let (state, calls) = resource_state("data-source-attribute-validation", false);
    let mut invalid_mysql = mysql_attributes();
    invalid_mysql["connectionMode"] = json!("NOT_A_MODE");
    for (resource_code, kind, attributes) in [
        (
            "MYSQL_INVALID_MODE",
            ResourceKind::MysqlConnection,
            invalid_mysql,
        ),
        (
            "REDIS_NEGATIVE_DATABASE",
            ResourceKind::RedisConnection,
            json!({
                "topology": "STANDALONE",
                "endpoints": [{ "host": "redis.internal", "port": 6379 }],
                "database": -1,
                "tls": { "mode": "DISABLED" }
            }),
        ),
        (
            "MYSQL_MISSING_TLS",
            ResourceKind::MysqlConnection,
            json!({
                "connectionMode": "SINGLE",
                "endpoints": [{ "host": "mysql.internal", "port": 3306 }],
                "database": "app",
                "credentials": {
                    "username": "app",
                    "password": "MYSQL_PASSWORD"
                }
            }),
        ),
    ] {
        let result = state
            .resources
            .create_data_source_for_http(
                ACTOR,
                DataSourceCreateCommand {
                    workspace_code: WORKSPACE.to_string(),
                    resource_code: resource_code.to_string(),
                    kind,
                    name: resource_code.to_string(),
                    description: String::new(),
                    attributes,
                    allow_inheritance: false,
                },
            )
            .await;
        assert!(matches!(
            result,
            Err(ResourceError::InvalidField { field, reason })
                if (field == "connectionMode" && reason.contains("SINGLE"))
                    || (field == "database" && reason.contains("non-negative"))
                    || (field == "tls" && reason.contains("missing field"))
        ));
    }
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
    assert!(direct.records[0].attributes.is_none());
    let direct = state
        .resources
        .get_resource(
            ACTOR,
            ResourceGetQuery {
                workspace_code: WORKSPACE.to_string(),
                resource_code: "MYSQL_DIRECT_PASSWORD".to_string(),
            },
        )
        .expect("direct connection should be readable without its secret");
    assert_eq!(
        direct
            .attributes
            .as_ref()
            .and_then(|attributes| attributes.pointer("/credentials/password/value"))
            .cloned(),
        Some(serde_json::Value::Null)
    );
    let variable = resources(&state, Some("MYSQL_VARIABLE_PASSWORD"));
    assert!(variable.records[0].attributes.is_none());
    let variable = state
        .resources
        .get_resource(
            ACTOR,
            ResourceGetQuery {
                workspace_code: WORKSPACE.to_string(),
                resource_code: "MYSQL_VARIABLE_PASSWORD".to_string(),
            },
        )
        .expect("variable connection should be readable");
    assert_eq!(
        variable
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
                initial_account: Some(password_account("SERVER_PROD_1_ROOT", "root")),
            },
        )
        .await
        .expect("verified server and first account should persist atomically");
    assert!(created.connectivity_verified);
    assert_eq!(created.server.kind, ResourceKind::ServerConnection);
    assert_eq!(
        created.initial_account.as_ref().map(|account| account.kind),
        Some(ResourceKind::ServerAccount)
    );
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
        assert!(account.attributes.is_none());
        let account = state
            .resources
            .get_resource(
                ACTOR,
                ResourceGetQuery {
                    workspace_code: WORKSPACE.to_string(),
                    resource_code: account_code.to_string(),
                },
            )
            .expect("account details should be queryable");
        assert_eq!(
            account
                .attributes
                .as_ref()
                .and_then(|value| value.get("serverCode"))
                .and_then(serde_json::Value::as_str),
            Some("SERVER_PROD_1")
        );
    }
    let private_key_account = state
        .resources
        .get_resource(
            ACTOR,
            ResourceGetQuery {
                workspace_code: WORKSPACE.to_string(),
                resource_code: "SERVER_PROD_1_AUTOMATION".to_string(),
            },
        )
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
async fn server_creation_can_defer_accounts_to_server_account_management() {
    let (state, calls) = resource_state("server-without-account", false);
    let created = state
        .resources
        .create_server_for_http(
            ACTOR,
            ServerCreateCommand {
                workspace_code: WORKSPACE.to_string(),
                server_code: "SERVER_NO_ACCOUNT".to_string(),
                name: "Server without account".to_string(),
                description: String::new(),
                host: "server.internal".to_string(),
                port: 22,
                allow_inheritance: false,
                initial_account: None,
            },
        )
        .await
        .expect("server should be created before its account");

    assert!(!created.connectivity_verified);
    assert!(created.initial_account.is_none());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(resources(&state, Some("SERVER_NO_ACCOUNT")).total, 1);
    assert_eq!(
        resources(&state, Some("SERVER_NO_ACCOUNT")).records.len(),
        1
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
                initial_account: Some(password_account("SERVER_GUARDED_ROOT", "root")),
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
                initial_account: Some(password_account("SERVER_FAILED_ROOT", "root")),
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

#[tokio::test]
async fn inherited_server_accepts_a_local_workspace_account() {
    let (state, _) = resource_state("inherited-server-account", false);
    state
        .resources
        .create_server_for_http(
            PLATFORM_ACTOR,
            ServerCreateCommand {
                workspace_code: PLATFORM_WORKSPACE.to_string(),
                server_code: "PLATFORM_SHARED_SERVER".to_string(),
                name: "Platform server".to_string(),
                description: String::new(),
                host: "server.internal".to_string(),
                port: 22,
                allow_inheritance: true,
                initial_account: Some(password_account_with_variable(
                    "PLATFORM_SHARED_SERVER_ROOT",
                    "root",
                    "ROUTER_TIMEOUT_MS",
                )),
            },
        )
        .await
        .expect("platform server should be created");

    let inherited = resources(&state, Some("PLATFORM_SHARED_SERVER"));
    assert_eq!(inherited.total, 1);
    assert_eq!(
        inherited.records[0].source_kind,
        tron_backend::domain::resource::ResourceSourceKind::Inherited,
    );

    let created = state
        .resources
        .create_server_account_for_http(
            ACTOR,
            ServerAccountCreateCommand {
                workspace_code: WORKSPACE.to_string(),
                server_code: "PLATFORM_SHARED_SERVER".to_string(),
                account: password_account("TRON_CORE_SHARED_SERVER_DEPLOY", "deploy"),
            },
        )
        .expect("child workspace should be able to bind its own account");
    assert_eq!(created.kind, ResourceKind::ServerAccount);
}

#[tokio::test]
async fn server_account_rejects_unknown_or_blank_variable_references() {
    let (state, _) = resource_state("server-account-variable-validation", false);
    state
        .resources
        .create_server_for_http(
            ACTOR,
            ServerCreateCommand {
                workspace_code: WORKSPACE.to_string(),
                server_code: "VARIABLE_CHECK_SERVER".to_string(),
                name: "Variable check server".to_string(),
                description: String::new(),
                host: "server.internal".to_string(),
                port: 22,
                allow_inheritance: false,
                initial_account: Some(password_account("VARIABLE_CHECK_ROOT", "root")),
            },
        )
        .await
        .expect("server should be created");

    let mut blank = password_account("VARIABLE_CHECK_BLANK", "blank");
    blank.password_variable_key = Some(" ".to_string());
    assert_eq!(
        state.resources.create_server_account_for_http(
            ACTOR,
            ServerAccountCreateCommand {
                workspace_code: WORKSPACE.to_string(),
                server_code: "VARIABLE_CHECK_SERVER".to_string(),
                account: blank,
            },
        ),
        Err(ResourceError::InvalidInput),
    );

    let unknown = password_account_with_variable(
        "VARIABLE_CHECK_UNKNOWN",
        "unknown",
        "MISSING_SERVER_PASSWORD",
    );
    assert_eq!(
        state.resources.create_server_account_for_http(
            ACTOR,
            ServerAccountCreateCommand {
                workspace_code: WORKSPACE.to_string(),
                server_code: "VARIABLE_CHECK_SERVER".to_string(),
                account: unknown,
            },
        ),
        Err(ResourceError::InvalidVariableReference),
    );
}

#[tokio::test]
async fn data_source_update_does_not_overwrite_a_concurrent_metadata_change() {
    let (mut state, _) = resource_state("data-source-update-conflict", false);
    state
        .resources
        .create_data_source_for_http(
            ACTOR,
            DataSourceCreateCommand {
                workspace_code: WORKSPACE.to_string(),
                resource_code: "MYSQL_CONCURRENT".to_string(),
                kind: ResourceKind::MysqlConnection,
                name: "Original name".to_string(),
                description: String::new(),
                attributes: mysql_attributes(),
                allow_inheritance: false,
            },
        )
        .await
        .expect("data source should be created");

    let started = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    state.resources =
        state
            .resources
            .clone()
            .with_connection_verifier(Arc::new(BlockingConnectionVerifier {
                release: release.clone(),
                started: started.clone(),
            }));
    let updating = state.resources.clone();
    let update = tokio::spawn(async move {
        updating
            .update_data_source_for_http(
                ACTOR,
                tron_backend::application::resource::DataSourceUpdateCommand {
                    workspace_code: WORKSPACE.to_string(),
                    resource_code: "MYSQL_CONCURRENT".to_string(),
                    name: None,
                    description: None,
                    attributes: Some(mysql_attributes_with_host("mysql-relocated.internal")),
                    allow_inheritance: None,
                },
            )
            .await
    });
    started.notified().await;

    state
        .resources
        .update_resource(
            ACTOR,
            tron_backend::application::resource::ResourceUpdateCommand {
                workspace_code: WORKSPACE.to_string(),
                resource_code: "MYSQL_CONCURRENT".to_string(),
                name: Some("Concurrent name".to_string()),
                description: None,
                allow_inheritance: None,
            },
        )
        .expect("metadata update should succeed while verification is pending");
    release.notify_one();

    assert_eq!(
        update.await.expect("update task should join"),
        Err(ResourceError::Conflict),
    );
    let detail = state
        .resources
        .get_resource(
            ACTOR,
            ResourceGetQuery {
                workspace_code: WORKSPACE.to_string(),
                resource_code: "MYSQL_CONCURRENT".to_string(),
            },
        )
        .expect("data source should remain readable");
    assert_eq!(detail.name, "Concurrent name");
    assert_eq!(
        detail
            .attributes
            .as_ref()
            .and_then(|attributes| attributes.pointer("/endpoints/0/host"))
            .and_then(serde_json::Value::as_str),
        Some("mysql.internal"),
    );
}

fn mysql_attributes() -> serde_json::Value {
    mysql_attributes_with_host("mysql.internal")
}

fn mysql_attributes_with_host(host: &str) -> serde_json::Value {
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
    password_account_with_variable(account_code, username, SECRET_VARIABLE)
}

fn password_account_with_variable(
    account_code: &str,
    username: &str,
    password_variable_key: &str,
) -> ServerAccountCreateInput {
    ServerAccountCreateInput {
        account_code: account_code.to_string(),
        name: format!("{username} account"),
        description: String::new(),
        username: username.to_string(),
        authentication_type: ServerAuthenticationType::Password,
        password_variable_key: Some(password_variable_key.to_string()),
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
                kinds: Vec::new(),
                source_kind: None,
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
