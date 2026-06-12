pub fn stage_result_from_definition(
    node_id: impl Into<String>,
    stage_type: &str,
    strategy_id: impl Into<String>,
    status: impl Into<String>,
    warnings: Vec<Value>,
) -> RagStageExecutionResult {
    let stage = stage_definition(stage_type);
    RagStageExecutionResult {
        node_id: node_id.into(),
        stage_type: stage_type.to_string(),
        strategy_id: strategy_id.into(),
        status: status.into(),
        duration_ms: 0,
        input_artifact_keys: stage
            .map(|definition| {
                definition
                    .input_artifact_keys
                    .iter()
                    .map(ToString::to_string)
                    .collect()
            })
            .unwrap_or_default(),
        output_artifact_keys: stage
            .map(|definition| {
                definition
                    .output_artifact_keys
                    .iter()
                    .map(ToString::to_string)
                    .collect()
            })
            .unwrap_or_default(),
        warnings,
    }
}

pub fn stage_type_for_execution_stage(stage_id: &str) -> Option<&'static str> {
    stage_definitions()
        .into_iter()
        .find(|stage| stage.execution_stage_id == stage_id)
        .map(|stage| stage.stage_type)
}

pub fn execution_stage_for_stage_type(stage_type: &str) -> Option<&'static str> {
    stage_definition(stage_type).map(|stage| stage.execution_stage_id)
}

pub fn stage_definition(stage_type: &str) -> Option<StageDefinition> {
    stage_definitions()
        .into_iter()
        .find(|stage| stage.stage_type == stage_type)
}

fn execution_levels(
    nodes: &[&RagPipelineNodeInput],
    edges: &[RagPipelineEdgeInput],
) -> Result<Vec<Vec<String>>, RagPipelineValidationIssue> {
    let mut incoming = nodes
        .iter()
        .map(|node| (node.node_id.clone(), 0usize))
        .collect::<BTreeMap<_, _>>();
    let mut outgoing: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for edge in edges {
        *incoming
            .get_mut(&edge.to)
            .ok_or_else(|| edge_issue("pipeline edge references an unknown target."))? += 1;
        outgoing
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
    }

    let mut ready = nodes
        .iter()
        .filter_map(|node| {
            incoming
                .get(&node.node_id)
                .is_some_and(|count| *count == 0)
                .then(|| node.node_id.clone())
        })
        .collect::<Vec<_>>();
    let mut levels = Vec::new();
    let mut visited = 0usize;
    while !ready.is_empty() {
        ready.sort();
        let current_level = ready;
        visited += current_level.len();
        let mut next_ready = Vec::new();
        for node in &current_level {
            if let Some(next_nodes) = outgoing.get(node) {
                for next in next_nodes {
                    let count = incoming
                        .get_mut(next)
                        .ok_or_else(|| edge_issue("pipeline edge references an unknown target."))?;
                    *count -= 1;
                    if *count == 0 {
                        next_ready.push(next.clone());
                    }
                }
            }
        }
        levels.push(current_level);
        ready = next_ready;
    }
    if visited != nodes.len() {
        return Err(issue(
            "RAG_PIPELINE_CYCLE",
            "pipeline edges must form an acyclic DAG.",
            Some("pipeline.edges"),
        ));
    }
    Ok(levels)
}

fn validate_repeatability(
    nodes: &[CompiledRagPipelineNode],
    stage_by_type: &BTreeMap<String, StageDefinition>,
    errors: &mut Vec<RagPipelineValidationIssue>,
) {
    let mut counts = BTreeMap::<&str, usize>::new();
    for node in nodes {
        *counts.entry(node.stage_type.as_str()).or_default() += 1;
    }
    for (stage_type, count) in counts {
        if count > 1
            && stage_by_type
                .get(stage_type)
                .is_some_and(|stage| !stage.repeatable)
        {
            errors.push(issue(
                "RAG_PIPELINE_STAGE_NOT_REPEATABLE",
                "pipeline stageType is not repeatable.",
                Some("pipeline.nodes.stageType"),
            ));
        }
    }
}

