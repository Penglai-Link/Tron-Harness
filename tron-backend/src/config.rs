use std::{env, error::Error, fmt, net::SocketAddr};

const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 8080;
const DEFAULT_DATABASE_URL: &str = "tron-backend/target/tron.sqlite3";
const TEST_DATABASE_URL: &str = ":memory:";
const DEFAULT_AUTH_JWT_SECRET: &str = "tron-dev-jwt-secret";
const DEFAULT_AUTH_ACCESS_TOKEN_TTL_SECONDS: i64 = 900;
const DEFAULT_AUTH_REFRESH_TOKEN_TTL_SECONDS: i64 = 604_800;
const DEFAULT_AUTH_DEV_OAUTH2_NAME: &str = "Development OAuth2";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppConfig {
    pub bind_address: String,
    pub port: u16,
    pub database_url: String,
    pub auth: AuthConfig,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub access_token_ttl_seconds: i64,
    pub refresh_token_ttl_seconds: i64,
    pub cookie_secure: bool,
    pub dev_oauth2_enabled: bool,
    pub dev_oauth2_name: String,
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
            dev_oauth2_enabled: parse_bool_env("TRON_AUTH_DEV_OAUTH2_ENABLED", true)?,
            dev_oauth2_name: env::var("TRON_AUTH_DEV_OAUTH2_NAME")
                .unwrap_or_else(|_| DEFAULT_AUTH_DEV_OAUTH2_NAME.into()),
        };

        Ok(Self {
            bind_address,
            port,
            database_url,
            auth,
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
            auth: AuthConfig {
                jwt_secret: DEFAULT_AUTH_JWT_SECRET.into(),
                access_token_ttl_seconds: DEFAULT_AUTH_ACCESS_TOKEN_TTL_SECONDS,
                refresh_token_ttl_seconds: DEFAULT_AUTH_REFRESH_TOKEN_TTL_SECONDS,
                cookie_secure: false,
                dev_oauth2_enabled: true,
                dev_oauth2_name: DEFAULT_AUTH_DEV_OAUTH2_NAME.into(),
            },
        }
    }
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
            Self::InvalidSocketAddr { source, .. } => Some(source),
        }
    }
}

fn parse_i64_env(key: &str, default: i64) -> Result<i64, ConfigError> {
    match env::var(key) {
        Ok(value) => value
            .parse()
            .map_err(|source| ConfigError::InvalidInteger {
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
