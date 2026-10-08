use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;

use crate::{
    application::resource::{
        ConnectionEndpoint, ConnectionTlsMaterial, KafkaConnectionTarget, KafkaSaslTarget,
        MysqlConnectionTarget, PostgresqlConnectionTarget, RedisConnectionTarget,
        RedisSentinelTarget, RedisTopology, ResourceConnectionTarget, ServerAccountCreateInput,
        SshAuthenticationTarget, SshConnectionTarget,
    },
    domain::{
        resource::{ResourceError, ResourceKind, ServerAuthenticationType},
        variable::VariableError,
    },
};

use super::ResourceService;

mod validation;

use validation::{
    validate_kafka_attributes, validate_mysql_attributes, validate_postgresql_attributes,
    validate_redis_attributes,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EndpointAttributes {
    host: String,
    port: u16,
    #[serde(default)]
    role: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CredentialsAttributes {
    #[serde(default)]
    username: Option<String>,
    password: SecretReferenceAttributes,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum SecretReferenceAttributes {
    LegacyVariable(String),
    Reference(SecretReference),
}

#[derive(Deserialize)]
#[serde(tag = "source", deny_unknown_fields)]
enum SecretReference {
    #[serde(rename = "DIRECT")]
    Direct { value: String },
    #[serde(rename = "VARIABLE")]
    Variable {
        #[serde(rename = "variableKey")]
        variable_key: String,
    },
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TlsAttributes {
    mode: String,
    #[serde(default)]
    ca_certificate: Option<String>,
    #[serde(default)]
    client_certificate: Option<String>,
    #[serde(default)]
    client_key: Option<String>,
    #[serde(default)]
    client_key_password: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MysqlAttributes {
    connection_mode: String,
    endpoints: Vec<EndpointAttributes>,
    database: String,
    credentials: CredentialsAttributes,
    tls: TlsAttributes,
    #[serde(default)]
    properties: BTreeMap<String, Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PostgresqlAttributes {
    endpoints: Vec<EndpointAttributes>,
    database: String,
    credentials: CredentialsAttributes,
    target_session: String,
    load_balance: String,
    tls: TlsAttributes,
    #[serde(default)]
    properties: BTreeMap<String, Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RedisSentinelAttributes {
    master_name: String,
    #[serde(default)]
    credentials: Option<CredentialsAttributes>,
    tls: TlsAttributes,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RedisAttributes {
    topology: String,
    endpoints: Vec<EndpointAttributes>,
    database: i64,
    #[serde(default)]
    credentials: Option<CredentialsAttributes>,
    tls: TlsAttributes,
    #[serde(default)]
    sentinel: Option<RedisSentinelAttributes>,
    #[serde(default)]
    properties: BTreeMap<String, Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct KafkaSaslAttributes {
    mechanism: String,
    username: String,
    password: SecretReferenceAttributes,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct KafkaAttributes {
    brokers: Vec<EndpointAttributes>,
    security_protocol: String,
    #[serde(default)]
    sasl: Option<KafkaSaslAttributes>,
    #[serde(default)]
    tls: Option<TlsAttributes>,
}

impl ResourceService {
    pub(super) fn data_source_connection_target(
        &self,
        workspace_code: &str,
        kind: ResourceKind,
        attributes: &Value,
    ) -> Result<ResourceConnectionTarget, ResourceError> {
        if serde_json::to_vec(attributes)
            .map_err(|_| ResourceError::InvalidField {
                field: "attributes".to_string(),
                reason: "must be a valid JSON value".to_string(),
            })?
            .len()
            > self.effective_config().limits.attributes_max_bytes
        {
            return Err(ResourceError::InvalidField {
                field: "attributes".to_string(),
                reason: format!(
                    "must not exceed {} bytes",
                    self.effective_config().limits.attributes_max_bytes
                ),
            });
        }
        match kind {
            ResourceKind::MysqlConnection => {
                let attributes: MysqlAttributes = parse(attributes)?;
                validate_mysql_attributes(&attributes)?;
                let values = self.resolve_connection_variables(
                    workspace_code,
                    mysql_variable_keys(&attributes),
                )?;
                Ok(ResourceConnectionTarget::Mysql(MysqlConnectionTarget {
                    endpoints: endpoints(attributes.endpoints),
                    database: attributes.database,
                    username: direct_text_required(attributes.credentials.username.as_deref())?,
                    password: resolve_secret_reference(&values, &attributes.credentials.password)?,
                    tls: tls_material(attributes.tls, &values)?,
                    properties: scalar_properties(attributes.properties)?,
                }))
            }
            ResourceKind::PostgresqlConnection => {
                let attributes: PostgresqlAttributes = parse(attributes)?;
                validate_postgresql_attributes(&attributes)?;
                let values = self.resolve_connection_variables(
                    workspace_code,
                    postgresql_variable_keys(&attributes),
                )?;
                Ok(ResourceConnectionTarget::Postgresql(
                    PostgresqlConnectionTarget {
                        endpoints: endpoints(attributes.endpoints),
                        database: attributes.database,
                        username: direct_text_required(attributes.credentials.username.as_deref())?,
                        password: resolve_secret_reference(
                            &values,
                            &attributes.credentials.password,
                        )?,
                        tls: tls_material(attributes.tls, &values)?,
                        properties: scalar_properties(attributes.properties)?,
                    },
                ))
            }
            ResourceKind::RedisConnection => {
                let attributes: RedisAttributes = parse(attributes)?;
                validate_redis_attributes(&attributes)?;
                let values = self.resolve_connection_variables(
                    workspace_code,
                    redis_variable_keys(&attributes),
                )?;
                let credentials = attributes.credentials;
                let sentinel = attributes
                    .sentinel
                    .map(|sentinel| {
                        let credentials = sentinel.credentials;
                        Ok(RedisSentinelTarget {
                            master_name: sentinel.master_name,
                            username: optional_direct_text(
                                credentials
                                    .as_ref()
                                    .and_then(|value| value.username.as_deref()),
                            ),
                            password: optional_secret_reference(
                                &values,
                                credentials.as_ref().map(|value| &value.password),
                            )?,
                            tls: tls_material(sentinel.tls, &values)?,
                        })
                    })
                    .transpose()?;
                Ok(ResourceConnectionTarget::Redis(RedisConnectionTarget {
                    topology: redis_topology(&attributes.topology)?,
                    endpoints: endpoints(attributes.endpoints),
                    database: attributes.database,
                    username: optional_direct_text(
                        credentials
                            .as_ref()
                            .and_then(|value| value.username.as_deref()),
                    ),
                    password: optional_secret_reference(
                        &values,
                        credentials.as_ref().map(|value| &value.password),
                    )?,
                    tls: tls_material(attributes.tls, &values)?,
                    sentinel,
                }))
            }
            ResourceKind::KafkaConnection => {
                let attributes: KafkaAttributes = parse(attributes)?;
                validate_kafka_attributes(&attributes)?;
                let values = self.resolve_connection_variables(
                    workspace_code,
                    kafka_variable_keys(&attributes),
                )?;
                let sasl = attributes
                    .sasl
                    .map(|sasl| {
                        Ok(KafkaSaslTarget {
                            mechanism: sasl.mechanism,
                            username: direct_text_required(Some(&sasl.username))?,
                            password: resolve_secret_reference(&values, &sasl.password)?,
                        })
                    })
                    .transpose()?;
                let tls = attributes
                    .tls
                    .map(|tls| tls_material(tls, &values))
                    .transpose()?;
                Ok(ResourceConnectionTarget::Kafka(KafkaConnectionTarget {
                    brokers: endpoints(attributes.brokers),
                    security_protocol: attributes.security_protocol,
                    sasl,
                    tls,
                }))
            }
            _ => Err(ResourceError::InvalidInput),
        }
    }

    pub(super) fn ssh_connection_target(
        &self,
        workspace_code: &str,
        host: String,
        port: u16,
        account: &ServerAccountCreateInput,
    ) -> Result<ResourceConnectionTarget, ResourceError> {
        let mut keys = BTreeSet::new();
        extend_key(&mut keys, account.password_variable_key.as_deref());
        extend_key(&mut keys, account.private_key_variable_key.as_deref());
        extend_key(
            &mut keys,
            account.private_key_passphrase_variable_key.as_deref(),
        );
        let values = self.resolve_connection_variables(workspace_code, keys)?;
        let authentication = match account.authentication_type {
            ServerAuthenticationType::Password => SshAuthenticationTarget::Password {
                password: secret_required(&values, account.password_variable_key.as_deref())?,
            },
            ServerAuthenticationType::PrivateKey => SshAuthenticationTarget::PrivateKey {
                private_key: secret_required(&values, account.private_key_variable_key.as_deref())?,
                passphrase: optional_secret(
                    &values,
                    account.private_key_passphrase_variable_key.as_deref(),
                )?,
            },
        };
        Ok(ResourceConnectionTarget::Ssh(SshConnectionTarget {
            endpoint: ConnectionEndpoint { host, port },
            username: account.username.trim().to_string(),
            authentication,
        }))
    }
}

fn parse<T: DeserializeOwned>(value: &Value) -> Result<T, ResourceError> {
    let encoded = serde_json::to_vec(value).map_err(|_| ResourceError::InvalidInput)?;
    let mut deserializer = serde_json::Deserializer::from_slice(&encoded);
    serde_path_to_error::deserialize(&mut deserializer).map_err(|error| {
        let detail = error.inner().to_string();
        let path = error.path().to_string();
        let field = if path == "." || path.is_empty() {
            serde_error_field(&detail).unwrap_or(path)
        } else {
            path
        };
        ResourceError::InvalidField {
            field,
            reason: detail,
        }
    })
}

fn serde_error_field(detail: &str) -> Option<String> {
    [
        "missing field \u{0060}",
        "unknown field \u{0060}",
        "missing field '",
        "unknown field '",
    ]
    .iter()
    .find_map(|marker| {
        let start = detail.find(marker)? + marker.len();
        let value = &detail[start..];
        let end = value.find(['\u{0060}', '\''])?;
        Some(value[..end].to_string())
    })
}

fn endpoints(values: Vec<EndpointAttributes>) -> Vec<ConnectionEndpoint> {
    values
        .into_iter()
        .map(|value| {
            let _ = value.role;
            ConnectionEndpoint {
                host: value.host,
                port: value.port,
            }
        })
        .collect()
}

fn mysql_variable_keys(attributes: &MysqlAttributes) -> BTreeSet<String> {
    let mut keys = credentials_variable_keys(Some(&attributes.credentials));
    extend_tls_keys(&mut keys, &attributes.tls);
    keys
}

fn postgresql_variable_keys(attributes: &PostgresqlAttributes) -> BTreeSet<String> {
    let mut keys = credentials_variable_keys(Some(&attributes.credentials));
    extend_tls_keys(&mut keys, &attributes.tls);
    keys
}

fn redis_variable_keys(attributes: &RedisAttributes) -> BTreeSet<String> {
    let mut keys = credentials_variable_keys(attributes.credentials.as_ref());
    extend_tls_keys(&mut keys, &attributes.tls);
    if let Some(sentinel) = &attributes.sentinel {
        keys.extend(credentials_variable_keys(sentinel.credentials.as_ref()));
        extend_tls_keys(&mut keys, &sentinel.tls);
    }
    keys
}

fn kafka_variable_keys(attributes: &KafkaAttributes) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    if let Some(sasl) = &attributes.sasl {
        extend_secret_reference_key(&mut keys, &sasl.password);
    }
    if let Some(tls) = &attributes.tls {
        extend_tls_keys(&mut keys, tls);
    }
    keys
}

fn credentials_variable_keys(value: Option<&CredentialsAttributes>) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    if let Some(value) = value {
        extend_secret_reference_key(&mut keys, &value.password);
    }
    keys
}

fn extend_tls_keys(keys: &mut BTreeSet<String>, tls: &TlsAttributes) {
    extend_key(keys, tls.ca_certificate.as_deref());
    extend_key(keys, tls.client_certificate.as_deref());
    extend_key(keys, tls.client_key.as_deref());
    extend_key(keys, tls.client_key_password.as_deref());
}

fn extend_key(keys: &mut BTreeSet<String>, key: Option<&str>) {
    if let Some(key) = key {
        keys.insert(key.trim().to_string());
    }
}

fn extend_secret_reference_key(keys: &mut BTreeSet<String>, secret: &SecretReferenceAttributes) {
    match secret {
        SecretReferenceAttributes::LegacyVariable(key) => extend_key(keys, Some(key)),
        SecretReferenceAttributes::Reference(SecretReference::Variable { variable_key }) => {
            extend_key(keys, Some(variable_key));
        }
        SecretReferenceAttributes::Reference(SecretReference::Direct { .. }) => {}
    }
}

impl ResourceService {
    fn resolve_connection_variables(
        &self,
        workspace_code: &str,
        keys: BTreeSet<String>,
    ) -> Result<BTreeMap<String, String>, ResourceError> {
        self.variables
            .resolve_runtime_variable_values(workspace_code, &keys)
            .map_err(map_variable_error)
    }
}

fn secret(values: &BTreeMap<String, String>, key: &str) -> Result<String, ResourceError> {
    values
        .get(key.trim())
        .cloned()
        .ok_or(ResourceError::InvalidVariableReference)
}

fn resolve_secret_reference(
    values: &BTreeMap<String, String>,
    reference: &SecretReferenceAttributes,
) -> Result<String, ResourceError> {
    match reference {
        SecretReferenceAttributes::LegacyVariable(key) => secret(values, key),
        SecretReferenceAttributes::Reference(SecretReference::Direct { value }) => {
            direct_text_required(Some(value))
        }
        SecretReferenceAttributes::Reference(SecretReference::Variable { variable_key }) => {
            secret(values, variable_key)
        }
    }
}

fn secret_required(
    values: &BTreeMap<String, String>,
    key: Option<&str>,
) -> Result<String, ResourceError> {
    secret(values, key.ok_or(ResourceError::InvalidVariableReference)?)
}

fn optional_secret(
    values: &BTreeMap<String, String>,
    key: Option<&str>,
) -> Result<Option<String>, ResourceError> {
    key.map(|key| secret(values, key)).transpose()
}

fn optional_secret_reference(
    values: &BTreeMap<String, String>,
    secret: Option<&SecretReferenceAttributes>,
) -> Result<Option<String>, ResourceError> {
    secret
        .map(|value| resolve_secret_reference(values, value))
        .transpose()
}

fn direct_text_required(value: Option<&str>) -> Result<String, ResourceError> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or(ResourceError::InvalidInput)
}

fn optional_direct_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn tls_material(
    tls: TlsAttributes,
    values: &BTreeMap<String, String>,
) -> Result<ConnectionTlsMaterial, ResourceError> {
    Ok(ConnectionTlsMaterial {
        mode: tls.mode,
        ca_certificate: optional_secret(values, tls.ca_certificate.as_deref())?,
        client_certificate: optional_secret(values, tls.client_certificate.as_deref())?,
        client_key: optional_secret(values, tls.client_key.as_deref())?,
        client_key_password: optional_secret(values, tls.client_key_password.as_deref())?,
    })
}

fn scalar_properties(
    values: BTreeMap<String, Value>,
) -> Result<BTreeMap<String, String>, ResourceError> {
    values
        .into_iter()
        .map(|(key, value)| {
            let value = match value {
                Value::String(value) => value,
                Value::Number(value) => value.to_string(),
                Value::Bool(value) => value.to_string(),
                _ => return Err(ResourceError::InvalidInput),
            };
            Ok((key, value))
        })
        .collect()
}

fn redis_topology(value: &str) -> Result<RedisTopology, ResourceError> {
    match value {
        "STANDALONE" => Ok(RedisTopology::Standalone),
        "SENTINEL" => Ok(RedisTopology::Sentinel),
        "CLUSTER" => Ok(RedisTopology::Cluster),
        _ => Err(ResourceError::InvalidInput),
    }
}

fn map_variable_error(error: VariableError) -> ResourceError {
    match error {
        VariableError::NotFound | VariableError::InvalidKey => {
            ResourceError::InvalidVariableReference
        }
        VariableError::InvalidWorkspace | VariableError::WorkspaceRequired => {
            ResourceError::InvalidWorkspace
        }
        VariableError::StorageFailure => ResourceError::StorageFailure,
        _ => ResourceError::InvalidInput,
    }
}
