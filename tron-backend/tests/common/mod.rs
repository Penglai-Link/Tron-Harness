#![allow(dead_code)]

use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    Json, Router,
    body::{Body, to_bytes},
    extract::State,
    http::{HeaderMap, HeaderValue, Method, Request, header},
    routing::{get, post},
};
use serde_json::{Value, json};
use tokio::{net::TcpListener, task::JoinHandle};
use tower::ServiceExt;
use tron_backend::{
    adapters::outbound::persistence::{
        SqliteAppRepository, SqliteOpenAppRepository, SqliteRoleRepository, SqliteUserRepository,
        SqliteVariableRepository, SqliteWorkspaceRepository,
    },
    app::build_router_with_id_provider,
    application::{
        app::AppRepository, open_app::OpenAppRepository, role::RoleRepository,
        user::UserRepository, variable::VariableRepository, workspace::WorkspaceRepository,
    },
    config::{AppConfig, AuthConfig, GitHubOAuthConfig, VariableCryptoConfig},
    domain::{
        app::{App, AppRuntimeRole, AppStatus, AppTag},
        open_app::{
            OPEN_API_CAPABILITY_AGENT_CHAT, OPEN_API_CAPABILITY_TERMINAL, OpenApiCanonicalRequest,
            OpenApp, OpenAppCapabilityGrant, OpenAppCredential, OpenAppCredentialStatus,
            OpenAppStatus, OpenAppUserGrant, build_open_api_canonical_request,
            compute_open_api_signature, open_app_secret_fingerprint,
        },
        role::seeded_roles,
        user::{ListUsersQuery, PhoneNumber, User, UserSourceType, UserStatus},
        variable::Variable,
        workspace::{Workspace, WorkspaceRoleAccess, WorkspaceStatus, WorkspaceUserAccess},
    },
    shared::{
        crypto::Aes256SecretBox,
        id::{FixedSnowflakeIdProvider, SnowflakeId},
        random::random_hex,
        time::{current_timestamp, current_timestamp_seconds},
    },
};

pub const AUTH_EMAIL: &str = "auth.operator@tron.ai";
pub const AUTH_LOGIN: &str = "auth.operator";
pub const AUTH_PASSWORD: &str = "TronAuthPass123!";
pub const ARCHIVED_LOGIN: &str = "archived.auth";
const GITHUB_TEST_CLIENT_ID: &str = "github-client-id";
const GITHUB_TEST_CLIENT_SECRET: &str = "github-client-secret";
const GITHUB_TEST_ACCESS_TOKEN: &str = "github-access-token";
pub const GITHUB_TEST_CODE: &str = "github-test-code";
const FIXTURE_TIMESTAMP: &str = "2026-05-16T00:00:00Z";
static TEST_DB_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static TEST_ID_SEQUENCE: AtomicU64 = AtomicU64::new(734_900_000_000_000_000);
pub const OPEN_API_TEST_APP_CODE: &str = "OPENAPP-FIXTURE-0001";
pub const OPEN_API_TEST_APP_KEY: &str = "dev-open-api-key";
pub const OPEN_API_TEST_USER_CODE: &str = "USR-0001";

#[derive(Clone)]
struct GitHubMockState {
    access_token: &'static str,
    profile: GitHubProfileFixture,
}

#[derive(Clone)]
pub struct GitHubEmailFixture {
    pub email: String,
    pub primary: bool,
    pub verified: bool,
}

#[derive(Clone)]
pub struct GitHubProfileFixture {
    pub id: i64,
    pub login: String,
    pub name: Option<String>,
    pub emails: Vec<GitHubEmailFixture>,
}

impl Default for GitHubProfileFixture {
    fn default() -> Self {
        Self {
            id: 734_103_002_743_185_500,
            login: AUTH_LOGIN.into(),
            name: Some("Auth Operator".into()),
            emails: vec![GitHubEmailFixture {
                email: AUTH_EMAIL.into(),
                primary: true,
                verified: true,
            }],
        }
    }
}

pub struct GitHubMockServer {
    pub authorize_url: String,
    pub token_url: String,
    pub user_api_url: String,
    pub emails_api_url: String,
    task: JoinHandle<()>,
}

pub fn app() -> Router {
    build_router_with_test_ids(prepared_config(AppConfig::default()))
}

pub fn app_with_config(config: AppConfig) -> Router {
    build_router_with_test_ids(prepared_config(config))
}

fn build_router_with_test_ids(config: AppConfig) -> Router {
    build_router_with_id_provider(
        config,
        Arc::new(FixedSnowflakeIdProvider::new(next_test_id_seed())),
    )
    .expect("test router should build")
}

