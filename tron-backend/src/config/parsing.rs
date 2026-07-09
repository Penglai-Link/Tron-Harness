use std::env;

use super::{
    ConfigError, DEFAULT_AUTH_GITHUB_AUTHORIZE_URL, DEFAULT_AUTH_GITHUB_EMAILS_API_URL,
    DEFAULT_AUTH_GITHUB_NAME, DEFAULT_AUTH_GITHUB_SCOPE, DEFAULT_AUTH_GITHUB_TOKEN_URL,
    DEFAULT_AUTH_GITHUB_USER_API_URL, DEFAULT_AUTH_LDAP_NAME, ENV_BACKEND_PORT, ENV_LDAP_BIND_DN,
    ENV_LDAP_BIND_PASSWORD, ENV_LDAP_SEARCH_BASE, ENV_LDAP_SEARCH_FILTER, ENV_LDAP_URL,
    GitHubOAuthConfig, LdapConfig,
};

const ENV_AUTH_GITHUB_ENABLED: &str = "TRON_AUTH_GITHUB_ENABLED";
const ENV_AUTH_GITHUB_CLIENT_ID: &str = "TRON_AUTH_GITHUB_CLIENT_ID";
const ENV_AUTH_GITHUB_CLIENT_SECRET: &str = "TRON_AUTH_GITHUB_CLIENT_SECRET";
const ENV_AUTH_GITHUB_REDIRECT_URI: &str = "TRON_AUTH_GITHUB_REDIRECT_URI";
const ENV_AUTH_GITHUB_NAME: &str = "TRON_AUTH_GITHUB_NAME";
const ENV_AUTH_GITHUB_SCOPE: &str = "TRON_AUTH_GITHUB_SCOPE";
const ENV_AUTH_GITHUB_AUTHORIZE_URL: &str = "TRON_AUTH_GITHUB_AUTHORIZE_URL";
const ENV_AUTH_GITHUB_TOKEN_URL: &str = "TRON_AUTH_GITHUB_TOKEN_URL";
const ENV_AUTH_GITHUB_USER_API_URL: &str = "TRON_AUTH_GITHUB_USER_API_URL";
const ENV_AUTH_GITHUB_EMAILS_API_URL: &str = "TRON_AUTH_GITHUB_EMAILS_API_URL";

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

pub(super) fn github_oauth_env_override(
    current: Option<GitHubOAuthConfig>,
) -> Result<Option<GitHubOAuthConfig>, ConfigError> {
    github_oauth_env_override_from(current, |key| env::var(key).ok())
}

fn github_oauth_env_override_from<F>(
    current: Option<GitHubOAuthConfig>,
    mut read_env: F,
) -> Result<Option<GitHubOAuthConfig>, ConfigError>
where
    F: FnMut(&str) -> Option<String>,
{
    let enabled = read_env_value(&mut read_env, ENV_AUTH_GITHUB_ENABLED);
    let client_id = read_env_value(&mut read_env, ENV_AUTH_GITHUB_CLIENT_ID);
    let client_secret = read_env_value(&mut read_env, ENV_AUTH_GITHUB_CLIENT_SECRET);
    let redirect_uri = read_env_value(&mut read_env, ENV_AUTH_GITHUB_REDIRECT_URI);
    let name = read_env_value(&mut read_env, ENV_AUTH_GITHUB_NAME);
    let scope = read_env_value(&mut read_env, ENV_AUTH_GITHUB_SCOPE);
    let authorize_url = read_env_value(&mut read_env, ENV_AUTH_GITHUB_AUTHORIZE_URL);
    let token_url = read_env_value(&mut read_env, ENV_AUTH_GITHUB_TOKEN_URL);
    let user_api_url = read_env_value(&mut read_env, ENV_AUTH_GITHUB_USER_API_URL);
    let emails_api_url = read_env_value(&mut read_env, ENV_AUTH_GITHUB_EMAILS_API_URL);
    let has_github_env = enabled.is_some()
        || client_id.is_some()
        || client_secret.is_some()
        || redirect_uri.is_some()
        || name.is_some()
        || scope.is_some()
        || authorize_url.is_some()
        || token_url.is_some()
        || user_api_url.is_some()
        || emails_api_url.is_some();

    if !has_github_env {
        return Ok(current);
    }

    let enabled = match enabled {
        Some(value) => parse_bool_env(ENV_AUTH_GITHUB_ENABLED, value)?,
        None => current.is_some(),
    };
    if !enabled {
        return Ok(None);
    }

    let current = current.as_ref();
    Ok(Some(GitHubOAuthConfig {
        client_id: required_config_string(
            client_id.or_else(|| current.map(|config| config.client_id.clone())),
            ENV_AUTH_GITHUB_CLIENT_ID,
        )?,
        client_secret: required_config_string(
            client_secret.or_else(|| current.map(|config| config.client_secret.clone())),
            ENV_AUTH_GITHUB_CLIENT_SECRET,
        )?,
        redirect_uri: required_config_string(
            redirect_uri.or_else(|| current.map(|config| config.redirect_uri.clone())),
            ENV_AUTH_GITHUB_REDIRECT_URI,
        )?,
        name: name
            .or_else(|| current.map(|config| config.name.clone()))
            .unwrap_or_else(|| DEFAULT_AUTH_GITHUB_NAME.to_string()),
        scope: scope
            .or_else(|| current.map(|config| config.scope.clone()))
            .unwrap_or_else(|| DEFAULT_AUTH_GITHUB_SCOPE.to_string()),
        authorize_url: authorize_url
            .or_else(|| current.map(|config| config.authorize_url.clone()))
            .unwrap_or_else(|| DEFAULT_AUTH_GITHUB_AUTHORIZE_URL.to_string()),
        token_url: token_url
            .or_else(|| current.map(|config| config.token_url.clone()))
            .unwrap_or_else(|| DEFAULT_AUTH_GITHUB_TOKEN_URL.to_string()),
        user_api_url: user_api_url
            .or_else(|| current.map(|config| config.user_api_url.clone()))
            .unwrap_or_else(|| DEFAULT_AUTH_GITHUB_USER_API_URL.to_string()),
        emails_api_url: emails_api_url
            .or_else(|| current.map(|config| config.emails_api_url.clone()))
            .unwrap_or_else(|| DEFAULT_AUTH_GITHUB_EMAILS_API_URL.to_string()),
    }))
}

