fn project_retrieval_profile_view(profile: &RagProfileRecord) -> RagProfileManagementView {
    let routes = profile
        .config
        .get("routes")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .enumerate()
                .map(|(index, route)| {
                    let object = route.as_object().cloned().unwrap_or_default();
                    let strategy_id = object
                        .get("retriever")
                        .or_else(|| object.get("retrieverId"))
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    RagRetrievalRoutePayload {
                        route_id: object
                            .get("routeId")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned)
                            .unwrap_or_else(|| format!("route-{index}")),
                        corpus_id: object
                            .get("corpusId")
                            .and_then(Value::as_str)
                            .unwrap_or_else(|| {
                                profile.primary_corpus_id.as_deref().unwrap_or_default()
                            })
                            .to_string(),
                        enabled: true,
                        route_type: infer_route_type(&strategy_id).to_string(),
                        strategy_id,
                        top_k: object
                            .get("topK")
                            .and_then(Value::as_u64)
                            .map(|value| value as usize),
                        weight: object.get("weight").and_then(Value::as_f64),
                        filters: object.get("filters").cloned(),
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    RagProfileManagementView {
        profile_id: profile.profile_id.clone(),
        profile_type: profile.profile_type.as_str().to_string(),
        status: profile.status.as_str().to_string(),
        description: profile_description(&profile.config),
        primary_corpus_id: profile.primary_corpus_id.clone(),
        corpus_ids: profile.corpus_ids.clone(),
        required_source_version: profile.required_source_version.clone(),
        pipeline: profile_pipeline_from_config(profile),
        pipeline_stages: Vec::new(),
        embedding_model_code: profile
            .config
            .get("embeddingModelCode")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        embedding_version: None,
        graph_write_policy: None,
        source_input_policy: None,
        routes,
        filters: filters_to_rows(&profile.filters),
        fusion_policy: Some(
            profile
                .config
                .get("fusionPolicy")
                .and_then(Value::as_object)
                .and_then(|policy| policy.get("type"))
                .and_then(Value::as_str)
                .unwrap_or("SUM_SCORE")
                .to_string(),
        ),
        dedup_policy: Some(
            profile
                .config
                .get("dedupPolicy")
                .and_then(Value::as_object)
                .and_then(|policy| policy.get("type"))
                .and_then(Value::as_str)
                .unwrap_or("TEXT_UNIT")
                .to_string(),
        ),
        topic_policy: Some(topic_policy_code(&profile.config)),
        graph_expansion_policy: Some(graph_expansion_policy_code(&profile.config)),
        context_strategies: profile
            .config
            .get("contextPolicy")
            .and_then(Value::as_object)
            .and_then(|policy| policy.get("supportedStrategies"))
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default(),
        rerank_enabled: profile
            .config
            .get("rerankPolicy")
            .and_then(Value::as_object)
            .and_then(|policy| policy.get("enabled"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
        rerank_strategy: profile
            .config
        .get("rerankStrategy")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| Some("Disabled".into())),
        rerank_model_code: profile
            .config
            .get("rerankModelCode")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        degrade_policy: Some(profile.degrade_policy.clone()),
    }
}

fn normalize_filter_object(
    corpus: &RagCorpusRecord,
    filters: &Value,
) -> (Value, Vec<RagValidationIssueView>) {
    let Some(object) = filters.as_object() else {
        return (
            json!({}),
            vec![validation_issue(
                "RAG_MANAGEMENT_REQUEST_INVALID",
                "filters must be an object.",
                Some("filters"),
            )],
        );
    };
    let mut normalized = serde_json::Map::new();
    let mut errors = Vec::new();
    let schema = parse_filter_schema(&corpus.filter_schema);
    for (field, raw_value) in object {
        let Some(schema_field) = schema.get(field) else {
            errors.push(validation_issue(
                "RAG_FILTER_FIELD_INVALID",
                "filter field must exist in the selected corpus schema.",
                Some(field),
            ));
            continue;
        };
        let (operator, value) = extract_filter_operator(raw_value);
        if !schema_field.operators.contains(operator) {
            errors.push(validation_issue(
                "RAG_FILTER_OPERATOR_UNSUPPORTED",
                "filter operator is not supported for the selected field.",
                Some(field),
            ));
            continue;
        }
        if !filter_value_matches(&schema_field.field_type, operator, value) {
            errors.push(validation_issue(
                "RAG_MANAGEMENT_REQUEST_INVALID",
                "filter value does not match the schema field type.",
                Some(field),
            ));
            continue;
        }
        normalized.insert(
            field.clone(),
            json!({
                "op": operator,
                "value": value.clone(),
            }),
        );
    }
    (Value::Object(normalized), errors)
}

fn validate_filter_rows(
    filters: &[RagRetrievalFilterPayload],
    routes: &[&RagRetrievalRoutePayload],
    corpora_by_id: &BTreeMap<String, &RagCorpusRecord>,
) -> Vec<RagValidationIssueView> {
    let route_corpus_ids = routes
        .iter()
        .map(|route| route.corpus_id.clone())
        .collect::<Vec<_>>();
    let mut errors = Vec::new();
    for filter in filters {
        let Some((field_type, operators)) =
            resolve_filter_schema_entry(&filter.field, &route_corpus_ids, corpora_by_id)
        else {
            errors.push(validation_issue(
                "RAG_FILTER_FIELD_INVALID",
                "filter field must belong to one of the selected route corpora.",
                Some("filters"),
            ));
            continue;
        };
        if !operators.contains(filter.operator.as_str()) {
            errors.push(validation_issue(
                "RAG_FILTER_OPERATOR_UNSUPPORTED",
                "filter operator is not supported for the selected field.",
                Some("filters"),
            ));
            continue;
        }
        if coerce_filter_value(&field_type, &filter.value).is_err() {
            errors.push(validation_issue(
                "RAG_MANAGEMENT_REQUEST_INVALID",
                "filter value does not match the selected field type.",
                Some("filters"),
            ));
        }
    }
    errors
}

fn validate_storage_binding(
    binding: &RagStorageBindingPayload,
    capabilities: &RagCorpusCapabilitiesPayload,
) -> Result<RagStorageBinding, Vec<RagValidationIssueView>> {
    let mut errors = Vec::new();
    if binding.mode != "TARGETED" {
        errors.push(validation_issue(
            "RAG_STORAGE_TOPOLOGY_UNSUPPORTED",
            "Only TARGETED storageBinding mode is accepted by the management API.",
            Some("storageBinding"),
        ));
        return Err(errors);
    }

    let segment_targets = runtime_segment_targets(binding)?;
    if binding.targets.is_empty() {
        errors.push(validation_issue(
            "RAG_STORAGE_TOPOLOGY_UNSUPPORTED",
            "At least one storage target is required.",
            Some("storageBinding"),
        ));
    }
    if !matches!(
        binding.bindings.get("source"),
        Some(target_id) if segment_targets.contains_key(target_id)
    ) || !matches!(
        binding.bindings.get("textUnit"),
        Some(target_id) if segment_targets.contains_key(target_id)
    ) || !matches!(
        binding.bindings.get("vector"),
        Some(target_id) if segment_targets.contains_key(target_id)
    ) {
        errors.push(validation_issue(
            "RAG_STORAGE_TOPOLOGY_UNSUPPORTED",
            "source, textUnit, and vector bindings are all required.",
            Some("storageBinding"),
        ));
    }
    for segment in binding.bindings.keys() {
        if !matches!(
            binding.bindings.get(segment),
            Some(target_id) if segment_targets.contains_key(target_id)
        ) {
            errors.push(validation_issue(
                "RAG_STORAGE_TOPOLOGY_UNSUPPORTED",
                "Every segment binding must reference an existing storage target.",
                Some("storageBinding"),
            ));
        }
    }
    for target in segment_targets.values() {
        match target.backend {
            RagStorageBackend::Sqlite => {
                if target.layout != "TABLE" || target.table.as_deref().is_none_or(str::is_empty) {
                    errors.push(validation_issue(
                        "RAG_STORAGE_TOPOLOGY_UNSUPPORTED",
                        "SQLite targets must use TABLE layout and provide a table name.",
                        Some("storageBinding"),
                    ));
                }
            }
            RagStorageBackend::Graph => {
                let tree_target = binding
                    .bindings
                    .iter()
                    .any(|(segment, target_id)| segment == "tree" && target_id == &target.id);
                let layout_supported = target.layout == "GRAPH_SINGLE_NODE"
                    || (tree_target && target.layout == "GRAPH_TREE");
                if !layout_supported
                    || target.namespace.as_deref().is_none_or(str::is_empty)
                    || target.node_label.as_deref().is_none_or(str::is_empty)
                {
                    errors.push(validation_issue(
                        "RAG_STORAGE_TOPOLOGY_UNSUPPORTED",
                        "GRAPH targets must provide GRAPH_SINGLE_NODE layout, namespace, and nodeLabel; tree targets may use GRAPH_TREE.",
                        Some("storageBinding"),
                    ));
                }
            }
        }
    }
    let tree_backend = binding
        .bindings
        .get("tree")
        .and_then(|target_id| segment_targets.get(target_id))
        .map(|target| target.backend);
    let text_unit_backend = binding
        .bindings
        .get("textUnit")
        .and_then(|target_id| segment_targets.get(target_id))
        .map(|target| target.backend);
    if tree_backend == Some(RagStorageBackend::Sqlite)
        && text_unit_backend != Some(RagStorageBackend::Sqlite)
    {
        errors.push(validation_issue(
            "RAG_STORAGE_TOPOLOGY_UNSUPPORTED",
            "SQLite tree storage requires SQLite textUnit storage.",
            Some("storageBinding"),
        ));
    }

    let has_business_graph_target = binding
        .bindings
        .iter()
        .filter(|(segment, _)| segment.as_str() != "diagnostics")
        .any(|(_, target_id)| {
            segment_targets
                .get(target_id)
                .is_some_and(|target| target.backend == RagStorageBackend::Graph)
        });
    if capabilities.graph && !has_business_graph_target {
        errors.push(validation_issue(
            "RAG_STORAGE_GRAPH_BINDING_REQUIRED",
            "Graph enabled requires at least one non-diagnostics GRAPH target.",
            Some("storageBinding"),
        ));
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(RagStorageBinding {
        segment_bindings: binding.bindings.clone(),
        segment_targets,
    })
}

fn runtime_segment_targets(
    binding: &RagStorageBindingPayload,
) -> Result<BTreeMap<String, RagStorageSegmentTarget>, Vec<RagValidationIssueView>> {
    binding
        .targets
        .iter()
        .map(|target| {
            let backend = RagStorageBackend::try_from(target.backend.as_str()).map_err(|_| {
                vec![validation_issue(
                    "RAG_STORAGE_TOPOLOGY_UNSUPPORTED",
                    "Storage targets must use a supported backend.",
                    Some("storageBinding"),
                )]
            })?;
            Ok((
                target.id.clone(),
                RagStorageSegmentTarget {
                    id: target.id.clone(),
                    backend,
                    layout: target.layout.clone(),
                    table: target.table.clone(),
                    namespace: target.namespace.clone(),
                    node_label: target.node_label.clone(),
                    code_strategy: target.code_strategy.clone(),
                },
            ))
        })
        .collect()
}

fn parse_status(status: &str) -> Result<RagStatus, RagError> {
    RagStatus::try_from(status).map_err(|_| RagError::ManagementRequestInvalid)
}

fn parse_profile_type(profile_type: &str) -> Result<RagProfileType, RagError> {
    RagProfileType::try_from(profile_type).map_err(|_| RagError::ManagementRequestInvalid)
}

fn references_corpus(profile: &RagProfileRecord, corpus_id: &str) -> bool {
    profile.primary_corpus_id.as_deref() == Some(corpus_id)
        || profile.corpus_ids.iter().any(|value| value == corpus_id)
}

fn best_profile_reference<'a>(
    profiles: &'a [RagProfileRecord],
    profile_type: RagProfileType,
    corpus_id: &str,
) -> Option<&'a RagProfileRecord> {
    let preferred_profile_id = match profile_type {
        RagProfileType::Ingest => preferred_ingest_profile_id_for_corpus(corpus_id),
        RagProfileType::Retrieval => None,
    };
    profiles
        .iter()
        .filter(|profile| {
            profile.profile_type == profile_type && references_corpus(profile, corpus_id)
        })
        .min_by_key(|profile| {
            (
                usize::from(Some(profile.profile_id.as_str()) != preferred_profile_id),
                usize::from(profile.primary_corpus_id.as_deref() != Some(corpus_id)),
                profile.corpus_ids.len(),
                profile.profile_id.as_str(),
            )
        })
}

fn preferred_ingest_profile_id_for_corpus(corpus_id: &str) -> Option<&'static str> {
    match corpus_id {
        "memory-card" => Some("memory-card-ingest-v1"),
        "project-doc" => Some("project-doc-mineru-ingest-v1"),
        "skill" => Some("skill-ingest-v1"),
        "skill-graph" => Some("skill-graph-ingest-v1"),
        _ => None,
    }
}

fn storage_summary(binding: &RagStorageBinding) -> String {
    ["source", "textUnit", "fulltext", "vector", "tree"]
        .into_iter()
        .map(|segment| {
            let target = binding.target_for_segment(segment);
            let physical = match target {
                Some(target) if target.backend == RagStorageBackend::Graph => format!(
                    "graph.{}/{}",
                    target.namespace.as_deref().unwrap_or("unknown"),
                    target.node_label.as_deref().unwrap_or("node")
                ),
                Some(target) => format!("sqlite.{}", target.table.as_deref().unwrap_or("unknown")),
                None => "unbound".into(),
            };
            format!("{segment}={physical}")
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

fn retrieval_route_type_options_for_corpus(
    corpus: &RagCorpusRecord,
) -> RagRetrievalRouteTypeOptionView {
    RagRetrievalRouteTypeOptionView {
        corpus_id: corpus.corpus_id.clone(),
        route_types: retrieval_route_types_for_corpus(corpus),
    }
}

fn retrieval_route_types_for_corpus(corpus: &RagCorpusRecord) -> Vec<String> {
    let mut route_types = Vec::new();
    if retrieval_strategy_ids_for_corpus(corpus, "FULLTEXT").is_some() {
        route_types.push("FULLTEXT".into());
    }
    if retrieval_strategy_ids_for_corpus(corpus, "VECTOR").is_some() {
        route_types.push("VECTOR".into());
    }
    if retrieval_strategy_ids_for_corpus(corpus, "TREE_FULLTEXT").is_some() {
        route_types.push("TREE_FULLTEXT".into());
    }
    if retrieval_strategy_ids_for_corpus(corpus, "TREE_VECTOR").is_some() {
        route_types.push("TREE_VECTOR".into());
    }
    if retrieval_strategy_ids_for_corpus(corpus, "STRUCTURED_FILTER").is_some() {
        route_types.push("STRUCTURED_FILTER".into());
    }
    route_types
}

fn retrieval_strategy_options_for_corpus(
    corpus: &RagCorpusRecord,
) -> Vec<RagRetrievalStrategyOptionView> {
    let route_types = retrieval_route_types_for_corpus(corpus);
    route_types
        .into_iter()
        .filter_map(|route_type| {
            retrieval_strategy_ids_for_corpus(corpus, &route_type).map(|strategy_ids| {
                RagRetrievalStrategyOptionView {
                    corpus_id: corpus.corpus_id.clone(),
                    route_type,
                    strategy_ids,
                }
            })
        })
        .collect()
}

fn retrieval_strategy_ids_for_corpus(
    corpus: &RagCorpusRecord,
    route_type: &str,
) -> Option<Vec<String>> {
    let capabilities = capabilities_from_value(&corpus.capabilities);
    match route_type {
        "FULLTEXT" if capabilities.fulltext => {
            match retrieval_backend_for_segment(corpus, "fulltext")? {
                RagStorageBackend::Sqlite => Some(vec!["FULLTEXT_SQLITE_FTS5_SIMPLE_V1".into()]),
                RagStorageBackend::Graph => Some(vec!["FULLTEXT_GRAPH_SINGLE_NODE_V1".into()]),
            }
        }
        "VECTOR" if capabilities.vector => match retrieval_backend_for_segment(corpus, "vector")? {
            RagStorageBackend::Sqlite => Some(vec!["VECTOR_SQLITE_VEC0_COSINE_V1".into()]),
            RagStorageBackend::Graph => Some(vec!["VECTOR_GRAPH_SINGLE_NODE_V1".into()]),
        },
        "TREE_FULLTEXT"
            if capabilities.tree && capabilities.fulltext && has_tree_storage(corpus) =>
        {
            Some(vec!["TREE_FULLTEXT_V1".into()])
        }
        "TREE_VECTOR" if capabilities.tree && capabilities.vector && has_tree_storage(corpus) => {
            Some(vec!["TREE_VECTOR_V1".into()])
        }
        "STRUCTURED_FILTER" if has_sqlite_text_unit_storage(corpus) => {
            Some(vec!["STRUCTURED_FILTER_SQLITE_V1".into()])
        }
        "GRAPH" => legacy_graph_strategy_ids_for_corpus(corpus),
        _ => None,
    }
}

fn retrieval_backend_for_segment(
    corpus: &RagCorpusRecord,
    segment: &str,
) -> Option<RagStorageBackend> {
    segment_target_backend(&corpus.storage_binding, segment)
}

fn segment_target_backend(binding: &RagStorageBinding, segment: &str) -> Option<RagStorageBackend> {
    let target_id = binding.segment_bindings.get(segment).or_else(|| {
        if segment == "fulltext" {
            binding.segment_bindings.get("textUnit")
        } else {
            None
        }
    })?;
    binding
        .segment_targets
        .get(target_id)
        .map(|target| target.backend)
}

fn has_sqlite_text_unit_storage(corpus: &RagCorpusRecord) -> bool {
    matches!(
        segment_target_backend(&corpus.storage_binding, "textUnit"),
        Some(RagStorageBackend::Sqlite)
    )
}

fn has_tree_storage(corpus: &RagCorpusRecord) -> bool {
    matches!(
        segment_target_backend(&corpus.storage_binding, "tree"),
        Some(RagStorageBackend::Sqlite | RagStorageBackend::Graph)
    )
}

fn legacy_graph_strategy_ids_for_corpus(corpus: &RagCorpusRecord) -> Option<Vec<String>> {
    let capabilities = capabilities_from_value(&corpus.capabilities);
    let mut strategy_ids = Vec::new();
    if capabilities.fulltext
        && matches!(
            retrieval_backend_for_segment(corpus, "fulltext"),
            Some(RagStorageBackend::Graph)
        )
    {
        strategy_ids.push("FULLTEXT_GRAPH_SINGLE_NODE_V1".into());
    }
    if capabilities.vector
        && matches!(
            retrieval_backend_for_segment(corpus, "vector"),
            Some(RagStorageBackend::Graph)
        )
    {
        strategy_ids.push("VECTOR_GRAPH_SINGLE_NODE_V1".into());
    }
    (!strategy_ids.is_empty()).then_some(strategy_ids)
}

fn route_strategy_matches_corpus_storage(
    route: &RagRetrievalRoutePayload,
    corpus: &RagCorpusRecord,
) -> bool {
    retrieval_strategy_ids_for_corpus(corpus, &route.route_type).is_some_and(|strategy_ids| {
        strategy_ids
            .iter()
            .any(|strategy_id| strategy_id == &route.strategy_id)
    })
}
