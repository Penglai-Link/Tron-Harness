use tron_backend::shared::id::{IdGenerationError, IdGenerator, SnowflakedSnowflakeIdProvider};

struct ConsumerIdService<'a> {
    generator: &'a dyn IdGenerator,
}

impl<'a> ConsumerIdService<'a> {
    fn new(generator: &'a dyn IdGenerator) -> Self {
        Self { generator }
    }

    fn allocate_internal_id(&self) -> Result<i64, IdGenerationError> {
        self.generator.next_id().map(|id| id.value())
    }
}

#[test]
fn common_snowflake_consumer_boundary_allocates_id_via_shared_trait_only() {
    let provider = SnowflakedSnowflakeIdProvider::new();
    let service = ConsumerIdService::new(&provider);
    let generated = service.allocate_internal_id();

    assert!(
        generated.is_ok(),
        "common_snowflake_consumer_boundary: consumer can compile against shared trait, but runtime generation is still missing: {generated:?}",
    );
}
