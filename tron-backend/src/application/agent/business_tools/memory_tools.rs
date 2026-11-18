use crate::{
    application::{memory::*, rag::*},
    domain::agent::{
        AgentRunContext, Tool, ToolError, ToolExecutionResult, ToolObservation, ToolRequest,
    },
};
use serde::Deserialize;
use std::{sync::Arc, thread};
use tron_i18n::t;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SaveInput {
    memory_code: Option<String>,
    version: Option<u32>,
    scope: Option<MemoryScope>,
    scope_code: Option<String>,
    title: Option<String>,
    text: Option<String>,
    enabled: Option<bool>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GetInput {
    memory_code: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DeleteInput {
    memory_code: String,
    version: u32,
}
#[derive(Clone)]
struct MemoryTool<R, E> {
    memory: MemoryService<R, E>,
    name: &'static str,
}
pub(super) fn memory_business_tools<R, E>(memory: MemoryService<R, E>) -> Vec<Arc<dyn Tool>>
where
    R: RagIngestRepository
        + RagSearchRepository
        + RagDiagnosticsRepository
        + Clone
        + Send
        + Sync
        + 'static,
    E: RagEmbeddingRuntime
        + crate::application::rag::RagGraphRuntime
        + RagSearchRuntime
        + Send
        + Sync
        + 'static,
{
    [
        "memory_search",
        "memory_get",
        "memory_save",
        "memory_delete",
        "assistant_memory_search",
        "assistant_memory_get",
        "assistant_memory_save",
        "assistant_memory_delete",
    ]
    .into_iter()
    .map(|name| {
        Arc::new(MemoryTool {
            memory: memory.clone(),
            name,
        }) as Arc<dyn Tool>
    })
    .collect()
}
impl<R, E> MemoryTool<R, E>
where
    R: RagIngestRepository
        + RagSearchRepository
        + RagDiagnosticsRepository
        + Clone
        + Send
        + Sync
        + 'static,
    E: RagEmbeddingRuntime
        + crate::application::rag::RagGraphRuntime
        + RagSearchRuntime
        + Send
        + Sync
        + 'static,
{
    async fn execute(
        &self,
        request: ToolRequest,
        ctx: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let locale = ctx.locale.as_str();
        let fail = |e: MemoryError| {
            ToolError::ExecutionFailed(match e {
                MemoryError::Conflict(_) => {
                    t!("agent_runtime.tool.memory.error.conflict", locale = locale).into_owned()
                }
                MemoryError::NotFound(_) => {
                    t!("agent_runtime.tool.memory.error.not_found", locale = locale).into_owned()
                }
                MemoryError::Forbidden(_) => {
                    t!("agent_runtime.tool.memory.error.forbidden", locale = locale).into_owned()
                }
                MemoryError::Validation(_) => t!(
                    "agent_runtime.tool.memory.error.validation",
                    locale = locale
                )
                .into_owned(),
                _ => t!("agent_runtime.tool.memory.error.failed", locale = locale).into_owned(),
            })
        };
        let user = ctx
            .user_code
            .as_deref()
            .filter(|u| !u.is_empty())
            .ok_or_else(|| fail(MemoryError::Forbidden(String::new())))?;
        let assistant = self.name.starts_with("assistant_");
        if assistant != ctx.is_owning_assistant() {
            return Err(fail(MemoryError::Forbidden(String::new())));
        }
        let c = MemoryRunContext::new(
            user,
            if assistant {
                ""
            } else {
                ctx.execution_workspace_code()
            },
        );
        let name = self.name.strip_prefix("assistant_").unwrap_or(self.name);
        let value=match name {
            "memory_search"=>{
                let cmd:MemorySearchCommand=parse_input(request.input,locale)?;
                let binding=self.memory.bind_backend(c.clone(),cmd.scope,cmd.scope_code);
                let query=eliteforge_agent_core::MemoryQuery{text:cmd.query,limit:cmd.limit.unwrap_or(8),max_bytes:2*1024*1024};
                let entries=binding.backend.search(&binding.namespace,&query).await.map_err(|e|ToolError::ExecutionFailed(e.to_string()))?;
                let mut items=vec![];
                let codes=entries.iter().filter_map(|entry|entry.version.rsplit_once(':').map(|(code,_)|code.to_owned())).collect::<Vec<_>>();
                let records=self.memory.recallable_records(&c,&codes).map_err(fail)?;
                for entry in entries {
                    let Some((code,version))=entry.version.rsplit_once(':') else {continue};
                    let Ok(version)=version.parse::<u32>() else {continue};
                    let Some(r)=records.get(code).filter(|r|r.version==version) else {continue};
                    items.push(MemoryItem{memory_code:r.memory_code.clone(),scope:r.scope,scope_code:r.scope_code.clone(),title:r.title.clone(),text:entry.value.content,source_refs:r.source_refs.clone(),source_label:r.source_label.clone(),version:r.version});
                }
                let current=self.memory.recallable_records(&c,&codes).map_err(fail)?;
                items.retain(|item|current.get(&item.memory_code).is_some_and(|r|r.version==item.version));
                let warnings=binding.diagnostics.lock().map(|v|v.clone()).unwrap_or_default();
                serde_json::to_value(MemorySearchResponse{items,warnings})
            },
            "memory_get"=>{let cmd:GetInput=parse_input(request.input,locale)?;serde_json::to_value(self.memory.get(&c,&cmd.memory_code).map_err(fail)?)},
            "memory_save"=>{
                let cmd:SaveInput=parse_input(request.input,locale)?;
                let source=serde_json::json!({"kind":"AgentTool","sessionCode":ctx.conversation_code,"runCode":ctx.run_code,"userCode":user});
                let cmd=MemorySaveCommand{memory_code:cmd.memory_code,version:cmd.version,scope:cmd.scope,scope_code:cmd.scope_code,title:cmd.title,text:cmd.text,enabled:cmd.enabled,source_label:"AgentTool".into(),source_refs:vec![source],..Default::default()};
                serde_json::to_value(self.memory.save(c,cmd).await.map_err(fail)?)
            },
            _=>{let cmd:DeleteInput=parse_input(request.input,locale)?;serde_json::to_value(self.memory.delete(c,MemoryDeleteCommand{memory_code:cmd.memory_code,version:cmd.version}).await.map_err(fail)?)},
        }.map_err(|e|ToolError::ExecutionFailed(e.to_string()))?;
        Ok(ToolExecutionResult::observation(ToolObservation {
            tool_name: self.name.into(),
            output: serde_json::to_string(&value)
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?,
        }))
    }
}
impl<R, E> Tool for MemoryTool<R, E>
where
    R: RagIngestRepository
        + RagSearchRepository
        + RagDiagnosticsRepository
        + Clone
        + Send
        + Sync
        + 'static,
    E: RagEmbeddingRuntime
        + crate::application::rag::RagGraphRuntime
        + RagSearchRuntime
        + Send
        + Sync
        + 'static,
{
    fn name(&self) -> &str {
        self.name
    }
    fn exec_async<'a>(
        &'a self,
        request: ToolRequest,
        ctx: &'a AgentRunContext,
        control: &'a dyn crate::domain::agent::ToolExecutionControl,
    ) -> Option<crate::domain::agent::ToolExecutionFuture<'a>> {
        Some(Box::pin(async move {
            control.check_active()?;
            let output = tokio::select! {
                _=control.cancelled()=>{control.check_active()?;return Err(ToolError::ExecutionFailed(t!("agent_runtime.tool.memory.cancelled",locale=ctx.locale.as_str()).into_owned()))},
                result=tokio::time::timeout(control.remaining(),self.execute(request,ctx))=>result.map_err(|_|ToolError::ExecutionFailed(t!("agent_runtime.tool.memory.timed_out",locale=ctx.locale.as_str()).into_owned()))??,
            };
            control.check_active()?;
            Ok(output)
        }))
    }
    fn exec(
        &self,
        request: ToolRequest,
        ctx: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let tool = self.clone();
        let locale = ctx.locale.clone();
        let ctx = ctx.clone();
        thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?
                .block_on(tool.execute(request, &ctx))
        })
        .join()
        .map_err(|_| {
            ToolError::ExecutionFailed(
                t!("agent_runtime.tool.memory.worker_failed", locale = &locale).into_owned(),
            )
        })?
    }
}
fn parse_input<T: for<'de> Deserialize<'de>>(
    input: serde_json::Value,
    locale: &str,
) -> Result<T, ToolError> {
    serde_json::from_value(input).map_err(|_| {
        ToolError::ExecutionFailed(
            t!(
                "agent_runtime.tool.memory.invalid_parameters",
                locale = locale
            )
            .into_owned(),
        )
    })
}
