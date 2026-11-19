use super::FrameworkHost;
use crate::application::assistant::NotificationInput;
use crate::application::capability::{
    CAPABILITY_QUEUE_ERROR_LIMIT, CAPABILITY_QUEUE_NAME_LIMIT, CAPABILITY_UNINSTALL_BUNDLED,
    CAPABILITY_UNINSTALL_BUSY, CAPABILITY_UNINSTALL_INCOMPLETE, CAPABILITY_UNINSTALL_REFERENCED,
    CapabilityEnqueueRecord, CapabilityOperationRecord, CapabilityQueueItem,
    CapabilityQueueOperation, CapabilityQueueOutcome, CapabilityQueueStatus,
};

const BUSY_MESSAGES: [&str; 3] = [
    "internal agent is busy",
    "native continuation is busy",
    "external runtime is busy",
];

#[derive(Debug)]
pub enum CapabilityQueueEnqueueError {
    Full,
    Unavailable(String),
}

#[derive(Debug, Clone)]
pub struct CapabilityQueueSnapshot {
    pub agent_busy: bool,
    pub items: Vec<CapabilityQueueItem>,
    pub log: Vec<CapabilityOperationRecord>,
}

struct QueueFailure {
    message: String,
    code: Option<String>,
}

/// Clears the in-memory "running" marker on every exit path, including the
/// transient-busy return that leaves the item queued.
struct RunningGuard<'a>(&'a std::sync::Mutex<Option<String>>);

impl Drop for RunningGuard<'_> {
    fn drop(&mut self) {
        if let Ok(mut running) = self.0.lock() {
            *running = None;
        }
    }
}

fn is_busy_message(error: &str) -> bool {
    BUSY_MESSAGES.contains(&error)
}

impl FrameworkHost {
    pub(crate) fn bind_capability_queue(
        &self,
        service: crate::application::capability::CapabilityQueueService,
    ) -> Result<(), String> {
        self.0
            .capability_queue
            .set(service)
            .map_err(|_| "capability queue is already bound".to_string())
    }

    fn capability_queue(
        &self,
    ) -> Result<&crate::application::capability::CapabilityQueueService, String> {
        self.0
            .capability_queue
            .get()
            .ok_or_else(|| "capability queue is unavailable".to_string())
    }

    pub fn external_idle(&self) -> bool {
        self.ensure_external_idle().is_ok()
    }

    pub fn has_pending_capability_operations(&self) -> Result<bool, String> {
        self.capability_queue()?.has_items()
    }

    /// Persist one operation intent. Identical pending operations are returned
    /// as-is, so a double click cannot create duplicate work; a forced uninstall
    /// upgrades a waiting non-forced uninstall.
    pub fn enqueue_capability_operation(
        &self,
        pack_id: &str,
        pack_name: Option<&str>,
        operation: CapabilityQueueOperation,
        force: bool,
        locale: &str,
    ) -> Result<(CapabilityQueueItem, bool), CapabilityQueueEnqueueError> {
        let service = self
            .capability_queue()
            .map_err(CapabilityQueueEnqueueError::Unavailable)?;
        let pack_name = pack_name
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(|name| {
                name.chars()
                    .take(CAPABILITY_QUEUE_NAME_LIMIT)
                    .collect::<String>()
            })
            .unwrap_or_else(|| pack_id.to_owned());
        let item = CapabilityQueueItem {
            id: eliteforge_agent_core::SessionRecord::new("").id,
            pack_id: pack_id.to_owned(),
            pack_name,
            operation,
            force: operation == CapabilityQueueOperation::Uninstall && force,
            locale: locale.to_owned(),
            created_at: chrono::Utc::now().timestamp_millis(),
            status: CapabilityQueueStatus::Queued,
        };
        match service
            .enqueue(item)
            .map_err(CapabilityQueueEnqueueError::Unavailable)?
        {
            CapabilityEnqueueRecord::Queued { item, was_empty } => Ok((item, was_empty)),
            CapabilityEnqueueRecord::Existing(item) => Ok((item, false)),
            CapabilityEnqueueRecord::Full => Err(CapabilityQueueEnqueueError::Full),
        }
    }

