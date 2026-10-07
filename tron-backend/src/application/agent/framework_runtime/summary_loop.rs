//! The product authorizes the source; Framework owns execution and summarization.
use eliteforge_agent_core as fw;
use eliteforge_agent_memory::{CompressionOptions, MemoryCompactor, SummaryStore};
use eliteforge_agent_official_plugin_storage_local::SqliteDatabase;
use std::sync::Arc;
#[derive(Clone)]
pub(crate) struct SummarySource {
    pub snapshot: fw::ConversationSnapshot,
    pub store: Arc<dyn SummaryStore>,
    pub lifecycle: Arc<dyn fw::SessionLifecycleStore>,
    pub database: SqliteDatabase,
    pub options: CompressionOptions,
    pub reason: Option<String>,
    pub note: Option<String>,
}
pub(super) struct SummaryLoop;
#[async_trait::async_trait]
impl fw::AgentLoop for SummaryLoop {
    fn execution_kind(&self) -> &str {
        "SUMMARY"
    }
    fn requires_conversations(&self) -> bool {
        false
    }
    fn supports_tool_policy(&self) -> bool {
        false
    }
    async fn run(
        &self,
        request: &fw::RunRequest,
        run: &fw::RunContext,
        services: &fw::AgentServices,
    ) -> Result<fw::RunResult, fw::AgentError> {
        let source = request
            .extensions
            .get::<SummarySource>()
            .ok_or(fw::AgentError::MissingService("SummarySource"))?;
        let model = request
            .model
            .clone()
            .ok_or(fw::AgentError::MissingService("SummaryModel"))?;
        let models = services
            .models
            .as_ref()
            .ok_or(fw::AgentError::MissingService("ChatExecutor"))?;
        let (key, summary, result) = MemoryCompactor::new(model, source.options.clone())?
            .summarize_snapshot(
                &source.snapshot,
                "tron.session-summary.v1",
                run,
                models.as_ref(),
                run.limits(),
            )
            .await?;
        let lifecycle = &source.lifecycle;
        let projection=source.database.transaction(async {
            let current=services.conversations.history(&run.session_id).await.map_err(|e|fw::StateError::Backend(e.to_string()))?;
            if !current.iter().any(|snapshot|snapshot.key==source.snapshot.key && snapshot.generation==source.snapshot.generation && snapshot.revision==source.snapshot.revision && snapshot.messages==source.snapshot.messages) { return Err(fw::StateError::Conflict); }
            let prior=source.store.load(&key).await.map_err(|e|fw::StateError::Backend(e.to_string()))?;
            if !source.store.compare_exchange(&key,prior.as_ref().map(|v|v.version.as_str()),&summary).await.map_err(|e|fw::StateError::Backend(e.to_string()))? {return Err(fw::StateError::Conflict);}
            let stored=source.store.load(&key).await.map_err(|e|fw::StateError::Backend(e.to_string()))?.ok_or(fw::StateError::NotFound)?;
            let mut projection=serde_json::json!({"sessionCode":run.session_id,"summaryCode":stored.version,"sourceRevision":summary.source_revision,"coveredMessages":summary.covered_messages,"summary":summary.summary,"createdAt":fw::lifecycle_time_ms(),"reason":source.reason,"note":source.note});
            let prior=lifecycle.read_context(&run.session_id,"tron.session-summary-view").await?;
            lifecycle.write_context(&run.session_id,"tron.session-summary-view",prior.map(|(revision,_)|revision),projection.clone()).await?;
            projection["usage"]=serde_json::to_value(&result.usage).map_err(|e|fw::StateError::Backend(e.to_string()))?;
            let mut event=fw::SessionEvent::new(&run.session_id,"tron.session-summary-view",projection.clone());event.run_id=Some(run.run_id);
            lifecycle.append_event(event).await?;
            Ok(projection)
        }).await?;
        Ok(fw::RunResult {
            output: result.summary.clone(),
            content: result.summary.into(),
            structured: Some(projection),
            interactions: vec![],
            status: fw::RunStatus::Completed,
            usage: result.usage,
            steps: 1,
            tool_calls: 0,
            external_session: None,
        })
    }
}
