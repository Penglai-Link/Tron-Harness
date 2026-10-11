//! Stable ownership for capability package resources.
use std::sync::{Arc, Mutex};
#[derive(Clone, Debug)]
pub struct CapabilityResource {
    pub pack_id: String,
    pub kind: String,
    pub code: String,
    pub available: bool,
}
pub trait CapabilityResourceRepository: Send {
    fn list(&mut self) -> Result<Vec<CapabilityResource>, String>;
    fn register(&mut self, resources: &[CapabilityResource]) -> Result<(), String>;
    fn transition(&mut self, pack: &str, enabled: bool, agents: &[String]) -> Result<(), String>;
    fn remove(&mut self, pack: &str) -> Result<(), String>;
}
#[derive(Clone)]
pub struct CapabilityResources(
    Arc<Mutex<Box<dyn CapabilityResourceRepository>>>,
    Arc<Mutex<()>>,
    Arc<Mutex<std::collections::BTreeSet<String>>>,
);
impl CapabilityResources {
    pub fn new(repo: impl CapabilityResourceRepository + 'static) -> Self {
        Self(
            Arc::new(Mutex::new(Box::new(repo))),
            Arc::new(Mutex::new(())),
            Default::default(),
        )
    }
    pub fn assignment_guard(&self) -> std::sync::MutexGuard<'_, ()> {
        self.1.lock().unwrap()
    }
    pub fn list(&self) -> Result<Vec<CapabilityResource>, String> {
        let mut items = self.0.lock().unwrap().list()?;
        let pending = self.2.lock().unwrap();
        for item in &mut items {
            item.available &= !pending.contains(&item.pack_id);
        }
        Ok(items)
    }
    pub fn register(&self, resources: &[CapabilityResource]) -> Result<(), String> {
        self.0.lock().unwrap().register(resources)
    }
    pub fn transition(&self, pack: &str, enabled: bool, agents: &[String]) -> Result<(), String> {
        let _guard = self.assignment_guard();
        self.2.lock().unwrap().insert(pack.into());
        self.0.lock().unwrap().transition(pack, enabled, agents)?;
        self.2.lock().unwrap().remove(pack);
        Ok(())
    }
    pub fn remove(&self, pack: &str) -> Result<(), String> {
        let _guard = self.assignment_guard();
        self.2.lock().unwrap().insert(pack.into());
        self.0.lock().unwrap().remove(pack)?;
        self.2.lock().unwrap().remove(pack);
        Ok(())
    }
    pub fn resource(&self, kind: &str, code: &str) -> Result<Option<CapabilityResource>, String> {
        Ok(self
            .list()?
            .into_iter()
            .find(|r| r.kind == kind && r.code == code))
    }
    pub fn available(&self, kind: &str, code: &str) -> bool {
        self.resource(kind, code)
            .is_ok_and(|r| r.is_none_or(|r| r.available))
    }
}
