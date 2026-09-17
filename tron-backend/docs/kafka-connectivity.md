# Kafka connectivity

Tron verifies Kafka data source connections with [RSKafka](https://github.com/influxdata/rskafka), a native Rust Kafka client. TLS uses Rustls. The backend does not build or link librdkafka or OpenSSL for Kafka, so Kafka support does not require Strawberry Perl or a platform-installed Kafka C library.

Supported security protocols are `PLAINTEXT`, `SSL`, `SASL_PLAINTEXT`, and `SASL_SSL`. SASL supports `PLAIN`, `SCRAM_SHA_256`, and `SCRAM_SHA_512`. SSL connections use the platform-independent WebPKI root set and may include additional PEM CA certificates. Mutual TLS accepts a PEM client certificate and an **unencrypted PEM private key**; encrypted private keys are not supported by the Rustls PEM loader.

Connection verification requests broker metadata and is bounded by the resource connectivity timeout. Kafka-specific librdkafka `properties` are not accepted: connection options are represented by the explicit broker, SASL, and TLS fields in the data source configuration.

Build and check through the repository Makefile:

```sh
make be-check
make dev-api
make desktop-build
```
