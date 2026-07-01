fn build_retrieval_profile_config(input: &RagProfileManagementInput) -> Value {
    let derived_routes;
    let route_source = if let Some(pipeline) = &input.pipeline {
        derived_routes = retrieval_routes_from_pipeline(input, pipeline);
        derived_routes.iter().collect::<Vec<_>>()
    } else {
        input
            .routes
            .iter()
            .filter(|route| route.enabled)
            .collect::<Vec<_>>()
    };
    let routes = route_source
        .into_iter()
        .map(|route| {
            json!({
                "routeId": route.route_id,
                "corpusId": route.corpus_id,
                "retriever": route.strategy_id,
                "topK": route.top_k.unwrap_or(20),
                "weight": route.weight.unwrap_or(1.0),
                "filters": route.filters.clone().unwrap_or_else(|| json!({})),
            })
        })
        .collect::<Vec<_>>();

    let pipeline_fusion = input.pipeline.as_ref().and_then(|pipeline| {
        pipeline
            .nodes
            .iter()
            .find(|node| node.enabled && node.stage_type == "FUSE_RESULTS")
            .map(|node| node.strategy_id.clone())
    });
    let pipeline_dedup = input.pipeline.as_ref().and_then(|pipeline| {
        pipeline
            .nodes
            .iter()
            .find(|node| node.enabled && node.stage_type == "DEDUP_RESULTS")
            .map(|node| node.strategy_id.clone())
    });
    let topic_policy = match input.topic_policy.as_deref().unwrap_or("NONE") {
        "EMBEDDING_MMR_V1" => json!({
            "enabled": true,
            "type": "EMBEDDING_MMR_V1",
            "applyStage": "BEFORE_RERANK",
            "candidateWindow": 20,
            "lambda": 0.72,
            "similarityThreshold": 0.82,
            "maxPerTopic": 0,
            "fallback": "KEEP_ORDER"
        }),
        _ => json!({
            "enabled": false
        }),
    };

    let graph_expansion_policy = if input
        .graph_expansion_policy
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        json!({
            "enabled": true,
            "direction": "OUTGOING",
            "maxDepth": 1,
            "maxRelatedNodes": 20,
            "failurePolicy": "WARN_AND_RETURN_BASE_HITS"
        })
    } else {
        json!({
            "enabled": false
        })
    };

    let mut config = serde_json::Map::from_iter([
        (
            "description".into(),
            Value::String(input.description.clone()),
        ),
        ("routes".into(), Value::Array(routes)),
        (
            "fusionPolicy".into(),
            json!({
                "type": pipeline_fusion
                    .or_else(|| input.fusion_policy.clone())
                    .unwrap_or_else(|| "SUM_SCORE".into()),
                "rankConstant": 60
            }),
        ),
        (
            "dedupPolicy".into(),
            json!({
                "type": pipeline_dedup
                    .or_else(|| input.dedup_policy.clone())
                    .unwrap_or_else(|| "TEXT_UNIT".into())
            }),
        ),
        ("topicPolicy".into(), topic_policy),
        ("graphExpansionPolicy".into(), graph_expansion_policy),
        (
            "contextPolicy".into(),
            json!({
                "supportedStrategies": dedup_strings(&input.context_strategies)
            }),
        ),
        (
            "rerankPolicy".into(),
            json!({
                "enabled": input.rerank_enabled.unwrap_or(false)
            }),
        ),
        (
            "hydratePolicy".into(),
            json!({
                "type": "TEXT_UNIT"
            }),
        ),
    ]);
    if let Some(rerank_model_code) = input
        .rerank_model_code
        .clone()
        .filter(|value| !value.trim().is_empty())
    {
        config.insert("rerankModelCode".into(), Value::String(rerank_model_code));
    }
    if let Some(pipeline) = &input.pipeline {
        config.insert(
            "pipeline".into(),
            serde_json::to_value(pipeline).unwrap_or_else(|_| json!({})),
        );
    }
    Value::Object(config)
}

fn build_profile_filters(
    filters: &[RagRetrievalFilterPayload],
    corpora: &[RagCorpusRecord],
    corpus_ids: &[String],
) -> Result<Value, RagError> {
    let corpora_by_id = corpora
        .iter()
        .map(|corpus| (corpus.corpus_id.clone(), corpus))
        .collect::<BTreeMap<_, _>>();
    let mut normalized = serde_json::Map::new();
    for filter in filters {
        let Some((field_type, operators)) =
            resolve_filter_schema_entry(&filter.field, corpus_ids, &corpora_by_id)
        else {
            return Err(RagError::FilterFieldInvalid);
        };
        if !operators.contains(filter.operator.as_str()) {
            return Err(RagError::FilterOperatorUnsupported);
        }
        normalized.insert(
            filter.field.clone(),
            json!({
                "op": filter.operator,
                "value": coerce_filter_value(&field_type, &filter.value)?,
            }),
        );
    }
    Ok(Value::Object(normalized))
}

