use super::model::{HostExecution, HostRunState};
use crate::domain::agent::{ToolObservation, ToolRequest};
use async_trait::async_trait;
use eliteforge_agent_core as fw;
use eliteforge_agent_official_plugin_tools_local as local;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn official_name(name: &str) -> &str {
    match name {
        "bash_exec" => "system_exec",
        "bash_task_read" => "system_exec_task_read",
        "bash_task_stop" => "system_exec_task_stop",
        _ => name,
    }
}
pub(super) fn official_id(name: &str) -> Option<String> {
    local::local_tool_catalog()
        .into_iter()
        .find(|tool| tool.call_name == official_name(name))
        .map(|tool| tool.id)
}
fn id(name: &str) -> String {
    official_id(name).unwrap_or_else(|| format!("tron.{name}"))
}
fn visible(context: &crate::domain::agent::AgentRunContext) -> Vec<String> {
    context
        .tools
        .iter()
        .map(|tool| id(&tool.tool_name))
        .collect()
}
fn failed(error: impl ToString) -> fw::ToolError {
    fw::ToolError::Failed(error.to_string())
}

pub(super) struct LocalScope;
#[async_trait]
impl local::LocalToolScopeProvider for LocalScope {
    async fn grant(&self, run: &fw::RunContext) -> Result<local::LocalToolGrant, fw::AgentError> {
        if run
            .extensions
            .get::<super::external::ExternalExecution>()
            .is_some()
        {
            return Ok(local::LocalToolGrant {
                terminal_owner: None,
                filesystem: local::FileSystemToolsConfig::new("."),
                commands: local::SystemCommandConfig::restricted(std::iter::empty::<String>()),
                tool_ids: BTreeSet::new(),
                call_names: BTreeMap::new(),
                exposed_tool_ids: None,
            });
        }
        let host = HostExecution::from_extensions(&run.extensions)?;
        let state = host.state.lock().await;
        let mut names = BTreeMap::new();
        let mut ids = BTreeSet::new();
        for tool in state.context.authorized_tools() {
            if let Some(id) = official_id(&tool.tool_name) {
                ids.insert(id.clone());
                names.insert(id, tool.tool_name.clone());
            }
        }
        // No filesystem/command authority is required for a model-only run.
        // The empty grant exposes no executor; never substitute this root for
        // a configured directory when any local capability is actually granted.
        let filesystem = if ids.is_empty() {
            local::FileSystemToolsConfig::new(".")
                .with_authorized_roots(Vec::<std::path::PathBuf>::new())
        } else {
            crate::application::agent::business_tools::framework_local_scope(&state.context)
                .map_err(|error| fw::AgentError::ToolSetup(error.to_string()))?
        };
        let shell = crate::application::shell_program::resolve_user_shell();
        Ok(local::LocalToolGrant {
            terminal_owner: Some(crate::application::agent::terminal_owner(
                &state.context.workspace_code,
                state.context.user_code.as_deref(),
                state
                    .context
                    .conversation_code
                    .as_deref()
                    .unwrap_or(&state.context.run_code),
            )),
            filesystem,
            commands: local::SystemCommandConfig::new().with_shell(shell),
            tool_ids: ids.clone(),
            call_names: names,
            exposed_tool_ids: Some(
                visible(&state.context)
                    .into_iter()
                    .filter(|id| ids.contains(id))
                    .collect(),
            ),
        })
    }
}