pub fn prepared_config(mut config: AppConfig) -> AppConfig {
    if config.database_url == ":memory:" {
        config.database_url = temp_database_path("integration")
            .to_string_lossy()
            .into_owned();
    }
    if config.variable_crypto.key_base64.trim().is_empty() {
        config.variable_crypto = VariableCryptoConfig::test_default();
    }
    seed_default_fixture_data(&config);
    config
}

pub async fn github_mock_server() -> GitHubMockServer {
    github_mock_server_with_profile(GitHubProfileFixture::default()).await
}

pub async fn github_mock_server_with_profile(profile: GitHubProfileFixture) -> GitHubMockServer {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("github mock server should bind");
    let address = listener
        .local_addr()
        .expect("github mock server should expose a local address");
    let base_url = format!("http://{address}");
    let state = Arc::new(GitHubMockState {
        access_token: GITHUB_TEST_ACCESS_TOKEN,
        profile,
    });
    let app = Router::new()
        .route("/login/oauth/access_token", post(github_token))
        .route("/api/user", get(github_user))
        .route("/api/user/emails", get(github_user_emails))
        .with_state(state);
    let task = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("github mock server should stay available during the test");
    });

    GitHubMockServer {
        authorize_url: format!("{base_url}/login/oauth/authorize"),
        token_url: format!("{base_url}/login/oauth/access_token"),
        user_api_url: format!("{base_url}/api/user"),
        emails_api_url: format!("{base_url}/api/user/emails"),
        task,
    }
}

pub fn github_oauth_config(server: &GitHubMockServer) -> AppConfig {
    let mut config = AppConfig::default();
    config.auth = AuthConfig {
        dev_oauth2_enabled: false,
        dev_oauth2_name: "OAuth2".into(),
        github_oauth: Some(GitHubOAuthConfig {
            client_id: GITHUB_TEST_CLIENT_ID.into(),
            client_secret: GITHUB_TEST_CLIENT_SECRET.into(),
            redirect_uri: "http://127.0.0.1:5173/api/v1/auth/oauth2/callback/github".into(),
            name: "GitHub".into(),
            scope: "read:user user:email".into(),
            authorize_url: server.authorize_url.clone(),
            token_url: server.token_url.clone(),
            user_api_url: server.user_api_url.clone(),
            emails_api_url: server.emails_api_url.clone(),
        }),
        ldap: None,
        ..config.auth
    };
    config
}

fn seed_default_fixture_data(config: &AppConfig) {
    seed_roles(&config.database_url);
    seed_users(&config.database_url);
    seed_workspaces(&config.database_url);
    seed_apps(&config.database_url);
    seed_variables(&config.database_url);
    seed_open_apps(config);
}

fn next_test_id_seed() -> i64 {
    TEST_ID_SEQUENCE.fetch_add(10_000, Ordering::Relaxed) as i64
}

fn seed_roles(database_url: &str) {
    let mut repository =
        SqliteRoleRepository::connect(database_url).expect("role test repository should connect");
    if repository
        .list_roles()
        .expect("role fixtures should load")
        .is_empty()
    {
        repository
            .replace_roles(&seeded_roles())
            .expect("role fixtures should seed");
    }
}

fn seed_users(database_url: &str) {
    let mut repository =
        SqliteUserRepository::connect(database_url).expect("user test repository should connect");
    let query = ListUsersQuery {
        page: None,
        current: Some(1),
        size: Some(1),
        status: None,
        search: None,
        role_code: None,
        space_code: None,
        include_archived: Some(true),
    };
    if repository
        .list_users(query)
        .expect("user fixtures should load")
        .total
        == 0
    {
        for user in seeded_users() {
            repository
                .insert_user(&user)
                .expect("user fixtures should seed");
        }
    }
}

fn seed_workspaces(database_url: &str) {
    let mut repository = SqliteWorkspaceRepository::connect(database_url)
        .expect("workspace test repository should connect");
    if repository
        .list_workspaces()
        .expect("workspace fixtures should load")
        .is_empty()
    {
        for workspace in seeded_workspaces() {
            repository
                .create_workspace(&workspace)
                .expect("workspace fixtures should seed");
        }
        repository
            .replace_user_access("TRON_CORE", &seeded_workspace_user_access())
            .expect("workspace user access fixtures should seed");
        repository
            .replace_role_access("PLATFORM", &seeded_workspace_role_access())
            .expect("workspace role access fixtures should seed");
    }
}

