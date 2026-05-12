use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Deserialize;

pub const MAX_INSTANCE_ID: i64 = 1023;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnowflakeConfig {
    instance_id: u16,
    epoch_ms: u64,
}

impl SnowflakeConfig {
    pub fn from_parts(instance_id: u16, epoch_ms: u64) -> Result<Self, SnowflakeConfigError> {
        let instance_id = i64::from(instance_id);
        Self::validate_instance_id(instance_id)?;
        Self::validate_epoch_ms(epoch_ms)?;

        Ok(Self {
            instance_id: instance_id as u16,
            epoch_ms,
        })
    }

    pub fn from_yaml_str(content: &str) -> Result<Self, SnowflakeConfigError> {
        let root: RawRoot =
            serde_yaml::from_str(content).map_err(SnowflakeConfigError::InvalidYaml)?;

        let instance_id =
            root.id
                .snowflake
                .instance_id
                .ok_or(SnowflakeConfigError::MissingField(
                    "id.snowflake.instance_id",
                ))?;
        let epoch_ms = root
            .id
            .snowflake
            .epoch_ms
            .ok_or(SnowflakeConfigError::MissingField("id.snowflake.epoch_ms"))?;

        Self::from_raw(instance_id, epoch_ms)
    }

    pub fn from_yaml_file(path: impl AsRef<Path>) -> Result<Self, SnowflakeConfigError> {
        let path = path.as_ref();
        let content = fs::read_to_string(path).map_err(|source| SnowflakeConfigError::Io {
            path: path.to_path_buf(),
            source,
        })?;

        Self::from_yaml_str(&content)
    }

    pub fn from_raw(instance_id: i64, epoch_ms: u64) -> Result<Self, SnowflakeConfigError> {
        Self::validate_instance_id(instance_id)?;
        Self::validate_epoch_ms(epoch_ms)?;

        Ok(Self {
            instance_id: instance_id as u16,
            epoch_ms,
        })
    }

    pub const fn instance_id(self) -> u16 {
        self.instance_id
    }

    pub const fn epoch_ms(self) -> u64 {
        self.epoch_ms
    }

    pub fn epoch_system_time(self) -> SystemTime {
        UNIX_EPOCH + Duration::from_millis(self.epoch_ms)
    }

    fn validate_instance_id(instance_id: i64) -> Result<(), SnowflakeConfigError> {
        if (0..=MAX_INSTANCE_ID).contains(&instance_id) {
            Ok(())
        } else {
            Err(SnowflakeConfigError::InvalidInstanceId {
                value: instance_id,
                min: 0,
                max: MAX_INSTANCE_ID,
            })
        }
    }

    fn validate_epoch_ms(epoch_ms: u64) -> Result<(), SnowflakeConfigError> {
        let now_ms = now_unix_millis()?;
        if epoch_ms > now_ms {
            return Err(SnowflakeConfigError::EpochInFuture { epoch_ms, now_ms });
        }

        Ok(())
    }
}

impl Default for SnowflakeConfig {
    fn default() -> Self {
        Self {
            instance_id: 0,
            epoch_ms: 0,
        }
    }
}

#[derive(Debug)]
pub enum SnowflakeConfigError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    InvalidYaml(serde_yaml::Error),
    MissingField(&'static str),
    InvalidInstanceId {
        value: i64,
        min: i64,
        max: i64,
    },
    EpochInFuture {
        epoch_ms: u64,
        now_ms: u64,
    },
    ClockUnavailable(std::time::SystemTimeError),
}

impl Display for SnowflakeConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SnowflakeConfigError::Io { path, .. } => {
                write!(
                    f,
                    "failed to read snowflake config file `{}`",
                    path.display()
                )
            }
            SnowflakeConfigError::InvalidYaml(_) => f.write_str("invalid snowflake yaml content"),
            SnowflakeConfigError::MissingField(field) => {
                write!(f, "missing required snowflake config field `{field}`")
            }
            SnowflakeConfigError::InvalidInstanceId { value, min, max } => write!(
                f,
                "invalid snowflake instance_id `{value}`; expected range {min}..={max}"
            ),
            SnowflakeConfigError::EpochInFuture { epoch_ms, now_ms } => write!(
                f,
                "invalid snowflake epoch_ms `{epoch_ms}`; must not be in the future relative to `{now_ms}`"
            ),
            SnowflakeConfigError::ClockUnavailable(_) => {
                f.write_str("system clock unavailable for snowflake epoch validation")
            }
        }
    }
}

impl Error for SnowflakeConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SnowflakeConfigError::Io { source, .. } => Some(source),
            SnowflakeConfigError::InvalidYaml(source) => Some(source),
            SnowflakeConfigError::ClockUnavailable(source) => Some(source),
            SnowflakeConfigError::MissingField(_)
            | SnowflakeConfigError::InvalidInstanceId { .. }
            | SnowflakeConfigError::EpochInFuture { .. } => None,
        }
    }
}

#[derive(Debug, Deserialize)]
struct RawRoot {
    id: RawId,
}

#[derive(Debug, Deserialize)]
struct RawId {
    snowflake: RawSnowflake,
}

#[derive(Debug, Deserialize)]
struct RawSnowflake {
    instance_id: Option<i64>,
    epoch_ms: Option<u64>,
}

fn now_unix_millis() -> Result<u64, SnowflakeConfigError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(SnowflakeConfigError::ClockUnavailable)?;
    Ok(duration.as_millis() as u64)
}
