use super::*;

#[derive(Clone)]
pub(super) struct AutomationRuntime {
    registered_codes: Arc<Mutex<BTreeSet<String>>>,
    scheduler: Arc<Mutex<Scheduler<Utc>>>,
}

impl AutomationRuntime {
    pub(super) fn new() -> Self {
        Self {
            registered_codes: Arc::new(Mutex::new(BTreeSet::new())),
            scheduler: Arc::new(Mutex::new(Scheduler::utc())),
        }
    }

    pub(super) fn register(
        &self,
        automation_code: &str,
        scheduler_expression: &str,
        repository: Arc<Mutex<Box<dyn AutomationRepository>>>,
        event_bus: EventBusHandle,
    ) -> Result<(), AutomationError> {
        self.unregister(automation_code);
        let code_for_name = automation_code.to_string();
        let code_for_run = code_for_name.clone();
        let job = Job::named(
            &code_for_name,
            scheduler_expression.to_string(),
            move || {
                let repository = repository.clone();
                let event_bus = event_bus.clone();
                let automation_code = code_for_run.clone();
                async move {
                    let _ = record_execution(
                        repository,
                        event_bus,
                        &automation_code,
                        AutomationTriggerSource::Scheduler,
                    );
                }
            },
        );
        self.scheduler
            .lock()
            .expect("automation scheduler mutex poisoned")
            .add(job);
        self.registered_codes
            .lock()
            .expect("automation scheduler registry mutex poisoned")
            .insert(code_for_name);
        Ok(())
    }

    pub(super) fn unregister(&self, automation_code: &str) {
        self.scheduler
            .lock()
            .expect("automation scheduler mutex poisoned")
            .cancel_by_name(automation_code);
        self.registered_codes
            .lock()
            .expect("automation scheduler registry mutex poisoned")
            .remove(automation_code);
    }

    pub(super) fn snapshot(&self) -> Vec<String> {
        self.registered_codes
            .lock()
            .expect("automation scheduler registry mutex poisoned")
            .iter()
            .cloned()
            .collect()
    }
}