pub(super) struct HarnessTools;
#[async_trait]
impl fw::ToolProvider for HarnessTools {
    async fn lease(&self, run: &fw::RunContext) -> Result<fw::ToolLease, fw::AgentError> {
        if run
            .extensions
            .get::<super::external::ExternalExecution>()
            .is_some()
        {
            return Ok(fw::ToolLease::new(vec![]));
        }
        let host = HostExecution::from_extensions(&run.extensions)?;
        let state = host.state.lock().await;
        let leases = host
            .harness
            .framework_registry()
            .take_framework_leases(&state.context.run_code);
        let mut tools = Vec::new();
        let mut resources = Vec::new();
        for mut lease in leases {
            for mut binding in std::mem::take(&mut lease.tools) {
                if !state
                    .context
                    .is_tool_authorized(&binding.descriptor.call_name)
                {
                    continue;
                }
                binding.exposed = state
                    .context
                    .tools
                    .iter()
                    .any(|tool| tool.tool_name == binding.descriptor.call_name);
                tools.push(binding);
            }
            resources.push(lease);
        }
        for tool in state.context.authorized_tools() {
            if official_id(&tool.tool_name).is_some()
                || tools
                    .iter()
                    .any(|binding| binding.descriptor.call_name == tool.tool_name)
            {
                continue;
            }
            let mut binding = fw::ToolBinding::new(
                fw::ToolDescriptor::new(
                    id(&tool.tool_name),
                    &tool.tool_name,
                    &tool.description,
                    tool.input_schema.clone(),
                ),
                HarnessTool {
                    name: tool.tool_name.clone(),
                },
            );
            binding.exposed = state
                .context
                .tools
                .iter()
                .any(|candidate| candidate.tool_name == tool.tool_name);
            // Match Tron's configured parallel execution policy. Catalog authority
            // and side effects remain checked again at the native service boundary.
            binding.parallel_safe = tool.tool_name != crate::application::agent::CHAT_ASK_USER_TOOL;
            tools.push(binding);
        }
        Ok(fw::ToolLease::new(tools).with_shutdown(move || async move {
            for mut lease in resources {
                lease.close(std::time::Duration::from_secs(5)).await;
            }
            Ok(())
        }))
    }
}
struct HarnessTool {
    name: String,
}
#[async_trait]
impl fw::Tool for HarnessTool {
    async fn call(
        &self,
        arguments: Value,
        run: &fw::RunContext,
    ) -> Result<fw::ToolOutput, fw::ToolError> {
        let host = HostExecution::from_extensions(&run.extensions).map_err(failed)?;
        let identity = run
            .extensions
            .get::<fw::ToolCallIdentity>()
            .ok_or_else(|| failed("missing Framework tool identity"))?;
        if self.name == crate::application::agent::CHAT_ASK_USER_TOOL {
            let input = serde_json::from_value(arguments).map_err(failed)?;
            let interaction =
                eliteforge_agent_official_plugin_tools_human::AskUserTool::request(input)
                    .map_err(failed)?;
            let mut question = interaction
                .presentation
                .clone()
                .ok_or_else(|| failed("question presentation is missing"))?;
            question["frameworkInteractionId"] = interaction.id.to_string().into();
            return Ok(fw::ToolOutput::text(question.to_string()).request_input(interaction));
        }
        let context = host.state.lock().await.context.clone();
        let registry = host.harness.framework_registry();
        let harness = host.harness.clone();
        let name = self.name.clone();
        let invocation = identity.invocation_id.clone();
        let activity_id = invocation.clone();
        let execution = tokio::task::spawn_blocking(move || {
            let mut progress = |line: String| {
                harness.framework_tool_activity(&context, &activity_id, &name, &line)
            };
            registry.execute_allowed_with_progress(
                ToolRequest {
                    tool_name: name.clone(),
                    input: arguments,
                    invocation_id: Some(invocation),
                },
                &context,
                &mut progress,
            )
        })
        .await
        .map_err(|error| fw::ToolError::Indeterminate(error.to_string()))?;
        let execution = match execution {
            Ok(result) => result,
            Err(error) => return Ok(fw::ToolOutput::error(error.to_string())),
        };
        let mut state = host.state.lock().await;
        let HostRunState { context, turn, .. } = &mut *state;
        host.harness
            .framework_apply_delta(context, turn, &execution.context_delta)
            .map_err(failed)?;
        let output = fw::ToolOutput::text(execution.observation.output);
        Ok(output)
    }
}

pub(super) struct ToolAudit;
#[async_trait]
impl fw::ToolMiddleware for ToolAudit {
    async fn intercept(
        &self,
        invocation: &fw::ToolInvocationContext<'_>,
        next: fw::ToolNext<'_>,
    ) -> Result<fw::ToolOutput, fw::ToolError> {
        let host = HostExecution::from_extensions(&invocation.run.extensions).map_err(failed)?;
        let mut state = host.state.lock().await;
        state.tool_sequence += 1;
        let sequence = state.tool_sequence;
        state
            .tool_sequences
            .insert(invocation.invocation_id.into(), sequence);
        host.harness
            .framework_tool_started(
                &state.context,
                invocation.invocation_id,
                &invocation.descriptor.call_name,
                &invocation.call.id,
                &invocation.call.arguments,
                sequence,
            )
            .map_err(failed)?;
        let input_error = state
            .context
            .messages
            .iter()
            .rev()
            .flat_map(|message| &message.parts)
            .find_map(|part| {
                if let crate::domain::agent::AgentMessagePart::ToolCall {
                    call_id: Some(id),
                    input_error,
                    ..
                } = part
                {
                    if id == &invocation.call.id {
                        return input_error.clone();
                    }
                }
                None
            });
        drop(state);
        if let Some(error) = input_error {
            return Ok(fw::ToolOutput::error(format!(
                "tool input is invalid: {error}"
            )));
        }
        next.run().await
    }
}

