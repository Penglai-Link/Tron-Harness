use std::sync::Arc;

use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct SetupStateService {
    store: Arc<dyn SetupStateStore>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupState {
    pub initialized: bool,
    pub initialized_at: Option<String>,
    pub initialized_by: Option<String>,
}

#[derive(Debug)]
pub enum SetupStateError {
    ReadFailed,
    WriteFailed,
}

pub trait SetupStateStore: Send + Sync {
    fn read(&self) -> Result<SetupState, SetupStateError>;
    fn write(&self, state: &SetupState) -> Result<(), SetupStateError>;
}

impl SetupStateService {
    pub fn new(store: Arc<dyn SetupStateStore>) -> Self {
        Self { store }
    }

    pub fn get(&self) -> Result<SetupState, SetupStateError> {
        self.store.read()
    }

    pub fn complete(&self, user_code: &str) -> Result<SetupState, SetupStateError> {
        let state = SetupState {
            initialized: true,
            initialized_at: Some(chrono::Utc::now().to_rfc3339()),
            initialized_by: Some(user_code.to_owned()),
        };
        self.store.write(&state)?;
        Ok(state)
    }
}
