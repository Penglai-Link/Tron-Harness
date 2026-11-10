use std::collections::BTreeMap;
use crate::application::system_config::{ShortcutBinding, ShortcutDefaultsStore, SystemConfigError};
use super::framework_storage::SharedDatabase;

pub struct SqliteShortcutDefaults(SharedDatabase);

impl SqliteShortcutDefaults {
    pub fn connect(database_url: &str) -> Result<Self, SystemConfigError> {
        let connection = SharedDatabase::open(database_url).map_err(|_| SystemConfigError::ReadFailed)?;
        connection.call(|connection| super::schema::initialize_business(connection).map_err(|_| SystemConfigError::ReadFailed), || SystemConfigError::ReadFailed)?;
        Ok(Self(connection))
    }
}

impl ShortcutDefaultsStore for SqliteShortcutDefaults {
    fn load(&self) -> Result<BTreeMap<String, ShortcutBinding>, SystemConfigError> {
        self.0.call_read(|connection| {
            let mut statement = connection.prepare("SELECT action,key,mod_key,alt_key,shift_key FROM shell_shortcut_defaults").map_err(|_| SystemConfigError::ReadFailed)?;
            statement.query_map([], |row| Ok((row.get(0)?, ShortcutBinding {
                key: row.get(1)?, mod_key: row.get(2)?, alt_key: row.get(3)?, shift_key: row.get(4)?,
            }))).map_err(|_| SystemConfigError::ReadFailed)?.collect::<Result<BTreeMap<_, _>, _>>().map_err(|_| SystemConfigError::ReadFailed)
        }, || SystemConfigError::ReadFailed)
    }
}
