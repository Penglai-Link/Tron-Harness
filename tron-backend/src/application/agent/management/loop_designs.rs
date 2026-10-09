use super::*;
use crate::domain::agent::loop_design::*;
use eliteforge_agent_core as fw;

pub(crate) fn stage(node: &LoopNode) -> fw::WorkflowStage {
    let mut stage = fw::WorkflowStage::new(&node.id);
    if !node.instructions.trim().is_empty() {
        stage
            .instructions
            .push(fw::Message::system(node.instructions.clone()));
    }
    stage
}
pub(crate) fn workflow(design: &LoopDesign) -> fw::WorkflowDefinition {
    fw::WorkflowDefinition {
        id: design.loop_code.clone(),
        version: design.version,
        stages: design.nodes.iter().map(stage).collect(),
    }
}
pub(crate) fn dag(design: &LoopDesign) -> Result<fw::DagWorkflowDefinition, AgentManagementError> {
    let nodes = design
        .nodes
        .iter()
        .map(|node| {
            let mut result = match node.kind {
                LoopNodeKind::Model => {
                    let mut n = fw::DagNode::react(stage(node));
                    n.kind = "tron.model-stage".into();
                    n
                }
                LoopNodeKind::Branch => fw::DagNode::new(
                    &node.id,
                    "tron.branch",
                    serde_json::to_value(&node.condition)
                        .map_err(|_| AgentManagementError::LoopInvalid)?,
                ),
            };
            result.join = match node.join {
                LoopJoin::Selected => fw::WorkflowJoin::Selected,
                LoopJoin::All => fw::WorkflowJoin::All,
            };
            Ok(result)
        })
        .collect::<Result<_, AgentManagementError>>()?;
    Ok(fw::DagWorkflowDefinition {
        id: design.loop_code.clone(),
        version: design.version,
        nodes,
        edges: design
            .edges
            .iter()
            .map(|edge| fw::DagEdge::new(&edge.from, &edge.to))
            .collect(),
        max_parallel_nodes: design.max_parallel_nodes as usize,
    })
}
pub(crate) fn validate(design: &LoopDesign) -> Result<(), AgentManagementError> {
    let invalid = AgentManagementError::LoopInvalid;
    if !valid_code(&design.loop_code)
        || design.name.trim().is_empty()
        || design.name.chars().count() > 100
        || design.nodes.len() > 64
        || design.edges.len() > 256
        || !(1..=16).contains(&design.max_parallel_nodes)
        || serde_json::to_vec(design)
            .map_err(|_| invalid.clone())?
            .len()
            > 262_144
    {
        return Err(invalid);
    }
    if design.kind == LoopKind::React {
        if !design.nodes.is_empty() || !design.edges.is_empty() || design.max_parallel_nodes != 1 {
            return Err(invalid);
        }
        return Ok(());
    }
    for node in &design.nodes {
        if !valid_code(&node.id)
            || node.instructions.len() > 65_536
            || node.position.as_ref().is_some_and(|p| {
                !p.x.is_finite()
                    || !p.y.is_finite()
                    || p.x.abs() > 100_000.0
                    || p.y.abs() > 100_000.0
            })
        {
            return Err(invalid);
        }
        match node.kind {
            LoopNodeKind::Model if node.condition.is_none() => {}
            LoopNodeKind::Branch
                if design.kind == LoopKind::Dag && node.instructions.is_empty() =>
            {
                let condition = node.condition.as_ref().ok_or(invalid.clone())?;
                if (!condition.pointer.is_empty() && !condition.pointer.starts_with('/'))
                    || condition.match_target == condition.else_target
                    || !design
                        .edges
                        .iter()
                        .any(|e| e.from == condition.source_node && e.to == node.id)
                    || design.edges.iter().filter(|e| e.from == node.id).count() != 2
                    || ![&condition.match_target, &condition.else_target]
                        .iter()
                        .all(|target| {
                            design
                                .edges
                                .iter()
                                .any(|e| e.from == node.id && &e.to == *target)
                        })
                {
                    return Err(invalid);
                }
            }
            _ => return Err(invalid),
        }
    }
    if design.kind == LoopKind::Workflow {
        if !design.edges.is_empty()
            || design.max_parallel_nodes != 1
            || design.nodes.iter().any(|n| n.join != LoopJoin::Selected)
        {
            return Err(invalid);
        }
        workflow(design).validate().map_err(|_| invalid)
    } else {
        dag(design)?.validate().map_err(|_| invalid)
    }
}
fn valid_code(code: &str) -> bool {
    !code.is_empty()
        && code.len() <= 64
        && code.as_bytes()[0].is_ascii_lowercase()
        && code
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-' || c == b'_')
}
impl AgentManagementService {
    pub fn list_loop_designs(&self) -> Result<Vec<LoopDesign>, AgentManagementError> {
        let mut designs = vec![LoopDesign::react()];
        designs.extend(self.repository().list_loop_designs()?);
        Ok(designs)
    }
    pub fn loop_design(&self, code: &str) -> Result<LoopDesign, AgentManagementError> {
        self.list_loop_designs()?
            .into_iter()
            .find(|d| d.loop_code == code)
            .ok_or(AgentManagementError::LoopNotFound)
    }
    pub fn validate_loop_selection(
        &self,
        code: Option<&str>,
    ) -> Result<String, AgentManagementError> {
        let code = code.unwrap_or(REACT_LOOP_CODE);
        self.loop_design(code)?;
        Ok(code.into())
    }
    pub fn save_loop_design(
        &self,
        mut design: LoopDesign,
        expected_version: u32,
    ) -> Result<LoopDesign, AgentManagementError> {
        if design.loop_code == REACT_LOOP_CODE {
            return Err(AgentManagementError::LoopConflict);
        }
        design.version = expected_version
            .checked_add(1)
            .ok_or(AgentManagementError::LoopInvalid)?;
        design.name = design.name.trim().into();
        design.updated_at = chrono::Utc::now().timestamp_millis();
        validate(&design)?;
        self.repository()
            .save_loop_design(&design, expected_version)?;
        Ok(design)
    }
    pub fn delete_loop_design(&self, code: &str) -> Result<(), AgentManagementError> {
        if code == REACT_LOOP_CODE {
            return Err(AgentManagementError::LoopInUse);
        }
        self.repository().delete_loop_design(code)
    }
}
