fn validate_profile_input<S>(
    input: &RagProfileManagementInput,
    corpora: &[RagCorpusRecord],
    strategy_registry: &S,
) -> Result<Vec<RagValidationIssueView>, RagError>
where
    S: RagStrategyRegistry,
{
    let mut errors = Vec::new();
    if input.profile_id.trim().is_empty() || input.primary_corpus_id.trim().is_empty() {
        errors.push(validation_issue(
            "RAG_MANAGEMENT_REQUEST_INVALID",
            "profileId and primaryCorpusId are required.",
            None,
        ));
        return Ok(errors);
    }

    let corpora_by_id = corpora
        .iter()
        .map(|corpus| (corpus.corpus_id.clone(), corpus))
        .collect::<BTreeMap<_, _>>();

    let primary_corpus = match corpora_by_id.get(&input.primary_corpus_id) {
        Some(corpus) => *corpus,
        None => {
            errors.push(validation_issue(
                "RAG_PROFILE_CORPUS_INVALID",
                "primary corpus must reference an existing corpus.",
                Some("primaryCorpusId"),
            ));
            return Ok(errors);
        }
    };

    let corpus_ids = if input.corpus_ids.is_empty() {
        vec![input.primary_corpus_id.clone()]
    } else {
        input.corpus_ids.clone()
    };
    for corpus_id in &corpus_ids {
        if !corpora_by_id.contains_key(corpus_id) {
            errors.push(validation_issue(
                "RAG_PROFILE_CORPUS_INVALID",
                "corpusIds must reference existing corpora.",
                Some("corpusIds"),
            ));
        }
    }

    let profile_type = parse_profile_type(&input.profile_type)?;
    if let Some(pipeline) = &input.pipeline {
        if let Err(pipeline_errors) = validate_pipeline(pipeline, profile_type.clone()) {
            errors.extend(pipeline_errors.into_iter().map(|error| {
                validation_issue(&error.code, &error.message, error.field.as_deref())
            }));
        }
    }

    match profile_type {
        RagProfileType::Ingest => {
            if input.pipeline.is_none() {
                let enabled_stages = input
                    .pipeline_stages
                    .iter()
                    .filter(|stage| stage.enabled)
                    .collect::<Vec<_>>();
                if enabled_stages.is_empty() {
                    errors.push(validation_issue(
                        "RAG_MANAGEMENT_REQUEST_INVALID",
                        "At least one pipeline stage must be enabled.",
                        Some("pipelineStages"),
                    ));
                }
                for stage in enabled_stages {
                    if stage.strategy_id.trim().is_empty()
                        || strategy_registry.resolve_stage(&stage.strategy_id).status
                            != StrategyRegistrationStatus::Enabled
                    {
                        errors.push(validation_issue(
                            "RAG_PROFILE_STRATEGY_INVALID",
                            "enabled pipeline stages must use registered strategy IDs.",
                            Some("pipelineStages"),
                        ));
                    }
                }
            }
            if errors.is_empty() {
                let record = build_profile_record(input, corpora)?;
                if compile_ingest_profile(&record, primary_corpus, strategy_registry).is_err() {
                    errors.push(validation_issue(
                        "RAG_MANAGEMENT_REQUEST_INVALID",
                        "ingest profile could not be compiled by the current runtime contract.",
                        None,
                    ));
                }
            }
        }
        RagProfileType::Retrieval => {
            if input
                .rerank_strategy
                .as_deref()
                .is_some_and(|value| !matches!(value, "Disabled" | "Native" | "LlmFallback"))
            {
                errors.push(validation_issue(
                    "RAG_MANAGEMENT_REQUEST_INVALID",
                    "rerankStrategy must be Disabled, Native, or LlmFallback.",
                    Some("rerankStrategy"),
                ));
            }
            if input.rerank_enabled.unwrap_or(false)
                && input.rerank_strategy.as_deref().unwrap_or("Disabled") == "Native"
                && input
                    .rerank_model_code
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty())
            {
                errors.push(validation_issue(
                    "RAG_MANAGEMENT_REQUEST_INVALID",
                    "rerankModelCode is required when Native rerank is enabled.",
                    Some("rerankModelCode"),
                ));
            }
            let derived_routes;
            let enabled_routes = if let Some(pipeline) = &input.pipeline {
                derived_routes = retrieval_routes_from_pipeline(input, pipeline);
                derived_routes.iter().collect::<Vec<_>>()
            } else {
                input
                    .routes
                    .iter()
                    .filter(|route| route.enabled)
                    .collect::<Vec<_>>()
            };
            if enabled_routes.is_empty() {
                errors.push(validation_issue(
                    "RAG_MANAGEMENT_REQUEST_INVALID",
                    "At least one retrieval route must be enabled.",
                    Some("pipeline"),
                ));
            }
            for route in &enabled_routes {
                if !corpora_by_id.contains_key(&route.corpus_id)
                    || !corpus_ids.contains(&route.corpus_id)
                {
                    errors.push(validation_issue(
                        "RAG_PROFILE_CORPUS_INVALID",
                        "route corpora must belong to the saved corpus set.",
                        Some("pipeline"),
                    ));
                }
                if input.pipeline.is_none()
                    && !strategy_matches_route_type(
                        &route.route_type,
                        &route.strategy_id,
                        strategy_registry,
                    )
                {
                    errors.push(validation_issue(
                        "RAG_PROFILE_STRATEGY_INVALID",
                        "routeType must match a registered strategy for that route.",
                        Some("routes"),
                    ));
                }
                if let Some(route_corpus) = corpora_by_id.get(&route.corpus_id) {
                    if !route_strategy_matches_corpus_storage(route, route_corpus) {
                        errors.push(validation_issue(
                            "RAG_PROFILE_STRATEGY_INVALID",
                            "route strategy must match the selected corpus storage backend.",
                            Some(if input.pipeline.is_some() {
                                "pipeline"
                            } else {
                                "routes"
                            }),
                        ));
                    }
                }
            }
            if !enabled_routes
                .iter()
                .any(|route| route.corpus_id == input.primary_corpus_id)
            {
                errors.push(validation_issue(
                    "RAG_PROFILE_CORPUS_INVALID",
                    "primary corpus must appear in at least one enabled route.",
                    Some("routes"),
                ));
            }
            if input
                .graph_expansion_policy
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
                && !enabled_routes
                    .iter()
                    .any(|route| route.strategy_id.contains("GRAPH"))
            {
                errors.push(validation_issue(
                    "RAG_PROFILE_STRATEGY_INVALID",
                    "graph expansion requires an enabled graph-backed route.",
                    Some("graphExpansionPolicy"),
                ));
            }
            errors.extend(validate_filter_rows(
                &input.filters,
                &enabled_routes,
                &corpora_by_id,
            ));
            if errors.is_empty() {
                let record = build_profile_record(input, corpora)?;
                if compile_retrieval_profile(&record, primary_corpus, strategy_registry).is_err() {
                    errors.push(validation_issue(
                        "RAG_MANAGEMENT_REQUEST_INVALID",
                        "retrieval profile could not be compiled by the current runtime contract.",
                        None,
                    ));
                }
            }
        }
    }

    Ok(errors)
}

