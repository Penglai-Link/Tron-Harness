//! Graph tools use trusted run identity and the same use cases as the management API.
use crate::{
    app::AppRagService,
    application::{
        knowledge_base::KnowledgeBaseService,
        rag::{RagIdentity, graph::*},
    },
    domain::agent::{
        AgentRunContext, Tool, ToolError, ToolExecutionControl, ToolExecutionFuture,
        ToolExecutionResult, ToolObservation, ToolRequest,
    },
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{borrow::Cow, sync::Arc};
use tron_i18n::t;

#[derive(Clone, Copy)]
enum Kind {
    Search,
    Expand,
}
#[derive(Clone)]
struct GraphTool {
    rag: AppRagService,
    access: KnowledgeBaseService,
    kind: Kind,
}
pub fn rag_graph_tools(rag: AppRagService, access: KnowledgeBaseService) -> Vec<Arc<dyn Tool>> {
    [Kind::Search, Kind::Expand]
        .into_iter()
        .map(|kind| {
            Arc::new(GraphTool {
                rag: rag.clone(),
                access: access.clone(),
                kind,
            }) as Arc<dyn Tool>
        })
        .collect()
}
fn filters() -> Value {
    json!({})
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SearchArgs {
    query: String,
    #[serde(default)]
    corpus_ids: Vec<String>,
    #[serde(default = "filters")]
    filters: Value,
    #[serde(default)]
    options: GraphReadOptions,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ExpandArgs {
    #[serde(default)]
    node_ids: Vec<String>,
    #[serde(default)]
    corpus_ids: Vec<String>,
    #[serde(default = "filters")]
    filters: Value,
    #[serde(default)]
    options: GraphReadOptions,
    #[serde(default)]
    cursor: Option<String>,
}

impl GraphTool {
    async fn execute(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let locale = context.locale.as_str();
        let invalid = || {
            failure(t!(
                "agent_runtime.tool.graph.invalid_parameters",
                locale = locale
            ))
        };
        if serde_json::to_vec(&request.input)
            .map_err(|_| invalid())?
            .len()
            > 64 * 1024
        {
            return Err(invalid());
        }
        let identity = RagIdentity {
            user_code: context
                .user_code
                .clone()
                .filter(|v| !v.is_empty())
                .ok_or_else(|| {
                    failure(t!(
                        "agent_runtime.tool.graph.missing_user_identity",
                        locale = locale
                    ))
                })?,
            workspace_code: context.execution_workspace_code().to_owned(),
            agent_code: context
                .agent_code
                .clone()
                .filter(|v| !v.is_empty())
                .ok_or_else(|| {
                    failure(t!(
                        "agent_runtime.tool.graph.missing_agent_identity",
                        locale = locale
                    ))
                })?,
        };
        let output = match self.kind {
            Kind::Search => {
                let args: SearchArgs =
                    serde_json::from_value(request.input).map_err(|_| invalid())?;
                self.rag
                    .graph_search(
                        &self.access,
                        &identity,
                        GraphSearchInput {
                            query: args.query,
                            corpus_ids: args.corpus_ids,
                            filters: args.filters,
                            options: args.options,
                        },
                        None,
                    )
                    .await
            }
            Kind::Expand => {
                let args: ExpandArgs =
                    serde_json::from_value(request.input).map_err(|_| invalid())?;
                self.rag
                    .graph_expand(
                        &self.access,
                        &identity,
                        GraphExpandInput {
                            node_ids: args.node_ids,
                            corpus_ids: args.corpus_ids,
                            filters: args.filters,
                            options: args.options,
                            cursor: args.cursor,
                        },
                    )
                    .await
            }
        }
        .map_err(|error| {
            failure(format!(
                "{}: {}",
                error.descriptor().code,
                localized_error(&error, locale)
            ))
        })?;
        let result = GraphToolResult::from(GraphPresentation::from(output));
        Ok(ToolExecutionResult::observation(ToolObservation {
            tool_name: self.name().into(),
            output: serde_json::to_string(&result).map_err(failure)?,
        }))
    }
}
impl Tool for GraphTool {
    fn name(&self) -> &str {
        match self.kind {
            Kind::Search => "rag_graph_search",
            Kind::Expand => "rag_graph_expand",
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
            let locale = context.locale.as_str();
            let result = tokio::select! {
                _=control.cancelled()=>Err(failure(t!("agent_runtime.tool.graph.cancelled", locale = locale))),
                output=tokio::time::timeout(control.remaining(),self.execute(request,context))=>output.map_err(|_|failure(t!("agent_runtime.tool.graph.timed_out", locale = locale)))?,
            };
            control.check_active()?;
            result
        }))
    }
    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let tool = self.clone();
        let context = context.clone();
        std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(failure)?
                .block_on(tool.execute(request, &context))
        })
        .join()
        .map_err(|_| failure("graph query worker failed"))?
    }
}
fn failure(error: impl ToString) -> ToolError {
    ToolError::ExecutionFailed(error.to_string())
}
fn localized_error(error: &GraphQueryError, locale: &str) -> Cow<'static, str> {
    match error {
        GraphQueryError::Invalid => t!("agent_runtime.tool.graph.error.invalid", locale = locale),
        GraphQueryError::Forbidden => {
            t!("agent_runtime.tool.graph.error.forbidden", locale = locale)
        }
        GraphQueryError::Disabled => t!("agent_runtime.tool.graph.error.disabled", locale = locale),
        GraphQueryError::NodeNotFound => {
            t!(
                "agent_runtime.tool.graph.error.node_not_found",
                locale = locale
            )
        }
        GraphQueryError::SnapshotChanged => {
            t!(
                "agent_runtime.tool.graph.error.snapshot_changed",
                locale = locale
            )
        }
        GraphQueryError::Unavailable(_) => {
            t!(
                "agent_runtime.tool.graph.error.unavailable",
                locale = locale
            )
        }
        _ => t!(
            "agent_runtime.tool.graph.error.read_failed",
            locale = locale
        ),
    }
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct GraphToolResult {
    snapshot_id: String,
    seeds: Vec<GraphSeedView>,
    nodes: Vec<GraphNodeView>,
    edges: Vec<GraphEdgeView>,
    references: Vec<GraphSourceReference>,
    stages: Vec<GraphStageView>,
    next_cursor: Option<String>,
    expires_at: Option<i64>,
    unfinished: Vec<GraphUnfinishedPosition>,
    truncated_by: Vec<String>,
    duration_ms: u64,
}
impl From<GraphPresentation> for GraphToolResult {
    fn from(v: GraphPresentation) -> Self {
        Self {
            snapshot_id: v.snapshot_id,
            seeds: v.seeds,
            nodes: v.nodes,
            edges: v.edges,
            references: v.references,
            stages: v.stages,
            next_cursor: v.next_cursor,
            expires_at: v.expires_at,
            unfinished: v.unfinished,
            truncated_by: v.truncated_by,
            duration_ms: v.duration_ms,
        }
    }
}
