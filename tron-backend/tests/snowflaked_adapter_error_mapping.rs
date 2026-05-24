use tron_backend::shared::id::{
    SnowflakeIdError, SnowflakeIdProvider, SnowflakedSnowflakeIdProvider,
};

#[test]
fn snowflaked_adapter_error_mapping_returns_tron_owned_generation_error() {
    let provider = SnowflakedSnowflakeIdProvider::new();
    let generated = provider.next_id();

    assert_ne!(
        generated,
        Err(SnowflakeIdError::NotImplemented),
        "snowflaked_adapter_error_mapping: adapter must not leak placeholder `NotImplemented` behavior",
    );

    if let Err(error) = generated {
        assert_eq!(
            error,
            SnowflakeIdError::GenerationFailed,
            "snowflaked_adapter_error_mapping: non-success adapter outcome must map to `GenerationFailed`",
        );
    }
}
