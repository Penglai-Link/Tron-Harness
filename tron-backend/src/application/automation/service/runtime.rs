use super::*;

#[derive(Clone)]
pub(super) struct AutomationRuntime {
    jobs: Arc<Mutex<BTreeMap<String, JoinHandle<()>>>>,
}

impl AutomationRuntime {
    pub(super) fn new() -> Self {
        Self {
            jobs: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    pub(super) fn register(
        &self,
        automation_code: &str,
        scheduler_expression: &str,
        repository: Arc<Mutex<Box<dyn AutomationRepository>>>,
        event_bus: EventBusHandle,
        execution_runner: Arc<Mutex<Option<Arc<dyn AutomationExecutionRunner>>>>,
    ) -> Result<(), AutomationError> {
        self.unregister(automation_code);
        let schedule =
            Schedule::from_str(scheduler_expression).map_err(|_| AutomationError::InvalidCron)?;
        let automation_code = automation_code.to_string();
        let job_code = automation_code.clone();
        let handle = tokio::spawn(async move {
            run_scheduled_automation(job_code, schedule, repository, event_bus, execution_runner)
                .await;
        });
        self.jobs
            .lock()
            .expect("automation scheduler registry mutex poisoned")
            .insert(automation_code, handle);
        Ok(())
    }

    pub(super) fn unregister(&self, automation_code: &str) {
        if let Some(handle) = self
            .jobs
            .lock()
            .expect("automation scheduler registry mutex poisoned")
            .remove(automation_code)
        {
            handle.abort();
        }
    }

    pub(super) fn snapshot(&self) -> Vec<String> {
        self.jobs
            .lock()
            .expect("automation scheduler registry mutex poisoned")
            .keys()
            .cloned()
            .collect()
    }
}

impl Drop for AutomationRuntime {
    fn drop(&mut self) {
        if Arc::strong_count(&self.jobs) != 1 {
            return;
        }
        for (_, handle) in self
            .jobs
            .lock()
            .expect("automation scheduler registry mutex poisoned")
            .iter()
        {
            handle.abort();
        }
    }
}

async fn run_scheduled_automation(
    automation_code: String,
    schedule: Schedule,
    repository: Arc<Mutex<Box<dyn AutomationRepository>>>,
    event_bus: EventBusHandle,
    execution_runner: Arc<Mutex<Option<Arc<dyn AutomationExecutionRunner>>>>,
) {
    loop {
        let Some(scheduled_at) = schedule.upcoming(Utc).next() else {
            return;
        };
        let delay = (scheduled_at - Utc::now())
            .to_std()
            .unwrap_or(Duration::ZERO);
        tokio::time::sleep(delay).await;
        let _ = record_execution(
            repository.clone(),
            event_bus.clone(),
            execution_runner.clone(),
            &automation_code,
            AutomationTriggerSource::Scheduler,
            Some(format_timestamp(scheduled_at)),
        );
    }
}