fn seed_apps(database_url: &str) {
    let mut repository =
        SqliteAppRepository::connect(database_url).expect("app test repository should connect");
    if repository
        .list_apps()
        .expect("app fixtures should load")
        .is_empty()
    {
        for (app, tags) in seeded_apps() {
            repository
                .create_app(&app)
                .expect("app fixtures should seed");
            repository
                .replace_app_tags(&app.app_code, &tags)
                .expect("app tag fixtures should seed");
        }
    }
}

fn seed_variables(database_url: &str) {
    let mut repository = SqliteVariableRepository::connect(database_url)
        .expect("variable test repository should connect");
    if repository
        .list_variables()
        .expect("variable fixtures should load")
        .is_empty()
    {
        for variable in seeded_variables() {
            repository
                .create_variable(&variable)
                .expect("variable fixtures should seed");
        }
    }
}

fn seed_open_apps(config: &AppConfig) {
    let mut repository = SqliteOpenAppRepository::connect(&config.database_url)
        .expect("open-app test repository should connect");
    if repository
        .find_open_app_by_open_app_code(OPEN_API_TEST_APP_CODE)
        .expect("open-app fixtures should load")
        .is_some()
    {
        return;
    }

    let now = current_timestamp();
    let secret_box = Aes256SecretBox::new(&config.auth.jwt_secret);
    let open_app = OpenApp {
        id: SnowflakeId::new(734_103_002_743_196_000),
        open_app_code: OPEN_API_TEST_APP_CODE.into(),
        name: "EliteForge KB".into(),
        description: Some("OpenAPI integration fixture".into()),
        status: OpenAppStatus::Enabled,
        created_at: now.clone(),
        updated_at: now.clone(),
        last_rotated_at: Some(now.clone()),
    };
    let credential = OpenAppCredential {
        id: SnowflakeId::new(734_103_002_743_196_001),
        credential_code: "OPENAPP-CRED-FIXTURE-0001".into(),
        open_app_code: open_app.open_app_code.clone(),
        status: OpenAppCredentialStatus::Active,
        secret_ciphertext: secret_box
            .encrypt(OPEN_API_TEST_APP_KEY)
            .expect("fixture app key should encrypt"),
        secret_fingerprint: open_app_secret_fingerprint(OPEN_API_TEST_APP_KEY),
        created_at: now.clone(),
        revoked_at: None,
    };

    repository
        .create_open_app(&open_app, &credential)
        .expect("open-app fixtures should seed");
    repository
        .insert_user_grant(&OpenAppUserGrant {
            id: SnowflakeId::new(734_103_002_743_196_002),
            grant_code: "OPENAPP-USER-GRANT-FIXTURE-0001".into(),
            open_app_code: open_app.open_app_code.clone(),
            user_code: OPEN_API_TEST_USER_CODE.into(),
            created_at: now.clone(),
        })
        .expect("open-app user grant fixture should seed");

    for (offset, operation_code) in [
        "session.read",
        "session.write",
        "message.read",
        "message.write",
        "stream.consume",
    ]
    .into_iter()
    .enumerate()
    {
        repository
            .insert_capability_grant(&OpenAppCapabilityGrant {
                id: SnowflakeId::new(734_103_002_743_196_100 + offset as i64),
                grant_code: format!("OPENAPP-CAP-GRANT-FIXTURE-{:04}", offset + 1),
                open_app_code: open_app.open_app_code.clone(),
                capability_code: OPEN_API_CAPABILITY_AGENT_CHAT.into(),
                operation_code: operation_code.into(),
                created_at: now.clone(),
            })
            .expect("open-app capability grant fixture should seed");
    }
    for (offset, operation_code) in ["socket.connect", "command.generate"]
        .into_iter()
        .enumerate()
    {
        repository
            .insert_capability_grant(&OpenAppCapabilityGrant {
                id: SnowflakeId::new(734_103_002_743_196_200 + offset as i64),
                grant_code: format!("OPENAPP-TERMINAL-GRANT-FIXTURE-{:04}", offset + 1),
                open_app_code: open_app.open_app_code.clone(),
                capability_code: OPEN_API_CAPABILITY_TERMINAL.into(),
                operation_code: operation_code.into(),
                created_at: now.clone(),
            })
            .expect("open-app terminal capability grant fixture should seed");
    }
}

