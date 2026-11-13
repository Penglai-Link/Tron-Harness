use crate::{
    application::{memory::*, rag::*},
    domain::agent::{
        AgentRunContext, Tool, ToolError, ToolExecutionResult, ToolObservation, ToolRequest,
    },
};
use serde::Deserialize;
use std::{sync::Arc, thread};
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
        "dude_memory_search",
        "dude_memory_get",
        "dude_memory_save",
        "dude_memory_delete",
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
        let en = ctx.locale.starts_with("en");
        let fail = |e: MemoryError| {
            ToolError::ExecutionFailed(match e {
                MemoryError::Conflict(_) => if en {
                    "Memory changed; get the current version before editing."
                } else {
                    "记忆已变更，请读取当前版本后编辑。"
                }
                .into(),
                MemoryError::NotFound(_) => if en {
                    "Memory not found."
                } else {
                    "记忆不存在。"
                }
                .into(),
                MemoryError::Forbidden(_) => if en {
                    "Memory scope is outside this run."
                } else {
                    "记忆范围超出当前运行授权。"
                }
                .into(),
                MemoryError::Validation(_) => if en {
                    "Invalid memory content, scope or version."
                } else {
                    "记忆内容、范围或版本无效。"
                }
                .into(),
                _ => if en {
                    "Memory operation failed; retry or inspect diagnostics."
                } else {
                    "记忆操作失败，请重试或查看诊断。"
                }
                .into(),
            })
        };
        let user = ctx
            .user_code
            .as_deref()
            .filter(|u| !u.is_empty())
            .ok_or_else(|| fail(MemoryError::Forbidden(String::new())))?;
        let dude = self.name.starts_with("dude_");
        if dude != ctx.is_owning_dude() {
            return Err(fail(MemoryError::Forbidden(String::new())));
        }
        let c = MemoryRunContext::new(
            user,
            if dude {
                ""
            } else {
                ctx.execution_workspace_code()
            },
        );
        let name = self.name.strip_prefix("dude_").unwrap_or(self.name);
        let value=match name {
            "memory_search"=>{
                let cmd:MemorySearchCommand=parse_input(request.input,en)?;
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
            "memory_get"=>{let cmd:GetInput=parse_input(request.input,en)?;serde_json::to_value(self.memory.get(&c,&cmd.memory_code).map_err(fail)?)},
            "memory_save"=>{
                let cmd:SaveInput=parse_input(request.input,en)?;
                let source=serde_json::json!({"kind":"AgentTool","sessionCode":ctx.conversation_code,"runCode":ctx.run_code,"userCode":user});
                let cmd=MemorySaveCommand{memory_code:cmd.memory_code,version:cmd.version,scope:cmd.scope,scope_code:cmd.scope_code,title:cmd.title,text:cmd.text,enabled:cmd.enabled,source_label:"AgentTool".into(),source_refs:vec![source],..Default::default()};
                serde_json::to_value(self.memory.save(c,cmd).await.map_err(fail)?)
            },
            _=>{let cmd:DeleteInput=parse_input(request.input,en)?;serde_json::to_value(self.memory.delete(c,MemoryDeleteCommand{memory_code:cmd.memory_code,version:cmd.version}).await.map_err(fail)?)},
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
                _=control.cancelled()=>{control.check_active()?;return Err(ToolError::ExecutionFailed(if ctx.locale.starts_with("en"){"Memory request cancelled"}else{"记忆请求已取消"}.into()))},
                result=tokio::time::timeout(control.remaining(),self.execute(request,ctx))=>result.map_err(|_|ToolError::ExecutionFailed(if ctx.locale.starts_with("en"){"Memory request timed out"}else{"记忆请求超时"}.into()))??,
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
        let ctx = ctx.clone();
        thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?
                .block_on(tool.execute(request, &ctx))
        })
        .join()
        .map_err(|_| ToolError::ExecutionFailed("memory worker failed".into()))?
    }
}
fn parse_input<T: for<'de> Deserialize<'de>>(
    input: serde_json::Value,
    en: bool,
) -> Result<T, ToolError> {
    serde_json::from_value(input).map_err(|_| {
        ToolError::ExecutionFailed(
            if en {
                "Invalid memory tool parameters"
            } else {
                "记忆工具参数无效"
            }
            .into(),
        )
    })
}
