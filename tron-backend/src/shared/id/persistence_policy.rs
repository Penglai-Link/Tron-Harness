use std::error::Error;
use std::fmt::{self, Display, Formatter};

pub const SQLITE_INTEGER_TYPE: &str = "INTEGER";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnowflakeIdPersistencePolicy;

impl SnowflakeIdPersistencePolicy {
    pub const fn sqlite_column_type() -> &'static str {
        SQLITE_INTEGER_TYPE
    }

    pub const fn requires_decimal_string_json() -> bool {
        true
    }

    pub fn validate_internal_id_value(value: i64) -> Result<(), IdPersistenceConstraintError> {
        if value >= 0 {
            Ok(())
        } else {
            Err(IdPersistenceConstraintError::NegativeInternalId(value))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdPersistenceConstraintError {
    NegativeInternalId(i64),
}

impl Display for IdPersistenceConstraintError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            IdPersistenceConstraintError::NegativeInternalId(value) => {
                write!(
                    f,
                    "snowflake internal id must be non-negative, got `{value}`"
                )
            }
        }
    }
}

impl Error for IdPersistenceConstraintError {}