fn build_profile_record(
    input: &RagProfileManagementInput,
    corpora: &[RagCorpusRecord],
) -> Result<RagProfileRecord, RagError> {
    let profile_type = parse_profile_type(&input.profile_type)?;
    let corpus_ids = if input.corpus_ids.is_empty() {
        vec![input.primary_corpus_id.clone()]
    } else {
        dedup_strings(&input.corpus_ids)
    };
    let filters = build_profile_filters(&input.filters, corpora, &corpus_ids)?;
    let config = match profile_type {
        RagProfileType::Ingest => build_ingest_profile_config(input),
        RagProfileType::Retrieval => build_retrieval_profile_config(input),
    };
    Ok(RagProfileRecord {
        profile_id: input.profile_id.clone(),
        profile_type,
        status: parse_status(&input.status)?,
        primary_corpus_id: Some(input.primary_corpus_id.clone()),
        corpus_ids,
        required_source_version: Some(
            input
                .required_source_version
                .clone()
                .unwrap_or_else(|| "v1".into()),
        ),
        config,
        filters,
        degrade_policy: input
            .degrade_policy
            .clone()
            .unwrap_or_else(|| "SKIP_VECTOR_ON_UNAVAILABLE".into()),
    })
}

fn build_ingest_profile_config(input: &RagProfileManagementInput) -> Value {
    if let Some(pipeline) = &input.pipeline {
        return build_ingest_pipeline_profile_config(input, pipeline);
    }

    let enabled = input
        .pipeline_stages
        .iter()
        .filter(|stage| stage.enabled)
        .collect::<Vec<_>>();
    let nodes = enabled
        .iter()
        .map(|stage| Value::String(stage.id.clone()))
        .collect::<Vec<_>>();
    let edges = enabled
        .windows(2)
        .map(|pair| {
            json!({
                "from": pair[0].id,
                "to": pair[1].id,
            })
        })
        .collect::<Vec<_>>();
    let mut stages = serde_json::Map::new();
    for stage in enabled {
        let mut stage_object = serde_json::Map::from_iter([(
            "strategyId".into(),
            Value::String(stage.strategy_id.clone()),
        )]);
        if let Some(config) = sanitize_stage_config(stage.config.as_ref()) {
            stage_object.insert("config".into(), config);
        }
        stages.insert(stage.id.clone(), Value::Object(stage_object));
    }
    let mut config = serde_json::Map::from_iter([
        (
            "description".into(),
            Value::String(input.description.clone()),
        ),
        (
            "dag".into(),
            json!({
                "nodes": nodes,
                "edges": edges,
            }),
        ),
        ("stages".into(), Value::Object(stages)),
        (
            "embeddingVersion".into(),
            Value::String(
                input
                    .embedding_version
                    .clone()
                    .unwrap_or_else(|| "v1".into()),
            ),
        ),
        (
            "requiredSourceVersion".into(),
            Value::String(
                input
                    .required_source_version
                    .clone()
                    .unwrap_or_else(|| "v1".into()),
            ),
        ),
    ]);
    if let Some(embedding_model_code) = input
        .embedding_model_code
        .clone()
        .filter(|value| !value.trim().is_empty())
    {
        config.insert(
            "embeddingModelCode".into(),
            Value::String(embedding_model_code),
        );
    }
    if let Some(graph_write_policy) = input.graph_write_policy.clone() {
        config.insert("graphWritePolicy".into(), graph_write_policy);
    }
    if let Some(source_input_policy) =
        sanitize_source_input_policy(input.source_input_policy.as_ref())
    {
        config.insert("sourceInputPolicy".into(), source_input_policy);
    }
    Value::Object(config)
}

