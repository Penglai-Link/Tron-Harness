//! An additional host ceiling can only narrow existing SDK run/step budgets.
use async_trait::async_trait;
use eliteforge_agent_core as fw;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
pub(super) struct AuxiliaryAllowance<'a> {
    pub inner: &'a dyn fw::ChatExecutor,
    pub calls: Arc<AtomicUsize>,
    pub limit: usize,
}
#[async_trait]
impl fw::ChatExecutor for AuxiliaryAllowance<'_> {
    fn catalog(&self) -> &fw::ModelCatalog {
        self.inner.catalog()
    }
    fn catalog_snapshot(&self) -> Arc<fw::ModelCatalog> {
        self.inner.catalog_snapshot()
    }
    fn catalog_for_run(&self, run: &fw::RunContext) -> Arc<fw::ModelCatalog> {
        self.inner.catalog_for_run(run)
    }
    async fn validate_async(&self) -> Result<(), fw::AgentError> {
        self.inner.validate_async().await
    }
    async fn invoke(
        &self,
        call: fw::ModelCall<'_>,
        memory: &mut dyn fw::ConversationSession,
        run: &fw::RunContext,
    ) -> Result<fw::ChatResponse, fw::AgentError> {
        self.inner.invoke(call, memory, run).await
    }
    async fn invoke_auxiliary(
        &self,
        model: &fw::ModelRef,
        request: fw::ChatRequest,
        stage: &str,
        limits: &fw::RunLimits,
        run: &fw::RunContext,
    ) -> Result<fw::ChatResponse, fw::AgentError> {
        run.check_cancelled()?;
        self.calls
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |calls| {
                (calls < self.limit).then(|| calls + 1)
            })
            .map_err(|_| {
                fw::ContextError::BudgetExceeded(
                    "shared context auxiliary call allowance exhausted".into(),
                )
            })?;
        // SDK remains the authority for actual dispatch, usage, timeout and answer reserve.
        self.inner
            .invoke_auxiliary(model, request, stage, limits, run)
            .await
    }
}