    /// Execute waiting operations in order until the queue is empty, the remote
    /// idle gate closes, or `limit` operations have completed. A real failure is
    /// recorded and removed; later items still run.
    pub async fn process_capability_queue(&self, limit: usize) -> Result<(), String> {
        let _serial = self.0.capability_queue_lock.lock().await;
        let Some(service) = self.0.capability_queue.get() else {
            return Ok(());
        };
        for _ in 0..limit {
            let Some(item) = service.items()?.into_iter().next() else {
                return Ok(());
            };
            // Reconfiguring while a run is active is rejected by the operations
            // below. Staying queued is not a failure and must not consume the log.
            if self.ensure_external_idle().is_err() {
                return Ok(());
            }
            if let Some(failure) = self.capability_queue_preflight(&item).await {
                let record = self.capability_queue_record(
                    &item,
                    false,
                    Some(failure.message.clone()),
                    failure.code,
                );
                service.complete(&item.id, &record)?;
                tracing::warn!(
                    pack = %item.pack_id,
                    operation = item.operation.storage_value(),
                    error = %failure.message,
                    "queued capability operation skipped"
                );
                continue;
            }
            let outcome = {
                if let Ok(mut running) = self.0.capability_queue_running.lock() {
                    *running = Some(item.id.clone());
                }
                let _guard = RunningGuard(&self.0.capability_queue_running);
                match item.operation {
                    CapabilityQueueOperation::Uninstall => {
                        self.uninstall_pack(&item.pack_id, item.force).await
                    }
                    CapabilityQueueOperation::Enable => {
                        self.set_pack_enabled(&item.pack_id, true).await
                    }
                    CapabilityQueueOperation::Disable => {
                        self.set_pack_enabled(&item.pack_id, false).await
                    }
                }
            };
            match outcome {
                Ok(()) => {
                    let record = self.capability_queue_record(&item, true, None, None);
                    service.complete(&item.id, &record)?;
                    self.notify_capability_queue_result(&item);
                    tracing::info!(
                        pack = %item.pack_id,
                        operation = item.operation.storage_value(),
                        "queued capability operation completed"
                    );
                }
                Err(error) if is_busy_message(&error) => {
                    // Another run started between the idle check and execution.
                    // Keep the head item queued and retry on the next pass.
                    return Ok(());
                }
                Err(error) => {
                    let (message, code) = self.capability_queue_failure(&item, &error);
                    let record =
                        self.capability_queue_record(&item, false, Some(message.clone()), code);
                    service.complete(&item.id, &record)?;
                    tracing::warn!(
                        pack = %item.pack_id,
                        operation = item.operation.storage_value(),
                        error = %message,
                        "queued capability operation failed"
                    );
                }
            }
        }
        Ok(())
    }

    pub fn capability_queue_snapshot(&self) -> Result<CapabilityQueueSnapshot, String> {
        let service = self.capability_queue()?;
        let agent_busy = !self.external_idle();
        let mut items = service.items()?;
        if let Some(running) = self
            .0
            .capability_queue_running
            .lock()
            .ok()
            .and_then(|running| running.clone())
        {
            for item in &mut items {
                if item.id == running {
                    item.status = CapabilityQueueStatus::Running;
                }
            }
        }
        Ok(CapabilityQueueSnapshot {
            agent_busy,
            items,
            log: service.log()?,
        })
    }

    pub fn capability_queue_outcome(
        &self,
        item_id: &str,
    ) -> Result<CapabilityQueueOutcome, String> {
        self.capability_queue()?.outcome(item_id)
    }

    /// `None` means the operation can still run. A failure is written to the
    /// operation log and the next queued item continues.
    async fn capability_queue_preflight(&self, item: &CapabilityQueueItem) -> Option<QueueFailure> {
        let is_builtin = crate::application::agent::capability_pack_catalog()
            .iter()
            .any(|pack| pack.id == item.pack_id);
        if is_builtin {
            if item.operation == CapabilityQueueOperation::Uninstall {
                return Some(QueueFailure {
                    message: tron_i18n::t!(
                        "capabilities.uninstall_bundled",
                        locale = item.locale.as_str()
                    )
                    .into_owned(),
                    code: Some(CAPABILITY_UNINSTALL_BUNDLED.into()),
                });
            }
            return None;
        }
        let root = self.0.pack_root.clone();
        let pack_id = item.pack_id.clone();
        let locale = item.locale.clone();
        let installed = tokio::task::spawn_blocking(move || {
            crate::application::agent::query_installed_capability_packs(
                &root,
                &locale,
                false,
                Some(&pack_id),
            )
            .map(|packs| !packs.is_empty())
        })
        .await;
        let failure_code = (item.operation == CapabilityQueueOperation::Uninstall)
            .then(|| CAPABILITY_UNINSTALL_INCOMPLETE.to_string());
        match installed {
            Ok(Ok(true)) => None,
            Ok(Ok(false)) => Some(QueueFailure {
                message: tron_i18n::t!(
                    "capabilities.queue.pack_missing",
                    locale = item.locale.as_str(),
                    name = item.pack_name.as_str()
                ),
                code: failure_code,
            }),
            Ok(Err(error)) => Some(QueueFailure {
                message: error,
                code: failure_code,
            }),
            Err(error) => Some(QueueFailure {
                message: error.to_string(),
                code: failure_code,
            }),
        }
    }

