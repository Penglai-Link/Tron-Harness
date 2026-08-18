use sqlx::{
    Connection, Executor, MySqlConnection,
    mysql::{MySqlConnectOptions, MySqlSslMode},
};

use crate::application::resource::{ConnectionTlsMaterial, MysqlConnectionTarget};

pub(super) async fn verify(target: MysqlConnectionTarget) -> Result<(), ()> {
    for endpoint in &target.endpoints {
        let options = mysql_options(&target, &endpoint.host, endpoint.port)?;
        let Ok(mut connection) = MySqlConnection::connect_with(&options).await else {
            continue;
        };
        if connection.execute("SELECT 1").await.is_ok() {
            let _ = connection.close().await;
            return Ok(());
        }
    }
    Err(())
}

fn mysql_options(
    target: &MysqlConnectionTarget,
    host: &str,
    port: u16,
) -> Result<MySqlConnectOptions, ()> {
    let mut options = MySqlConnectOptions::new()
        .host(host)
        .port(port)
        .username(&target.username)
        .password(&target.password)
        .database(&target.database)
        .ssl_mode(mysql_ssl_mode(&target.tls.mode)?);
    options = apply_tls(options, &target.tls);
    Ok(options)
}

fn mysql_ssl_mode(mode: &str) -> Result<MySqlSslMode, ()> {
    match mode {
        "DISABLED" => Ok(MySqlSslMode::Disabled),
        "PREFERRED" => Ok(MySqlSslMode::Preferred),
        "REQUIRED" => Ok(MySqlSslMode::Required),
        "VERIFY_CA" => Ok(MySqlSslMode::VerifyCa),
        "VERIFY_IDENTITY" => Ok(MySqlSslMode::VerifyIdentity),
        _ => Err(()),
    }
}

fn apply_tls(mut options: MySqlConnectOptions, tls: &ConnectionTlsMaterial) -> MySqlConnectOptions {
    if let Some(value) = &tls.ca_certificate {
        options = options.ssl_ca_from_pem(value.as_bytes().to_vec());
    }
    if let Some(value) = &tls.client_certificate {
        options = options.ssl_client_cert_from_pem(value.as_bytes());
    }
    if let Some(value) = &tls.client_key {
        options = options.ssl_client_key_from_pem(value.as_bytes());
    }
    options
}