fn validate_orchestration(
    profile_type: RagProfileType,
    nodes: &[CompiledRagPipelineNode],
    edges: &[RagPipelineEdgeInput],
    stage_by_type: &BTreeMap<String, StageDefinition>,
    errors: &mut Vec<RagPipelineValidationIssue>,
) {
    if !errors.is_empty() {
        return;
    }

    let enabled_stage_types = nodes
        .iter()
        .map(|node| node.stage_type.as_str())
        .collect::<BTreeSet<_>>();
    for stage in stage_by_type.values() {
        let orchestration = stage_orchestration(stage.stage_type);
        if stage.supports_profile(profile_type.clone())
            && orchestration.required
            && !enabled_stage_types.contains(stage.stage_type)
        {
            errors.push(issue(
                "RAG_PIPELINE_REQUIRED_STAGE_MISSING",
                "pipeline is missing a required stage declared by the stage catalog.",
                Some("pipeline.nodes.stageType"),
            ));
        }
    }

    let mut reported = BTreeSet::new();
    for node in nodes {
        let orchestration = stage_orchestration(node.stage_type.as_str());
        for before_stage_type in orchestration.must_run_before_stage_types {
            for target in nodes
                .iter()
                .filter(|target| target.stage_type == *before_stage_type)
            {
                if !edge_path_exists(node.node_id.as_str(), target.node_id.as_str(), edges) {
                    push_control_dependency_issue(
                        &mut reported,
                        errors,
                        node.node_id.as_str(),
                        target.node_id.as_str(),
                    );
                }
            }
        }
        for after_stage_type in orchestration.must_run_after_stage_types {
            for source in nodes
                .iter()
                .filter(|source| source.stage_type == *after_stage_type)
            {
                if !edge_path_exists(source.node_id.as_str(), node.node_id.as_str(), edges) {
                    push_control_dependency_issue(
                        &mut reported,
                        errors,
                        source.node_id.as_str(),
                        node.node_id.as_str(),
                    );
                }
            }
        }
    }
}

fn push_control_dependency_issue(
    reported: &mut BTreeSet<String>,
    errors: &mut Vec<RagPipelineValidationIssue>,
    from: &str,
    to: &str,
) {
    if reported.insert(format!("{from}->{to}")) {
        errors.push(issue(
            "RAG_PIPELINE_CONTROL_DEPENDENCY_MISSING",
            "pipeline edges must preserve control dependencies declared by the stage catalog.",
            Some("pipeline.edges"),
        ));
    }
}

fn edge_path_exists(from: &str, to: &str, edges: &[RagPipelineEdgeInput]) -> bool {
    if from == to {
        return true;
    }
    let mut outgoing = BTreeMap::<&str, Vec<&str>>::new();
    for edge in edges {
        outgoing
            .entry(edge.from.as_str())
            .or_default()
            .push(edge.to.as_str());
    }
    let mut visited = BTreeSet::new();
    let mut stack = vec![from];
    while let Some(node) = stack.pop() {
        if !visited.insert(node) {
            continue;
        }
        for next in outgoing.get(node).into_iter().flatten() {
            if *next == to {
                return true;
            }
            stack.push(next);
        }
    }
    false
}

fn validate_artifact_availability(
    profile_type: RagProfileType,
    nodes: &[CompiledRagPipelineNode],
    execution_levels: &[Vec<String>],
    errors: &mut Vec<RagPipelineValidationIssue>,
) {
    if execution_levels.is_empty() || !errors.is_empty() {
        return;
    }
    let by_id = nodes
        .iter()
        .map(|node| (node.node_id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    let mut available = initial_artifact_keys(profile_type);
    for level in execution_levels {
        for node_id in level {
            let Some(node) = by_id.get(node_id.as_str()) else {
                continue;
            };
            for key in &node.input_artifact_keys {
                if !available.contains(key) {
                    errors.push(issue(
                        "RAG_PIPELINE_INPUT_ARTIFACT_MISSING",
                        "pipeline node input artifact is not available from initial request or upstream outputs.",
                        Some("pipeline.nodes.inputArtifactKeys"),
                    ));
                }
            }
        }
        for node_id in level {
            let Some(node) = by_id.get(node_id.as_str()) else {
                continue;
            };
            for key in &node.output_artifact_keys {
                available.insert(key.clone());
            }
        }
    }
}

fn initial_artifact_keys(profile_type: RagProfileType) -> BTreeSet<String> {
    match profile_type {
        RagProfileType::Ingest => ["request.payload", "request.accessContext"]
            .into_iter()
            .map(ToOwned::to_owned)
            .collect(),
        RagProfileType::Retrieval => [
            "query.original",
            "query.effective",
            "request.filters",
            "request.accessContext",
        ]
        .into_iter()
        .map(ToOwned::to_owned)
        .collect(),
    }
}

fn contains_artifact_key_override(value: &Value) -> bool {
    match value {
        Value::Object(object) => object.iter().any(|(key, value)| {
            matches!(key.as_str(), "inputArtifactKeys" | "outputArtifactKeys")
                || contains_artifact_key_override(value)
        }),
        Value::Array(items) => items.iter().any(contains_artifact_key_override),
        _ => false,
    }
}

fn issue(code: &str, message: &str, field: Option<&str>) -> RagPipelineValidationIssue {
    RagPipelineValidationIssue {
        code: code.to_string(),
        message: message.to_string(),
        field: field.map(ToOwned::to_owned),
    }
}

fn edge_issue(message: &str) -> RagPipelineValidationIssue {
    issue("RAG_PIPELINE_EDGE_INVALID", message, Some("pipeline.edges"))
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Copy)]
pub struct StageDefinition {
    stage_type: &'static str,
    display_name: &'static str,
    profile_types: &'static [RagProfileType],
    input_artifact_keys: &'static [&'static str],
    output_artifact_keys: &'static [&'static str],
    repeatable: bool,
    parallel: bool,
    default_join_policy: &'static str,
    default_failure_policy: &'static str,
    strategies: &'static [StrategyDefinition],
    execution_stage_id: &'static str,
}

