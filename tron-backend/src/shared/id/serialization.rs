use std::error::Error;
use std::fmt::{self, Display, Formatter};

use super::SnowflakeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnowflakeIdSerializationPolicy;

impl SnowflakeIdSerializationPolicy {
    pub fn serialize(id: SnowflakeId) -> String {
        id.value().to_string()
    }

    pub fn deserialize(raw: &str) -> Result<SnowflakeId, SnowflakeIdSerializationError> {
        if raw.is_empty() {
            return Err(SnowflakeIdSerializationError::EmptyInput);
        }

        if !raw.as_bytes().iter().all(u8::is_ascii_digit) {
            return Err(SnowflakeIdSerializationError::NonDecimalInput);
        }

        let value = raw
            .parse::<i64>()
            .map_err(|_| SnowflakeIdSerializationError::OutOfRange)?;

        Ok(SnowflakeId::new(value))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnowflakeIdSerializationError {
    EmptyInput,
    NonDecimalInput,
    OutOfRange,
}

impl Display for SnowflakeIdSerializationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SnowflakeIdSerializationError::EmptyInput => {
                f.write_str("snowflake id decimal string cannot be empty")
            }
            SnowflakeIdSerializationError::NonDecimalInput => {
                f.write_str("snowflake id decimal string must contain digits only")
            }
            SnowflakeIdSerializationError::OutOfRange => {
                f.write_str("snowflake id decimal string is out of i64 range")
            }
        }
    }
}

impl Error for SnowflakeIdSerializationError {}
