use super::{SkinError, SkinStore};
use crate::domain::skin::Skin;
use std::sync::Arc;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct SkinService {
    store: Arc<dyn SkinStore>,
    changes: broadcast::Sender<()>,
}

impl SkinService {
    pub fn new(store: Arc<dyn SkinStore>) -> Self {
        Self {
            store,
            changes: broadcast::channel(16).0,
        }
    }
    pub fn list(&self) -> Result<Vec<Skin>, SkinError> {
        self.store.list()
    }
    pub fn active(&self) -> Result<Skin, SkinError> {
        self.store.active()
    }
    pub fn activate(&self, id: &str) -> Result<Skin, SkinError> {
        let skin = self.store.activate(id)?;
        let _ = self.changes.send(());
        Ok(skin)
    }
    pub fn install(&self, bytes: &[u8]) -> Result<Skin, SkinError> {
        let skin = self.store.install(bytes)?;
        let _ = self.changes.send(());
        Ok(skin)
    }
    pub fn uninstall(&self, id: &str) -> Result<(), SkinError> {
        self.store.uninstall(id)?;
        let _ = self.changes.send(());
        Ok(())
    }
    pub fn install_market(&self, bytes: &[u8], market_id: &str) -> Result<Skin, SkinError> {
        let skin = self.store.install_market(bytes, market_id)?;
        let _ = self.changes.send(());
        Ok(skin)
    }
    pub fn resource(&self, id: &str, path: &str) -> Result<Vec<u8>, SkinError> {
        self.store.resource(id, path)
    }
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.changes.subscribe()
    }
}
