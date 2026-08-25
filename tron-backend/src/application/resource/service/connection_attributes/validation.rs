use std::collections::{BTreeMap, HashSet};

use serde_json::Value;

use crate::domain::resource::ResourceError;

use super::{
    CredentialsAttributes, EndpointAttributes, KafkaAttributes, MysqlAttributes,
    PostgresqlAttributes, RedisAttributes, SecretReference, SecretReferenceAttributes,
    TlsAttributes,
};

pub(super) fn validate_mysql_attributes(attributes: &MysqlAttributes) -> Result<(), ResourceError> {
    let mode = attributes.connection_mode.as_str();
    if !matches!(mode, "SINGLE" | "FAILOVER" | "LOAD_BALANCE" | "REPLICATION") {
        return Err(invalid(
            "connectionMode",
            "must be one of SINGLE, FAILOVER, LOAD_BALANCE, REPLICATION",
        ));
    }
    validate_endpoints(&attributes.endpoints, mode == "REPLICATION")?;
    match mode {
        "SINGLE" if attributes.endpoints.len() != 1 => {
            return Err(invalid(
                "endpoints",
                "SINGLE mode requires exactly one endpoint",
            ));
        }
        "REPLICATION" if attributes.endpoints.len() < 2 => {
            return Err(invalid(
                "endpoints",
                "REPLICATION mode requires at least two endpoints",
            ));
        }
        "REPLICATION" => {
            let roles = attributes
                .endpoints
                .iter()
                .map(|endpoint| endpoint.role.as_deref())
                .collect::<Vec<_>>();
            if roles
                .iter()
                .any(|role| !matches!(role.map(str::trim), Some("PRIMARY" | "REPLICA")))
                || !roles
                    .iter()
                    .any(|role| role.is_some_and(|role| role.trim() == "PRIMARY"))
                || !roles
                    .iter()
                    .any(|role| role.is_some_and(|role| role.trim() == "REPLICA"))
            {
                return Err(invalid(
                    "endpoints[].role",
                    "REPLICATION endpoints must include PRIMARY and REPLICA roles",
                ));
            }
        }
        _ if attributes
            .endpoints
            .iter()
            .any(|endpoint| endpoint.role.is_some()) =>
        {
            return Err(invalid(
                "endpoints[].role",
                "roles are only allowed in REPLICATION mode",
            ));
        }
        _ => {}
    }
    validate_text(&attributes.database, "database", 1, 64)?;
    validate_credentials(&attributes.credentials, true)?;
    validate_tls(
        &attributes.tls,
        &[
            "DISABLED",
            "PREFERRED",
            "REQUIRED",
            "VERIFY_CA",
            "VERIFY_IDENTITY",
        ],
        false,
    )?;
    validate_properties(&attributes.properties)
}

pub(super) fn validate_postgresql_attributes(
    attributes: &PostgresqlAttributes,
) -> Result<(), ResourceError> {
    validate_endpoints(&attributes.endpoints, false)?;
    validate_text(&attributes.database, "database", 1, 63)?;
    validate_credentials(&attributes.credentials, true)?;
    if !matches!(
        attributes.target_session.as_str(),
        "ANY" | "READ_WRITE" | "READ_ONLY" | "PRIMARY" | "STANDBY" | "PREFER_STANDBY"
    ) || !matches!(attributes.load_balance.as_str(), "DISABLED" | "RANDOM")
    {
        return Err(invalid(
            "targetSession/loadBalance",
            "must use a supported PostgreSQL session and load-balance mode",
        ));
    }
    validate_tls(
        &attributes.tls,
        &[
            "DISABLED",
            "ALLOW",
            "PREFERRED",
            "REQUIRED",
            "VERIFY_CA",
            "VERIFY_FULL",
        ],
        true,
    )?;
    validate_properties(&attributes.properties)
}