fn read_env_value<F>(read_env: &mut F, key: &str) -> Option<String>
where
    F: FnMut(&str) -> Option<String>,
{
    read_env(key)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn parse_bool_env(key: &str, value: String) -> Result<bool, ConfigError> {
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(ConfigError::InvalidEnvValue {
            key: key.to_string(),
            value,
        }),
    }
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn github_oauth_env_override_updates_existing_redirect_uri() {
        let override_env = HashMap::from([(
            ENV_AUTH_GITHUB_REDIRECT_URI,
            "http://127.0.0.1:8081/api/v1/auth/oauth2/callback/github",
        )]);

        let config = github_oauth_env_override_from(Some(github_config()), |key| {
            override_env.get(key).map(|value| (*value).to_string())
        })
        .expect("github env override should load")
        .expect("github oauth should stay enabled");

        assert_eq!(
            config.redirect_uri,
            "http://127.0.0.1:8081/api/v1/auth/oauth2/callback/github"
        );
        assert_eq!(config.client_id, "github-client-id");
    }

    #[test]
    fn github_oauth_env_override_can_disable_existing_config() {
        let override_env = HashMap::from([(ENV_AUTH_GITHUB_ENABLED, "false")]);

        let config = github_oauth_env_override_from(Some(github_config()), |key| {
            override_env.get(key).map(|value| (*value).to_string())
        })
        .expect("github env override should load");

        assert!(config.is_none());
    }

    #[test]
    fn github_oauth_env_override_can_enable_without_yaml_block() {
        let override_env = HashMap::from([
            (ENV_AUTH_GITHUB_ENABLED, "true"),
            (ENV_AUTH_GITHUB_CLIENT_ID, "env-client-id"),
            (ENV_AUTH_GITHUB_CLIENT_SECRET, "env-client-secret"),
            (
                ENV_AUTH_GITHUB_REDIRECT_URI,
                "http://127.0.0.1:8080/api/v1/auth/oauth2/callback/github",
            ),
        ]);

        let config = github_oauth_env_override_from(None, |key| {
            override_env.get(key).map(|value| (*value).to_string())
        })
        .expect("github env override should load")
        .expect("github oauth should be enabled");

        assert_eq!(config.client_id, "env-client-id");
        assert_eq!(config.client_secret, "env-client-secret");
        assert_eq!(config.name, DEFAULT_AUTH_GITHUB_NAME);
        assert_eq!(config.scope, DEFAULT_AUTH_GITHUB_SCOPE);
    }

    fn github_config() -> GitHubOAuthConfig {
        GitHubOAuthConfig {
            client_id: "github-client-id".into(),
            client_secret: "github-client-secret".into(),
            redirect_uri: "http://127.0.0.1:5173/api/v1/auth/oauth2/callback/github".into(),
            name: "GitHub".into(),
            scope: "read:user user:email".into(),
            authorize_url: DEFAULT_AUTH_GITHUB_AUTHORIZE_URL.into(),
            token_url: DEFAULT_AUTH_GITHUB_TOKEN_URL.into(),
            user_api_url: DEFAULT_AUTH_GITHUB_USER_API_URL.into(),
            emails_api_url: DEFAULT_AUTH_GITHUB_EMAILS_API_URL.into(),
        }
    }
}
