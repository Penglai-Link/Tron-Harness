use std::{
    collections::{BTreeMap, HashMap},
    error::Error,
    fmt::{self, Display, Formatter},
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::shared::event_bus::{
    BusEventDraft, BusOutcome, BusRequestRef, EventBusHandle, PublishOutcome,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AgentKind {
    React,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AgentRuntimeKind {
    InProcess,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AgentModelProtocol {
    ChatCompletions,
    Responses,
    Embeddings,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentModelConfig {
    pub model_code: String,
    pub protocol: AgentModelProtocol,
    pub base_url: String,
    pub api_path: Option<String>,
    pub api_key: String,
    pub model_name: String,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f64>,
    pub timeout_ms: Option<u64>,
    pub max_retries: Option<u32>,
}

impl AgentModelConfig {
    pub fn chat_completions_request(&self, request: &ModelRequest) -> Value {
        let mut payload = json!({
            "model": self.model_name,
            "messages": request.messages.iter().map(chat_message_json).collect::<Vec<_>>(),
        });
        if let Some(temperature) = self.temperature {
            payload["temperature"] = json!(temperature);
        }
        if let Some(max_tokens) = self.max_tokens {
            payload["max_tokens"] = json!(max_tokens);
        }
        if let Some(top_p) = self.top_p {
            payload["top_p"] = json!(top_p);
        }
        if !request.tools.is_empty() {
            payload["tools"] = json!(
                request
                    .tools
                    .iter()
                    .map(|tool| json!({
                        "type": "function",
                        "function": {
                            "name": tool.tool_name,
                            "description": tool.description,
                            "parameters": tool.input_schema,
                        }
                    }))
                    .collect::<Vec<_>>()
            );
        }
        payload
    }

    pub fn responses_request(&self, request: &ModelRequest) -> Value {
        let mut payload = json!({
            "model": self.model_name,
            "input": request.messages.iter().map(response_message_json).collect::<Vec<_>>(),
        });
        if let Some(temperature) = self.temperature {
            payload["temperature"] = json!(temperature);
        }
        if let Some(max_tokens) = self.max_tokens {
            payload["max_output_tokens"] = json!(max_tokens);
        }
        if !request.tools.is_empty() {
            payload["tools"] = json!(
                request
                    .tools
                    .iter()
                    .map(|tool| json!({
                        "type": "function",
                        "name": tool.tool_name,
                        "description": tool.description,
                        "parameters": tool.input_schema,
                    }))
                    .collect::<Vec<_>>()
            );
        }
        payload
    }

    pub fn embeddings_request(&self, input: &str) -> Value {
        json!({
            "model": self.model_name,
            "input": input,
        })
    }

    fn endpoint_url(&self, default_path: &str) -> Result<String, AgentError> {
        let path = self.api_path.as_deref().unwrap_or(default_path);
        let base = self.base_url.trim().trim_end_matches('/');
        let path = path.trim().trim_start_matches('/');
        if base.is_empty() || path.is_empty() {
            return Err(AgentError::InvalidConfiguration(
                "model baseUrl and apiPath are required".into(),
            ));
        }
        Ok(format!("{base}/{path}"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveVariable {
    pub key: String,
    pub value: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveSkill {
    pub skill_name: String,
    pub description: String,
    pub instruction: Option<String>,
    pub tool_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveTool {
    pub tool_name: String,
    pub description: String,
    pub input_schema: Value,
    pub skill_name: Option<String>,
    pub required_variable_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AgentExecutionScope {
    pub app_code: Option<String>,
    pub repository_url: Option<String>,
    pub local_directory: Option<String>,
    pub branch_name: Option<String>,
    pub task_code: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunLimits {
    pub max_steps: u32,
    pub max_tool_calls: u32,
}

impl Default for AgentRunLimits {
    fn default() -> Self {
        Self {
            max_steps: 8,
            max_tool_calls: 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMessage {
    pub role: String,
    pub content: String,
}

impl AgentMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: content.into(),
        }
    }

    pub fn tool(content: impl Into<String>) -> Self {
        Self {
            role: "tool".into(),
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunContext {
    pub conversation_code: Option<String>,
    pub run_code: String,
    pub workspace_code: String,
    pub user_code: Option<String>,
    pub request_id: Option<String>,
    pub stream_key: Option<String>,
    pub agent_kind: AgentKind,
    pub model: AgentModelConfig,
    pub variables: Vec<EffectiveVariable>,
    pub skills: Vec<EffectiveSkill>,
    pub tools: Vec<EffectiveTool>,
    pub execution_scope: AgentExecutionScope,
    pub limits: AgentRunLimits,
    pub messages: Vec<AgentMessage>,
    pub observations: Vec<ToolObservation>,
    pub started_at: String,
}

impl AgentRunContext {
    fn allowed_tool(&self, tool_name: &str) -> bool {
        self.tools.iter().any(|tool| tool.tool_name == tool_name)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRequest {
    pub messages: Vec<AgentMessage>,
    pub tools: Vec<EffectiveTool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    pub tool_name: String,
    pub input: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelResponse {
    FinalOutput(String),
    ToolCall(ToolCall),
}

impl ModelResponse {
    pub fn final_output(output: impl Into<String>) -> Self {
        Self::FinalOutput(output.into())
    }

    pub fn tool_call(tool_name: impl Into<String>, input: Value) -> Self {
        Self::ToolCall(ToolCall {
            tool_name: tool_name.into(),
            input,
        })
    }
}

pub trait ModelClient: Send + Sync {
    fn generate<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        request: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>>;
}

#[derive(Debug, Clone, Default)]
pub struct OpenAiCompatibleModelClient {
    client: reqwest::Client,
}

impl ModelClient for OpenAiCompatibleModelClient {
    fn generate<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        request: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async move {
            match config.protocol {
                AgentModelProtocol::ChatCompletions => self.chat_completions(config, request).await,
                AgentModelProtocol::Responses => self.responses(config, request).await,
                AgentModelProtocol::Embeddings => {
                    Err("embedding models cannot generate text".into())
                }
            }
        })
    }
}

impl OpenAiCompatibleModelClient {
    async fn chat_completions(
        &self,
        config: &AgentModelConfig,
        request: &ModelRequest,
    ) -> Result<ModelResponse, String> {
        let url = config
            .endpoint_url("chat/completions")
            .map_err(|error| error.to_string())?;
        let response = self
            .client
            .post(url)
            .bearer_auth(&config.api_key)
            .json(&config.chat_completions_request(request))
            .send()
            .await
            .map_err(|error| error.to_string())?;
        let status = response.status();
        let body: Value = response.json().await.map_err(|error| error.to_string())?;
        if !status.is_success() {
            return Err(format!("model provider returned {status}"));
        }
        parse_chat_response(&body).ok_or_else(|| "model response was unsupported".into())
    }

    async fn responses(
        &self,
        config: &AgentModelConfig,
        request: &ModelRequest,
    ) -> Result<ModelResponse, String> {
        let url = config
            .endpoint_url("responses")
            .map_err(|error| error.to_string())?;
        let response = self
            .client
            .post(url)
            .bearer_auth(&config.api_key)
            .json(&config.responses_request(request))
            .send()
            .await
            .map_err(|error| error.to_string())?;
        let status = response.status();
        let body: Value = response.json().await.map_err(|error| error.to_string())?;
        if !status.is_success() {
            return Err(format!("model provider returned {status}"));
        }
        parse_responses_response(&body).ok_or_else(|| "model response was unsupported".into())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolRequest {
    pub tool_name: String,
    pub input: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolObservation {
    pub tool_name: String,
    pub output: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolError {
    MissingTool(String),
    UnauthorizedTool(String),
    ExecutionFailed(String),
}

impl Display for ToolError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingTool(tool) => write!(f, "tool `{tool}` is not registered"),
            Self::UnauthorizedTool(tool) => write!(f, "tool `{tool}` is not allowed"),
            Self::ExecutionFailed(message) => f.write_str(message),
        }
    }
}

impl Error for ToolError {}

pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn execute(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolObservation, ToolError>;
}

pub struct InProcessTool<F>
where
    F: Fn(ToolRequest) -> Result<ToolObservation, ToolError> + Send + Sync + 'static,
{
    name: String,
    handler: F,
}

impl<F> InProcessTool<F>
where
    F: Fn(ToolRequest) -> Result<ToolObservation, ToolError> + Send + Sync + 'static,
{
    pub fn new(name: impl Into<String>, handler: F) -> Self {
        Self {
            name: name.into(),
            handler,
        }
    }
}

impl<F> Tool for InProcessTool<F>
where
    F: Fn(ToolRequest) -> Result<ToolObservation, ToolError> + Send + Sync + 'static,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn execute(
        &self,
        request: ToolRequest,
        _context: &AgentRunContext,
    ) -> Result<ToolObservation, ToolError> {
        (self.handler)(request)
    }
}

#[derive(Default)]
pub struct ToolRegistry {
    tools: Mutex<HashMap<String, Arc<dyn Tool>>>,
}

impl ToolRegistry {
    pub fn register(&self, tool: Arc<dyn Tool>) -> Result<(), ToolError> {
        let name = tool.name().trim();
        if name.is_empty() {
            return Err(ToolError::ExecutionFailed("tool name is required".into()));
        }
        self.tools
            .lock()
            .expect("tool registry mutex poisoned")
            .insert(name.to_string(), tool);
        Ok(())
    }

    pub fn execute_allowed(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolObservation, ToolError> {
        if !context.allowed_tool(&request.tool_name) {
            return Err(ToolError::UnauthorizedTool(request.tool_name));
        }
        let tool = self
            .tools
            .lock()
            .expect("tool registry mutex poisoned")
            .get(&request.tool_name)
            .cloned()
            .ok_or_else(|| ToolError::MissingTool(request.tool_name.clone()))?;
        tool.execute(request, context)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentError {
    InvalidConfiguration(String),
    ModelFailure(String),
    MissingTool(String),
    UnauthorizedTool(String),
    ToolFailure(String),
    MaxStepsExceeded,
    EventPublishFailed(String),
}

impl Display for AgentError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfiguration(message) => f.write_str(message),
            Self::ModelFailure(message) => write!(f, "model failure: {message}"),
            Self::MissingTool(tool) => write!(f, "missing tool: {tool}"),
            Self::UnauthorizedTool(tool) => write!(f, "unauthorized tool: {tool}"),
            Self::ToolFailure(message) => write!(f, "tool failure: {message}"),
            Self::MaxStepsExceeded => f.write_str("agent run exceeded max steps"),
            Self::EventPublishFailed(message) => write!(f, "event publish failed: {message}"),
        }
    }
}

impl Error for AgentError {}

impl From<ToolError> for AgentError {
    fn from(value: ToolError) -> Self {
        match value {
            ToolError::MissingTool(tool) => Self::MissingTool(tool),
            ToolError::UnauthorizedTool(tool) => Self::UnauthorizedTool(tool),
            ToolError::ExecutionFailed(message) => Self::ToolFailure(message),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunResult {
    pub run_code: String,
    pub conversation_code: Option<String>,
    pub workspace_code: String,
    pub final_output: String,
    pub step_count: u32,
    pub observations: Vec<ToolObservation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AgentRunStatus {
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunHistoryRecord {
    pub id: i64,
    pub run_code: String,
    pub conversation_code: Option<String>,
    pub workspace_code: String,
    pub user_code: Option<String>,
    pub app_code: Option<String>,
    pub task_code: Option<String>,
    pub model_code: String,
    pub status: AgentRunStatus,
    pub outcome: String,
    pub step_count: u32,
    pub observation_count: u32,
    pub output_preview: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub started_at: String,
    pub finished_at: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PagedAgentRunHistoryRecords<T> {
    pub current: usize,
    pub size: usize,
    pub total: usize,
    pub records: Vec<T>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunHistoryQuery {
    pub workspace_code: Option<String>,
    pub conversation_code: Option<String>,
    pub current: Option<usize>,
    pub size: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentRunHistoryError {
    StorageFailure,
    RunNotFound,
}

impl Display for AgentRunHistoryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::StorageFailure => f.write_str("agent run history storage failure"),
            Self::RunNotFound => f.write_str("agent run was not found"),
        }
    }
}

impl Error for AgentRunHistoryError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentAguiStreamState {
    pub stream_key: String,
    pub active_streams: usize,
    pub total_streams: usize,
    pub total_replay_bytes: usize,
    pub replay_evicted: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentAguiEventSnapshot {
    pub stream_key: String,
    pub active_streams: usize,
    pub total_streams: usize,
    pub events: Vec<AgentAguiEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentAguiEvent {
    pub id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub event_name: String,
    pub run_code: Option<String>,
    pub stream_key: String,
    pub occurred_at: String,
    pub outcome: String,
    pub metadata: BTreeMap<String, String>,
}

pub struct ReactAgentBuilder {
    model_client: Arc<dyn ModelClient>,
    tool_registry: Arc<ToolRegistry>,
    event_bus: EventBusHandle,
}

impl ReactAgentBuilder {
    pub fn new(
        model_client: Arc<dyn ModelClient>,
        tool_registry: Arc<ToolRegistry>,
        event_bus: EventBusHandle,
    ) -> Self {
        Self {
            model_client,
            tool_registry,
            event_bus,
        }
    }

    pub fn build(self) -> Result<ReactAgent, AgentError> {
        Ok(ReactAgent {
            model_client: self.model_client,
            tool_registry: self.tool_registry,
            event_bus: self.event_bus,
        })
    }
}

#[derive(Clone)]
pub struct ReactAgent {
    model_client: Arc<dyn ModelClient>,
    tool_registry: Arc<ToolRegistry>,
    event_bus: EventBusHandle,
}

impl ReactAgent {
    pub async fn run(&self, mut context: AgentRunContext) -> Result<AgentRunResult, AgentError> {
        validate_context(&context)?;
        self.publish(&context, "agent.run.started", BusOutcome::Started, None)?;
        let mut step_count = 0;
        let mut tool_calls = 0;

        loop {
            if step_count >= context.limits.max_steps {
                self.publish(
                    &context,
                    "agent.run.failed",
                    BusOutcome::Failure,
                    Some("max_steps_exceeded"),
                )?;
                return Err(AgentError::MaxStepsExceeded);
            }
            step_count += 1;
            let request = model_request_from_context(&context);
            self.publish(
                &context,
                "agent.model.requested",
                BusOutcome::Started,
                Some("model_requested"),
            )?;
            let response = match self.model_client.generate(&context.model, &request).await {
                Ok(response) => response,
                Err(message) => {
                    self.publish(
                        &context,
                        "agent.run.failed",
                        BusOutcome::Failure,
                        Some("model_failure"),
                    )?;
                    return Err(AgentError::ModelFailure(message));
                }
            };
            self.publish(
                &context,
                "agent.model.responded",
                BusOutcome::Completed,
                Some("model_responded"),
            )?;

            match response {
                ModelResponse::FinalOutput(output) => {
                    self.publish(
                        &context,
                        "agent.output.created",
                        BusOutcome::Completed,
                        Some("output_created"),
                    )?;
                    self.publish(
                        &context,
                        "agent.run.completed",
                        BusOutcome::Completed,
                        Some("run_completed"),
                    )?;
                    return Ok(AgentRunResult {
                        run_code: context.run_code,
                        conversation_code: context.conversation_code,
                        workspace_code: context.workspace_code,
                        final_output: output,
                        step_count,
                        observations: context.observations,
                    });
                }
                ModelResponse::ToolCall(call) => {
                    if tool_calls >= context.limits.max_tool_calls {
                        self.publish(
                            &context,
                            "agent.run.failed",
                            BusOutcome::Failure,
                            Some("max_tool_calls_exceeded"),
                        )?;
                        return Err(AgentError::MaxStepsExceeded);
                    }
                    tool_calls += 1;
                    self.publish(
                        &context,
                        "agent.tool.selected",
                        BusOutcome::Progress,
                        Some(call.tool_name.as_str()),
                    )?;
                    self.publish(
                        &context,
                        "agent.tool.started",
                        BusOutcome::Started,
                        Some(call.tool_name.as_str()),
                    )?;
                    match self.tool_registry.execute_allowed(
                        ToolRequest {
                            tool_name: call.tool_name,
                            input: call.input,
                        },
                        &context,
                    ) {
                        Ok(observation) => {
                            self.publish(
                                &context,
                                "agent.tool.completed",
                                BusOutcome::Completed,
                                Some(observation.tool_name.as_str()),
                            )?;
                            context.messages.push(AgentMessage::tool(format!(
                                "tool:{}",
                                observation.tool_name
                            )));
                            context.observations.push(observation);
                            self.publish(
                                &context,
                                "agent.observation.created",
                                BusOutcome::Progress,
                                Some("observation_created"),
                            )?;
                        }
                        Err(error) => {
                            self.publish(
                                &context,
                                "agent.tool.failed",
                                BusOutcome::Failure,
                                Some("tool_failed"),
                            )?;
                            self.publish(
                                &context,
                                "agent.run.failed",
                                BusOutcome::Failure,
                                Some("tool_failure"),
                            )?;
                            return Err(error.into());
                        }
                    }
                }
            }
        }
    }

    fn publish(
        &self,
        context: &AgentRunContext,
        event_name: &'static str,
        outcome: BusOutcome,
        summary: Option<&str>,
    ) -> Result<(), AgentError> {
        let mut draft = BusEventDraft::agui(event_name, "agent_runtime")
            .outcome(outcome)
            .subject("agent_run", context.run_code.clone())
            .attr("workspace_code", context.workspace_code.clone())
            .attr("agent_kind", "react")
            .attr("runtime_kind", "in_process")
            .attr("model_code", context.model.model_code.clone());
        if let Some(summary) = summary {
            draft = draft.summary(summary);
        }
        if let Some(conversation_code) = &context.conversation_code {
            draft = draft.attr("conversation_code", conversation_code.clone());
        }
        if let Some(user_code) = &context.user_code {
            draft = draft.attr("user_code", user_code.clone());
        }
        if let Some(app_code) = &context.execution_scope.app_code {
            draft = draft.attr("app_code", app_code.clone());
        }
        if let Some(task_code) = &context.execution_scope.task_code {
            draft = draft.attr("task_code", task_code.clone());
        }
        if let Some(stream_key) = &context.stream_key {
            draft = draft.request(BusRequestRef::with_stream_key(stream_key.clone()));
        }

        match self.event_bus.publish(draft) {
            PublishOutcome::Disabled | PublishOutcome::Accepted { .. } => Ok(()),
            outcome => Err(AgentError::EventPublishFailed(format!("{outcome:?}"))),
        }
    }
}

fn validate_context(context: &AgentRunContext) -> Result<(), AgentError> {
    if context.run_code.trim().is_empty() {
        return Err(AgentError::InvalidConfiguration(
            "runCode is required".into(),
        ));
    }
    if context.workspace_code.trim().is_empty() {
        return Err(AgentError::InvalidConfiguration(
            "workspaceCode is required".into(),
        ));
    }
    if context.model.model_name.trim().is_empty() {
        return Err(AgentError::InvalidConfiguration(
            "modelName is required".into(),
        ));
    }
    if context.limits.max_steps == 0 {
        return Err(AgentError::InvalidConfiguration(
            "maxSteps must be greater than zero".into(),
        ));
    }
    Ok(())
}

fn model_request_from_context(context: &AgentRunContext) -> ModelRequest {
    let mut messages = context.messages.clone();
    for observation in &context.observations {
        messages.push(AgentMessage::tool(format!(
            "tool:{}",
            observation.tool_name
        )));
    }
    ModelRequest {
        messages,
        tools: context.tools.clone(),
    }
}

fn chat_message_json(message: &AgentMessage) -> Value {
    json!({
        "role": message.role,
        "content": message.content,
    })
}

fn response_message_json(message: &AgentMessage) -> Value {
    json!({
        "role": message.role,
        "content": message.content,
    })
}

fn parse_chat_response(body: &Value) -> Option<ModelResponse> {
    let message = body.get("choices")?.as_array()?.first()?.get("message")?;
    if let Some(tool_call) = message
        .get("tool_calls")
        .and_then(Value::as_array)
        .and_then(|calls| calls.first())
    {
        let function = tool_call.get("function")?;
        let name = function.get("name")?.as_str()?;
        let arguments = function
            .get("arguments")
            .and_then(Value::as_str)
            .and_then(|raw| serde_json::from_str(raw).ok())
            .unwrap_or_else(|| json!({}));
        return Some(ModelResponse::tool_call(name, arguments));
    }
    message
        .get("content")
        .and_then(Value::as_str)
        .map(ModelResponse::final_output)
}

fn parse_responses_response(body: &Value) -> Option<ModelResponse> {
    if let Some(text) = body.get("output_text").and_then(Value::as_str) {
        return Some(ModelResponse::final_output(text));
    }
    let output = body.get("output")?.as_array()?.first()?;
    if output.get("type").and_then(Value::as_str) == Some("function_call") {
        let name = output.get("name")?.as_str()?;
        let arguments = output
            .get("arguments")
            .and_then(Value::as_str)
            .and_then(|raw| serde_json::from_str(raw).ok())
            .unwrap_or_else(|| json!({}));
        return Some(ModelResponse::tool_call(name, arguments));
    }
    output
        .get("content")
        .and_then(Value::as_array)
        .and_then(|content| content.first())
        .and_then(|item| item.get("text"))
        .and_then(Value::as_str)
        .map(ModelResponse::final_output)
}
