use tron_backend::shared::id::{SnowflakeIdError, SnowflakeIdProvider, SnowflakedSnowflakeIdProvider};

#[test]
fn snowflaked_adapter_error_mapping_returns_tron_owned_generation_error() {
    let provider = SnowflakedSnowflakeIdProvider::new();
    let generated = provider.next_id();

    assert_eq!(
        generated,
        Err(SnowflakeIdError::GenerationFailed),
        "snowflaked_adapter_error_mapping: expected adapter to map third-party failure into `GenerationFailed`, but mapping is not implemented yet (RED expected)",
    );
}