impl StageDefinition {
    fn supports_profile(&self, profile_type: RagProfileType) -> bool {
        self.profile_types.contains(&profile_type)
    }
}

#[derive(Debug, Clone, Copy)]
struct StrategyDefinition {
    strategy_id: &'static str,
    display_name: &'static str,
    required_capabilities: &'static [&'static str],
    required_storage_segments: &'static [&'static str],
}

#[derive(Debug, Clone, Copy)]
struct StageOrchestrationDefinition {
    required: bool,
    acts_as_barrier: bool,
    must_run_before_stage_types: &'static [&'static str],
    must_run_after_stage_types: &'static [&'static str],
}

impl Default for StageOrchestrationDefinition {
    fn default() -> Self {
        Self {
            required: false,
            acts_as_barrier: false,
            must_run_before_stage_types: &[],
            must_run_after_stage_types: &[],
        }
    }
}

fn stage_orchestration(stage_type: &str) -> StageOrchestrationDefinition {
    match stage_type {
        "VALIDATE_SOURCE" => StageOrchestrationDefinition {
            required: true,
            acts_as_barrier: true,
            must_run_before_stage_types: &["PREPARE_PAYLOAD", "RESOLVE_SOURCE"],
            must_run_after_stage_types: &[],
        },
        _ => StageOrchestrationDefinition::default(),
    }
}

impl From<StageDefinition> for RagPipelineStageDefinitionView {
    fn from(stage: StageDefinition) -> Self {
        let orchestration = stage_orchestration(stage.stage_type);
        Self {
            stage_type: stage.stage_type.to_string(),
            display_name: stage.display_name.to_string(),
            profile_types: stage
                .profile_types
                .iter()
                .map(RagProfileType::as_str)
                .map(ToOwned::to_owned)
                .collect(),
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
            orchestration: RagPipelineStageOrchestrationView {
                required: orchestration.required,
                acts_as_barrier: orchestration.acts_as_barrier,
                must_run_before_stage_types: orchestration
                    .must_run_before_stage_types
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
                must_run_after_stage_types: orchestration
                    .must_run_after_stage_types
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
            },
            repeatable: stage.repeatable,
            parallel: stage.parallel,
            default_join_policy: stage.default_join_policy.to_string(),
            default_failure_policy: stage.default_failure_policy.to_string(),
            strategies: stage
                .strategies
                .iter()
                .map(|strategy| RagPipelineStrategyDefinitionView {
                    strategy_id: strategy.strategy_id.to_string(),
                    display_name: strategy.display_name.to_string(),
                    config_schema: strategy_config_schema(strategy.strategy_id),
                    required_capabilities: strategy
                        .required_capabilities
                        .iter()
                        .map(ToString::to_string)
                        .collect(),
                    required_storage_segments: strategy
                        .required_storage_segments
                        .iter()
                        .map(ToString::to_string)
                        .collect(),
                })
                .collect(),
        }
    }
}