fn seeded_users() -> Vec<User> {
    vec![
        seeded_user(
            734_102_998_871_252_992,
            "USR-0001",
            "alex.johnson",
            "Alex",
            "Johnson",
            "Alex Johnson",
            "alex.johnson@tron.ai",
            UserStatus::Active,
            vec!["DEVELOPER", "VIEWER"],
            vec!["PLATFORM", "DATA"],
        ),
        seeded_user(
            734_103_002_743_185_408,
            "USR-0002",
            "maya.singh",
            "Maya",
            "Singh",
            "Maya Singh",
            "maya.singh@tron.ai",
            UserStatus::Active,
            vec!["VIEWER", "DEVELOPER"],
            vec!["ENTERPRISE", "DATA"],
        ),
        seeded_user(
            734_103_002_743_185_409,
            "USR-0007",
            "sam.viewer",
            "Sam",
            "Viewer",
            "Sam Viewer",
            "sam.viewer@tron.ai",
            UserStatus::Active,
            vec!["VIEWER"],
            vec!["SUPPORT", "PLATFORM"],
        ),
        seeded_user(
            734_103_002_743_185_410,
            "USR-0009",
            "archived.restore",
            "Archived",
            "Restore",
            "Archived Restore",
            "archived.restore@tron.ai",
            UserStatus::Archived,
            vec!["DEVELOPER", "VIEWER"],
            vec!["DATA", "SUPPORT"],
        ),
        seeded_user(
            734_103_002_743_185_411,
            "USR-1001",
            "reset.target",
            "Reset",
            "Target",
            "Reset Target",
            "reset.target@tron.ai",
            UserStatus::Active,
            vec!["ADMIN", "DEVELOPER"],
            vec!["PLATFORM", "ENTERPRISE"],
        ),
        seeded_user(
            9_007_199_254_740_993,
            "USR-LARGE-ID",
            "large.id",
            "Large",
            "Identifier",
            "Large Identifier",
            "large.id@tron.ai",
            UserStatus::Active,
            vec!["WORKSPACE_ADMIN", "DEVELOPER"],
            vec!["ENTERPRISE", "PLATFORM"],
        ),
        seeded_user(
            734_103_002_743_185_412,
            "USR-ARCHIVED",
            "archived.only",
            "Archived",
            "Only",
            "Archived Only",
            "archived.only@tron.ai",
            UserStatus::Archived,
            vec!["VIEWER"],
            vec!["SUPPORT", "DATA"],
        ),
        seeded_auth_user(
            734_103_002_743_185_500,
            "USR-AUTH-0001",
            AUTH_LOGIN,
            "Auth",
            "Operator",
            "Auth Operator",
            AUTH_EMAIL,
            UserStatus::Active,
        ),
        seeded_auth_user(
            734_103_002_743_185_501,
            "USR-AUTH-ARCHIVED",
            ARCHIVED_LOGIN,
            "Archived",
            "Auth",
            "Archived Auth",
            "archived.auth@tron.ai",
            UserStatus::Archived,
        ),
    ]
}

fn seeded_user(
    id: i64,
    user_code: &str,
    username: &str,
    first_name: &str,
    last_name: &str,
    display_name: &str,
    email: &str,
    status: UserStatus,
    role_codes: Vec<&str>,
    space_codes: Vec<&str>,
) -> User {
    User {
        id: SnowflakeId::new(id),
        user_code: user_code.into(),
        source_type: UserSourceType::Internal,
        username: username.into(),
        first_name: first_name.into(),
        last_name: last_name.into(),
        display_name: display_name.into(),
        email: email.into(),
        phone: Some(PhoneNumber {
            country_code: "+1".into(),
            number: "5550100".into(),
        }),
        avatar_url: None,
        locale: "en-US".into(),
        timezone: "America/Los_Angeles".into(),
        status,
        role_codes: role_codes.into_iter().map(str::to_owned).collect(),
        space_codes: space_codes.into_iter().map(str::to_owned).collect(),
        password_hash: "$2b$04$wzYH5I9Aw4f3ZV5h9tczhOJtT5QxQfyeZDJmW5hE4d1tij8nS0V9G".into(),
        password_reset_required: false,
        last_active_at: Some("2026-05-11T08:30:00Z".into()),
        created_at: "2026-05-11T08:00:00Z".into(),
        updated_at: "2026-05-11T08:00:00Z".into(),
        archived_at: (status == UserStatus::Archived).then(|| "2026-05-11T08:00:00Z".into()),
    }
}

