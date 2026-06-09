use std::env;

use super::{
    ConfigError, DEFAULT_AUTH_LDAP_NAME, ENV_BACKEND_PORT, ENV_LDAP_BIND_DN,
    ENV_LDAP_BIND_PASSWORD, ENV_LDAP_SEARCH_BASE, ENV_LDAP_SEARCH_FILTER, ENV_LDAP_URL, LdapConfig,
};

pub(super) fn required_config_string(
    value: Option<String>,
    key: &str,
) -> Result<String, ConfigError> {
    match value {
        Some(value) if !value.trim().is_empty() => Ok(value),
        _ => Err(ConfigError::MissingConfigField {
            key: key.to_string(),
        }),
    }
}

pub(super) fn env_port_override() -> Result<Option<u16>, ConfigError> {
    let Some(value) = env::var(ENV_BACKEND_PORT)
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
            key: ENV_BACKEND_PORT.to_string(),
            value,
        })
}

pub(super) fn ldap_env_override() -> Option<Result<LdapConfig, ConfigError>> {
    let url = env::var(ENV_LDAP_URL)
        .ok()
        .filter(|value| !value.trim().is_empty());
    let bind_dn = env::var(ENV_LDAP_BIND_DN)
        .ok()
        .filter(|value| !value.trim().is_empty());
    let bind_password = env::var(ENV_LDAP_BIND_PASSWORD)
        .ok()
        .filter(|value| !value.trim().is_empty());
    let search_base = env::var(ENV_LDAP_SEARCH_BASE)
        .ok()
        .filter(|value| !value.trim().is_empty());
    let search_filter = env::var(ENV_LDAP_SEARCH_FILTER)
        .ok()
        .filter(|value| !value.trim().is_empty());

    if url.is_none()
        && bind_dn.is_none()
        && bind_password.is_none()
        && search_base.is_none()
        && search_filter.is_none()
    {
        return None;
    }

    Some((|| -> Result<LdapConfig, ConfigError> {
        Ok(LdapConfig {
            name: DEFAULT_AUTH_LDAP_NAME.into(),
            url: required_config_string(url, ENV_LDAP_URL)?,
            bind_dn: required_config_string(bind_dn, ENV_LDAP_BIND_DN)?,
            bind_password: required_config_string(bind_password, ENV_LDAP_BIND_PASSWORD)?,
            search_base: required_config_string(search_base, ENV_LDAP_SEARCH_BASE)?,
            search_filter: required_config_string(search_filter, ENV_LDAP_SEARCH_FILTER)?,
        })
    })())
}
