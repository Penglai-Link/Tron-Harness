use std::{error::Error, fmt};

#[derive(Debug)]
pub enum ConfigError {
    MissingConfigField {
        key: String,
    },
    InvalidEnvValue {
        key: String,
        value: String,
    },
    InvalidSocketAddr {
        bind_address: String,
        port: u16,
        source: std::net::AddrParseError,
    },
    ConfigFileReadFailed {
        path: String,
        source: std::io::Error,
    },
    ConfigFileInvalid {
        path: String,
        source: serde_yaml::Error,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingConfigField { key } => {
                write!(formatter, "missing required config field `{key}`")
            }
            Self::InvalidEnvValue { key, value } => {
                write!(formatter, "invalid value `{value}` for backend env `{key}`")
            }
            Self::InvalidSocketAddr {
                bind_address, port, ..
            } => write!(
                formatter,
                "invalid backend bind socket address `{bind_address}:{port}`"
            ),
            Self::ConfigFileReadFailed { path, .. } => {
                write!(formatter, "failed to read backend config file `{path}`")
            }
            Self::ConfigFileInvalid { path, .. } => {
                write!(formatter, "invalid backend config file `{path}`")
            }
        }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::MissingConfigField { .. } | Self::InvalidEnvValue { .. } => None,
            Self::InvalidSocketAddr { source, .. } => Some(source),
            Self::ConfigFileReadFailed { source, .. } => Some(source),
            Self::ConfigFileInvalid { source, .. } => Some(source),
        }
    }
}
