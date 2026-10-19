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
use std::sync::Arc;

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
        let english = context.locale.starts_with("en");
        let invalid = || {
            failure(if english {
                "Invalid graph query parameters"
            } else {
                "图谱查询参数无效"
            })
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
                    failure(if english {
                        "Missing trusted user identity"
                    } else {
                        "缺少可信用户身份"
                    })
                })?,
            workspace_code: context.workspace_code.clone(),
            agent_code: context
                .agent_code
                .clone()
                .filter(|v| !v.is_empty())
                .ok_or_else(|| {
                    failure(if english {
                        "Missing trusted Agent identity"
                    } else {
                        "缺少可信智能体身份"
                    })
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
                localized_error(&error, english)
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
            let english = context.locale.starts_with("en");
            let result = tokio::select! {
                _=control.cancelled()=>Err(failure(if english{"Graph query cancelled"}else{"图谱查询已取消"})),
                output=tokio::time::timeout(control.remaining(),self.execute(request,context))=>output.map_err(|_|failure(if english{"Graph query timed out"}else{"图谱查询超时"}))?,
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
fn localized_error(error: &GraphQueryError, en: bool) -> &'static str {
    match (error, en) {
        (GraphQueryError::Invalid, true) => "Invalid graph query parameters",
        (GraphQueryError::Invalid, false) => "图谱查询参数无效",
        (GraphQueryError::Forbidden, true) => "Graph access denied",
        (GraphQueryError::Forbidden, false) => "无权查询该语料图谱",
        (GraphQueryError::Disabled, true) => "Graph indexing is disabled",
        (GraphQueryError::Disabled, false) => "该语料尚未启用图谱",
        (GraphQueryError::NodeNotFound, true) => "Node not found in accessible scope",
        (GraphQueryError::NodeNotFound, false) => "节点不存在或不在可访问范围内",
        (GraphQueryError::SnapshotChanged, true) => {
            "Graph snapshot changed or expired; search again"
        }
        (GraphQueryError::SnapshotChanged, false) => "授权、发布版本或游标已变化，请重新检索",
        (GraphQueryError::Unavailable(_), true) => "Graph retrieval unavailable",
        (GraphQueryError::Unavailable(_), false) => "图谱检索通道暂不可用",
        (_, true) => "Could not read graph data",
        (_, false) => "图谱读取失败",
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
