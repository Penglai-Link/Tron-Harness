//! Human interventions reuse native Thread history and interaction continuations.
use super::*;
use crate::application::agent::AgentInvocationService;

impl FrameworkHost {
    async fn intervention_runtime(
        &self,
        dude: &str,
        id: &str,
        user: &str,
        locale: &str,
        service: &AgentInvocationService,
    ) -> Result<(Runtime, Arc<Thread>), fw::AgentError> {
        let record = self
            .thread_record(id)
            .await?
            .filter(|record| {
                !record.closed && record.owner.kind == "DUDE" && record.owner.code == dude
            })
            .ok_or_else(|| failure("thread not found"))?;
        // Platform route authenticates the human. Keep the original owner and authorization scope.
        let builder = service
            .run_context_builder()
            .with_execution_identity(record.workspace_code.is_empty(), Some(record.owner.clone()));
        let router = builder
            .model_router()
            .ok_or_else(|| failure("Agent Auto model router is unavailable"))?;
        let (model, _) = router
            .resolve_initial(&record.workspace_code, id, &record.agent_code, None)
            .map_err(failure)?;
        let mut command: AgentInvocationCommand = serde_json::from_value(serde_json::json!({"prompt":"Prepare human Thread intervention","workspaceCode":record.workspace_code,"agentCode":record.agent_code,"model":model})).map_err(failure)?;
        command.execution_scope_override = Some(record.execution_scope.clone());
        let prepared = builder
            .prepare(command, record.user_code.clone(), Some(locale.into()))
            .await
            .map_err(|error| failure(error.message()))?;
        let runtime = Runtime {
            host: Arc::downgrade(&self.0),
            builder,
            context: prepared.context,
        };
        let thread = runtime.thread(id).await?;
        let _ = user; // The trusted human identity is carried in provenance, never ownership.
        Ok((runtime, thread))
    }
    pub async fn dude_thread_send(
        &self,
        dude: &str,
        id: &str,
        message: &str,
        user: &str,
        locale: &str,
        service: &AgentInvocationService,
    ) -> Result<fw::ThreadSummary, fw::AgentError> {
        if message.trim().is_empty() {
            return Err(failure("Thread message cannot be empty"));
        }
        let (runtime, thread) = self
            .intervention_runtime(dude, id, user, locale, service)
            .await?;
        let _transition = thread.transition.lock().await;
        runtime.ensure_registered(id, &thread).await?;
        match thread.state.lock().unwrap().summary.status {
            fw::ThreadStatus::Running => return Err(fw::AgentError::SessionBusy(id.into())),
            fw::ThreadStatus::AwaitingInput => {
                return Err(failure("thread is awaiting input; answer the interaction"));
            }
            _ => {}
        }
        let request = fw::RunRequest::new(id, message);
        let (_, request, _) = runtime
            .prepare_owned(
                request,
                id.into(),
                &thread.context,
                &runtime.context,
                service.collaboration_harness().map_err(failure)?,
                None,
                ExecutionLimits::AgentDefinition,
                Some(human(user)),
            )
            .await?;
        runtime
            .launch(
                thread.clone(),
                request,
                None,
                fw::ThreadRunCause::Message,
                None,
            )
            .await
    }
    pub async fn dude_thread_answer(
        &self,
        dude: &str,
        id: &str,
        interaction: fw::RunId,
        value: serde_json::Value,
        user: &str,
        locale: &str,
        service: &AgentInvocationService,
    ) -> Result<fw::ThreadSummary, fw::AgentError> {
        let (runtime, thread) = self
            .intervention_runtime(dude, id, user, locale, service)
            .await?;
        let _transition = thread.transition.lock().await;
        runtime.ensure_registered(id, &thread).await?;
        if thread.state.lock().unwrap().summary.status != fw::ThreadStatus::AwaitingInput {
            return Err(failure("thread is not awaiting input"));
        }
        let request = thread.agent.answer(id, interaction, value).await?;
        let (_, request, _) = runtime
            .prepare_owned(
                request,
                id.into(),
                &thread.context,
                &runtime.context,
                service.collaboration_harness().map_err(failure)?,
                None,
                ExecutionLimits::AgentDefinition,
                Some(human(user)),
            )
            .await?;
        runtime
            .launch(
                thread.clone(),
                request,
                None,
                fw::ThreadRunCause::InteractionResponse,
                Some(interaction),
            )
            .await
    }
}
fn human(user: &str) -> fw::MessageSender {
    fw::MessageSender {
        kind: "HUMAN".into(),
        id: Some(user.into()),
        display_name: Some(user.into()),
        avatar_url: None,
    }
}
