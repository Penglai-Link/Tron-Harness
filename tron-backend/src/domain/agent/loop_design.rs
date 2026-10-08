//! Business configuration; Framework remains the workflow executor.
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const REACT_LOOP_CODE: &str = "builtin.react-loop";
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoopKind {
    React,
    Workflow,
    Dag,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoopNodeKind {
    Model,
    Branch,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoopJoin {
    Selected,
    All,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BranchCondition {
    pub source_node: String,
    pub pointer: String,
    pub equals: Value,
    pub match_target: String,
    pub else_target: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LoopNode {
    pub id: String,
    pub kind: LoopNodeKind,
    pub instructions: String,
    pub join: LoopJoin,
    pub condition: Option<BranchCondition>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LoopEdge {
    pub from: String,
    pub to: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LoopDesign {
    pub loop_code: String,
    pub name: String,
    pub kind: LoopKind,
    pub version: u32,
    pub nodes: Vec<LoopNode>,
    pub edges: Vec<LoopEdge>,
    pub max_parallel_nodes: u32,
    pub updated_at: i64,
}
impl LoopDesign {
    pub fn react() -> Self {
        Self {
            loop_code: REACT_LOOP_CODE.into(),
            name: "ReAct".into(),
            kind: LoopKind::React,
            version: 1,
            nodes: vec![],
            edges: vec![],
            max_parallel_nodes: 1,
            updated_at: 0,
        }
    }
}
