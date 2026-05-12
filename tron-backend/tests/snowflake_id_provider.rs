use tron_backend::shared::id::{
    IdGenerator, SnowflakeIdError, SnowflakeIdProvider, SnowflakedSnowflakeIdProvider,
};

#[test]
fn snowflake_id_provider_generates_tron_owned_i64_ids() {
    let provider = SnowflakedSnowflakeIdProvider::new();
    let generated = SnowflakeIdProvider::next_id(&provider);

    assert!(
        generated.is_ok(),
        "snowflake_id_provider: expected adapter-backed provider to generate SnowflakeId, but production capability is missing: {generated:?}",
    );
}

#[test]
fn snowflake_id_provider_trait_object_keeps_consumer_boundary() {
    let provider = SnowflakedSnowflakeIdProvider::new();
    let generator: &dyn IdGenerator = &provider;
    let generated = generator.next_id();

    assert!(
        generated.is_ok(),
        "snowflake_id_provider: expected shared IdGenerator boundary to produce IDs, but implementation is not ready: {generated:?}",
    );
}

#[test]
fn snowflake_id_provider_maps_adapter_failure_to_runtime_error() {
    let provider = SnowflakedSnowflakeIdProvider::new();
    let generated = SnowflakeIdProvider::next_id(&provider);

    assert_ne!(
        generated,
        Err(SnowflakeIdError::NotImplemented),
        "snowflake_id_provider: RED expected; provider still returns NotImplemented instead of production adapter behavior",
    );
}
