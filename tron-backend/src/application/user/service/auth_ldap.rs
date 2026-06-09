use std::time::Duration;

use ldap3::{LdapConnAsync, LdapConnSettings, Scope, SearchEntry, ldap_escape};

use super::*;

const LDAP_CONNECT_TIMEOUT_MS: u64 = 5_000;
const LDAP_PROFILE_ATTRIBUTES: &[&str] = &[
    "mail",
    "userPrincipalName",
    "uid",
    "sAMAccountName",
    "displayName",
    "cn",
    "name",
];

pub(super) struct DefaultLdapAuthenticator;

impl LdapAuthenticator for DefaultLdapAuthenticator {
    fn authenticate<'a>(
        &'a self,
        config: &'a LdapConfig,
        login: &'a str,
        password: &'a str,
    ) -> LdapAuthFuture<'a> {
        Box::pin(async move {
            let settings = LdapConnSettings::new()
                .set_conn_timeout(Duration::from_millis(LDAP_CONNECT_TIMEOUT_MS));
            let (conn, mut ldap) = LdapConnAsync::with_settings(settings, &config.url)
                .await
                .map_err(|_| AuthError::LdapUpstreamFailed)?;
            ldap3::drive!(conn);

            ldap.simple_bind(&config.bind_dn, &config.bind_password)
                .await
                .map_err(|_| AuthError::LdapUpstreamFailed)?
                .success()
                .map_err(|_| AuthError::LdapUpstreamFailed)?;

            let filter = render_ldap_search_filter(&config.search_filter, login);
            let (entries, _result) = ldap
                .search(
                    &config.search_base,
                    Scope::Subtree,
                    &filter,
                    LDAP_PROFILE_ATTRIBUTES.to_vec(),
                )
                .await
                .map_err(|_| AuthError::LdapUpstreamFailed)?
                .success()
                .map_err(|_| AuthError::LdapUpstreamFailed)?;

            if entries.len() != 1 {
                let _ = ldap.unbind().await;
                return Err(AuthError::InvalidCredentials);
            }

            let entry = SearchEntry::construct(
                entries
                    .into_iter()
                    .next()
                    .expect("one LDAP entry should be present after length check"),
            );
            let email = entry_attr(&entry, &["mail", "userPrincipalName"])
                .map(|value| value.to_ascii_lowercase())
                .ok_or(AuthError::LdapProfileInvalid)?;
            let directory_login = entry_attr(&entry, &["uid", "sAMAccountName", "cn", "mail"])
                .unwrap_or_else(|| login.trim().to_ascii_lowercase());
            let display_name = entry_attr(&entry, &["displayName", "cn", "name"])
                .unwrap_or_else(|| directory_login.clone());
            let subject = entry.dn.clone();

            ldap.simple_bind(&entry.dn, password)
                .await
                .map_err(|_| AuthError::InvalidCredentials)?
                .success()
                .map_err(|_| AuthError::InvalidCredentials)?;
            let _ = ldap.unbind().await;

            Ok(AuthIdentityProfile {
                provider_code: LDAP_PROVIDER_CODE.into(),
                provider_name: config.name.clone(),
                subject,
                login: directory_login,
                email,
                display_name,
            })
        })
    }
}

fn render_ldap_search_filter(template: &str, login: &str) -> String {
    let escaped_login = ldap_escape(login.trim()).to_string();
    template
        .replace("{{login}}", &escaped_login)
        .replace("{login}", &escaped_login)
}

