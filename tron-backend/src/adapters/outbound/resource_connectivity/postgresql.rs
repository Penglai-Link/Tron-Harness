use sqlx::{
    Connection, Executor, PgConnection,
    postgres::{PgConnectOptions, PgSslMode},
};

use crate::application::resource::{ConnectionTlsMaterial, PostgresqlConnectionTarget};

pub(super) async fn verify(target: PostgresqlConnectionTarget) -> Result<(), ()> {
    for endpoint in &target.endpoints {
        let options = postgresql_options(&target, &endpoint.host, endpoint.port)?;
        let Ok(mut connection) = PgConnection::connect_with(&options).await else {
            continue;
        };
        if connection.execute("SELECT 1").await.is_ok() {
            let _ = connection.close().await;
            return Ok(());
        }
    }
    Err(())
}

fn postgresql_options(
    target: &PostgresqlConnectionTarget,
    host: &str,
    port: u16,
) -> Result<PgConnectOptions, ()> {
    let mut options = PgConnectOptions::new()
        .host(host)
        .port(port)
        .username(&target.username)
        .password(&target.password)
        .database(&target.database)
        .ssl_mode(postgresql_ssl_mode(&target.tls.mode)?);
    options = apply_tls(options, &target.tls);
    Ok(options)
}

fn postgresql_ssl_mode(mode: &str) -> Result<PgSslMode, ()> {
    match mode {
        "DISABLED" => Ok(PgSslMode::Disable),
        "ALLOW" => Ok(PgSslMode::Allow),
        "PREFERRED" => Ok(PgSslMode::Prefer),
        "REQUIRED" => Ok(PgSslMode::Require),
        "VERIFY_CA" => Ok(PgSslMode::VerifyCa),
        "VERIFY_FULL" => Ok(PgSslMode::VerifyFull),
        _ => Err(()),
    }
}

fn apply_tls(mut options: PgConnectOptions, tls: &ConnectionTlsMaterial) -> PgConnectOptions {
    if let Some(value) = &tls.ca_certificate {
        options = options.ssl_root_cert_from_pem(value.as_bytes().to_vec());
    }
    if let Some(value) = &tls.client_certificate {
        options = options.ssl_client_cert_from_pem(value.as_bytes());
    }
    if let Some(value) = &tls.client_key {
        options = options.ssl_client_key_from_pem(value.as_bytes());
    }
    options
}