#[async_trait]
impl fw::ToolResultProcessor for ToolAudit {
    async fn process(
        &self,
        invocation: &fw::ToolInvocationContext<'_>,
        mut result: fw::ToolOutput,
    ) -> Result<fw::ToolOutput, fw::AgentError> {
        let host = HostExecution::from_extensions(&invocation.run.extensions)?;
        let mut state = host.state.lock().await;
        let sequence = if let Some(sequence) = state.tool_sequences.get(invocation.invocation_id) {
            *sequence
        } else {
            state.tool_sequence += 1;
            let sequence = state.tool_sequence;
            state
                .tool_sequences
                .insert(invocation.invocation_id.into(), sequence);
            host.harness
                .framework_tool_started(
                    &state.context,
                    invocation.invocation_id,
                    &invocation.call.call_name,
                    &invocation.call.id,
                    &invocation.call.arguments,
                    sequence,
                )
                .map_err(|error| fw::AgentError::Run(error.to_string()))?;
            sequence
        };
        let input_error = state
            .context
            .messages
            .iter()
            .rev()
            .flat_map(|message| &message.parts)
            .find_map(|part| {
                if let crate::domain::agent::AgentMessagePart::ToolCall {
                    call_id: Some(id),
                    input_error,
                    ..
                } = part
                {
                    if id == &invocation.call.id {
                        return input_error.clone();
                    }
                }
                None
            });
        if let Some(error) = input_error {
            result = fw::ToolOutput::error(format!("tool input is invalid: {error}"));
        } else if invocation.descriptor.id == "unavailable" {
            result = fw::ToolOutput::error(format!(
                "tool `{}` is not allowed",
                invocation.call.call_name
            ));
        }
        let output = result.content.protocol_text();
        if result.is_error {
            state.last_tool_failure = Some(format!("{}: {}", invocation.call.call_name, output));
        }
        let HostRunState { context, turn, .. } = &mut *state;
        let message = host
            .harness
            .framework_tool_finished(
                context,
                turn,
                invocation.invocation_id,
                &invocation.call.id,
                sequence,
                ToolObservation {
                    tool_name: invocation.call.call_name.clone(),
                    output,
                },
                result.is_error,
            )
            .map_err(|error| fw::AgentError::Run(error.to_string()))?;
        result.content = super::messages::encode(&message)
            .map_err(fw::AgentError::Run)?
            .content;
        if matches!(result.effect, fw::ToolEffect::Continue) {
            result.effect = fw::ToolEffect::SetExposedTools(visible(context));
        }
        Ok(result)
    }
}

pub(super) struct AttachmentReader(pub crate::application::resource::ResourceService);
#[async_trait]
impl local::FileReferenceReader for AttachmentReader {
    async fn read(
        &self,
        reference: Value,
        max_bytes: usize,
        run: &fw::RunContext,
    ) -> Result<local::ReferencedFile, fw::ToolError> {
        let host = HostExecution::from_extensions(&run.extensions).map_err(failed)?;
        let context = host.state.lock().await.context.clone();
        let resources = self.0.clone();
        tokio::task::spawn_blocking(move || {
            crate::application::agent::business_tools::framework_file_reference(
                reference, &context, &resources, max_bytes,
            )
        })
        .await
        .map_err(failed)?
        .map_err(failed)
    }
}

pub(super) struct CommandProgress {
    pub harness: crate::application::agent::runtime::ReactAgent,
    pub context: crate::domain::agent::AgentRunContext,
}
impl local::CommandProgressSink for CommandProgress {
    fn chunk(&self, run: &fw::RunContext, stream: &str, text: &str) {
        if let Some(identity) = run.extensions.get::<fw::ToolCallIdentity>() {
            let name = self
                .context
                .authorized_tools()
                .iter()
                .find(|tool| {
                    official_id(&tool.tool_name).as_deref() == Some(identity.tool_id.as_str())
                })
                .map(|tool| tool.tool_name.as_str())
                .unwrap_or("bash_exec");
            for line in text.lines() {
                let line = if stream == "stderr" {
                    format!("[stderr] {line}")
                } else {
                    line.into()
                };
                self.harness.framework_tool_activity(
                    &self.context,
                    &identity.invocation_id,
                    name,
                    &line,
                );
            }
        }
    }
}
