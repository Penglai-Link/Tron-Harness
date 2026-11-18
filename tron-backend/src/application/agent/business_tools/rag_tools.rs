use crate::{
    app::AppRagService,
    application::{
        knowledge_base::KnowledgeBaseService,
        rag::{RagIdentity, RagQueryInput},
    },
    domain::agent::{
        AgentRunContext, Tool, ToolError, ToolExecutionControl, ToolExecutionFuture,
        ToolExecutionResult, ToolObservation, ToolRequest,
    },
};
use std::sync::Arc;
use tron_i18n::t;
pub fn rag_search_tool(rag: AppRagService, access: KnowledgeBaseService) -> Arc<dyn Tool> {
    Arc::new(RagSearchTool { rag, access })
}
struct RagSearchTool {
    rag: AppRagService,
    access: KnowledgeBaseService,
}
impl Tool for RagSearchTool {
    fn exec_async<'a>(
        &'a self,
        request: ToolRequest,
        context: &'a AgentRunContext,
        control: &'a dyn ToolExecutionControl,
    ) -> Option<ToolExecutionFuture<'a>> {
        Some(Box::pin(async move {
            control.check_active()?;
            let locale = context.locale.as_str();
            let (input, identity) = search_input(request, context)?;
            let result = tokio::select! {
                _ = control.cancelled() => return Err(ToolError::ExecutionFailed(t!("agent_runtime.tool.rag.cancelled", locale = locale).into_owned())),
                result = tokio::time::timeout(control.remaining(), self.rag.search(&self.access, &identity, input, None)) =>
                    result.map_err(|_| ToolError::ExecutionFailed(t!("agent_runtime.tool.rag.timed_out", locale = locale).into_owned()))?.map_err(|error| ToolError::ExecutionFailed(error.to_string()))?,
            };
            control.check_active()?;
            observation(self.name(), &result)
        }))
    }
    fn name(&self) -> &str {
        "rag_search"
    }
    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let (input, identity) = search_input(request, context)?;
        let rag = self.rag.clone();
        let access = self.access.clone();
        let output = std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?
                .block_on(rag.search(&access, &identity, input, None))
                .map_err(|error| error.to_string())
        })
        .join()
        .map_err(|_| {
            ToolError::ExecutionFailed(
                t!(
                    "agent_runtime.tool.rag.worker_failed",
                    locale = context.locale.as_str()
                )
                .into_owned(),
            )
        })?
        .map_err(ToolError::ExecutionFailed)?;
        observation(self.name(), &output)
    }
}

fn search_input(
    request: ToolRequest,
    context: &AgentRunContext,
) -> Result<(RagQueryInput, RagIdentity), ToolError> {
    let input: RagQueryInput = serde_json::from_value(request.input)
        .map_err(|error| ToolError::ExecutionFailed(error.to_string()))?;
    let identity = RagIdentity {
        user_code: context.user_code.clone().ok_or_else(|| {
            ToolError::ExecutionFailed(
                t!(
                    "agent_runtime.tool.rag.missing_user_identity",
                    locale = context.locale.as_str()
                )
                .into_owned(),
            )
        })?,
        workspace_code: context.execution_workspace_code().to_owned(),
        agent_code: context.agent_code.clone().ok_or_else(|| {
            ToolError::ExecutionFailed(
                t!(
                    "agent_runtime.tool.rag.missing_agent_identity",
                    locale = context.locale.as_str()
                )
                .into_owned(),
            )
        })?,
    };
    Ok((input, identity))
}
fn observation(
    name: &str,
    output: &impl serde::Serialize,
) -> Result<ToolExecutionResult, ToolError> {
    Ok(ToolExecutionResult::observation(ToolObservation {
        tool_name: name.into(),
        output: serde_json::to_string(output)
            .map_err(|error| ToolError::ExecutionFailed(error.to_string()))?,
    }))
}
