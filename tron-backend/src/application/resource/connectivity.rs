use std::{collections::BTreeMap, future::Future, pin::Pin};

use crate::domain::resource::ResourceError;

pub type ResourceConnectionVerificationFuture<'a> =
    Pin<Box<dyn Future<Output = Result<(), ResourceError>> + Send + 'a>>;

pub trait ResourceConnectionVerifier: Send + Sync {
    fn verify(&self, target: ResourceConnectionTarget) -> ResourceConnectionVerificationFuture<'_>;
}

#[derive(Clone)]
pub enum ResourceConnectionTarget {
    Mysql(MysqlConnectionTarget),
    Postgresql(PostgresqlConnectionTarget),
    Redis(RedisConnectionTarget),
    Kafka(KafkaConnectionTarget),
    Ssh(SshConnectionTarget),
}

#[derive(Clone)]
pub struct ConnectionEndpoint {
    pub host: String,
    pub port: u16,
}

#[derive(Clone, Default)]
pub struct ConnectionTlsMaterial {
    pub mode: String,
    pub ca_certificate: Option<String>,
    pub client_certificate: Option<String>,
    pub client_key: Option<String>,
    pub client_key_password: Option<String>,
}

#[derive(Clone)]
pub struct MysqlConnectionTarget {
    pub endpoints: Vec<ConnectionEndpoint>,
    pub database: String,
    pub username: String,
    pub password: String,
    pub tls: ConnectionTlsMaterial,
    pub properties: BTreeMap<String, String>,
}

#[derive(Clone)]
pub struct PostgresqlConnectionTarget {
    pub endpoints: Vec<ConnectionEndpoint>,
    pub database: String,
    pub username: String,
    pub password: String,
    pub tls: ConnectionTlsMaterial,
    pub properties: BTreeMap<String, String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RedisTopology {
    Standalone,
    Sentinel,
    Cluster,
}

#[derive(Clone)]
pub struct RedisSentinelTarget {
    pub master_name: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub tls: ConnectionTlsMaterial,
}

#[derive(Clone)]
pub struct RedisConnectionTarget {
    pub topology: RedisTopology,
    pub endpoints: Vec<ConnectionEndpoint>,
    pub database: i64,
    pub username: Option<String>,
    pub password: Option<String>,
    pub tls: ConnectionTlsMaterial,
    pub sentinel: Option<RedisSentinelTarget>,
}

#[derive(Clone)]
pub struct KafkaSaslTarget {
    pub mechanism: String,
    pub username: String,
    pub password: String,
}

#[derive(Clone)]
pub struct KafkaConnectionTarget {
    pub brokers: Vec<ConnectionEndpoint>,
    pub security_protocol: String,
    pub sasl: Option<KafkaSaslTarget>,
    pub tls: Option<ConnectionTlsMaterial>,
    pub properties: BTreeMap<String, String>,
}

#[derive(Clone)]
pub enum SshAuthenticationTarget {
    Password {
        password: String,
    },
    PrivateKey {
        private_key: String,
        passphrase: Option<String>,
    },
}

#[derive(Clone)]
pub struct SshConnectionTarget {
    pub endpoint: ConnectionEndpoint,
    pub username: String,
    pub authentication: SshAuthenticationTarget,
}
