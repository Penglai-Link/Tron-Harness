use std::{env, error::Error, fmt, net::SocketAddr, path::PathBuf};

use crate::shared::event_bus::EventBusConfig;

const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 8080;
const DEFAULT_DATABASE_URL: &str = "tron-backend/target/tron.sqlite3";
const TEST_DATABASE_URL: &str = ":memory:";
const DEFAULT_AUTH_JWT_SECRET: &str = "tron-dev-jwt-secret";
const DEFAULT_AUTH_ACCESS_TOKEN_TTL_SECONDS: i64 = 900;
const DEFAULT_AUTH_REFRESH_TOKEN_TTL_SECONDS: i64 = 604_800;
const DEFAULT_AUTH_DEV_OAUTH2_NAME: &str = "OAuth2";
const DEFAULT_AUTH_GITHUB_NAME: &str = "GitHub";
const DEFAULT_AUTH_GITHUB_AUTHORIZE_URL: &str = "https://github.com/login/oauth/authorize";
const DEFAULT_AUTH_GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const DEFAULT_AUTH_GITHUB_USER_API_URL: &str = "https://api.github.com/user";
const DEFAULT_AUTH_GITHUB_EMAILS_API_URL: &str = "https://api.github.com/user/emails";
const DEFAULT_AUTH_GITHUB_SCOPE: &str = "read:user user:email";
const DEFAULT_SKILLS_REGISTRY_ROOT_SUFFIX: &str = ".tron/skills-registry";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppConfig {
    pub bind_address: String,
    pub port: u16,
    pub database_url: String,
    pub skills_registry_root: String,
    pub auth: AuthConfig,
    pub event_bus: EventBusConfig,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub access_token_ttl_seconds: i64,
    pub refresh_token_ttl_seconds: i64,
    pub cookie_secure: bool,
    pub dev_oauth2_enabled: bool,
    pub dev_oauth2_name: String,
    pub github_oauth: Option<GitHubOAuthConfig>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitHubOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub name: String,
    pub scope: String,
    pub authorize_url: String,
    pub token_url: String,
    pub user_api_url: String,
    pub emails_api_url: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let bind_address =
            env::var("TRON_BACKEND_BIND_ADDRESS").unwrap_or_else(|_| DEFAULT_BIND_ADDRESS.into());
        let port = match env::var("TRON_BACKEND_PORT") {
            Ok(value) => value
                .parse()
                .map_err(|source| ConfigError::InvalidPort { value, source })?,
            Err(_) => DEFAULT_PORT,
        };
        let database_url =
            env::var("TRON_BACKEND_DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.into());
        let skills_registry_root = env::var("TRON_SKILLS_REGISTRY_ROOT")
            .unwrap_or_else(|_| default_skills_registry_root_from_home());
        let github_oauth_enabled = parse_bool_env("TRON_AUTH_GITHUB_ENABLED", false)?;
        let event_bus = EventBusConfig {
            enabled: parse_bool_env("TRON_EVENT_BUS_ENABLED", true)?,
            critical_ingress_capacity: parse_usize_env(
                "TRON_EVENT_BUS_CRITICAL_INGRESS_CAPACITY",
                EventBusConfig::default().critical_ingress_capacity,
            )?,
            standard_ingress_capacity: parse_usize_env(
                "TRON_EVENT_BUS_STANDARD_INGRESS_CAPACITY",
                EventBusConfig::default().standard_ingress_capacity,
            )?,
            default_sink_capacity: parse_usize_env(
                "TRON_EVENT_BUS_DEFAULT_SINK_CAPACITY",
                EventBusConfig::default().default_sink_capacity,
            )?,
            agui_replay_events_per_stream: parse_usize_env(
                "TRON_EVENT_BUS_AGUI_REPLAY_EVENTS_PER_STREAM",
                EventBusConfig::default().agui_replay_events_per_stream,
            )?,
            agui_replay_bytes_per_stream: parse_usize_env(
                "TRON_EVENT_BUS_AGUI_REPLAY_BYTES_PER_STREAM",
                EventBusConfig::default().agui_replay_bytes_per_stream,
            )?,
            agui_global_replay_bytes: parse_usize_env(
                "TRON_EVENT_BUS_AGUI_GLOBAL_REPLAY_BYTES",
                EventBusConfig::default().agui_global_replay_bytes,
            )?,
            agui_stream_idle_ttl_seconds: parse_u64_env(
                "TRON_EVENT_BUS_AGUI_STREAM_IDLE_TTL_SECONDS",
                EventBusConfig::default().agui_stream_idle_ttl_seconds,
            )?,
            ..EventBusConfig::default()
        };
        let auth = AuthConfig {
            jwt_secret: env::var("TRON_AUTH_JWT_SECRET")
                .unwrap_or_else(|_| DEFAULT_AUTH_JWT_SECRET.into()),
            access_token_ttl_seconds: parse_i64_env(
                "TRON_AUTH_ACCESS_TOKEN_TTL_SECONDS",
                DEFAULT_AUTH_ACCESS_TOKEN_TTL_SECONDS,
            )?,
            refresh_token_ttl_seconds: parse_i64_env(
                "TRON_AUTH_REFRESH_TOKEN_TTL_SECONDS",
                DEFAULT_AUTH_REFRESH_TOKEN_TTL_SECONDS,
            )?,
            cookie_secure: parse_bool_env("TRON_AUTH_COOKIE_SECURE", false)?,
            dev_oauth2_enabled: parse_bool_env("TRON_AUTH_DEV_OAUTH2_ENABLED", false)?,
            dev_oauth2_name: env::var("TRON_AUTH_DEV_OAUTH2_NAME")
                .unwrap_or_else(|_| DEFAULT_AUTH_DEV_OAUTH2_NAME.into()),
            github_oauth: github_oauth_enabled
                .then(|| {
                    Ok(GitHubOAuthConfig {
                        client_id: required_env("TRON_AUTH_GITHUB_CLIENT_ID")?,
                        client_secret: required_env("TRON_AUTH_GITHUB_CLIENT_SECRET")?,
                        redirect_uri: required_env("TRON_AUTH_GITHUB_REDIRECT_URI")?,
                        name: env::var("TRON_AUTH_GITHUB_NAME")
                            .unwrap_or_else(|_| DEFAULT_AUTH_GITHUB_NAME.into()),
                        scope: env::var("TRON_AUTH_GITHUB_SCOPE")
                            .unwrap_or_else(|_| DEFAULT_AUTH_GITHUB_SCOPE.into()),
                        authorize_url: env::var("TRON_AUTH_GITHUB_AUTHORIZE_URL")
                            .unwrap_or_else(|_| DEFAULT_AUTH_GITHUB_AUTHORIZE_URL.into()),
                        token_url: env::var("TRON_AUTH_GITHUB_TOKEN_URL")
                            .unwrap_or_else(|_| DEFAULT_AUTH_GITHUB_TOKEN_URL.into()),
                        user_api_url: env::var("TRON_AUTH_GITHUB_USER_API_URL")
                            .unwrap_or_else(|_| DEFAULT_AUTH_GITHUB_USER_API_URL.into()),
                        emails_api_url: env::var("TRON_AUTH_GITHUB_EMAILS_API_URL")
                            .unwrap_or_else(|_| DEFAULT_AUTH_GITHUB_EMAILS_API_URL.into()),
                    })
                })
                .transpose()?,
        };

        Ok(Self {
            bind_address,
            port,
            database_url,
            skills_registry_root,
            auth,
            event_bus,
        })
    }

    pub fn socket_addr(&self) -> Result<SocketAddr, ConfigError> {
        format!("{}:{}", self.bind_address, self.port)
            .parse()
            .map_err(|source| ConfigError::InvalidSocketAddr {
                bind_address: self.bind_address.clone(),
                port: self.port,
                source,
            })
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            bind_address: DEFAULT_BIND_ADDRESS.into(),
            port: DEFAULT_PORT,
            database_url: TEST_DATABASE_URL.into(),
            skills_registry_root: std::env::temp_dir()
                .join("tron-skills-registry-test")
                .display()
                .to_string(),
            auth: AuthConfig {
                jwt_secret: DEFAULT_AUTH_JWT_SECRET.into(),
                access_token_ttl_seconds: DEFAULT_AUTH_ACCESS_TOKEN_TTL_SECONDS,
                refresh_token_ttl_seconds: DEFAULT_AUTH_REFRESH_TOKEN_TTL_SECONDS,
                cookie_secure: false,
                dev_oauth2_enabled: true,
                dev_oauth2_name: DEFAULT_AUTH_DEV_OAUTH2_NAME.into(),
                github_oauth: None,
            },
            event_bus: EventBusConfig::default(),
        }
    }
}

