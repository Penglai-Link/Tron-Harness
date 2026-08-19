use serde::{Deserialize, Serialize};

use super::ResourceKind;

pub const MYSQL_CONNECTION_CODE: &str = "MYSQL_CONNECTION";
pub const POSTGRESQL_CONNECTION_CODE: &str = "POSTGRESQL_CONNECTION";
pub const REDIS_CONNECTION_CODE: &str = "REDIS_CONNECTION";
pub const KAFKA_CONNECTION_CODE: &str = "KAFKA_CONNECTION";
pub const SERVER_CONNECTION_CODE: &str = "SERVER_CONNECTION";
pub const SERVER_ACCOUNT_CODE: &str = "SERVER_ACCOUNT";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerAuthenticationType {
    Password,
    PrivateKey,
}

pub fn is_data_source_kind(code: &str) -> bool {
    matches!(
        code,
        MYSQL_CONNECTION_CODE | POSTGRESQL_CONNECTION_CODE | REDIS_CONNECTION_CODE | KAFKA_CONNECTION_CODE
    )
}

pub fn is_connection_managed_kind(code: &str) -> bool {
    is_data_source_kind(code) || matches!(code, SERVER_CONNECTION_CODE | SERVER_ACCOUNT_CODE)
}

impl ResourceKind {
    pub fn is_data_source(&self) -> bool {
        matches!(
            self,
            Self::MysqlConnection
                | Self::PostgresqlConnection
                | Self::RedisConnection
                | Self::KafkaConnection
        )
    }
}
