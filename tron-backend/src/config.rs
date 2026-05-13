use std::{env, error::Error, fmt, net::SocketAddr};

const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 8080;
const DEFAULT_DATABASE_URL: &str = "tron-backend/target/tron.sqlite3";
const TEST_DATABASE_URL: &str = ":memory:";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppConfig {
    pub bind_address: String,
    pub port: u16,
    pub database_url: String,
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

        Ok(Self {
            bind_address,
            port,
            database_url,
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
        }
    }
}

#[derive(Debug)]
pub enum ConfigError {
    InvalidPort {
        value: String,
        source: std::num::ParseIntError,
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
            Self::InvalidSocketAddr { source, .. } => Some(source),
        }
    }
}