fn default_skills_registry_root_from_home() -> String {
    env::var("HOME")
        .map(|home| PathBuf::from(home).join(DEFAULT_SKILLS_REGISTRY_ROOT_SUFFIX))
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_SKILLS_REGISTRY_ROOT_SUFFIX))
        .display()
        .to_string()
}

#[derive(Debug)]
pub enum ConfigError {
    InvalidPort {
        value: String,
        source: std::num::ParseIntError,
    },
    InvalidInteger {
        key: String,
        value: String,
        source: std::num::ParseIntError,
    },
    InvalidBoolean {
        key: String,
        value: String,
    },
    MissingEnv {
        key: String,
    },
    InvalidSocketAddr {
        bind_address: String,
        port: u16,
        source: std::net::AddrParseError,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPort { value, .. } => {
                write!(formatter, "invalid TRON_BACKEND_PORT value `{value}`")
            }
            Self::InvalidInteger { key, value, .. } => {
                write!(formatter, "invalid {key} value `{value}`")
            }
            Self::InvalidBoolean { key, value } => {
                write!(formatter, "invalid {key} value `{value}`")
            }
            Self::MissingEnv { key } => {
                write!(formatter, "missing required environment variable `{key}`")
            }
            Self::InvalidSocketAddr {
                bind_address, port, ..
            } => write!(
                formatter,
                "invalid backend bind socket address `{bind_address}:{port}`"
            ),
        }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidPort { source, .. } => Some(source),
            Self::InvalidInteger { source, .. } => Some(source),
            Self::InvalidBoolean { .. } => None,
            Self::MissingEnv { .. } => None,
            Self::InvalidSocketAddr { source, .. } => Some(source),
        }
    }
}

