use crate::{application::skill::SkillRegistryService, domain::dude_memory::*};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

pub trait DudeMemoryRepository: Send {
    fn list_batches(&mut self, user: &str, dude: &str)
    -> Result<Vec<MemoryBatch>, DudeMemoryError>;
    fn get_batch(&mut self, code: &str) -> Result<MemoryBatch, DudeMemoryError>;
    fn cursor_versions(
        &mut self,
        user: &str,
        dude: &str,
    ) -> Result<BTreeMap<String, String>, DudeMemoryError>;
    fn save_batch(
        &mut self,
        previous: Option<&MemoryBatch>,
        next: &MemoryBatch,
        commit_cursors: bool,
    ) -> Result<(), DudeMemoryError>;
    fn list_candidates(&mut self, user: &str) -> Result<Vec<SkillCandidate>, DudeMemoryError>;
    fn get_candidate(&mut self, code: &str) -> Result<SkillCandidate, DudeMemoryError>;
    fn save_candidate(
        &mut self,
        previous: Option<&SkillCandidate>,
        next: &SkillCandidate,
    ) -> Result<(), DudeMemoryError>;
}
#[derive(Debug, Clone)]
pub struct MemoryNotice {
    pub user_code: String,
    pub dude_code: String,
    pub related_code: String,
    pub revision: u32,
    pub kind: &'static str,
    pub message: String,
}
pub trait DudeMemoryNotifications: Send + Sync {
    fn publish(&self, notice: MemoryNotice) -> Result<(), DudeMemoryError>;
}
#[derive(Clone)]
pub struct DudeMemoryService {
    pub(super) repository: Arc<Mutex<Box<dyn DudeMemoryRepository>>>,
    pub(super) skills: SkillRegistryService,
    pub(super) staging_root: PathBuf,
    pub(super) notifications: Arc<std::sync::OnceLock<Arc<dyn DudeMemoryNotifications>>>,
    pub(super) installation_lock: Arc<Mutex<()>>,
}
impl DudeMemoryService {
    pub fn new(
        repository: impl DudeMemoryRepository + 'static,
        skills: SkillRegistryService,
        staging_root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
            skills,
            staging_root: staging_root.into(),
            notifications: Default::default(),
            installation_lock: Default::default(),
        }
    }
    pub fn set_notifications(&self, notifications: Arc<dyn DudeMemoryNotifications>) {
        let _ = self.notifications.set(notifications);
    }
    pub(super) fn notify(
        &self,
        candidate: &SkillCandidate,
        kind: &'static str,
        message: String,
    ) -> Result<(), DudeMemoryError> {
        if let Some(notifications) = self.notifications.get() {
            notifications.publish(MemoryNotice {
                user_code: candidate.user_code.clone(),
                dude_code: candidate.dude_code.clone(),
                related_code: candidate.candidate_code.clone(),
                revision: candidate.revision,
                kind,
                message,
            })?;
        }
        Ok(())
    }
    pub fn list_candidates(&self, user: &str) -> Result<Vec<SkillCandidate>, DudeMemoryError> {
        self.repository.lock().unwrap().list_candidates(user)
    }
    pub fn get_candidate(&self, user: &str, code: &str) -> Result<SkillCandidate, DudeMemoryError> {
        let candidate = self.repository.lock().unwrap().get_candidate(code)?;
        if candidate.user_code != user {
            return Err(DudeMemoryError::NotFound);
        }
        Ok(candidate)
    }
}
pub(super) fn now() -> i64 {
    chrono::Utc::now().timestamp_millis()
}
pub(super) fn new_code(prefix: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    format!(
        "{prefix}-{}-{}-{:x}",
        now(),
        NEXT.fetch_add(1, Ordering::Relaxed),
        rand::random::<u64>()
    )
}

pub struct CallbackDudeMemoryNotifications<
    F: Fn(MemoryNotice) -> Result<(), DudeMemoryError> + Send + Sync,
>(pub F);
impl<F: Fn(MemoryNotice) -> Result<(), DudeMemoryError> + Send + Sync> DudeMemoryNotifications
    for CallbackDudeMemoryNotifications<F>
{
    fn publish(&self, notice: MemoryNotice) -> Result<(), DudeMemoryError> {
        (self.0)(notice)
    }
}