fn entry_attr(entry: &SearchEntry, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        entry
            .attrs
            .get(*key)
            .and_then(|values| values.iter().find(|value| !value.trim().is_empty()))
            .cloned()
    })
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        sync::Arc,
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;
    use crate::{
        adapters::outbound::{crypto::BcryptPasswordHasher, persistence::SqliteUserRepository},
        config::AppConfig,
        shared::event_bus::EventBusHandle,
    };

    static NEXT_DB_ID: AtomicU64 = AtomicU64::new(0);

    #[derive(Clone)]
    struct StaticLdapAuthenticator {
        result: Result<AuthIdentityProfile, AuthError>,
    }

    impl StaticLdapAuthenticator {
        fn success(profile: AuthIdentityProfile) -> Self {
            Self {
                result: Ok(profile),
            }
        }

        fn failure(error: AuthError) -> Self {
            Self { result: Err(error) }
        }
    }

    impl LdapAuthenticator for StaticLdapAuthenticator {
        fn authenticate<'a>(
            &'a self,
            _config: &'a LdapConfig,
            _login: &'a str,
            _password: &'a str,
        ) -> LdapAuthFuture<'a> {
            let result = self.result.clone();
            Box::pin(async move { result })
        }
    }

    #[tokio::test]
    async fn ldap_login_auto_provisions_and_returns_ldap_session() {
        let database_path = unique_database_path();
        let service = test_service(
            &database_path,
            StaticLdapAuthenticator::success(test_profile(
                "uid=auth.operator,ou=people,dc=tron,dc=ai",
                "auth.operator",
                "auth.operator@ldap.tron.ai",
                "Auth Operator",
            )),
        );

        let session = service
            .login_ldap("auth.operator", "TronAuthPass123!")
            .await
            .expect("ldap login should succeed");

        assert_eq!(session.session.auth_method, AuthMethod::Ldap);
        assert_eq!(
            session
                .session
                .provider
                .as_ref()
                .map(|provider| provider.code.as_str()),
            Some("ldap"),
        );
        assert_eq!(session.session.user.username, "auth.operator");
        assert_eq!(session.session.user.email, "auth.operator@ldap.tron.ai");
        assert_eq!(session.session.user.role_codes, vec!["VIEWER"]);
        assert_eq!(session.session.user.space_codes, vec!["PLATFORM"]);
        assert!(session.session.user.password_reset_required);

        let connection =
            rusqlite::Connection::open(&database_path).expect("test database should be readable");
        let ldap_link_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM user_oauth_identities WHERE provider_code = 'ldap'",
                [],
                |row| row.get(0),
            )
            .expect("ldap identity link should persist");
        assert_eq!(ldap_link_count, 1);

        let _ = fs::remove_file(&database_path);
    }

    #[tokio::test]
    async fn ldap_login_reuses_existing_subject_link_when_directory_email_changes() {
        let database_path = unique_database_path();
        let first_service = test_service(
            &database_path,
            StaticLdapAuthenticator::success(test_profile(
                "uid=clouds3n,ou=people,dc=tron,dc=ai",
                "clouds3n",
                "clouds3n@tron.ai",
                "Cloud S3n",
            )),
        );
        let first_session = first_service
            .login_ldap("clouds3n", "Password1!")
            .await
            .expect("first ldap login should succeed");

        let second_service = test_service(
            &database_path,
            StaticLdapAuthenticator::success(test_profile(
                "uid=clouds3n,ou=people,dc=tron,dc=ai",
                "clouds3n",
                "clouds3n.changed@tron.ai",
                "Cloud S3n",
            )),
        );
        let second_session = second_service
            .login_ldap("clouds3n", "Password1!")
            .await
            .expect("second ldap login should succeed");

        assert_eq!(
            second_session.session.user.user_code,
            first_session.session.user.user_code,
        );
        assert_eq!(second_session.session.user.email, "clouds3n@tron.ai");

        let _ = fs::remove_file(&database_path);
    }

    #[tokio::test]
    async fn ldap_login_surfaces_invalid_credentials_without_creating_a_session() {
        let database_path = unique_database_path();
        let service = test_service(
            &database_path,
            StaticLdapAuthenticator::failure(AuthError::InvalidCredentials),
        );

        let error = service
            .login_ldap("auth.operator", "wrong-password")
            .await
            .expect_err("ldap login should reject invalid credentials");
        assert_eq!(error, AuthError::InvalidCredentials);

        let _ = fs::remove_file(&database_path);
    }

    #[test]
    fn auth_providers_include_ldap_when_configured() {
        let service = test_service(
            ":memory:",
            StaticLdapAuthenticator::failure(AuthError::InvalidCredentials),
        );
        let providers = service.auth_providers();

        assert!(providers.iter().any(|provider| provider.code == "ldap"));
    }

    fn test_service(database_url: &str, authenticator: StaticLdapAuthenticator) -> UserService {
        let mut auth_config = AppConfig::default().auth;
        auth_config.dev_oauth2_enabled = false;
        auth_config.ldap = Some(test_ldap_config());

        UserService::with_dependencies(
            SqliteUserRepository::connect(database_url)
                .expect("sqlite user repository should initialize"),
            BcryptPasswordHasher,
            auth_config,
            EventBusHandle::disabled(),
            reqwest::Client::new(),
            Arc::new(authenticator),
        )
    }

    fn test_ldap_config() -> LdapConfig {
        LdapConfig {
            name: "LDAP".into(),
            url: "ldap://127.0.0.1:389".into(),
            bind_dn: "cn=service,dc=tron,dc=ai".into(),
            bind_password: "service-password".into(),
            search_base: "ou=people,dc=tron,dc=ai".into(),
            search_filter: "(|(uid={login})(mail={login}))".into(),
        }
    }

    fn test_profile(
        subject: &str,
        login: &str,
        email: &str,
        display_name: &str,
    ) -> AuthIdentityProfile {
        AuthIdentityProfile {
            provider_code: "ldap".into(),
            provider_name: "LDAP".into(),
            subject: subject.into(),
            login: login.into(),
            email: email.into(),
            display_name: display_name.into(),
        }
    }

    fn unique_database_path() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos();
        let sequence = NEXT_DB_ID.fetch_add(1, Ordering::Relaxed);
        let process_id = std::process::id();
        std::env::temp_dir().join(format!(
            "tron-auth-ldap-{process_id}-{nanos}-{sequence}.sqlite3"
        ))
    }
}