fn seeded_auth_user(
    id: i64,
    user_code: &str,
    username: &str,
    first_name: &str,
    last_name: &str,
    display_name: &str,
    email: &str,
    status: UserStatus,
) -> User {
    let mut user = seeded_user(
        id,
        user_code,
        username,
        first_name,
        last_name,
        display_name,
        email,
        status,
        vec!["ADMIN", "DEVELOPER"],
        vec!["PLATFORM", "DATA"],
    );
    user.password_hash =
        bcrypt::hash(AUTH_PASSWORD, 4).expect("auth seed password hash should be generated");
    user
}

fn seeded_workspaces() -> Vec<Workspace> {
    vec![
        Workspace {
            id: SnowflakeId::new(734_400_000_000_000_001),
            workspace_code: "ENTERPRISE".into(),
            name: "Enterprise".into(),
            description: Some("Global governance workspace".into()),
            parent_workspace_code: None,
            status: WorkspaceStatus::Active,
            created_at: FIXTURE_TIMESTAMP.into(),
            updated_at: FIXTURE_TIMESTAMP.into(),
            archived_at: None,
        },
        Workspace {
            id: SnowflakeId::new(734_400_000_000_000_002),
            workspace_code: "PLATFORM".into(),
            name: "Platform".into(),
            description: Some("Core platform runtime workspace".into()),
            parent_workspace_code: Some("ENTERPRISE".into()),
            status: WorkspaceStatus::Active,
            created_at: FIXTURE_TIMESTAMP.into(),
            updated_at: FIXTURE_TIMESTAMP.into(),
            archived_at: None,
        },
        Workspace {
            id: SnowflakeId::new(734_400_000_000_000_003),
            workspace_code: "TRON_CORE".into(),
            name: "Tron Core".into(),
            description: Some("Main Tron console workspace".into()),
            parent_workspace_code: Some("PLATFORM".into()),
            status: WorkspaceStatus::Active,
            created_at: FIXTURE_TIMESTAMP.into(),
            updated_at: FIXTURE_TIMESTAMP.into(),
            archived_at: None,
        },
        Workspace {
            id: SnowflakeId::new(734_400_000_000_000_004),
            workspace_code: "COMPLIANCE".into(),
            name: "Compliance".into(),
            description: Some("Compliance evidence workspace".into()),
            parent_workspace_code: None,
            status: WorkspaceStatus::Active,
            created_at: FIXTURE_TIMESTAMP.into(),
            updated_at: FIXTURE_TIMESTAMP.into(),
            archived_at: None,
        },
    ]
}

fn seeded_workspace_user_access() -> Vec<WorkspaceUserAccess> {
    vec![
        WorkspaceUserAccess {
            user_code: "USR-AUTH-0001".into(),
            is_owner: true,
            granted_at: FIXTURE_TIMESTAMP.into(),
        },
        WorkspaceUserAccess {
            user_code: OPEN_API_TEST_USER_CODE.into(),
            is_owner: false,
            granted_at: FIXTURE_TIMESTAMP.into(),
        },
    ]
}

fn seeded_workspace_role_access() -> Vec<WorkspaceRoleAccess> {
    vec![WorkspaceRoleAccess {
        role_code: "WORKSPACE_ADMIN".into(),
        granted_at: FIXTURE_TIMESTAMP.into(),
    }]
}

