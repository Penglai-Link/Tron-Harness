use std::error::Error;
use std::fmt::{self, Display, Formatter};

use snowflaked as _;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SnowflakeId(i64);

impl SnowflakeId {
    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> i64 {
        self.0
    }
}

impl From<i64> for SnowflakeId {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnowflakeIdError {
    NotImplemented,
    GenerationFailed,
}

impl Display for SnowflakeIdError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SnowflakeIdError::NotImplemented => {
                f.write_str("snowflake id generation is not implemented")
            }
            SnowflakeIdError::GenerationFailed => f.write_str("snowflake id generation failed"),
        }
    }
}

impl Error for SnowflakeIdError {}

pub type IdGenerationError = SnowflakeIdError;

pub trait SnowflakeIdProvider: Send + Sync {
    fn next_id(&self) -> Result<SnowflakeId, IdGenerationError>;
}

pub trait IdGenerator: Send + Sync {
    fn next_id(&self) -> Result<SnowflakeId, IdGenerationError>;
}

impl<T> IdGenerator for T
where
    T: SnowflakeIdProvider + ?Sized,
{
    fn next_id(&self) -> Result<SnowflakeId, IdGenerationError> {
        SnowflakeIdProvider::next_id(self)
    }
}

#[derive(Debug, Default, Clone)]
pub struct SnowflakedSnowflakeIdProvider;

impl SnowflakedSnowflakeIdProvider {
    pub fn new() -> Self {
        Self
    }
}

impl SnowflakeIdProvider for SnowflakedSnowflakeIdProvider {
    fn next_id(&self) -> Result<SnowflakeId, IdGenerationError> {
        Err(SnowflakeIdError::NotImplemented)
    }
}
