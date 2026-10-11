mod text_patch;
pub use text_patch::{FilePatch, apply_text_hunks, parse_text_patch};
mod agui;
mod application_context;
mod collaboration;
mod context;
mod error;
mod history;
mod kind;
mod management;
mod message;
mod model_config;
mod model_exchange;
mod run;
mod tool;
mod tool_schema;

pub use agui::{
    AgentAguiEvent, AgentAguiEventSnapshot, AgentAguiProtocolEvent, AgentAguiStreamState,
};
pub use application_context::{
    ApplicationDirectoryContext, ApplicationDirectoryStatus, GitWorktreeContext,
};
pub use collaboration::{
    AgentAssignment, AgentExecutionMode, AgentRunPlanStep, CollaborationArtifact,
    CollaborationDecision, CollaborationRun, CollaborationRunStatus, CollaborationStep,
    CollaborationStepExecutionMode, CollaborationStepStatus, HandoffContext,
};
pub use context::{
    AgentCollaborationOwner, AgentExecutionScope, AgentPromptPrefixSource, AgentRunContext,
    AgentRunLimits, AgentShortTermMemoryRunContext, AgentToolExposureState, AutoModelCandidate,
    AutoModelRouteContext, EffectiveSkill, EffectiveTool, EffectiveVariable,
};
pub use error::AgentError;
pub use history::{
    AgentRunHistoryError, AgentRunHistoryQuery, AgentRunHistoryRecord, PagedAgentRunHistoryRecords,
};
pub use kind::{AgentKind, AgentModelProtocol, AgentRuntimeKind};
pub use management::{
    AgentManagementError, AgentRuntimeType, ExpandedDomainPersistenceMode,
    InternalAgentPolicySource, InternalAgentToolPolicy, ManagedAgentSource, ManagedAgentType,
    ManagedSystemAgentRecord, McpAccessMode, PlatformInternalAgentConfig,
    RuntimeAdapterCapabilityRecord, SystemAgentModelRecord, SystemAgentRegistryEntry,
    SystemAgentStatus, SystemAgentSyncResult, ToolAssignmentMode, ToolAvailabilityState,
    ToolCatalogRecord, ToolDomainRecord, ToolExposureMode, ToolOperationKind,
    WorkspaceInternalAgentDefinition, WorkspaceInternalAgentMcpServerAssignment,
    WorkspaceInternalAgentPolicy, WorkspaceInternalAgentToolAssignment, WorkspaceSystemAgentPolicy,
};
pub use message::{AgentMessage, AgentMessagePart};
pub use model_config::AgentModelConfig;
pub use model_exchange::{
    ModelFinish, ModelRequest, ModelResponse, ModelStreamEvent, ModelStreamEventKind,
    ModelTokenUsage, TokenEstimateSnapshot, TokenUsageAuditSnapshot, ToolCall,
};
pub(crate) use model_exchange::{responses_function_call_item_json, responses_input_item_json};
pub use run::{
    AgentChargedUsage, AgentNativeExecutionSnapshot, AgentNativeInteraction,
    AgentNativeInteractionKind, AgentNativeRunOutcome, AgentNativeRunStatus, AgentRunResult,
    AgentRunStatus,
};
pub use tool::{
    InProcessTool, Tool, ToolContextDelta, ToolError, ToolExecutionControl, ToolExecutionFuture,
    ToolExecutionResult, ToolObservation, ToolRegistry, ToolRequest,
};
pub use tool_schema::PreparedToolSchema;

pub mod loop_design;