fn seeded_apps() -> Vec<(App, Vec<AppTag>)> {
    vec![
        (
            App {
                id: SnowflakeId::new(734_900_000_000_000_001),
                app_code: "APP-TRON-CONSOLE".into(),
                workspace_code: "TRON_CORE".into(),
                name: "Tron Console".into(),
                description: Some("Primary operator-facing frontend.".into()),
                local_directory: "/workspace/tron/tron-frontend".into(),
                runtime_role: AppRuntimeRole::Frontend,
                repository_url: Some("https://github.com/clouds3n/tron-console".into()),
                default_branch: Some("master".into()),
                status: AppStatus::Active,
                created_at: "2026-05-19T00:00:00Z".into(),
                updated_at: "2026-05-19T00:00:00Z".into(),
                archived_at: None,
            },
            vec![
                AppTag {
                    app_code: "APP-TRON-CONSOLE".into(),
                    tag: "Operator".into(),
                    linked_at: "2026-05-19T00:00:00Z".into(),
                },
                AppTag {
                    app_code: "APP-TRON-CONSOLE".into(),
                    tag: "React".into(),
                    linked_at: "2026-05-19T00:00:00Z".into(),
                },
            ],
        ),
        (
            App {
                id: SnowflakeId::new(734_900_000_000_000_002),
                app_code: "APP-TRON-API".into(),
                workspace_code: "TRON_CORE".into(),
                name: "Tron API".into(),
                description: Some("Core backend serving task and admin workflows.".into()),
                local_directory: "/workspace/tron/tron-backend".into(),
                runtime_role: AppRuntimeRole::Backend,
                repository_url: Some("https://github.com/clouds3n/tron-backend".into()),
                default_branch: Some("master".into()),
                status: AppStatus::Active,
                created_at: "2026-05-19T00:00:00Z".into(),
                updated_at: "2026-05-19T00:00:00Z".into(),
                archived_at: None,
            },
            vec![
                AppTag {
                    app_code: "APP-TRON-API".into(),
                    tag: "Rust".into(),
                    linked_at: "2026-05-19T00:00:00Z".into(),
                },
                AppTag {
                    app_code: "APP-TRON-API".into(),
                    tag: "Core".into(),
                    linked_at: "2026-05-19T00:00:00Z".into(),
                },
            ],
        ),
        (
            App {
                id: SnowflakeId::new(734_900_000_000_000_003),
                app_code: "APP-COMPLIANCE-API".into(),
                workspace_code: "COMPLIANCE".into(),
                name: "Compliance API".into(),
                description: Some("Backend used by regulated reporting workflows.".into()),
                local_directory: "/workspace/apps/compliance-api".into(),
                runtime_role: AppRuntimeRole::Backend,
                repository_url: Some("https://github.com/clouds3n/compliance-api".into()),
                default_branch: Some("master".into()),
                status: AppStatus::Active,
                created_at: "2026-05-19T00:00:00Z".into(),
                updated_at: "2026-05-19T00:00:00Z".into(),
                archived_at: None,
            },
            vec![AppTag {
                app_code: "APP-COMPLIANCE-API".into(),
                tag: "Audit".into(),
                linked_at: "2026-05-19T00:00:00Z".into(),
            }],
        ),
        (
            App {
                id: SnowflakeId::new(734_900_000_000_000_004),
                app_code: "APP-TRON-LEGACY".into(),
                workspace_code: "TRON_CORE".into(),
                name: "Legacy Console".into(),
                description: Some("Archived frontend kept for historical reference.".into()),
                local_directory: "/workspace/tron/legacy-console".into(),
                runtime_role: AppRuntimeRole::Frontend,
                repository_url: None,
                default_branch: None,
                status: AppStatus::Archived,
                created_at: "2026-05-19T00:00:00Z".into(),
                updated_at: "2026-05-19T00:00:00Z".into(),
                archived_at: Some("2026-05-19T00:00:00Z".into()),
            },
            vec![AppTag {
                app_code: "APP-TRON-LEGACY".into(),
                tag: "Legacy".into(),
                linked_at: "2026-05-19T00:00:00Z".into(),
            }],
        ),
    ]
}

