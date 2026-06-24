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

#[cfg(test)]
mod tests {
    use std::{
        panic::{AssertUnwindSafe, catch_unwind},
        time::{Duration, SystemTime},
    };

    use snowflaked::Snowflake;

    use super::*;

    #[test]
    fn local_clock_generates_monotonic_snowflake_ids() {
        let provider = SnowflakedSnowflakeIdProvider::default();
        let mut previous = provider
            .next_id()
            .expect("first snowflake id should generate");

        for _ in 0..10_000 {
            let current = provider
                .next_id()
                .expect("snowflake id should generate on local clock");
            assert!(
                current > previous,
                "snowflake ids should be strictly increasing: previous={}, current={}",
                previous.value(),
                current.value()
            );
            previous = current;
        }
    }

    #[test]
    fn raw_snowflaked_panics_when_epoch_is_in_the_future() {
        let generator: SnowflakedGenerator = SnowflakedBuilder::new()
            .instance(0)
            .epoch(SystemTime::now() + Duration::from_secs(1))
            .build();

        let generated = catch_unwind(AssertUnwindSafe(|| generator.generate::<i64>()));

        assert!(
            generated.is_err(),
            "raw snowflaked should panic when its clock source appears to move backwards"
        );
    }

    #[test]
    fn generated_id_timestamp_tracks_current_unix_millis() {
        let before = unix_millis();
        let id = SnowflakedSnowflakeIdProvider::default()
            .next_id()
            .expect("snowflake id should generate");
        let after = unix_millis();

        let timestamp = id.value().timestamp();
        assert!(
            (before..=after).contains(&timestamp),
            "snowflake timestamp should be within local wall-clock bounds: before={before}, timestamp={timestamp}, after={after}"
        );
    }

    fn unix_millis() -> u64 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("local clock should be after unix epoch")
            .as_millis() as u64
    }
}
