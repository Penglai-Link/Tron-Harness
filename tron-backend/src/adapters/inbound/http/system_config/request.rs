use serde::Deserialize;

use crate::application::system_config::SystemConfigChange;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSystemConfigRequest {
    pub revision: String,
    #[serde(default)]
    pub changes: Vec<SystemConfigChange>,
    #[serde(default)]
    pub confirm_migration: bool,
}