fn seeded_variables() -> Vec<Variable> {
    vec![
        Variable {
            id: SnowflakeId::new(734_500_000_000_000_001),
            workspace_code: "ENTERPRISE".into(),
            key: "DEFAULT_REGION".into(),
            value: "us-east-1".into(),
            description: Some("Shared default deployment region.".into()),
            is_sensitive: false,
            value_nonce: None,
            value_tag: None,
            value_algorithm: None,
            value_key_version: None,
            created_at: "2026-05-18T00:00:00Z".into(),
            updated_at: "2026-05-18T00:00:00Z".into(),
        },
        Variable {
            id: SnowflakeId::new(734_500_000_000_000_002),
            workspace_code: "ENTERPRISE".into(),
            key: "ROUTER_TIMEOUT_MS".into(),
            value: "800".into(),
            description: Some("Root timeout for interactive routing.".into()),
            is_sensitive: false,
            value_nonce: None,
            value_tag: None,
            value_algorithm: None,
            value_key_version: None,
            created_at: "2026-05-18T00:00:00Z".into(),
            updated_at: "2026-05-18T00:00:00Z".into(),
        },
        Variable {
            id: SnowflakeId::new(734_500_000_000_000_003),
            workspace_code: "PLATFORM".into(),
            key: "OPENAI_API_BASE_URL".into(),
            value: "https://api.openai.com/v1".into(),
            description: Some("Shared OpenAI gateway base URL.".into()),
            is_sensitive: false,
            value_nonce: None,
            value_tag: None,
            value_algorithm: None,
            value_key_version: None,
            created_at: "2026-05-18T00:00:00Z".into(),
            updated_at: "2026-05-18T00:00:00Z".into(),
        },
        Variable {
            id: SnowflakeId::new(734_500_000_000_000_004),
            workspace_code: "PLATFORM".into(),
            key: "ROUTER_TIMEOUT_MS".into(),
            value: "600".into(),
            description: Some("Platform override for interactive latency.".into()),
            is_sensitive: false,
            value_nonce: None,
            value_tag: None,
            value_algorithm: None,
            value_key_version: None,
            created_at: "2026-05-18T00:00:00Z".into(),
            updated_at: "2026-05-18T00:00:00Z".into(),
        },
        Variable {
            id: SnowflakeId::new(734_500_000_000_000_005),
            workspace_code: "TRON_CORE".into(),
            key: "OPENAI_API_KEY".into(),
            value: "sk-tron-core-demo".into(),
            description: Some("Workspace secret used by the demo workspace.".into()),
            is_sensitive: false,
            value_nonce: None,
            value_tag: None,
            value_algorithm: None,
            value_key_version: None,
            created_at: "2026-05-18T00:00:00Z".into(),
            updated_at: "2026-05-18T00:00:00Z".into(),
        },
        Variable {
            id: SnowflakeId::new(734_500_000_000_000_006),
            workspace_code: "TRON_CORE".into(),
            key: "ROUTER_TIMEOUT_MS".into(),
            value: "400".into(),
            description: Some("Tron Core override for agent console flows.".into()),
            is_sensitive: false,
            value_nonce: None,
            value_tag: None,
            value_algorithm: None,
            value_key_version: None,
            created_at: "2026-05-18T00:00:00Z".into(),
            updated_at: "2026-05-18T00:00:00Z".into(),
        },
        Variable {
            id: SnowflakeId::new(734_500_000_000_000_007),
            workspace_code: "COMPLIANCE".into(),
            key: "AUDIT_RETENTION_DAYS".into(),
            value: "365".into(),
            description: Some("Retention period for compliance exports.".into()),
            is_sensitive: false,
            value_nonce: None,
            value_tag: None,
            value_algorithm: None,
            value_key_version: None,
            created_at: "2026-05-18T00:00:00Z".into(),
            updated_at: "2026-05-18T00:00:00Z".into(),
        },
    ]
}

fn temp_database_path(test_name: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after Unix epoch")
        .as_nanos();
    let sequence = TEST_DB_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let process_id = std::process::id();
    std::env::temp_dir().join(format!(
        "tron-tests-{test_name}-{process_id}-{nanos}-{sequence}.sqlite3"
    ))
}

pub async fn request(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
    cookie_header: Option<&str>,
) -> axum::response::Response {
    request_with_headers(app, method, uri, body, cookie_header, None).await
}

pub async fn request_with_headers(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
    cookie_header: Option<&str>,
    authorization_header: Option<&str>,
) -> axum::response::Response {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(cookies) = cookie_header {
        builder = builder.header(header::COOKIE, cookies);
    }
    if let Some(authorization) = authorization_header {
        builder = builder.header(header::AUTHORIZATION, authorization);
    }
    let request_body = if let Some(json_body) = body {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        Body::from(json_body.to_string())
    } else {
        Body::empty()
    };

    app.clone()
        .oneshot(
            builder
                .body(request_body)
                .expect("request should be constructible"),
        )
        .await
        .expect("router should return a response")
}

pub async fn signed_open_api_request(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
) -> axum::response::Response {
    signed_open_api_request_as(
        app,
        method,
        uri,
        body,
        OPEN_API_TEST_APP_CODE,
        OPEN_API_TEST_APP_KEY,
        OPEN_API_TEST_USER_CODE,
    )
    .await
}

pub async fn signed_open_api_request_as(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
    open_app_code: &str,
    app_key: &str,
    user_code: &str,
) -> axum::response::Response {
    signed_open_api_request_custom(
        app,
        method,
        uri,
        body,
        open_app_code,
        app_key,
        user_code,
        current_timestamp_seconds(),
        &random_hex(12),
        "TRON-HMAC-SHA256-V1",
    )
    .await
}

