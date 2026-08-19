use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ResourceSourceKind {
    Local,
    Inherited,
    Override,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ResourceKind {
    File,
    Image,
    Pdf,
    MysqlConnection,
    PostgresqlConnection,
    RedisConnection,
    KafkaConnection,
    ServerConnection,
    ServerAccount,
}

impl ResourceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::File => "FILE",
            Self::Image => "IMAGE",
            Self::Pdf => "PDF",
            Self::MysqlConnection => "MYSQL_CONNECTION",
            Self::PostgresqlConnection => "POSTGRESQL_CONNECTION",
            Self::RedisConnection => "REDIS_CONNECTION",
            Self::KafkaConnection => "KAFKA_CONNECTION",
            Self::ServerConnection => "SERVER_CONNECTION",
            Self::ServerAccount => "SERVER_ACCOUNT",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "FILE" => Some(Self::File),
            "IMAGE" => Some(Self::Image),
            "PDF" => Some(Self::Pdf),
            "MYSQL_CONNECTION" => Some(Self::MysqlConnection),
            "POSTGRESQL_CONNECTION" => Some(Self::PostgresqlConnection),
            "REDIS_CONNECTION" => Some(Self::RedisConnection),
            "KAFKA_CONNECTION" => Some(Self::KafkaConnection),
            "SERVER_CONNECTION" => Some(Self::ServerConnection),
            "SERVER_ACCOUNT" => Some(Self::ServerAccount),
            _ => None,
        }
    }

    pub fn is_file_kind(&self) -> bool {
        matches!(self, Self::File | Self::Image | Self::Pdf)
    }

    pub fn is_connection_kind(&self) -> bool {
        !self.is_file_kind()
    }
}
