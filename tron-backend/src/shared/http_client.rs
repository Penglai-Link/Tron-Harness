use std::time::Duration;

use reqwest::Client;

use crate::config::{HttpClientProfileConfig, HttpClientProfilesConfig};

#[derive(Debug)]
pub enum HttpClientBuildError {
    InvalidProfile(String),
    BuildFailed(reqwest::Error),
}

impl std::fmt::Display for HttpClientBuildError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidProfile(message) => formatter.write_str(message),
            Self::BuildFailed(error) => {
                write!(formatter, "failed to build reqwest client: {error}")
            }
        }
    }
}

impl std::error::Error for HttpClientBuildError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidProfile(_) => None,
            Self::BuildFailed(error) => Some(error),
        }
    }
}

#[derive(Clone)]
pub struct HttpClientPool {
    default: Client,
    model_inference: Client,
    runtime_control: Client,
}

impl HttpClientPool {
    pub fn from_profiles(config: &HttpClientProfilesConfig) -> Result<Self, HttpClientBuildError> {
        Ok(Self {
            default: build_client("default", &config.default)?,
            model_inference: build_client("modelInference", &config.model_inference)?,
            runtime_control: build_client("runtimeControl", &config.runtime_control)?,
        })
    }

    pub fn default_client(&self) -> Client {
        self.default.clone()
    }

    pub fn model_inference_client(&self) -> Client {
        self.model_inference.clone()
    }

    pub fn runtime_control_client(&self) -> Client {
        self.runtime_control.clone()
    }
}

fn build_client(
    profile_name: &str,
    config: &HttpClientProfileConfig,
) -> Result<Client, HttpClientBuildError> {
    if config.connect_timeout_ms == 0 {
        return Err(HttpClientBuildError::InvalidProfile(format!(
            "http profile `{profile_name}` requires connectTimeoutMs > 0"
        )));
    }
    if config.pool_max_idle_per_host == 0 {
        return Err(HttpClientBuildError::InvalidProfile(format!(
            "http profile `{profile_name}` requires poolMaxIdlePerHost > 0"
        )));
    }

    let mut builder = Client::builder()
        .connect_timeout(Duration::from_millis(config.connect_timeout_ms))
        .pool_max_idle_per_host(config.pool_max_idle_per_host);

    if let Some(timeout_ms) = config.request_timeout_ms
        && timeout_ms > 0
    {
        builder = builder.timeout(Duration::from_millis(timeout_ms));
    }
    if let Some(timeout_ms) = config.pool_idle_timeout_ms
        && timeout_ms > 0
    {
        builder = builder.pool_idle_timeout(Duration::from_millis(timeout_ms));
    }
    if let Some(timeout_ms) = config.tcp_keepalive_ms
        && timeout_ms > 0
    {
        builder = builder.tcp_keepalive(Duration::from_millis(timeout_ms));
    }

    builder.build().map_err(HttpClientBuildError::BuildFailed)
}