fn project_profile_view(profile: &RagProfileRecord) -> RagProfileManagementView {
    match profile.profile_type {
        RagProfileType::Ingest => project_ingest_profile_view(profile),
        RagProfileType::Retrieval => project_retrieval_profile_view(profile),
    }
}

fn profile_pipeline_from_config(profile: &RagProfileRecord) -> Option<RagPipelineInput> {
    if let Some(stored) = profile
        .config
        .get("pipeline")
        .cloned()
        .and_then(|value| serde_json::from_value::<RagPipelineInput>(value).ok())
    {
        return Some(stored);
    }
    match profile.profile_type {
        RagProfileType::Ingest => legacy_ingest_pipeline_from_config(&profile.config),
        RagProfileType::Retrieval => legacy_retrieval_pipeline_from_config(profile),
    }
}

fn legacy_ingest_pipeline_from_config(config: &Value) -> Option<RagPipelineInput> {
    let dag = config.get("dag")?.as_object()?;
    let stage_config = config.get("stages")?.as_object()?;
    let execution_nodes = dag.get("nodes")?.as_array()?;
    let mut execution_to_node = BTreeMap::new();
    let nodes = execution_nodes
        .iter()
        .filter_map(Value::as_str)
        .filter_map(|execution_stage_id| {
            let stage_type = stage_type_for_execution_stage(execution_stage_id)?;
            let stage = stage_config
                .get(execution_stage_id)
                .and_then(Value::as_object);
            let node_id = stage
                .and_then(|stage| stage.get("nodeId"))
                .and_then(Value::as_str)
                .unwrap_or(execution_stage_id)
                .to_string();
            execution_to_node.insert(execution_stage_id.to_string(), node_id.clone());
            Some(RagPipelineNodeInput {
                node_id,
                stage_type: stage_type.to_string(),
                strategy_id: stage
                    .and_then(|stage| stage.get("strategyId"))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                enabled: true,
                join_policy: Some("ALL_SUCCESS".into()),
                failure_policy: Some("FAIL_PIPELINE".into()),
                config: stage.and_then(|stage| stage.get("config")).cloned(),
            })
        })
        .collect::<Vec<_>>();
    if nodes.is_empty() {
        return None;
    }
    let edges = dag
        .get("edges")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|edge| {
            let from = edge.get("from").and_then(Value::as_str)?;
            let to = edge.get("to").and_then(Value::as_str)?;
            Some(RagPipelineEdgeInput {
                from: execution_to_node
                    .get(from)
                    .cloned()
                    .unwrap_or_else(|| from.into()),
                to: execution_to_node
                    .get(to)
                    .cloned()
                    .unwrap_or_else(|| to.into()),
            })
        })
        .collect();
    Some(RagPipelineInput {
        mode: "DAG".into(),
        nodes,
        edges,
    })
}

