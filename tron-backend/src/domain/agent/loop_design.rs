//! Business configuration; Framework remains the workflow executor.
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
pub struct LoopPosition {
    pub x: f64,
    pub y: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LoopNode {
    pub id: String,
    pub kind: LoopNodeKind,
    pub instructions: String,
    pub join: LoopJoin,
    pub condition: Option<BranchCondition>,
    #[serde(default)]
    pub position: Option<LoopPosition>,
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
impl LoopDesign {}
