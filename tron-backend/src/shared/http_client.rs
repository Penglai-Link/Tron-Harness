use std::time::Duration;

use reqwest::Client;

use crate::config::{HttpClientProfileConfig, HttpClientProfilesConfig};

#[derive(Debug)]
pub enum HttpClientBuildError {
    InvalidProfile(String),
    BuildFailed(reqwest::Error),
    ModelBuildFailed(eliteforge_agent_core::ModelError),
}

impl std::fmt::Display for HttpClientBuildError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidProfile(message) => formatter.write_str(message),
            Self::BuildFailed(error) => {
                write!(formatter, "failed to build reqwest client: {error}")
            }
            Self::ModelBuildFailed(error) => {
                write!(formatter, "failed to build model clients: {error}")
            }
        }
    }
}

impl std::error::Error for HttpClientBuildError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidProfile(_) => None,
            Self::BuildFailed(error) => Some(error),
            Self::ModelBuildFailed(error) => Some(error),
        }
    }
}

#[derive(Clone)]
pub struct HttpClientPool {
    default: Client,
    model_inference: eliteforge_agent_official_plugin_model_http::ModelHttpClient,
    runtime_control: Client,
    document_parsing: Client,
}

impl HttpClientPool {
    pub fn from_profiles(config: &HttpClientProfilesConfig) -> Result<Self, HttpClientBuildError> {
        Ok(Self {
            default: build_client("default", &config.default)?,
            model_inference: build_model_client(&config.model_inference)?,
            runtime_control: build_client("runtimeControl", &config.runtime_control)?,
            document_parsing: build_client("documentParsing", &config.document_parsing)?,
        })
    }

    pub fn default_client(&self) -> Client {
        self.default.clone()
    }

    pub fn model_inference_client(
        &self,
    ) -> eliteforge_agent_official_plugin_model_http::ModelHttpClient {
        self.model_inference.clone()
    }

    pub fn runtime_control_client(&self) -> Client {
        self.runtime_control.clone()
    }

    pub fn document_parsing_client(&self) -> Client {
        self.document_parsing.clone()
    }
}

fn build_client(
    profile_name: &str,
    config: &HttpClientProfileConfig,
) -> Result<Client, HttpClientBuildError> {
    client_builder(profile_name, config)?
        .build()
        .map_err(HttpClientBuildError::BuildFailed)
}

fn client_builder(
    profile_name: &str,
    config: &HttpClientProfileConfig,
) -> Result<reqwest::ClientBuilder, HttpClientBuildError> {
    validate_profile(profile_name, config)?;

    let mut builder = Client::builder()
        .connect_timeout(Duration::from_millis(config.connect_timeout_ms))
        .pool_max_idle_per_host(config.pool_max_idle_per_host);

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

    Ok(builder)
}

fn validate_profile(
    profile_name: &str,
    config: &HttpClientProfileConfig,
) -> Result<(), HttpClientBuildError> {
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

    Ok(())
}

fn build_model_client(
    config: &HttpClientProfileConfig,
) -> Result<eliteforge_agent_official_plugin_model_http::ModelHttpClient, HttpClientBuildError> {
    use eliteforge_agent_official_plugin_model_http::{ModelHttpClient, ModelHttpClientOptions};
    validate_profile("modelInference", config)?;
    // Both protocol adapters receive the same host settings. The SDK owns the
    // HTTP client versions and the protocol-specific authentication behavior.
    ModelHttpClient::from_options(&ModelHttpClientOptions {
        connect_timeout: Some(Duration::from_millis(config.connect_timeout_ms)),
        pool_max_idle_per_host: Some(config.pool_max_idle_per_host),
        pool_idle_timeout: config
            .pool_idle_timeout_ms
            .filter(|value| *value > 0)
            .map(Duration::from_millis),
        tcp_keepalive: config
            .tcp_keepalive_ms
            .filter(|value| *value > 0)
            .map(Duration::from_millis),
        ..Default::default()
    })
    .map_err(HttpClientBuildError::ModelBuildFailed)
}
