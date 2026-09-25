use super::SkinError;
use crate::domain::skin::Skin;

pub trait SkinStore: Send + Sync {
    fn list(&self) -> Result<Vec<Skin>, SkinError>;
    fn active(&self) -> Result<Skin, SkinError>;
    fn activate(&self, id: &str) -> Result<Skin, SkinError>;
    fn install(&self, bytes: &[u8]) -> Result<Skin, SkinError>;
    fn uninstall(&self, id: &str) -> Result<(), SkinError>;
    fn resource(&self, id: &str, path: &str) -> Result<Vec<u8>, SkinError>;
}