fn required_env(key: &str) -> Result<String, ConfigError> {
    match env::var(key) {
        Ok(value) if !value.trim().is_empty() => Ok(value),
        _ => Err(ConfigError::MissingEnv { key: key.into() }),
    }
}

fn parse_i64_env(key: &str, default: i64) -> Result<i64, ConfigError> {
    match env::var(key) {
        Ok(value) => value.parse().map_err(|source| ConfigError::InvalidInteger {
            key: key.into(),
            value,
            source,
        }),
        Err(_) => Ok(default),
    }
}

fn parse_usize_env(key: &str, default: usize) -> Result<usize, ConfigError> {
    match env::var(key) {
        Ok(value) => value.parse().map_err(|source| ConfigError::InvalidInteger {
            key: key.into(),
            value,
            source,
        }),
        Err(_) => Ok(default),
    }
}

fn parse_u64_env(key: &str, default: u64) -> Result<u64, ConfigError> {
    match env::var(key) {
        Ok(value) => value.parse().map_err(|source| ConfigError::InvalidInteger {
            key: key.into(),
            value,
            source,
        }),
        Err(_) => Ok(default),
    }
}

fn parse_bool_env(key: &str, default: bool) -> Result<bool, ConfigError> {
    match env::var(key) {
        Ok(value) => match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Ok(true),
            "0" | "false" | "no" | "off" => Ok(false),
            _ => Err(ConfigError::InvalidBoolean {
                key: key.into(),
                value,
            }),
        },
        Err(_) => Ok(default),
    }
}
