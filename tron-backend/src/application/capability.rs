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
struct ResourceState {
    repository: Box<dyn CapabilityResourceRepository>,
    resources: Option<std::collections::BTreeMap<(String, String), CapabilityResource>>,
    revision: u64,
}
impl ResourceState {
    fn load(&mut self) -> Result<(), String> {
        if self.resources.is_none() {
            self.resources = Some(self.repository.list()?.into_iter().map(|r| ((r.kind.clone(), r.code.clone()), r)).collect());
        }
        Ok(())
    }
    fn invalidate(&mut self) {
        self.resources = None;
        self.revision = self.revision.wrapping_add(1);
    }
}
#[derive(Clone)]
pub struct CapabilityResources(
    Arc<Mutex<ResourceState>>,
    Arc<Mutex<()>>,
    Arc<Mutex<std::collections::BTreeSet<String>>>,
);
impl CapabilityResources {
    pub fn new(repo: impl CapabilityResourceRepository + 'static) -> Self {
        Self(
            Arc::new(Mutex::new(ResourceState { repository: Box::new(repo), resources: None, revision: 0 })),
            Arc::new(Mutex::new(())),
            Default::default(),
        )
    }
    pub fn assignment_guard(&self) -> std::sync::MutexGuard<'_, ()> {
        self.1.lock().unwrap()
    }
    pub fn list(&self) -> Result<Vec<CapabilityResource>, String> {
        let mut state = self.0.lock().unwrap();
        state.load()?;
        let mut items: Vec<_> = state.resources.as_ref().unwrap().values().cloned().collect();
        let pending = self.2.lock().unwrap();
        for item in &mut items {
            item.available &= !pending.contains(&item.pack_id);
        }
        Ok(items)
    }
    pub fn register(&self, resources: &[CapabilityResource]) -> Result<(), String> {
        let mut state = self.0.lock().unwrap();
        state.invalidate();
        state.repository.register(resources)
    }
    pub fn transition(&self, pack: &str, enabled: bool, agents: &[String]) -> Result<(), String> {
        let _guard = self.assignment_guard();
        self.2.lock().unwrap().insert(pack.into());
        let mut state = self.0.lock().unwrap();
        state.invalidate();
        state.repository.transition(pack, enabled, agents)?;
        self.2.lock().unwrap().remove(pack);
        Ok(())
    }
    pub fn remove(&self, pack: &str) -> Result<(), String> {
        let _guard = self.assignment_guard();
        self.2.lock().unwrap().insert(pack.into());
        let mut state = self.0.lock().unwrap();
        state.invalidate();
        state.repository.remove(pack)?;
        self.2.lock().unwrap().remove(pack);
        Ok(())
    }
    pub fn resource(&self, kind: &str, code: &str) -> Result<Option<CapabilityResource>, String> {
        let mut state = self.0.lock().unwrap();
        state.load()?;
        let mut resource = state.resources.as_ref().unwrap().get(&(kind.to_owned(), code.to_owned())).cloned();
        if let Some(resource) = &mut resource {
            resource.available &= !self.2.lock().unwrap().contains(&resource.pack_id);
        }
        Ok(resource)
    }
    pub(crate) fn revision(&self) -> u64 {
        self.0.lock().unwrap().revision
    }

    pub fn available(&self, kind: &str, code: &str) -> bool {
        self.resource(kind, code)
            .is_ok_and(|r| r.is_none_or(|r| r.available))
    }
}
