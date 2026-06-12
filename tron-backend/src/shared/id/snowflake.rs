use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    thread,
    time::Duration,
};

use snowflaked::{Builder as SnowflakedBuilder, sync::Generator as SnowflakedGenerator};

use super::{
    IdGenerationError, SnowflakeConfig, SnowflakeId, SnowflakeIdError, SnowflakeIdProvider,
};

const MAX_GENERATION_ATTEMPTS: usize = 4;
const CLOCK_DRIFT_RETRY_DELAY: Duration = Duration::from_millis(2);

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

    fn generate_once(&self) -> Result<SnowflakeId, IdGenerationError> {
        let generated = catch_unwind(AssertUnwindSafe(|| self.generator.generate::<i64>()));
        generated
            .map(SnowflakeId::new)
            .map_err(|_| SnowflakeIdError::GenerationFailed)
    }

    fn generate(&self) -> Result<SnowflakeId, IdGenerationError> {
        for attempt in 1..=MAX_GENERATION_ATTEMPTS {
            match self.generate_once() {
                Ok(id) => return Ok(id),
                Err(error) if attempt == MAX_GENERATION_ATTEMPTS => return Err(error),
                Err(_) => thread::sleep(CLOCK_DRIFT_RETRY_DELAY),
            }
        }

        Err(SnowflakeIdError::GenerationFailed)
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
