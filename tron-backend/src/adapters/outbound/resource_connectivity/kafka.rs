use std::{io::BufReader, sync::Arc, time::Duration};

use rskafka::client::{ClientBuilder, Credentials, SaslConfig};
use rustls::{ClientConfig, RootCertStore};

use crate::application::resource::KafkaConnectionTarget;

pub(super) async fn verify(target: KafkaConnectionTarget, timeout: Duration) -> Result<(), ()> {
    let brokers = target
        .brokers
        .iter()
        .map(|endpoint| {
            if endpoint.host.contains(':') && !endpoint.host.starts_with('[') {
                format!("[{}]:{}", endpoint.host, endpoint.port)
            } else {
                format!("{}:{}", endpoint.host, endpoint.port)
            }
        })
        .collect::<Vec<_>>();
    if brokers.is_empty() {
        return Err(());
    }

    let mut builder =
        ClientBuilder::new(brokers).client_id("tron-resource-connectivity-verification");
    if target.security_protocol.ends_with("SSL") {
        let tls = target.tls.as_ref().ok_or(())?;
        builder = builder.tls_config(Arc::new(build_tls_config(tls)?));
    }
    if target.security_protocol.starts_with("SASL_") {
        let sasl = target.sasl.as_ref().ok_or(())?;
        let credentials = Credentials::new(sasl.username.clone(), sasl.password.clone());
        builder = builder.sasl_config(match sasl.mechanism.as_str() {
            "PLAIN" => SaslConfig::Plain(credentials),
            "SCRAM_SHA_256" => SaslConfig::ScramSha256(credentials),
            "SCRAM_SHA_512" => SaslConfig::ScramSha512(credentials),
            _ => return Err(()),
        });
    }

    tokio::time::timeout(timeout, builder.build())
        .await
        .map_err(|_| ())?
        .map(|_| ())
        .map_err(|_| ())
}

fn build_tls_config(
    tls: &crate::application::resource::ConnectionTlsMaterial,
) -> Result<ClientConfig, ()> {
    if tls.client_key_password.is_some() {
        return Err(());
    }

    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    if let Some(ca_certificate) = &tls.ca_certificate {
        let certificates = rustls_pemfile::certs(&mut BufReader::new(ca_certificate.as_bytes()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| ())?;
        if certificates.is_empty() {
            return Err(());
        }
        for certificate in certificates {
            roots.add(certificate).map_err(|_| ())?;
        }
    }

    let builder =
        ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .map_err(|_| ())?
            .with_root_certificates(roots);
    match (&tls.client_certificate, &tls.client_key) {
        (Some(certificate_pem), Some(key_pem)) => {
            let certificates =
                rustls_pemfile::certs(&mut BufReader::new(certificate_pem.as_bytes()))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| ())?;
            let private_key = rustls_pemfile::private_key(&mut BufReader::new(key_pem.as_bytes()))
                .map_err(|_| ())?
                .ok_or(())?;
            builder
                .with_client_auth_cert(certificates, private_key)
                .map_err(|_| ())
        }
        (None, None) => Ok(builder.with_no_client_auth()),
        _ => Err(()),
    }
}
