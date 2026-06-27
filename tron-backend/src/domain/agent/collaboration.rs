use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AgentExecutionMode {
    SingleAgent,
    Collaboration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CollaborationRunStatus {
    Planned,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CollaborationStepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CollaborationStepExecutionMode {
    Serial,
    Parallel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunPlanStep {
    pub step_code: String,
    pub agent_code: String,
    pub input: String,
    pub depends_on: Vec<String>,
    pub execution_mode: CollaborationStepExecutionMode,
}

impl AgentRunPlanStep {
    pub fn serial(step_code: String, agent_code: String, input: String) -> Self {
        Self {
            step_code,
            agent_code,
            input,
            depends_on: Vec::new(),
            execution_mode: CollaborationStepExecutionMode::Serial,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentAssignment {
    pub agent_code: String,
    pub role: String,
    pub objective: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HandoffContext {
    pub from_agent_code: Option<String>,
    pub to_agent_code: String,
    pub summary: String,
    pub artifact_codes: Vec<String>,
    pub decision_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollaborationArtifact {
    pub artifact_code: String,
    pub step_code: Option<String>,
    pub kind: String,
    pub title: String,
    pub summary: Option<String>,
    pub content: Option<String>,
    pub metadata: Value,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollaborationDecision {
    pub decision_code: String,
    pub step_code: Option<String>,
    pub prompt: String,
    pub selected_option: String,
    pub rationale: Option<String>,
    pub metadata: Value,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollaborationStep {
    pub step_code: String,
    pub agent_code: String,
    pub input: String,
    pub depends_on: Vec<String>,
    pub execution_mode: CollaborationStepExecutionMode,
    pub status: CollaborationStepStatus,
    pub run_code: Option<String>,
    pub output: Option<String>,
    pub error: Option<String>,
    pub assignment: AgentAssignment,
    pub handoff_context: Option<HandoffContext>,
    pub artifacts: Vec<CollaborationArtifact>,
    pub decisions: Vec<CollaborationDecision>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
}

impl CollaborationStep {
    pub fn from_plan(plan: AgentRunPlanStep, created_at: String) -> Self {
        let AgentRunPlanStep {
            step_code,
            agent_code,
            input,
            depends_on,
            execution_mode,
        } = plan;

        Self {
            step_code,
            agent_code: agent_code.clone(),
            input: input.clone(),
            depends_on,
            execution_mode,
            status: CollaborationStepStatus::Pending,
            run_code: None,
            output: None,
            error: None,
            assignment: AgentAssignment {
                agent_code,
                role: "primary".into(),
                objective: input,
            },
            handoff_context: None,
            artifacts: Vec::new(),
            decisions: Vec::new(),
            created_at,
            started_at: None,
            finished_at: None,
        }
    }

    pub fn mark_running(&mut self, run_code: String, started_at: String) {
        self.status = CollaborationStepStatus::Running;
        self.run_code = Some(run_code);
        self.started_at = Some(started_at);
    }

    pub fn mark_completed(&mut self, run_code: String, output: String, finished_at: String) {
        self.status = CollaborationStepStatus::Completed;
        self.run_code = Some(run_code);
        self.output = Some(output);
        self.error = None;
        self.finished_at = Some(finished_at);
    }

    pub fn mark_failed(&mut self, run_code: String, error: String, finished_at: String) {
        self.status = CollaborationStepStatus::Failed;
        self.run_code = Some(run_code);
        self.output = None;
        self.error = Some(error);
        self.finished_at = Some(finished_at);
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollaborationRun {
    pub collaboration_run_code: String,
    pub workspace_code: String,
    pub conversation_code: Option<String>,
    pub user_code: Option<String>,
    pub objective: String,
    pub strategy: String,
    pub execution_mode: AgentExecutionMode,
    pub status: CollaborationRunStatus,
    pub steps: Vec<CollaborationStep>,
    pub artifacts: Vec<CollaborationArtifact>,
    pub decisions: Vec<CollaborationDecision>,
    pub created_at: String,
    pub finished_at: Option<String>,
}

impl CollaborationRun {
    pub fn single_agent(
        collaboration_run_code: String,
        workspace_code: String,
        conversation_code: Option<String>,
        user_code: Option<String>,
        objective: String,
        run_code: String,
        plan_step: AgentRunPlanStep,
        created_at: String,
    ) -> Self {
        let mut step = CollaborationStep::from_plan(plan_step, created_at.clone());
        step.mark_running(run_code, created_at.clone());

        Self {
            collaboration_run_code,
            workspace_code,
            conversation_code,
            user_code,
            objective,
            strategy: "single_agent_compatibility".into(),
            execution_mode: AgentExecutionMode::SingleAgent,
            status: CollaborationRunStatus::Running,
            steps: vec![step],
            artifacts: Vec::new(),
            decisions: Vec::new(),
            created_at,
            finished_at: None,
        }
    }

    pub fn complete_first_step(&mut self, run_code: String, output: String, finished_at: String) {
        if let Some(step) = self.steps.first_mut() {
            step.mark_completed(run_code, output, finished_at.clone());
        }
        self.status = CollaborationRunStatus::Completed;
        self.finished_at = Some(finished_at);
    }

    pub fn fail_first_step(&mut self, run_code: String, error: String, finished_at: String) {
        if let Some(step) = self.steps.first_mut() {
            step.mark_failed(run_code, error, finished_at.clone());
        }
        self.status = CollaborationRunStatus::Failed;
        self.finished_at = Some(finished_at);
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        AgentExecutionMode, AgentRunPlanStep, CollaborationRun, CollaborationRunStatus,
        CollaborationStepExecutionMode, CollaborationStepStatus,
    };

    #[test]
    fn single_agent_run_starts_first_step() {
        let run = CollaborationRun::single_agent(
            "COLLAB-RUN-1".into(),
            "TRON_CORE".into(),
            Some("CONV-1".into()),
            Some("USR-1".into()),
            "Summarize the task".into(),
            "RUN-1".into(),
            AgentRunPlanStep::serial(
                "RUN-1-STEP-0001".into(),
                "main-agent".into(),
                "Summarize the task".into(),
            ),
            "2026-06-27T00:00:00Z".into(),
        );

        assert_eq!(run.execution_mode, AgentExecutionMode::SingleAgent);
        assert_eq!(run.status, CollaborationRunStatus::Running);
        assert_eq!(run.steps.len(), 1);
        assert_eq!(run.steps[0].agent_code, "main-agent");
        assert_eq!(
            run.steps[0].execution_mode,
            CollaborationStepExecutionMode::Serial
        );
        assert_eq!(run.steps[0].status, CollaborationStepStatus::Running);
        assert_eq!(run.steps[0].run_code.as_deref(), Some("RUN-1"));
    }

    #[test]
    fn complete_first_step_records_output_and_finish_time() {
        let mut run = CollaborationRun::single_agent(
            "COLLAB-RUN-1".into(),
            "TRON_CORE".into(),
            None,
            None,
            "Build".into(),
            "RUN-1".into(),
            AgentRunPlanStep::serial(
                "RUN-1-STEP-0001".into(),
                "main-agent".into(),
                "Build".into(),
            ),
            "2026-06-27T00:00:00Z".into(),
        );

        run.complete_first_step("RUN-1".into(), "done".into(), "2026-06-27T00:01:00Z".into());

        assert_eq!(run.status, CollaborationRunStatus::Completed);
        assert_eq!(run.finished_at.as_deref(), Some("2026-06-27T00:01:00Z"));
        assert_eq!(run.steps[0].status, CollaborationStepStatus::Completed);
        assert_eq!(run.steps[0].output.as_deref(), Some("done"));
        assert_eq!(run.steps[0].error, None);
    }

    #[test]
    fn collaboration_run_serializes_with_platform_field_style() {
        let run = CollaborationRun::single_agent(
            "COLLAB-RUN-1".into(),
            "TRON_CORE".into(),
            Some("CONV-1".into()),
            Some("USR-1".into()),
            "Build".into(),
            "RUN-1".into(),
            AgentRunPlanStep::serial(
                "RUN-1-STEP-0001".into(),
                "main-agent".into(),
                "Build".into(),
            ),
            "2026-06-27T00:00:00Z".into(),
        );

        let value = serde_json::to_value(run).expect("collaboration run should serialize");

        assert_eq!(value["collaborationRunCode"], json!("COLLAB-RUN-1"));
        assert_eq!(value["executionMode"], json!("SINGLE_AGENT"));
        assert_eq!(value["steps"][0]["executionMode"], json!("SERIAL"));
        assert_eq!(value["steps"][0]["status"], json!("RUNNING"));
    }
}
