use std::env;

use super::ConfigError;

pub(super) fn env_port_override() -> Result<Option<u16>, ConfigError> {
    let Some(value) = env::var("TRON_BACKEND_PORT")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };

    value
        .parse::<u16>()
        .map(Some)
        .map_err(|_| ConfigError::InvalidEnvValue {
            key: "TRON_BACKEND_PORT".to_string(),
            value,
        })
}