fn legacy_retrieval_pipeline_from_config(profile: &RagProfileRecord) -> Option<RagPipelineInput> {
    let routes = profile.config.get("routes")?.as_array()?;
    if routes.is_empty() {
        return None;
    }
    let mut nodes = routes
        .iter()
        .enumerate()
        .filter_map(|(index, route)| {
            let object = route.as_object()?;
            let strategy_id = object
                .get("retriever")
                .or_else(|| object.get("retrieverId"))
                .and_then(Value::as_str)?;
            Some(RagPipelineNodeInput {
                node_id: object
                    .get("routeId")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(|| format!("route-{index}")),
                stage_type: "ROUTE_SEARCH".into(),
                strategy_id: strategy_id.to_string(),
                enabled: true,
                join_policy: Some("ALL_SUCCESS".into()),
                failure_policy: Some("CONTINUE_WITH_WARNING".into()),
                config: Some(json!({
                    "corpusId": object
                        .get("corpusId")
                        .and_then(Value::as_str)
                        .or(profile.primary_corpus_id.as_deref())
                        .unwrap_or_default(),
                    "topK": object.get("topK").cloned().unwrap_or_else(|| json!(20)),
                    "weight": object.get("weight").cloned().unwrap_or_else(|| json!(1.0)),
                    "filters": object.get("filters").cloned().unwrap_or_else(|| json!({}))
                })),
            })
        })
        .collect::<Vec<_>>();
    if nodes.is_empty() {
        return None;
    }
    let fusion_node_id = "fuse-results".to_string();
    let dedup_node_id = "dedup-results".to_string();
    let context_node_id = "build-context".to_string();
    let route_ids = nodes
        .iter()
        .map(|node| node.node_id.clone())
        .collect::<Vec<_>>();
    nodes.push(RagPipelineNodeInput {
        node_id: fusion_node_id.clone(),
        stage_type: "FUSE_RESULTS".into(),
        strategy_id: profile
            .config
            .get("fusionPolicy")
            .and_then(Value::as_object)
            .and_then(|policy| policy.get("type"))
            .and_then(Value::as_str)
            .unwrap_or("SUM_SCORE")
            .to_string(),
        enabled: true,
        join_policy: Some("ALL_SUCCESS".into()),
        failure_policy: Some("FAIL_PIPELINE".into()),
        config: Some(json!({})),
    });
    nodes.push(RagPipelineNodeInput {
        node_id: dedup_node_id.clone(),
        stage_type: "DEDUP_RESULTS".into(),
        strategy_id: profile
            .config
            .get("dedupPolicy")
            .and_then(Value::as_object)
            .and_then(|policy| policy.get("type"))
            .and_then(Value::as_str)
            .unwrap_or("TEXT_UNIT")
            .to_string(),
        enabled: true,
        join_policy: Some("ALL_SUCCESS".into()),
        failure_policy: Some("FAIL_PIPELINE".into()),
        config: Some(json!({})),
    });
    nodes.push(RagPipelineNodeInput {
        node_id: context_node_id.clone(),
        stage_type: "BUILD_CONTEXT".into(),
        strategy_id: profile
            .config
            .get("contextPolicy")
            .and_then(Value::as_object)
            .and_then(|policy| policy.get("supportedStrategies"))
            .and_then(Value::as_array)
            .and_then(|strategies| strategies.first())
            .and_then(Value::as_str)
            .unwrap_or("RESULT_LIST")
            .to_string(),
        enabled: true,
        join_policy: Some("ALL_SUCCESS".into()),
        failure_policy: Some("FAIL_PIPELINE".into()),
        config: Some(json!({})),
    });
    let mut edges = route_ids
        .into_iter()
        .map(|route_id| RagPipelineEdgeInput {
            from: route_id,
            to: fusion_node_id.clone(),
        })
        .collect::<Vec<_>>();
    edges.push(RagPipelineEdgeInput {
        from: fusion_node_id,
        to: dedup_node_id.clone(),
    });
    edges.push(RagPipelineEdgeInput {
        from: dedup_node_id,
        to: context_node_id,
    });
    Some(RagPipelineInput {
        mode: "DAG".into(),
        nodes,
        edges,
    })
}

fn project_ingest_profile_view(profile: &RagProfileRecord) -> RagProfileManagementView {
    let enabled_nodes = profile
        .config
        .get("dag")
        .and_then(|dag| dag.get("nodes"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    let stages = profile
        .config
        .get("stages")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    RagProfileManagementView {
        profile_id: profile.profile_id.clone(),
        profile_type: profile.profile_type.as_str().to_string(),
        status: profile.status.as_str().to_string(),
        description: profile_description(&profile.config),
        primary_corpus_id: profile.primary_corpus_id.clone(),
        corpus_ids: profile.corpus_ids.clone(),
        required_source_version: Some(
            profile
                .required_source_version
                .clone()
                .or_else(|| {
                    profile
                        .config
                        .get("requiredSourceVersion")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                })
                .unwrap_or_else(|| "v1".into()),
        ),
        pipeline: profile_pipeline_from_config(profile),
        pipeline_stages: DEFAULT_INGEST_STAGE_ORDER
            .into_iter()
            .map(|stage_id| {
                let stage = stages.get(stage_id).and_then(Value::as_object);
                RagPipelineStagePayload {
                    id: stage_id.to_string(),
                    enabled: enabled_nodes.contains(stage_id),
                    strategy_id: stage
                        .and_then(|stage| stage.get("strategyId"))
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    config: stage.and_then(|stage| stage.get("config")).cloned(),
                }
            })
            .collect(),
        embedding_model_code: profile
            .config
            .get("embeddingModelCode")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        embedding_version: profile
            .config
            .get("embeddingVersion")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        graph_write_policy: profile.config.get("graphWritePolicy").cloned(),
        source_input_policy: sanitize_source_input_policy(profile.config.get("sourceInputPolicy")),
        routes: Vec::new(),
        filters: Vec::new(),
        fusion_policy: None,
        dedup_policy: None,
        topic_policy: None,
        graph_expansion_policy: None,
        context_strategies: Vec::new(),
        rerank_enabled: false,
        rerank_model_code: None,
        degrade_policy: Some(profile.degrade_policy.clone()),
    }
}
