use std::panic::{AssertUnwindSafe, catch_unwind};

use snowflaked::{Builder as SnowflakedBuilder, sync::Generator as SnowflakedGenerator};

use super::{
    IdGenerationError, SnowflakeConfig, SnowflakeId, SnowflakeIdError, SnowflakeIdProvider,
};

#[derive(Debug)]
pub struct SnowflakedSnowflakeIdProvider {
    generator: SnowflakedGenerator,
}

impl SnowflakedSnowflakeIdProvider {
    pub fn new() -> Self {
        Self::with_config(SnowflakeConfig::default())
    }

    pub fn with_config(config: SnowflakeConfig) -> Self {
        let generator: SnowflakedGenerator = SnowflakedBuilder::new()
            .instance(config.instance_id())
            .epoch(config.epoch_system_time())
            .build();

        Self { generator }
    }

    fn generate(&self) -> Result<SnowflakeId, IdGenerationError> {
        let generated = catch_unwind(AssertUnwindSafe(|| self.generator.generate::<i64>()));
        generated
            .map(SnowflakeId::new)
            .map_err(|_| SnowflakeIdError::GenerationFailed)
    }
}

impl Default for SnowflakedSnowflakeIdProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl SnowflakeIdProvider for SnowflakedSnowflakeIdProvider {
    fn next_id(&self) -> Result<SnowflakeId, IdGenerationError> {
        self.generate()
    }
}
