use super::{
    AgentInvocationCommand, AgentInvocationError, AgentRunContextBuilder, AgentRunExecutionResult,
    AgentRunExecutor, ModelStreamSink, PreparedAgentRun,
};

#[derive(Debug, Clone)]
pub(crate) struct AgentRunExecutionMetadata {
    pub run_code: String,
    pub conversation_code: Option<String>,
    pub workspace_code: String,
    pub user_code: Option<String>,
    pub app_code: Option<String>,
    pub task_code: Option<String>,
    pub model_code: String,
    pub started_at: String,
}

#[derive(Debug)]
pub(crate) struct AgentOrchestrationRun {
    pub metadata: AgentRunExecutionMetadata,
    pub execution: Result<AgentRunExecutionResult, AgentInvocationError>,
}

#[derive(Clone)]
pub struct AgentOrchestrator {
    run_context_builder: AgentRunContextBuilder,
    run_executor: AgentRunExecutor,
}

impl AgentOrchestrator {
    pub(crate) fn new(
        run_context_builder: AgentRunContextBuilder,
        run_executor: AgentRunExecutor,
    ) -> Self {
        Self {
            run_context_builder,
            run_executor,
        }
    }

    pub(crate) async fn execute_single_agent(
        &self,
        command: AgentInvocationCommand,
        user_code: Option<String>,
        locale: Option<String>,
        sink: Option<ModelStreamSink>,
    ) -> Result<AgentOrchestrationRun, AgentInvocationError> {
        let prepared = self
            .run_context_builder
            .prepare(command, user_code, locale)
            .await?;
        let PreparedAgentRun {
            run_code,
            conversation_code,
            workspace_code,
            user_code,
            app_code,
            task_code,
            model_code,
            requested_system_agent_session_id,
            runtime_selection,
            context,
        } = prepared;
        let metadata = AgentRunExecutionMetadata {
            run_code,
            conversation_code,
            workspace_code,
            user_code,
            app_code,
            task_code,
            model_code,
            started_at: context.started_at.clone(),
        };
        let execution = self
            .run_executor
            .execute_with_selection(
                runtime_selection,
                context,
                requested_system_agent_session_id,
                sink,
            )
            .await;

        Ok(AgentOrchestrationRun {
            metadata,
            execution,
        })
    }
}