fn build_ingest_pipeline_profile_config(
    input: &RagProfileManagementInput,
    pipeline: &RagPipelineInput,
) -> Value {
    let compiled = validate_pipeline(pipeline, RagProfileType::Ingest).ok();
    let compiled_nodes = compiled
        .as_ref()
        .map(|compiled| compiled.nodes.clone())
        .unwrap_or_else(|| {
            pipeline
                .nodes
                .iter()
                .filter(|node| node.enabled)
                .filter_map(|node| {
                    let execution_stage_id = execution_stage_for_stage_type(&node.stage_type)?;
                    Some(CompiledRagPipelineNode {
                        node_id: node.node_id.clone(),
                        stage_type: node.stage_type.clone(),
                        strategy_id: node.strategy_id.clone(),
                        execution_stage_id: execution_stage_id.to_string(),
                        join_policy: node
                            .join_policy
                            .clone()
                            .unwrap_or_else(|| "ALL_SUCCESS".into()),
                        failure_policy: node
                            .failure_policy
                            .clone()
                            .unwrap_or_else(|| "FAIL_PIPELINE".into()),
                        input_artifact_keys: Vec::new(),
                        output_artifact_keys: Vec::new(),
                        config: node.config.clone(),
                    })
                })
                .collect()
        });
    let node_id_to_execution = compiled_nodes
        .iter()
        .map(|node| (node.node_id.clone(), node.execution_stage_id.clone()))
        .collect::<BTreeMap<_, _>>();
    let nodes = compiled_nodes
        .iter()
        .map(|node| Value::String(node.execution_stage_id.clone()))
        .collect::<Vec<_>>();
    let edges = pipeline
        .edges
        .iter()
        .filter_map(|edge| {
            let from = node_id_to_execution.get(&edge.from)?;
            let to = node_id_to_execution.get(&edge.to)?;
            Some(json!({ "from": from, "to": to }))
        })
        .collect::<Vec<_>>();
    let mut stages = serde_json::Map::new();
    for node in &compiled_nodes {
        let mut stage_object = serde_json::Map::from_iter([(
            "strategyId".into(),
            Value::String(node.strategy_id.clone()),
        )]);
        if let Some(config) = sanitize_stage_config(node.config.as_ref()) {
            stage_object.insert("config".into(), config);
        }
        stage_object.insert("nodeId".into(), Value::String(node.node_id.clone()));
        stage_object.insert("stageType".into(), Value::String(node.stage_type.clone()));
        stages.insert(node.execution_stage_id.clone(), Value::Object(stage_object));
    }
    let mut config = serde_json::Map::from_iter([
        (
            "description".into(),
            Value::String(input.description.clone()),
        ),
        (
            "pipeline".into(),
            serde_json::to_value(pipeline).unwrap_or_else(|_| json!({})),
        ),
        (
            "dag".into(),
            json!({
                "nodes": nodes,
                "edges": edges,
            }),
        ),
        ("stages".into(), Value::Object(stages)),
        (
            "embeddingVersion".into(),
            Value::String(
                input
                    .embedding_version
                    .clone()
                    .unwrap_or_else(|| "v1".into()),
            ),
        ),
        (
            "requiredSourceVersion".into(),
            Value::String(
                input
                    .required_source_version
                    .clone()
                    .unwrap_or_else(|| "v1".into()),
            ),
        ),
    ]);
    if let Some(embedding_model_code) = input
        .embedding_model_code
        .clone()
        .filter(|value| !value.trim().is_empty())
    {
        config.insert(
            "embeddingModelCode".into(),
            Value::String(embedding_model_code),
        );
    }
    if let Some(graph_write_policy) = input.graph_write_policy.clone() {
        config.insert("graphWritePolicy".into(), graph_write_policy);
    }
    if let Some(source_input_policy) =
        sanitize_source_input_policy(input.source_input_policy.as_ref())
    {
        config.insert("sourceInputPolicy".into(), source_input_policy);
    }
    Value::Object(config)
}

