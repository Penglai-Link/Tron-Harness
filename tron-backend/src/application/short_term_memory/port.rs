use crate::domain::short_term_memory::{
    ShortTermMemoryCompaction, ShortTermMemoryContextEpoch, ShortTermMemoryError,
    ShortTermMemoryEvent, ShortTermMemoryFamily, ShortTermMemoryFamilyMember,
    ShortTermMemoryFamilySnapshot, ShortTermMemoryGoal, ShortTermMemoryGoalCriterion,
    ShortTermMemoryGoalEvent, ShortTermMemoryGoalView, ShortTermMemoryInput,
    ShortTermMemoryMessage, ShortTermMemoryPart, ShortTermMemoryTodo,
};

pub trait ShortTermMemoryRepository: Send {
    fn session_workspace(
        &mut self,
        session_code: &str,
    ) -> Result<Option<String>, ShortTermMemoryError>;
    fn next_id(&mut self, table: &str) -> Result<i64, ShortTermMemoryError>;
    fn next_seq(&mut self, session_code: &str) -> Result<i64, ShortTermMemoryError>;
    fn input_code_by_idempotency_key(
        &mut self,
        session_code: &str,
        idempotency_key: &str,
    ) -> Result<Option<String>, ShortTermMemoryError>;
    fn create_input(&mut self, input: &ShortTermMemoryInput) -> Result<(), ShortTermMemoryError>;
    fn create_input_with_message_and_events(
        &mut self,
        input: &ShortTermMemoryInput,
        message: Option<&ShortTermMemoryMessage>,
        parts: &[ShortTermMemoryPart],
        events: &[ShortTermMemoryEvent],
    ) -> Result<(), ShortTermMemoryError>;
    fn list_inputs_for_run(
        &mut self,
        session_code: &str,
        run_code: &str,
    ) -> Result<Vec<ShortTermMemoryInput>, ShortTermMemoryError>;
    fn create_event(&mut self, event: &ShortTermMemoryEvent) -> Result<(), ShortTermMemoryError>;
    fn create_event_and_update_run_lease(
        &mut self,
        event: &ShortTermMemoryEvent,
    ) -> Result<(), ShortTermMemoryError>;
    fn create_message(
        &mut self,
        message: &ShortTermMemoryMessage,
        parts: &[ShortTermMemoryPart],
    ) -> Result<(), ShortTermMemoryError>;
    fn create_message_with_event(
        &mut self,
        message: &ShortTermMemoryMessage,
        parts: &[ShortTermMemoryPart],
        event: &ShortTermMemoryEvent,
    ) -> Result<(), ShortTermMemoryError>;
    fn create_message_with_events(
        &mut self,
        message: &ShortTermMemoryMessage,
        parts: &[ShortTermMemoryPart],
        events: &[ShortTermMemoryEvent],
    ) -> Result<(), ShortTermMemoryError>;
    fn create_context_epoch(
        &mut self,
        epoch: &ShortTermMemoryContextEpoch,
    ) -> Result<(), ShortTermMemoryError>;
    fn create_context_epoch_with_event(
        &mut self,
        epoch: &ShortTermMemoryContextEpoch,
        event: &ShortTermMemoryEvent,
    ) -> Result<ShortTermMemoryContextEpoch, ShortTermMemoryError>;
    fn latest_context_epoch_revision(
        &mut self,
        session_code: &str,
    ) -> Result<i64, ShortTermMemoryError>;
    fn list_messages_after_seq(
        &mut self,
        session_code: &str,
        min_seq: i64,
    ) -> Result<Vec<(ShortTermMemoryMessage, Vec<ShortTermMemoryPart>)>, ShortTermMemoryError>;
    fn list_messages_for_run(
        &mut self,
        session_code: &str,
        run_code: &str,
    ) -> Result<Vec<(ShortTermMemoryMessage, Vec<ShortTermMemoryPart>)>, ShortTermMemoryError>;
    fn list_recent_messages(
        &mut self,
        session_code: &str,
        min_seq: i64,
        limit: usize,
    ) -> Result<Vec<(ShortTermMemoryMessage, Vec<ShortTermMemoryPart>)>, ShortTermMemoryError>;
    fn message_parts(
        &mut self,
        message_code: &str,
    ) -> Result<Vec<ShortTermMemoryPart>, ShortTermMemoryError>;
    fn list_todos(
        &mut self,
        session_code: &str,
    ) -> Result<Vec<ShortTermMemoryTodo>, ShortTermMemoryError>;
    fn replace_todos(
        &mut self,
        session_code: &str,
        workspace_code: &str,
        todos: &[ShortTermMemoryTodo],
    ) -> Result<(), ShortTermMemoryError>;
    fn replace_todos_with_event(
        &mut self,
        session_code: &str,
        workspace_code: &str,
        todos: &[ShortTermMemoryTodo],
        event: &ShortTermMemoryEvent,
    ) -> Result<(), ShortTermMemoryError>;
    fn latest_compaction(
        &mut self,
        session_code: &str,
    ) -> Result<Option<ShortTermMemoryCompaction>, ShortTermMemoryError>;
    fn create_compaction(
        &mut self,
        compaction: &ShortTermMemoryCompaction,
        message: &ShortTermMemoryMessage,
        parts: &[ShortTermMemoryPart],
        event: Option<&ShortTermMemoryEvent>,
        enforce_idle: bool,
    ) -> Result<(), ShortTermMemoryError>;
    fn active_run_exists(&mut self, session_code: &str) -> Result<bool, ShortTermMemoryError>;
    fn upsert_family_memberships(
        &mut self,
        families: &[ShortTermMemoryFamily],
        members: &[ShortTermMemoryFamilyMember],
    ) -> Result<(), ShortTermMemoryError>;
    fn list_enabled_family_members(
        &mut self,
        session_code: &str,
    ) -> Result<Vec<(ShortTermMemoryFamily, ShortTermMemoryFamilyMember)>, ShortTermMemoryError>;
    fn list_completed_family_snapshots(
        &mut self,
        family_code: &str,
        exclude_session_code: &str,
        limit: usize,
    ) -> Result<Vec<ShortTermMemoryFamilySnapshot>, ShortTermMemoryError>;
    fn session_summary_for_run(
        &mut self,
        session_code: &str,
        run_code: &str,
    ) -> Result<Option<ShortTermMemoryFamilySnapshot>, ShortTermMemoryError>;
    fn latest_completed_session_summary(
        &mut self,
        session_code: &str,
    ) -> Result<Option<ShortTermMemoryFamilySnapshot>, ShortTermMemoryError>;
    fn latest_completed_family_handoff(
        &mut self,
        family_code: &str,
        session_code: &str,
    ) -> Result<Option<ShortTermMemoryFamilySnapshot>, ShortTermMemoryError>;
    fn list_family_session_codes(
        &mut self,
        family_code: &str,
        exclude_session_code: &str,
        limit: usize,
    ) -> Result<Vec<String>, ShortTermMemoryError>;
    fn create_family_snapshots(
        &mut self,
        snapshots: &[ShortTermMemoryFamilySnapshot],
    ) -> Result<(), ShortTermMemoryError>;
    fn recover_stale_session_summaries(
        &mut self,
        completed_at: &str,
    ) -> Result<usize, ShortTermMemoryError>;
    fn ensure_hidden_heartbeat_session(
        &mut self,
        workspace_code: &str,
        agent_code: &str,
        session_code: &str,
        id: i64,
        now: &str,
    ) -> Result<String, ShortTermMemoryError>;
    fn get_active_goal(
        &mut self,
        session_code: &str,
    ) -> Result<Option<ShortTermMemoryGoalView>, ShortTermMemoryError>;
    fn get_latest_goal(
        &mut self,
        session_code: &str,
    ) -> Result<Option<ShortTermMemoryGoalView>, ShortTermMemoryError>;
    fn list_open_goals_batch(
        &mut self,
        after_id: i64,
        limit: usize,
    ) -> Result<Vec<ShortTermMemoryGoalView>, ShortTermMemoryError>;
    fn get_goal(
        &mut self,
        goal_code: &str,
    ) -> Result<Option<ShortTermMemoryGoalView>, ShortTermMemoryError>;
    fn list_goal_events(
        &mut self,
        goal_code: &str,
        limit: usize,
    ) -> Result<Vec<ShortTermMemoryGoalEvent>, ShortTermMemoryError>;
    fn append_goal_event(
        &mut self,
        event: &ShortTermMemoryGoalEvent,
    ) -> Result<(), ShortTermMemoryError>;
    fn create_goal(
        &mut self,
        goal: &ShortTermMemoryGoal,
        criteria: &[ShortTermMemoryGoalCriterion],
        event: &ShortTermMemoryGoalEvent,
    ) -> Result<ShortTermMemoryGoalView, ShortTermMemoryError>;
    fn update_goal(
        &mut self,
        goal: &ShortTermMemoryGoal,
        criteria: &[ShortTermMemoryGoalCriterion],
        event: &ShortTermMemoryGoalEvent,
        expected_revision: i64,
    ) -> Result<ShortTermMemoryGoalView, ShortTermMemoryError>;
    fn recover_stale_runs(&mut self, limit: usize) -> Result<usize, ShortTermMemoryError>;
    fn cleanup_event_log_before(
        &mut self,
        cutoff_created_at_ms: i64,
        batch_size: usize,
        max_rows: usize,
    ) -> Result<usize, ShortTermMemoryError>;
    fn list_externalized_tool_output_paths(
        &mut self,
        root_path: &str,
    ) -> Result<Vec<String>, ShortTermMemoryError>;
}