    /// Maps a runtime failure to the log's localized message and stable code.
    /// Uninstall failures keep the direct endpoint's codes so the UI can offer
    /// the existing force confirmation after an immediate attempt.
    fn capability_queue_failure(
        &self,
        item: &CapabilityQueueItem,
        error: &str,
    ) -> (String, Option<String>) {
        if item.operation != CapabilityQueueOperation::Uninstall {
            return (error.to_owned(), None);
        }
        let locale = item.locale.as_str();
        if error.contains("bundled") {
            (
                tron_i18n::t!("capabilities.uninstall_bundled", locale = locale).into_owned(),
                Some(CAPABILITY_UNINSTALL_BUNDLED.into()),
            )
        } else if error.contains("referenced by a durable run") {
            (
                tron_i18n::t!("capabilities.uninstall_referenced", locale = locale).into_owned(),
                Some(CAPABILITY_UNINSTALL_REFERENCED.into()),
            )
        } else if error.contains("Busy") || error.contains("busy") || error.contains("active") {
            (
                tron_i18n::t!("capabilities.uninstall_busy", locale = locale).into_owned(),
                Some(CAPABILITY_UNINSTALL_BUSY.into()),
            )
        } else {
            (
                format!(
                    "{}: {error}",
                    tron_i18n::t!("capabilities.uninstall_incomplete", locale = locale)
                ),
                Some(CAPABILITY_UNINSTALL_INCOMPLETE.into()),
            )
        }
    }

    fn capability_queue_record(
        &self,
        item: &CapabilityQueueItem,
        succeeded: bool,
        error: Option<String>,
        error_code: Option<String>,
    ) -> CapabilityOperationRecord {
        CapabilityOperationRecord {
            item_id: item.id.clone(),
            pack_id: item.pack_id.clone(),
            pack_name: item.pack_name.clone(),
            operation: item.operation,
            force: item.force,
            succeeded,
            error: error.map(|error| error.chars().take(CAPABILITY_QUEUE_ERROR_LIMIT).collect()),
            error_code,
            finished_at: chrono::Utc::now().timestamp_millis(),
        }
    }

    fn notify_capability_queue_result(&self, item: &CapabilityQueueItem) {
        let Some(assistants) = self.assistants() else {
            return;
        };
        let locale = item.locale.as_str();
        let (title, body) = match item.operation {
            CapabilityQueueOperation::Enable => (
                tron_i18n::t!("capabilities.queue.enabled_title", locale = locale).into_owned(),
                tron_i18n::t!(
                    "capabilities.queue.enabled_body",
                    locale = locale,
                    name = item.pack_name.as_str()
                ),
            ),
            CapabilityQueueOperation::Disable => (
                tron_i18n::t!("capabilities.queue.disabled_title", locale = locale).into_owned(),
                tron_i18n::t!(
                    "capabilities.queue.disabled_body",
                    locale = locale,
                    name = item.pack_name.as_str()
                ),
            ),
            CapabilityQueueOperation::Uninstall => (
                tron_i18n::t!("capabilities.queue.uninstalled_title", locale = locale).into_owned(),
                tron_i18n::t!(
                    "capabilities.queue.uninstalled_body",
                    locale = locale,
                    name = item.pack_name.as_str()
                ),
            ),
        };
        // A removed package has no detail page to open.
        let href = if item.operation == CapabilityQueueOperation::Uninstall {
            "/settings/capability-packs".to_string()
        } else {
            format!("/settings/capability-packs/{}", item.pack_id)
        };
        let input = NotificationInput {
            // Replaying an item after a crash must not create a second notice.
            dedup_key: format!("capability-queue:{}", item.id),
            assistant_code: None,
            title,
            body,
            category: "Capability".into(),
            target: serde_json::json!({
                "href": href,
                "packId": item.pack_id,
            }),
        };
        if let Err(error) = assistants.notify(input) {
            tracing::warn!(%error, pack = %item.pack_id, "capability queue notification failed");
        }
    }
}
