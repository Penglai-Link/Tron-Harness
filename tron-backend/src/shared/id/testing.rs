use std::sync::atomic::{AtomicI64, Ordering};

use super::{IdGenerationError, SnowflakeId, SnowflakeIdProvider};

#[derive(Debug)]
pub struct FixedSnowflakeIdProvider {
    next: AtomicI64,
    step: i64,
}

impl FixedSnowflakeIdProvider {
    pub fn new(seed: i64) -> Self {
        Self {
            next: AtomicI64::new(seed),
            step: 1,
        }
    }

    pub fn with_step(seed: i64, step: i64) -> Self {
        Self {
            next: AtomicI64::new(seed),
            step,
        }
    }
}

impl Default for FixedSnowflakeIdProvider {
    fn default() -> Self {
        Self::new(1)
    }
}

impl SnowflakeIdProvider for FixedSnowflakeIdProvider {
    fn next_id(&self) -> Result<SnowflakeId, IdGenerationError> {
        let value = self.next.fetch_add(self.step, Ordering::Relaxed);
        Ok(SnowflakeId::new(value))
    }
}
