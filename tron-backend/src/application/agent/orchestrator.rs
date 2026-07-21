use super::{
    ActiveRunGuard, AgentInvocationCommand, AgentInvocationError, AgentRunContextBuilder,
    AgentRunExecutionResult, AgentRunExecutor, ModelStreamSink, PreparedAgentRun,
};

use crate::{
    domain::agent::{AgentExecutionMode, AgentRunPlanStep, CollaborationRun},
    shared::time::current_timestamp,
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
    pub execution_mode: AgentExecutionMode,
    pub plan_steps: Vec<AgentRunPlanStep>,
    pub collaboration: CollaborationRun,
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

    pub(crate) async fn execute(
        &self,
        command: AgentInvocationCommand,
        user_code: Option<String>,
        locale: Option<String>,
        sink: Option<ModelStreamSink>,
        reservation: Option<ActiveRunGuard>,
    ) -> Result<AgentOrchestrationRun, AgentInvocationError> {
        match self.execution_mode_for(&command) {
            AgentExecutionMode::SingleAgent => {
                self.execute_single_agent(command, user_code, locale, sink, reservation)
                    .await
            }
            AgentExecutionMode::Collaboration => Err(AgentInvocationError::RuntimeFailed(
                "collaboration execution is not enabled yet".into(),
            )),
        }
    }

    pub(crate) async fn execute_single_agent(
        &self,
        command: AgentInvocationCommand,
        user_code: Option<String>,
        locale: Option<String>,
        sink: Option<ModelStreamSink>,
        reservation: Option<ActiveRunGuard>,
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
            agent_code,
            objective,
            model_code,
            requested_system_agent_session_id,
            runtime_selection,
            context,
        } = prepared;
        let step_code = single_agent_step_code(&run_code);
        let plan_step = AgentRunPlanStep::serial(step_code, agent_code, objective.clone());
        let metadata = AgentRunExecutionMetadata {
            run_code: run_code.clone(),
            conversation_code,
            workspace_code: workspace_code.clone(),
            user_code,
            app_code,
            task_code,
            model_code,
            started_at: context.started_at.clone(),
        };
        let mut collaboration = CollaborationRun::single_agent(
            single_agent_collaboration_run_code(&run_code),
            workspace_code,
            metadata.conversation_code.clone(),
            metadata.user_code.clone(),
            objective,
            run_code.clone(),
            plan_step.clone(),
            context.started_at.clone(),
        );
        let execution = self
            .run_executor
            .execute_with_selection(
                runtime_selection,
                context,
                requested_system_agent_session_id,
                sink,
                reservation,
            )
            .await;
        let finished_at = current_timestamp();
        match &execution {
            Ok(result) => collaboration.complete_first_step(
                result.run.run_code.clone(),
                result.run.final_output.clone(),
                finished_at,
            ),
            Err(error) => collaboration.fail_first_step(
                run_code,
                format!("{}: {}", error.code(), error.message()),
                finished_at,
            ),
        }

        Ok(AgentOrchestrationRun {
            execution_mode: AgentExecutionMode::SingleAgent,
            plan_steps: vec![plan_step],
            collaboration,
            metadata,
            execution,
        })
    }

    fn execution_mode_for(&self, _command: &AgentInvocationCommand) -> AgentExecutionMode {
        AgentExecutionMode::SingleAgent
    }
}

fn single_agent_collaboration_run_code(run_code: &str) -> String {
    format!("COLLAB-{run_code}")
}

fn single_agent_step_code(run_code: &str) -> String {
    format!("{run_code}-STEP-0001")
}

#[cfg(test)]
mod tests {
    use super::{single_agent_collaboration_run_code, single_agent_step_code};

    #[test]
    fn single_agent_codes_are_derived_from_run_code() {
        assert_eq!(single_agent_collaboration_run_code("RUN-1"), "COLLAB-RUN-1");
        assert_eq!(single_agent_step_code("RUN-1"), "RUN-1-STEP-0001");
    }
}
