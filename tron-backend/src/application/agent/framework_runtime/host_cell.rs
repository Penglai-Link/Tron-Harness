//! Replaceable lazy runtime cache. Lifecycle callers hold the host admission lock.
use std::{future::Future, sync::RwLock};
pub(super) struct RuntimeCell<T> {
    value: RwLock<Option<T>>,
    init: tokio::sync::Mutex<()>,
}
impl<T: Clone> RuntimeCell<T> {
    pub fn new() -> Self {
        Self {
            value: RwLock::new(None),
            init: tokio::sync::Mutex::new(()),
        }
    }
    pub fn get(&self) -> Option<T> {
        self.value.read().unwrap().clone()
    }
    #[cfg(test)]
    pub fn set(&self, value: T) -> Result<(), T> {
        let mut slot = self.value.write().unwrap();
        if slot.is_some() {
            Err(value)
        } else {
            *slot = Some(value);
            Ok(())
        }
    }
    pub fn take(&self) -> Option<T> {
        self.value.write().unwrap().take()
    }
    pub async fn get_or_try_init<E, F, Fut>(&self, init: F) -> Result<T, E>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
    {
        if let Some(value) = self.get() {
            return Ok(value);
        }
        let _guard = self.init.lock().await;
        if let Some(value) = self.get() {
            return Ok(value);
        }
        let value = init().await?;
        *self.value.write().unwrap() = Some(value.clone());
        Ok(value)
    }
}