fn sanitize_stage_config(config: Option<&Value>) -> Option<Value> {
    let Some(Value::Object(config)) = config else {
        return config.cloned();
    };
    let mut sanitized = config.clone();
    sanitized.remove("supportedSourceTypes");
    (!sanitized.is_empty()).then_some(Value::Object(sanitized))
}

fn sanitize_source_input_policy(policy: Option<&Value>) -> Option<Value> {
    let Some(Value::Object(policy)) = policy else {
        return None;
    };
    let mut sanitized = policy.clone();
    sanitized.remove("supportedSourceTypes");
    (!sanitized.is_empty()).then_some(Value::Object(sanitized))
}

fn retrieval_routes_from_pipeline(
    input: &RagProfileManagementInput,
    pipeline: &RagPipelineInput,
) -> Vec<RagRetrievalRoutePayload> {
    pipeline
        .nodes
        .iter()
        .filter(|node| node.enabled && node.stage_type == "ROUTE_SEARCH")
        .map(|node| {
            let config = node.config.as_ref().and_then(Value::as_object);
            let corpus_id = config
                .and_then(|config| config.get("corpusId"))
                .and_then(Value::as_str)
                .unwrap_or(input.primary_corpus_id.as_str())
                .to_string();
            RagRetrievalRoutePayload {
                route_id: node.node_id.clone(),
                corpus_id,
                enabled: true,
                route_type: infer_route_type(&node.strategy_id).to_string(),
                strategy_id: node.strategy_id.clone(),
                top_k: config
                    .and_then(|config| config.get("topK"))
                    .and_then(Value::as_u64)
                    .and_then(|value| usize::try_from(value).ok()),
                weight: config
                    .and_then(|config| config.get("weight"))
                    .and_then(Value::as_f64),
                filters: config.and_then(|config| config.get("filters")).cloned(),
            }
        })
        .collect()
}
