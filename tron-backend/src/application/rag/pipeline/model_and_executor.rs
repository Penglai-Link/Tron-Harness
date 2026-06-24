use std::{
    any::Any,
    collections::{BTreeMap, BTreeSet},
};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::domain::rag::{RagError, RagProfileType};

const JOIN_POLICIES: [&str; 3] = ["ALL_SUCCESS", "ANY_SUCCESS", "BEST_EFFORT"];
const FAILURE_POLICIES: [&str; 3] = ["FAIL_PIPELINE", "SKIP_DOWNSTREAM", "CONTINUE_WITH_WARNING"];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagPipelineInput {
    pub mode: String,
    #[serde(default)]
    pub nodes: Vec<RagPipelineNodeInput>,
    #[serde(default)]
    pub edges: Vec<RagPipelineEdgeInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagPipelineNodeInput {
    pub node_id: String,
    pub stage_type: String,
    pub strategy_id: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub join_policy: Option<String>,
    #[serde(default)]
    pub failure_policy: Option<String>,
    #[serde(default)]
    pub config: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagPipelineEdgeInput {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagPipelineCatalogView {
    pub profile_type: String,
    pub stages: Vec<RagPipelineStageDefinitionView>,
    pub templates: Vec<RagPipelineTemplateView>,
    pub join_policies: Vec<String>,
    pub failure_policies: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagPipelineStageDefinitionView {
    pub stage_type: String,
    pub display_name: String,
    pub profile_types: Vec<String>,
    pub input_artifact_keys: Vec<String>,
    pub output_artifact_keys: Vec<String>,
    pub orchestration: RagPipelineStageOrchestrationView,
    pub repeatable: bool,
    pub parallel: bool,
    pub default_join_policy: String,
    pub default_failure_policy: String,
    pub strategies: Vec<RagPipelineStrategyDefinitionView>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagPipelineStageOrchestrationView {
    pub required: bool,
    pub acts_as_barrier: bool,
    pub must_run_before_stage_types: Vec<String>,
    pub must_run_after_stage_types: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagPipelineStrategyDefinitionView {
    pub strategy_id: String,
    pub display_name: String,
    pub config_schema: Value,
    pub required_capabilities: Vec<String>,
    pub required_storage_segments: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagPipelineTemplateView {
    pub template_id: String,
    pub display_name: String,
    pub description: String,
    pub pipeline: RagPipelineInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledRagPipeline {
    pub profile_type: RagProfileType,
    pub nodes: Vec<CompiledRagPipelineNode>,
    pub edges: Vec<RagPipelineEdgeInput>,
    pub execution_levels: Vec<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledRagPipelineNode {
    pub node_id: String,
    pub stage_type: String,
    pub strategy_id: String,
    pub execution_stage_id: String,
    pub join_policy: String,
    pub failure_policy: String,
    pub input_artifact_keys: Vec<String>,
    pub output_artifact_keys: Vec<String>,
    pub config: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RagPipelineValidationIssue {
    pub code: String,
    pub message: String,
    pub field: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagStageExecutionResult {
    pub node_id: String,
    pub stage_type: String,
    pub strategy_id: String,
    pub status: String,
    pub duration_ms: u64,
    pub input_artifact_keys: Vec<String>,
    pub output_artifact_keys: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<Value>,
}

#[derive(Default)]
pub(crate) struct RagArtifactStore {
    values: BTreeMap<String, Box<dyn Any + Send + Sync>>,
}

impl RagArtifactStore {
    pub(crate) fn put<T>(&mut self, key: impl Into<String>, value: T)
    where
        T: Send + Sync + 'static,
    {
        self.values.insert(key.into(), Box::new(value));
    }

    pub(crate) fn put_json(&mut self, key: impl Into<String>, value: Value) {
        self.put(key, value);
    }

    pub(crate) fn get<T>(&self, key: &str) -> Result<&T, RagPipelineExecutionError>
    where
        T: 'static,
    {
        self.values
            .get(key)
            .and_then(|value| value.downcast_ref::<T>())
            .ok_or_else(|| RagPipelineExecutionError::MissingArtifact(key.to_string()))
    }

    pub(crate) fn get_cloned<T>(&self, key: &str) -> Result<T, RagPipelineExecutionError>
    where
        T: Clone + 'static,
    {
        self.get::<T>(key).cloned()
    }

    pub(crate) fn get_optional_cloned<T>(
        &self,
        key: &str,
    ) -> Result<Option<T>, RagPipelineExecutionError>
    where
        T: Clone + 'static,
    {
        if self.contains(key) {
            self.get_cloned(key).map(Some)
        } else {
            Ok(None)
        }
    }

    pub(crate) fn contains(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RagPipelineExecutionError {
    MissingArtifact(String),
}

impl From<RagPipelineExecutionError> for RagError {
    fn from(_value: RagPipelineExecutionError) -> Self {
        Self::IngestStageFailed
    }
}

pub(crate) struct RagStageOutcome {
    pub(crate) status: String,
    pub(crate) warnings: Vec<Value>,
    pub(crate) strategy_id: Option<String>,
}

impl RagStageOutcome {
    pub(crate) fn completed() -> Self {
        Self {
            status: "COMPLETED".into(),
            warnings: Vec::new(),
            strategy_id: None,
        }
    }
}

pub(crate) struct RagPipelineExecutor;

impl RagPipelineExecutor {
    pub(crate) fn node_levels<'a>(
        plan: &'a CompiledRagPipeline,
    ) -> Vec<Vec<&'a CompiledRagPipelineNode>> {
        plan.execution_levels
            .iter()
            .map(|level| {
                plan.nodes
                    .iter()
                    .filter(|node| level.contains(&node.node_id))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    pub(crate) fn ensure_inputs<E>(
        node: &CompiledRagPipelineNode,
        store: &RagArtifactStore,
    ) -> Result<(), E>
    where
        E: From<RagPipelineExecutionError>,
    {
        for key in &node.input_artifact_keys {
            if !store.contains(key) {
                return Err(RagPipelineExecutionError::MissingArtifact(key.clone()).into());
            }
        }
        Ok(())
    }

    pub(crate) fn stage_result(
        node: &CompiledRagPipelineNode,
        status: impl Into<String>,
        duration_ms: u64,
        warnings: Vec<Value>,
        strategy_id: Option<String>,
    ) -> RagStageExecutionResult {
        stage_result_from_node(node, status, duration_ms, warnings, strategy_id)
    }
}

fn stage_result_from_node(
    node: &CompiledRagPipelineNode,
    status: impl Into<String>,
    duration_ms: u64,
    warnings: Vec<Value>,
    strategy_id: Option<String>,
) -> RagStageExecutionResult {
    RagStageExecutionResult {
        node_id: node.node_id.clone(),
        stage_type: node.stage_type.clone(),
        strategy_id: strategy_id.unwrap_or_else(|| node.strategy_id.clone()),
        status: status.into(),
        duration_ms,
        input_artifact_keys: node.input_artifact_keys.clone(),
        output_artifact_keys: node.output_artifact_keys.clone(),
        warnings,
    }
}

pub fn pipeline_catalog(profile_type: RagProfileType) -> RagPipelineCatalogView {
    RagPipelineCatalogView {
        profile_type: profile_type.as_str().to_string(),
        stages: stage_definitions()
            .into_iter()
            .filter(|stage| stage.supports_profile(profile_type.clone()))
            .map(RagPipelineStageDefinitionView::from)
            .collect(),
        templates: pipeline_templates(profile_type),
        join_policies: JOIN_POLICIES.into_iter().map(ToOwned::to_owned).collect(),
        failure_policies: FAILURE_POLICIES
            .into_iter()
            .map(ToOwned::to_owned)
            .collect(),
    }
}

pub fn validate_pipeline(
    pipeline: &RagPipelineInput,
    profile_type: RagProfileType,
) -> Result<CompiledRagPipeline, Vec<RagPipelineValidationIssue>> {
    let mut errors = Vec::new();
    if pipeline.mode != "DAG" {
        errors.push(issue(
            "RAG_PIPELINE_MODE_INVALID",
            "pipeline.mode must be DAG.",
            Some("pipeline.mode"),
        ));
    }
    if pipeline.nodes.is_empty() {
        errors.push(issue(
            "RAG_PIPELINE_NODE_MISSING",
            "pipeline.nodes must include at least one enabled node.",
            Some("pipeline.nodes"),
        ));
    }

    let mut seen_nodes = BTreeSet::new();
    for node in &pipeline.nodes {
        if node.node_id.trim().is_empty() {
            errors.push(issue(
                "RAG_PIPELINE_NODE_INVALID",
                "pipeline nodeId is required.",
                Some("pipeline.nodes"),
            ));
        } else if !seen_nodes.insert(node.node_id.clone()) {
            errors.push(issue(
                "RAG_PIPELINE_NODE_DUPLICATE",
                "pipeline nodeId values must be unique.",
                Some("pipeline.nodes"),
            ));
        }
    }

    let enabled_nodes = pipeline
        .nodes
        .iter()
        .filter(|node| node.enabled)
        .collect::<Vec<_>>();
    if enabled_nodes.is_empty() {
        errors.push(issue(
            "RAG_PIPELINE_NODE_MISSING",
            "pipeline.nodes must include at least one enabled node.",
            Some("pipeline.nodes"),
        ));
    }

    let stage_by_type = stage_definitions()
        .into_iter()
        .map(|stage| (stage.stage_type.to_string(), stage))
        .collect::<BTreeMap<_, _>>();
    let mut compiled_nodes = Vec::new();
    for node in &enabled_nodes {
        let Some(stage) = stage_by_type.get(node.stage_type.as_str()) else {
            errors.push(issue(
                "RAG_PIPELINE_STAGE_UNSUPPORTED",
                "pipeline stageType is not supported.",
                Some("pipeline.nodes.stageType"),
            ));
            continue;
        };
        if !stage.supports_profile(profile_type.clone()) {
            errors.push(issue(
                "RAG_PIPELINE_STAGE_UNSUPPORTED",
                "pipeline stageType does not support the requested profileType.",
                Some("pipeline.nodes.stageType"),
            ));
        }
        if !stage
            .strategies
            .iter()
            .any(|strategy| strategy.strategy_id == node.strategy_id)
        {
            errors.push(issue(
                "RAG_PIPELINE_STRATEGY_INVALID",
                "strategyId must belong to the selected stageType.",
                Some("pipeline.nodes.strategyId"),
            ));
        }
        let join_policy = node
            .join_policy
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(stage.default_join_policy);
        if !JOIN_POLICIES.contains(&join_policy) {
            errors.push(issue(
                "RAG_PIPELINE_JOIN_POLICY_INVALID",
                "joinPolicy is not supported.",
                Some("pipeline.nodes.joinPolicy"),
            ));
        }
        let failure_policy = node
            .failure_policy
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(stage.default_failure_policy);
        if !FAILURE_POLICIES.contains(&failure_policy) {
            errors.push(issue(
                "RAG_PIPELINE_FAILURE_POLICY_INVALID",
                "failurePolicy is not supported.",
                Some("pipeline.nodes.failurePolicy"),
            ));
        }
        if node
            .config
            .as_ref()
            .is_some_and(contains_artifact_key_override)
        {
            errors.push(issue(
                "RAG_PIPELINE_ARTIFACT_KEYS_OWNED_BY_STAGE",
                "inputArtifactKeys and outputArtifactKeys are owned by the stage contract.",
                Some("pipeline.nodes.config"),
            ));
        }
        compiled_nodes.push(CompiledRagPipelineNode {
            node_id: node.node_id.clone(),
            stage_type: stage.stage_type.to_string(),
            strategy_id: node.strategy_id.clone(),
            execution_stage_id: stage.execution_stage_id.to_string(),
            join_policy: join_policy.to_string(),
            failure_policy: failure_policy.to_string(),
            input_artifact_keys: stage
                .input_artifact_keys
                .iter()
                .map(ToString::to_string)
                .collect(),
            output_artifact_keys: stage
                .output_artifact_keys
                .iter()
                .map(ToString::to_string)
                .collect(),
            config: node.config.clone(),
        });
    }

    let node_ids = enabled_nodes
        .iter()
        .map(|node| node.node_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut enabled_edges = Vec::new();
    for edge in &pipeline.edges {
        if !node_ids.contains(edge.from.as_str()) || !node_ids.contains(edge.to.as_str()) {
            errors.push(issue(
                "RAG_PIPELINE_EDGE_INVALID",
                "pipeline edges must reference enabled nodeId values.",
                Some("pipeline.edges"),
            ));
            continue;
        }
        enabled_edges.push(edge.clone());
    }
    complete_legacy_persist_tree_dependency(
        profile_type.clone(),
        &compiled_nodes,
        &mut enabled_edges,
    );

    let execution_levels = if errors
        .iter()
        .any(|error| matches!(error.code.as_str(), "RAG_PIPELINE_NODE_DUPLICATE"))
    {
        Vec::new()
    } else {
        match execution_levels(&enabled_nodes, &enabled_edges) {
            Ok(levels) => levels,
            Err(error) => {
                errors.push(error);
                Vec::new()
            }
        }
    };
    validate_repeatability(&compiled_nodes, &stage_by_type, &mut errors);
    validate_orchestration(
        profile_type.clone(),
        &compiled_nodes,
        &enabled_edges,
        &stage_by_type,
        &mut errors,
    );
    validate_artifact_availability(
        profile_type.clone(),
        &compiled_nodes,
        &execution_levels,
        &mut errors,
    );

    if errors.is_empty() {
        Ok(CompiledRagPipeline {
            profile_type,
            nodes: compiled_nodes,
            edges: enabled_edges,
            execution_levels,
        })
    } else {
        Err(errors)
    }
}