pub async fn signed_open_api_request_custom(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
    open_app_code: &str,
    app_key: &str,
    user_code: &str,
    timestamp: i64,
    nonce: &str,
    signature_version: &str,
) -> axum::response::Response {
    let uri_value: axum::http::Uri = uri.parse().expect("signed open-api URI should parse");
    let request_body = body
        .as_ref()
        .map(|json_body| json_body.to_string().into_bytes())
        .unwrap_or_default();
    let canonical = build_open_api_canonical_request(&OpenApiCanonicalRequest {
        method: method.as_str(),
        path: uri_value.path(),
        raw_query: uri_value.query(),
        body: &request_body,
        open_app_code,
        user_code,
        timestamp,
        nonce,
    })
    .expect("fixture canonical request should build");
    let signature =
        compute_open_api_signature(app_key, &canonical).expect("fixture signature should build");
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("X-Tron-App-Code", open_app_code)
        .header("X-Tron-User-Code", user_code)
        .header("X-Tron-Timestamp", timestamp.to_string())
        .header("X-Tron-Nonce", nonce)
        .header("X-Tron-Signature-Version", signature_version)
        .header("X-Tron-Signature", signature);
    let request_body = if body.is_some() {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        Body::from(request_body)
    } else {
        Body::empty()
    };

    app.clone()
        .oneshot(
            builder
                .body(request_body)
                .expect("signed open-api request should be constructible"),
        )
        .await
        .expect("router should return a response")
}

pub async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    serde_json::from_slice(&bytes).expect("response body should be valid JSON")
}

pub async fn authenticated_refresh_cookie_header(app: &Router) -> String {
    let login_response = request(
        app,
        Method::POST,
        "/api/v1/auth/login",
        Some(json!({
            "login": AUTH_LOGIN,
            "password": AUTH_PASSWORD,
        })),
        None,
    )
    .await;
    assert_eq!(
        login_response.status(),
        axum::http::StatusCode::OK,
        "test auth bootstrap should obtain a valid login session",
    );
    cookie_header_from_set_cookie(
        &header_values(login_response.headers(), header::SET_COOKIE),
        &["tron_refresh_token"],
    )
}

pub async fn authenticated_request(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
) -> axum::response::Response {
    let access_token = authenticated_bearer_token(app).await;
    let authorization = format!("Bearer {access_token}");
    request_with_headers(app, method, uri, body, None, Some(&authorization)).await
}

pub async fn authenticated_request_with_config(
    config: AppConfig,
    method: Method,
    uri: &str,
    body: Option<Value>,
) -> axum::response::Response {
    let router = app_with_config(config);
    authenticated_request(&router, method, uri, body).await
}

pub async fn authenticated_bearer_token(app: &Router) -> String {
    let login_response = request(
        app,
        Method::POST,
        "/api/v1/auth/login",
        Some(json!({
            "login": AUTH_LOGIN,
            "password": AUTH_PASSWORD,
        })),
        None,
    )
    .await;
    assert_eq!(
        login_response.status(),
        axum::http::StatusCode::OK,
        "test auth bootstrap should obtain a valid login session",
    );
    let login_json = response_json(login_response).await;
    login_json["data"]["accessToken"]
        .as_str()
        .expect("login response should include an accessToken")
        .to_owned()
}

pub fn header_values(headers: &HeaderMap, name: header::HeaderName) -> Vec<String> {
    headers
        .get_all(name)
        .iter()
        .map(header_value_to_string)
        .collect()
}

pub fn header_value_to_string(value: &HeaderValue) -> String {
    value
        .to_str()
        .expect("header value should be valid text")
        .to_owned()
}

pub fn cookie_header_from_set_cookie(
    set_cookie_headers: &[String],
    cookie_names: &[&str],
) -> String {
    cookie_names
        .iter()
        .map(|name| {
            format!(
                "{name}={}",
                cookie_value_from_set_cookie(set_cookie_headers, name)
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

pub fn cookie_value_from_set_cookie(set_cookie_headers: &[String], cookie_name: &str) -> String {
    set_cookie_headers
        .iter()
        .find_map(|cookie| {
            let head = cookie.split(';').next()?.trim();
            head.strip_prefix(&format!("{cookie_name}="))
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| panic!("expected Set-Cookie header for {cookie_name}"))
}

impl Drop for GitHubMockServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn github_token(State(state): State<Arc<GitHubMockState>>) -> Json<Value> {
    Json(json!({
        "access_token": state.access_token,
        "token_type": "bearer",
        "scope": "read:user,user:email",
    }))
}

async fn github_user(State(state): State<Arc<GitHubMockState>>) -> Json<Value> {
    Json(json!({
        "id": state.profile.id,
        "login": state.profile.login,
        "name": state.profile.name,
    }))
}

async fn github_user_emails(State(state): State<Arc<GitHubMockState>>) -> Json<Value> {
    Json(json!(
        state
            .profile
            .emails
            .iter()
            .map(|email| json!({
                "email": email.email,
                "primary": email.primary,
                "verified": email.verified,
            }))
            .collect::<Vec<_>>()
    ))
}