pub(super) fn validate_redis_attributes(attributes: &RedisAttributes) -> Result<(), ResourceError> {
    let topology = attributes.topology.as_str();
    if !matches!(topology, "STANDALONE" | "SENTINEL" | "CLUSTER") {
        return Err(invalid(
            "topology",
            "must be one of STANDALONE, SENTINEL, CLUSTER",
        ));
    }
    validate_endpoints(&attributes.endpoints, false)?;
    if topology == "STANDALONE" && attributes.endpoints.len() != 1 {
        return Err(invalid(
            "endpoints",
            "STANDALONE topology requires exactly one endpoint",
        ));
    }
    if attributes.database < 0 {
        return Err(invalid("database", "must be non-negative"));
    }
    if topology == "CLUSTER" && attributes.database != 0 {
        return Err(invalid("database", "must be zero for CLUSTER topology"));
    }
    validate_optional_credentials(attributes.credentials.as_ref())?;
    validate_tls(
        &attributes.tls,
        &["DISABLED", "REQUIRED", "VERIFY_CA", "VERIFY_FULL"],
        true,
    )?;
    match (topology, attributes.sentinel.as_ref()) {
        ("SENTINEL", Some(sentinel)) => {
            let master_name = sentinel.master_name.trim();
            if master_name.is_empty()
                || master_name.len() > 128
                || master_name.chars().any(char::is_whitespace)
            {
                return Err(invalid(
                    "sentinel.masterName",
                    "must be non-blank, no longer than 128 characters, and contain no whitespace",
                ));
            }
            validate_optional_credentials(sentinel.credentials.as_ref())?;
            validate_tls(
                &sentinel.tls,
                &["DISABLED", "REQUIRED", "VERIFY_CA", "VERIFY_FULL"],
                true,
            )?;
        }
        ("SENTINEL", None) => {
            return Err(invalid("sentinel", "is required for SENTINEL topology"));
        }
        (_, Some(_)) => {
            return Err(invalid("sentinel", "is only allowed for SENTINEL topology"));
        }
        _ => {}
    }
    validate_properties(&attributes.properties)
}

pub(super) fn validate_kafka_attributes(attributes: &KafkaAttributes) -> Result<(), ResourceError> {
    validate_endpoints(&attributes.brokers, false)?;
    let security_protocol = attributes.security_protocol.as_str();
    if !matches!(
        security_protocol,
        "PLAINTEXT" | "SSL" | "SASL_PLAINTEXT" | "SASL_SSL"
    ) {
        return Err(invalid(
            "securityProtocol",
            "must be one of PLAINTEXT, SSL, SASL_PLAINTEXT, SASL_SSL",
        ));
    }
    match (
        security_protocol.starts_with("SASL_"),
        attributes.sasl.as_ref(),
    ) {
        (true, Some(sasl)) => {
            if !matches!(
                sasl.mechanism.as_str(),
                "PLAIN" | "SCRAM_SHA_256" | "SCRAM_SHA_512"
            ) {
                return Err(invalid(
                    "sasl.mechanism",
                    "must be one of PLAIN, SCRAM_SHA_256, SCRAM_SHA_512",
                ));
            }
            validate_text(&sasl.username, "sasl.username", 1, 128)?;
            validate_secret_reference(&sasl.password)?;
        }
        (true, None) => {
            return Err(invalid("sasl", "is required for SASL security protocols"));
        }
        (false, Some(_)) => {
            return Err(invalid(
                "sasl",
                "is only allowed for SASL security protocols",
            ));
        }
        _ => {}
    }
    match (security_protocol.ends_with("SSL"), attributes.tls.as_ref()) {
        (true, Some(tls)) => validate_tls(tls, &["REQUIRED", "VERIFY_CA", "VERIFY_FULL"], true)?,
        (true, None) => {
            return Err(invalid("tls", "is required for SSL security protocols"));
        }
        (false, Some(_)) => {
            return Err(invalid("tls", "is only allowed for SSL security protocols"));
        }
        _ => {}
    }
    validate_properties(&attributes.properties)
}

