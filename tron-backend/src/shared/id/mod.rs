use std::error::Error;
use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

mod config;
mod persistence_policy;
mod serialization;
mod snowflake;
mod testing;

pub use config::{MAX_INSTANCE_ID, SnowflakeConfig, SnowflakeConfigError};
pub use persistence_policy::{
    IdPersistenceConstraintError, SQLITE_INTEGER_TYPE, SnowflakeIdPersistencePolicy,
};
pub use serialization::{SnowflakeIdSerializationError, SnowflakeIdSerializationPolicy};
pub use snowflake::SnowflakedSnowflakeIdProvider;
pub use testing::FixedSnowflakeIdProvider;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SnowflakeId(i64);

impl SnowflakeId {
    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> i64 {
        self.0
    }

    pub fn as_decimal_string(self) -> String {
        SnowflakeIdSerializationPolicy::serialize(self)
    }

    pub fn from_decimal_string(value: &str) -> Result<Self, SnowflakeIdSerializationError> {
        SnowflakeIdSerializationPolicy::deserialize(value)
    }
}

impl From<i64> for SnowflakeId {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

impl Serialize for SnowflakeId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.as_decimal_string())
    }
}

impl<'de> Deserialize<'de> for SnowflakeId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        SnowflakeId::from_decimal_string(&raw).map_err(serde::de::Error::custom)
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
