use crate::{
    application::{
        chat::ChatService,
        knowledge_quality::{KnowledgeQualityService, QualityOptimizationOptions},
    },
    domain::agent::{
        AgentRunContext, Tool, ToolError, ToolExecutionControl, ToolExecutionFuture,
        ToolExecutionResult, ToolObservation, ToolRequest,
    },
};
use serde_json::{Value, json};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tron_i18n::t;

#[derive(Clone, Copy)]
enum Kind {
    Sources,
    Cases,
    Start,
    Status,
    Cancel,
}
pub fn knowledge_quality_tools(
    service: KnowledgeQualityService,
    chats: ChatService,
) -> Vec<Arc<dyn Tool>> {
    [
        Kind::Sources,
        Kind::Cases,
        Kind::Start,
        Kind::Status,
        Kind::Cancel,
    ]
    .into_iter()
    .map(|kind| {
        Arc::new(QualityTool {
            service: service.clone(),
            chats: chats.clone(),
            kind,
        }) as Arc<dyn Tool>
    })
    .collect()
}
struct QualityTool {
    service: KnowledgeQualityService,
    chats: ChatService,
    kind: Kind,
}
impl QualityTool {
    fn principal(
        &self,
        input: &Value,
        context: &AgentRunContext,
    ) -> Result<(String, Option<String>), ToolError> {
        let locale = context.locale.as_str();
        let user = context
            .user_code
            .clone()
            .filter(|user| !user.is_empty())
            .ok_or_else(|| {
                failure(t!(
                    "agent_runtime.tool.quality.missing_user_identity",
                    locale = locale
                ))
            })?;
        let mut library = input["libraryId"].as_str().map(str::to_owned);
        if let Some(code) = &context.conversation_code {
            let session = self.chats.get_session(code).map_err(failure)?;
            if session.user_code.as_deref() != Some(user.as_str()) {
                return Err(failure(t!(
                    "agent_runtime.tool.quality.no_session_permission",
                    locale = locale
                )));
            }
            if let Some(bound) = session
                .source_code
                .as_deref()
                .and_then(|source| source.strip_prefix("__knowledge:"))
            {
                if library.as_deref().is_some_and(|id| id != bound) {
                    return Err(failure(t!(
                        "agent_runtime.tool.quality.bound_library_only",
                        locale = locale
                    )));
                }
                library = Some(bound.into());
            }
        }
        if let Some(id) = &library {
            self.service.host.owned(&user, id).map_err(failure)?;
        }
        Ok((user, library))
    }
    async fn execute(
        &self,
        user: &str,
        library: Option<String>,
        mut input: Value,
        locale: &str,
    ) -> Result<Value, ToolError> {
        let Some(library) = library else {
            if !matches!(self.kind, Kind::Sources) {
                return Err(failure(t!(
                    "agent_runtime.tool.quality.library_required",
                    locale = locale
                )));
            }
            let libraries=self.service.host.platform_libraries(user).map_err(failure)?.into_iter().map(|library|json!({"id":library.id,"name":library.name,"documentCount":library.document_count})).collect::<Vec<_>>();
            return Ok(
                json!({"libraries":libraries,"strategies":self.service.host.chunking_strategies()}),
            );
        };
        match self.kind {
            Kind::Sources => {
                let offset = input["offset"].as_u64().unwrap_or(0) as usize;
                let limit = input["limit"].as_u64().unwrap_or(12) as usize;
                let (fingerprint, records) = self
                    .service
                    .source_excerpts(user, &library, offset, limit)
                    .await
                    .map_err(failure)?;
                Ok(
                    json!({"sourceFingerprint":fingerprint,"records":records,"nextOffset":offset+limit,"strategies":self.service.host.chunking_strategies()}),
                )
            }
            Kind::Cases => {
                if input.get("cases").is_none() {
                    return serde_json::to_value(
                        self.service.dataset(user, &library).map_err(failure)?,
                    )
                    .map_err(failure);
                }
                let fingerprint = input["sourceFingerprint"]
                    .as_str()
                    .ok_or_else(|| {
                        failure(t!(
                            "agent_runtime.tool.quality.source_fingerprint_required",
                            locale = locale
                        ))
                    })?
                    .to_owned();
                if let Some(cases) = input["cases"].as_array_mut() {
                    for case in cases {
                        case["provenance"] = json!("generated");
                    }
                }
                let cases = serde_json::from_value(input["cases"].clone()).map_err(failure)?;
                serde_json::to_value(
                    self.service
                        .save_dataset(user, &library, &fingerprint, cases)
                        .await
                        .map_err(failure)?,
                )
                .map_err(failure)
            }
            Kind::Start => {
                let request = input["requestId"].as_str().ok_or_else(|| {
                    failure(t!(
                        "agent_runtime.tool.quality.request_id_required",
                        locale = locale
                    ))
                })?;
                let options: QualityOptimizationOptions = serde_json::from_value(
                    input.get("options").cloned().unwrap_or_else(|| json!({})),
                )
                .map_err(failure)?;
                serde_json::to_value(
                    self.service
                        .start(user, &library, request, options)
                        .await
                        .map_err(failure)?,
                )
                .map_err(failure)
            }
            Kind::Status => {
                let Some(id) = input["runId"].as_str() else {
                    return serde_json::to_value(
                        self.service.runs(user, &library).map_err(failure)?,
                    )
                    .map_err(failure);
                };
                let seconds = input["waitSeconds"].as_u64().unwrap_or(0);
                if seconds > 25 {
                    return Err(failure(t!(
                        "agent_runtime.tool.quality.wait_seconds_limit",
                        locale = locale
                    )));
                }
                let deadline = Instant::now() + Duration::from_secs(seconds);
                loop {
                    let run = self.service.run(user, &library, id).map_err(failure)?;
                    if Instant::now() >= deadline
                        || !matches!(
                            run.status.as_str(),
                            "QUEUED" | "RUNNING" | "PUBLISHING" | "CANCEL_REQUESTED"
                        )
                    {
                        return serde_json::to_value(run).map_err(failure);
                    }
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
            Kind::Cancel => {
                let id = input["runId"].as_str().ok_or_else(|| {
                    failure(t!(
                        "agent_runtime.tool.quality.run_id_required",
                        locale = locale
                    ))
                })?;
                serde_json::to_value(self.service.cancel(user, &library, id).map_err(failure)?)
                    .map_err(failure)
            }
        }
    }
}
impl Tool for QualityTool {
    fn name(&self) -> &str {
        match self.kind {
            Kind::Sources => "rag_quality_sources",
            Kind::Cases => "rag_quality_cases",
            Kind::Start => "rag_quality_start",
            Kind::Status => "rag_quality_status",
            Kind::Cancel => "rag_quality_cancel",
        }
    }
    fn exec_async<'a>(
        &'a self,
        request: ToolRequest,
        context: &'a AgentRunContext,
        control: &'a dyn ToolExecutionControl,
    ) -> Option<ToolExecutionFuture<'a>> {
        Some(Box::pin(async move {
            control.check_active()?;
            let (user, library) = self.principal(&request.input, context)?;
            let locale = context.locale.as_str();
            let output = tokio::select! {_ = control.cancelled()=>return Err(failure(t!("agent_runtime.tool.quality.cancelled", locale = locale))),result=tokio::time::timeout(control.remaining(),self.execute(&user,library,request.input,locale))=>result.map_err(|_|failure(t!("agent_runtime.tool.quality.timed_out", locale = locale)))??};
            observation(self.name(), output)
        }))
    }
    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let (user, library) = self.principal(&request.input, context)?;
        let tool = QualityTool {
            service: self.service.clone(),
            chats: self.chats.clone(),
            kind: self.kind,
        };
        let name = self.name().to_owned();
        let locale = context.locale.clone();
        let runtime = tokio::runtime::Handle::try_current().map_err(|_| {
            failure(t!(
                "agent_runtime.tool.quality.async_runtime_required",
                locale = &locale
            ))
        })?;
        let worker_locale = locale.clone();
        let output = std::thread::spawn(move || {
            runtime.block_on(tool.execute(&user, library, request.input, &worker_locale))
        })
        .join()
        .map_err(|_| {
            failure(t!(
                "agent_runtime.tool.quality.worker_failed",
                locale = &locale
            ))
        })??;
        observation(&name, output)
    }
}
fn failure(error: impl std::fmt::Display) -> ToolError {
    ToolError::ExecutionFailed(error.to_string())
}
fn observation(name: &str, output: Value) -> Result<ToolExecutionResult, ToolError> {
    Ok(ToolExecutionResult::observation(ToolObservation {
        tool_name: name.into(),
        output: serde_json::to_string(&output).map_err(failure)?,
    }))
}