fn validate_endpoints(
    endpoints: &[EndpointAttributes],
    roles_are_allowed: bool,
) -> Result<(), ResourceError> {
    if endpoints.is_empty() || endpoints.len() > 128 {
        return Err(invalid(
            "endpoints",
            "must contain between 1 and 128 endpoints",
        ));
    }
    let mut unique = HashSet::new();
    for (index, endpoint) in endpoints.iter().enumerate() {
        let field = format!("endpoints[{index}].host");
        validate_text(&endpoint.host, &field, 1, 255)?;
        if endpoint.port == 0 || !unique.insert((endpoint.host.trim(), endpoint.port)) {
            return Err(invalid(
                format!("endpoints[{index}].port"),
                "must be between 1 and 65535 and endpoints must be unique",
            ));
        }
        if !roles_are_allowed && endpoint.role.is_some() {
            return Err(invalid(
                format!("endpoints[{index}].role"),
                "is not allowed for this connection mode",
            ));
        }
    }
    Ok(())
}

fn validate_credentials(
    credentials: &CredentialsAttributes,
    username_required: bool,
) -> Result<(), ResourceError> {
    match credentials.username.as_deref() {
        Some(username) => validate_text(username, "credentials.username", 1, 128)?,
        None if username_required => {
            return Err(invalid("credentials.username", "is required"));
        }
        None => {}
    }
    validate_secret_reference(&credentials.password)
}

fn validate_optional_credentials(
    credentials: Option<&CredentialsAttributes>,
) -> Result<(), ResourceError> {
    credentials
        .map(|credentials| validate_credentials(credentials, false))
        .transpose()?;
    Ok(())
}

fn validate_secret_reference(reference: &SecretReferenceAttributes) -> Result<(), ResourceError> {
    match reference {
        SecretReferenceAttributes::LegacyVariable(key) => {
            validate_text(key, "credentials.password", 1, 128)
        }
        SecretReferenceAttributes::Reference(SecretReference::Direct { value }) => {
            validate_text(value, "credentials.password.value", 1, 4096)
        }
        SecretReferenceAttributes::Reference(SecretReference::Variable { variable_key }) => {
            validate_text(variable_key, "credentials.password.variableKey", 1, 128)
        }
    }
}

fn validate_tls(
    tls: &TlsAttributes,
    allowed_modes: &[&str],
    enforce_client_key_dependencies: bool,
) -> Result<(), ResourceError> {
    if !allowed_modes.contains(&tls.mode.as_str()) {
        return Err(invalid("tls.mode", "must use a supported TLS mode"));
    }
    for key in [
        tls.ca_certificate.as_deref(),
        tls.client_certificate.as_deref(),
        tls.client_key.as_deref(),
        tls.client_key_password.as_deref(),
    ] {
        if let Some(key) = key {
            validate_text(key, "tls.material", 1, 128)?;
        }
    }
    if enforce_client_key_dependencies
        && (tls.client_certificate.is_some() != tls.client_key.is_some()
            || (tls.client_key_password.is_some() && tls.client_key.is_none()))
    {
        return Err(invalid(
            "tls",
            "clientCertificate and clientKey must be provided together, and clientKeyPassword requires clientKey",
        ));
    }
    Ok(())
}

fn validate_properties(values: &BTreeMap<String, Value>) -> Result<(), ResourceError> {
    if values.len() > 128 {
        return Err(invalid(
            "properties",
            "must contain no more than 128 entries",
        ));
    }
    for key in values.keys() {
        validate_text(key, "properties", 1, 128)?;
    }
    Ok(())
}

fn validate_text(
    value: &str,
    field: &str,
    minimum: usize,
    maximum: usize,
) -> Result<(), ResourceError> {
    let value = value.trim();
    if value.len() < minimum || value.len() > maximum {
        return Err(invalid(
            field,
            format!("must contain between {minimum} and {maximum} characters"),
        ));
    }
    Ok(())
}

fn invalid(field: impl Into<String>, reason: impl Into<String>) -> ResourceError {
    ResourceError::InvalidField {
        field: field.into(),
        reason: reason.into(),
    }
}
