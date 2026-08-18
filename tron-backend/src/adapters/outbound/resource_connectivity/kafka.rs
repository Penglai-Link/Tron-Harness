use std::time::Duration;

use rdkafka::{
    ClientConfig,
    consumer::{BaseConsumer, Consumer},
};

use crate::application::resource::KafkaConnectionTarget;

pub(super) async fn verify(target: KafkaConnectionTarget, timeout: Duration) -> Result<(), ()> {
    tokio::task::spawn_blocking(move || verify_blocking(target, timeout))
        .await
        .map_err(|_| ())?
}

fn verify_blocking(target: KafkaConnectionTarget, timeout: Duration) -> Result<(), ()> {
    let mut config = ClientConfig::new();
    for (key, value) in &target.properties {
        config.set(key, value);
    }
    let brokers = target
        .brokers
        .iter()
        .map(|endpoint| format!("{}:{}", endpoint.host, endpoint.port))
        .collect::<Vec<_>>()
        .join(",");
    config
        .set("bootstrap.servers", brokers)
        .set("group.id", "tron-resource-connectivity-verification")
        .set("enable.auto.commit", "false")
        .set(
            "security.protocol",
            kafka_security_protocol(&target.security_protocol)?,
        );
    if let Some(sasl) = &target.sasl {
        config
            .set("sasl.mechanism", kafka_sasl_mechanism(&sasl.mechanism)?)
            .set("sasl.username", &sasl.username)
            .set("sasl.password", &sasl.password);
    }
    if let Some(tls) = &target.tls {
        if let Some(value) = &tls.ca_certificate {
            config.set("ssl.ca.pem", value);
        }
        if let Some(value) = &tls.client_certificate {
            config.set("ssl.certificate.pem", value);
        }
        if let Some(value) = &tls.client_key {
            config.set("ssl.key.pem", value);
        }
        if let Some(value) = &tls.client_key_password {
            config.set("ssl.key.password", value);
        }
    }
    let consumer: BaseConsumer = config.create().map_err(|_| ())?;
    consumer
        .fetch_metadata(None, timeout)
        .map(|_| ())
        .map_err(|_| ())
}

fn kafka_security_protocol(value: &str) -> Result<&'static str, ()> {
    match value {
        "PLAINTEXT" => Ok("plaintext"),
        "SSL" => Ok("ssl"),
        "SASL_PLAINTEXT" => Ok("sasl_plaintext"),
        "SASL_SSL" => Ok("sasl_ssl"),
        _ => Err(()),
    }
}

fn kafka_sasl_mechanism(value: &str) -> Result<&'static str, ()> {
    match value {
        "PLAIN" => Ok("PLAIN"),
        "SCRAM_SHA_256" => Ok("SCRAM-SHA-256"),
        "SCRAM_SHA_512" => Ok("SCRAM-SHA-512"),
        _ => Err(()),
    }
}
