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
        return Err(invalid_with(
            "connectionMode",
            crate::shared::i18n_message::MessageText::new("resource.error.reason.one_of")
                .arg("allowed", "SINGLE, FAILOVER, LOAD_BALANCE, REPLICATION"),
        ));
    }
    validate_endpoints(&attributes.endpoints, mode == "REPLICATION")?;
    match mode {
        "SINGLE" if attributes.endpoints.len() != 1 => {
            return Err(invalid(
                "endpoints",
                "resource.error.reason.single_endpoint",
            ));
        }
        "REPLICATION" if attributes.endpoints.len() < 2 => {
            return Err(invalid(
                "endpoints",
                "resource.error.reason.replication_endpoints",
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
                    "resource.error.reason.replication_roles",
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
                "resource.error.reason.roles_replication_only",
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
            "resource.error.reason.postgres_session_mode",
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
        return Err(invalid_with(
            "topology",
            crate::shared::i18n_message::MessageText::new("resource.error.reason.one_of")
                .arg("allowed", "STANDALONE, SENTINEL, CLUSTER"),
        ));
    }
    validate_endpoints(&attributes.endpoints, false)?;
    if topology == "STANDALONE" && attributes.endpoints.len() != 1 {
        return Err(invalid(
            "endpoints",
            "resource.error.reason.standalone_endpoint",
        ));
    }
    if attributes.database < 0 {
        return Err(invalid("database", "resource.error.reason.non_negative"));
    }
    if topology == "CLUSTER" && attributes.database != 0 {
        return Err(invalid(
            "database",
            "resource.error.reason.cluster_database_zero",
        ));
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
                    "resource.error.reason.identifier_format",
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
            return Err(invalid(
                "sentinel",
                "resource.error.reason.required_sentinel",
            ));
        }
        (_, Some(_)) => {
            return Err(invalid("sentinel", "resource.error.reason.only_sentinel"));
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
        return Err(invalid_with(
            "securityProtocol",
            crate::shared::i18n_message::MessageText::new("resource.error.reason.one_of")
                .arg("allowed", "PLAINTEXT, SSL, SASL_PLAINTEXT, SASL_SSL"),
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
                return Err(invalid_with(
                    "sasl.mechanism",
                    crate::shared::i18n_message::MessageText::new("resource.error.reason.one_of")
                        .arg("allowed", "PLAIN, SCRAM_SHA_256, SCRAM_SHA_512"),
                ));
            }
            validate_text(&sasl.username, "sasl.username", 1, 128)?;
            validate_secret_reference(&sasl.password)?;
        }
        (true, None) => {
            return Err(invalid("sasl", "resource.error.reason.required_sasl"));
        }
        (false, Some(_)) => {
            return Err(invalid("sasl", "resource.error.reason.only_sasl"));
        }
        _ => {}
    }
    match (security_protocol.ends_with("SSL"), attributes.tls.as_ref()) {
        (true, Some(tls)) => {
            validate_tls(tls, &["REQUIRED", "VERIFY_CA", "VERIFY_FULL"], true)?;
            if tls.client_key_password.is_some() {
                return Err(invalid(
                    "tls.clientKeyPassword",
                    "resource.error.reason.unencrypted_client_key",
                ));
            }
        }
        (true, None) => {
            return Err(invalid("tls", "resource.error.reason.required_ssl"));
        }
        (false, Some(_)) => {
            return Err(invalid("tls", "resource.error.reason.only_ssl"));
        }
        _ => {}
    }
    Ok(())
}

fn validate_endpoints(
    endpoints: &[EndpointAttributes],
    roles_are_allowed: bool,
) -> Result<(), ResourceError> {
    if endpoints.is_empty() || endpoints.len() > 128 {
        return Err(invalid("endpoints", "resource.error.reason.endpoint_count"));
    }
    let mut unique = HashSet::new();
    for (index, endpoint) in endpoints.iter().enumerate() {
        let field = format!("endpoints[{index}].host");
        validate_text(&endpoint.host, &field, 1, 255)?;
        if endpoint.port == 0 || !unique.insert((endpoint.host.trim(), endpoint.port)) {
            return Err(invalid(
                format!("endpoints[{index}].port"),
                "resource.error.reason.endpoint_port",
            ));
        }
        if !roles_are_allowed && endpoint.role.is_some() {
            return Err(invalid(
                format!("endpoints[{index}].role"),
                "resource.error.reason.not_allowed_mode",
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
            return Err(invalid(
                "credentials.username",
                "resource.error.reason.required",
            ));
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
        return Err(invalid("tls.mode", "resource.error.reason.tls_mode"));
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
            "resource.error.reason.client_key_dependency",
        ));
    }
    Ok(())
}

fn validate_properties(values: &BTreeMap<String, Value>) -> Result<(), ResourceError> {
    if values.len() > 128 {
        return Err(invalid(
            "properties",
            "resource.error.reason.properties_limit",
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
        return Err(invalid_with(
            field,
            crate::shared::i18n_message::MessageText::new("resource.error.reason.length")
                .arg("minimum", minimum.to_string())
                .arg("maximum", maximum.to_string()),
        ));
    }
    Ok(())
}

fn invalid(field: impl Into<String>, key: &'static str) -> ResourceError {
    invalid_with(field, crate::shared::i18n_message::MessageText::new(key))
}

fn invalid_with(
    field: impl Into<String>,
    message: crate::shared::i18n_message::MessageText,
) -> ResourceError {
    ResourceError::InvalidField {
        field: field.into(),
        reason: message.encode(),
    }
}
