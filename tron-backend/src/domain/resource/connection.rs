use serde::{Deserialize, Serialize};

pub const MYSQL_CONNECTION_DEFINITION_CODE: &str = "MYSQL_CONNECTION";
pub const POSTGRESQL_CONNECTION_DEFINITION_CODE: &str = "POSTGRESQL_CONNECTION";
pub const REDIS_CONNECTION_DEFINITION_CODE: &str = "REDIS_CONNECTION";
pub const KAFKA_CONNECTION_DEFINITION_CODE: &str = "KAFKA_CONNECTION";
pub const SERVER_CONNECTION_DEFINITION_CODE: &str = "SERVER_CONNECTION";
pub const SERVER_ACCOUNT_DEFINITION_CODE: &str = "SERVER_ACCOUNT";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerAuthenticationType {
    Password,
    PrivateKey,
}

pub fn is_data_source_definition_code(code: &str) -> bool {
    matches!(
        code,
        MYSQL_CONNECTION_DEFINITION_CODE
            | POSTGRESQL_CONNECTION_DEFINITION_CODE
            | REDIS_CONNECTION_DEFINITION_CODE
            | KAFKA_CONNECTION_DEFINITION_CODE
    )
}

pub fn is_connection_managed_definition_code(code: &str) -> bool {
    is_data_source_definition_code(code)
        || matches!(
            code,
            SERVER_CONNECTION_DEFINITION_CODE | SERVER_ACCOUNT_DEFINITION_CODE
        )
}
