impl<R, S, E> RagSearchService<R, S, E>
where
    R: RagSearchRepository,
    S: RagStrategyRegistry,
    E: RagSearchRuntime,
{
    pub fn new(repository: R, strategy_registry: S, runtime: E) -> Self {
        Self {
            repository,
            strategy_registry,
            runtime,
            event_bus: EventBusHandle::disabled(),
        }
    }

    pub fn with_event_bus(mut self, event_bus: EventBusHandle) -> Self {
        self.event_bus = event_bus;
        self
    }

    pub async fn search(&self, request: RagSearchRequest) -> Result<RagSearchResult, RagError> {
        validate_request(&request)?;
        publish_search_event(&self.event_bus, "rag.search.started", &request);
        let profile = self
            .repository
            .get_profile(&request.profile_id)?
            .ok_or(RagError::RetrievalProfileNotFound)?;
        let corpus = self
            .repository
            .get_corpus(
                profile
                    .primary_corpus_id
                    .as_deref()
                    .ok_or(RagError::RetrievalProfileInvalid)?,
            )?
            .ok_or(RagError::CorpusNotFound)?;
        if corpus.status != RagStatus::Enabled {
            return Err(RagError::RetrievalProfileInvalid);
        }
        let compiled = compile_retrieval_profile(&profile, &corpus, &self.strategy_registry)?;
        if !compiled
            .supported_context_strategies
            .contains(&request.context.strategy)
        {
            return Err(RagError::RetrievalProfileInvalid);
        }
        let route_corpora = self.load_route_corpora(&compiled)?;
        validate_return_fields(&request.context.return_fields, &route_corpora)?;
        let mut pipeline_context =
            RagRetrievalPipelineContext::new(&route_corpora, &profile.filters, &request);
        let stage_results = self
            .execute_retrieval_pipeline(&compiled, &mut pipeline_context)
            .await?;
        let ranked_hits = pipeline_context
            .artifacts
            .get_optional_cloned::<Vec<RankedCandidate>>("hits.final")?
            .or_else(|| {
                pipeline_context
                    .artifacts
                    .get_optional_cloned::<Vec<RankedCandidate>>("hits.deduped")
                    .ok()
                    .flatten()
            })
            .unwrap_or_default();
        let context_pack = pipeline_context
            .artifacts
            .get_optional_cloned::<Option<ContextPack>>("context.pack")?
            .flatten();
        let mut warnings = pipeline_context.warnings;
        publish_route_events(&self.event_bus, &ranked_hits, &request);
        let trace_code = persist_trace_if_enabled(
            &self.repository,
            &request,
            &compiled,
            &ranked_hits,
            context_pack.as_ref(),
            &stage_results,
            &mut warnings,
        )?;
        publish_search_event(&self.event_bus, "rag.search.completed", &request);
        let hits = ranked_hits
            .into_iter()
            .map(|candidate| candidate.result)
            .collect();

        Ok(RagSearchResult {
            profile_id: compiled.profile_id,
            query: request.query,
            context_strategy: request.context.strategy,
            hits,
            context_pack,
            warnings,
            trace_code,
            stage_results,
        })
    }

    fn load_route_corpora(
        &self,
        compiled: &CompiledRetrievalProfile,
    ) -> Result<BTreeMap<String, crate::domain::rag::RagCorpusRecord>, RagError> {
        let mut corpora = BTreeMap::new();
        for corpus_id in &compiled.corpus_ids {
            let corpus = self
                .repository
                .get_corpus(corpus_id)?
                .ok_or(RagError::CorpusNotFound)?;
            if corpus.status != RagStatus::Enabled {
                return Err(RagError::RetrievalProfileInvalid);
            }
            corpora.insert(corpus_id.clone(), corpus);
        }
        Ok(corpora)
    }

    async fn execute_retrieval_pipeline(
        &self,
        compiled: &CompiledRetrievalProfile,
        context: &mut RagRetrievalPipelineContext<'_>,
    ) -> Result<Vec<RagStageExecutionResult>, RagError> {
        if context.request.query.trim().is_empty()
            && !self.has_blank_query_route(compiled, context)?
        {
            return Err(RagError::SearchRequestInvalid);
        }
        let request_filter_corpora = compiled
            .routes
            .iter()
            .filter_map(|route| context.route_corpora.get(&route.corpus_id))
            .collect::<Vec<_>>();
        validate_request_filters_for_corpora(&request_filter_corpora, &context.request.filters)?;
        let pipeline = compiled
            .pipeline
            .as_ref()
            .ok_or(RagError::RetrievalProfileInvalid)?;
        context
            .artifacts
            .put_json("query.original", json!(context.request.query.clone()));
        context
            .artifacts
            .put_json("query.effective", json!(context.request.query.clone()));
        context
            .artifacts
            .put_json("request.filters", context.request.filters.clone());
        context.artifacts.put_json(
            "request.accessContext",
            json!({
                "userCode": context.request.access_context.user_code,
                "agentCode": context.request.access_context.agent_code,
                "workspaceCode": context.request.access_context.workspace_code
            }),
        );
        let mut stage_results = Vec::new();
        for level in RagPipelineExecutor::node_levels(pipeline) {
            for node in level {
                if node
                    .input_artifact_keys
                    .iter()
                    .any(|key| !context.artifacts.contains(key))
                    && node.failure_policy == "FAIL_PIPELINE"
                {
                    RagPipelineExecutor::ensure_inputs::<RagError>(node, &context.artifacts)?;
                }
                let started = std::time::Instant::now();
                let outcome = self
                    .execute_retrieval_pipeline_node(compiled, context, node)
                    .await?;
                if outcome.status == "COMPLETED" {
                    for key in &node.output_artifact_keys {
                        if !context.artifacts.contains(key) {
                            context
                                .artifacts
                                .put_json(key.clone(), json!({"nodeId": node.node_id}));
                        }
                    }
                }
                if outcome.status == "SKIPPED" {
                    continue;
                }
                let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
                stage_results.push(RagPipelineExecutor::stage_result(
                    node,
                    outcome.status,
                    duration_ms,
                    outcome.warnings,
                    outcome.strategy_id,
                ));
            }
        }
        Ok(stage_results)
    }

    async fn execute_retrieval_pipeline_node(
        &self,
        compiled: &CompiledRetrievalProfile,
        context: &mut RagRetrievalPipelineContext<'_>,
        node: &CompiledRagPipelineNode,
    ) -> Result<RagStageOutcome, RagError> {
        match node.stage_type.as_str() {
            "QUESTION_GENERALIZATION" => self.execute_question_generalization_stage(context, node),
            "QUERY_REWRITE" => self.execute_query_rewrite_stage(context, node),
            "ROUTE_SEARCH" => self.execute_route_stage(compiled, context, node).await,
            "FUSE_RESULTS" => self.execute_fuse_stage(compiled, context).await,
            "DEDUP_RESULTS" => {
                let hits = context
                    .artifacts
                    .get_cloned::<Vec<RankedCandidate>>("hits.fused")?;
                context.artifacts.put("hits.deduped", hits);
                Ok(RagStageOutcome::completed())
            }
            "BUILD_CONTEXT" => self.execute_context_stage(compiled, context, node).await,
            _ => Err(RagError::RetrievalProfileInvalid),
        }
    }

    fn execute_question_generalization_stage(
        &self,
        context: &mut RagRetrievalPipelineContext<'_>,
        node: &CompiledRagPipelineNode,
    ) -> Result<RagStageOutcome, RagError> {
        let query = context.artifacts.get_cloned::<Value>("query.original")?;
        let mut query = query
            .as_str()
            .ok_or(RagError::RetrievalProfileInvalid)?
            .trim()
            .trim_end_matches(['?', '!', '.', ',', ';', ':'])
            .trim()
            .to_string();
        for term in node
            .config
            .as_ref()
            .and_then(|config| config.get("appendTerms"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            let term = term.trim();
            if !term.is_empty() && !query_contains_phrase(&query, term) {
                if !query.is_empty() {
                    query.push(' ');
                }
                query.push_str(term);
            }
        }
        context
            .artifacts
            .put_json("query.generalized", json!(query.clone()));
        context.artifacts.put_json("query.effective", json!(query));
        context.query_embedding = None;
        Ok(RagStageOutcome::completed())
    }

    fn execute_query_rewrite_stage(
        &self,
        context: &mut RagRetrievalPipelineContext<'_>,
        node: &CompiledRagPipelineNode,
    ) -> Result<RagStageOutcome, RagError> {
        let input_query = effective_query(context)?;
        let rewritten = node
            .config
            .as_ref()
            .and_then(|config| config.get("template"))
            .and_then(Value::as_str)
            .map(|template| template.replace("{query}", &input_query))
            .unwrap_or_else(|| input_query.clone());
        let rewritten = rewritten.trim().to_string();
        let rewritten = if rewritten.is_empty() {
            input_query
        } else {
            rewritten
        };
        context
            .artifacts
            .put_json("query.rewritten", json!(rewritten.clone()));
        context
            .artifacts
            .put_json("query.effective", json!(rewritten));
        context.query_embedding = None;
        Ok(RagStageOutcome::completed())
    }

    async fn execute_route_stage(
        &self,
        compiled: &CompiledRetrievalProfile,
        context: &mut RagRetrievalPipelineContext<'_>,
        node: &CompiledRagPipelineNode,
    ) -> Result<RagStageOutcome, RagError> {
        let route = compiled
            .routes
            .iter()
            .find(|route| route.route_id == node.node_id)
            .ok_or(RagError::RetrievalProfileInvalid)?;
        let strategy = retriever_strategy(&route.retriever)?;
        let effective_query = effective_query(context)?;
        if effective_query.trim().is_empty() && !strategy.can_run_without_query() {
            return Ok(RagStageOutcome {
                status: "SKIPPED".into(),
                warnings: Vec::new(),
                strategy_id: None,
            });
        }
        if strategy.is_structured_filter() {
            let corpus = context
                .route_corpora
                .get(&route.corpus_id)
                .ok_or(RagError::CorpusNotFound)?;
            if corpus
                .storage_binding
                .sqlite_table_for_segment("textUnit")
                .is_none()
            {
                return Ok(RagStageOutcome {
                    status: "SKIPPED".into(),
                    warnings: Vec::new(),
                    strategy_id: None,
                });
            }
        }
        if strategy.needs_query_embedding() && context.query_embedding.is_none() {
            match self
                .runtime
                .embed(RagEmbeddingCall {
                    workspace_code: context.request.workspace_code.clone(),
                    model_code: compiled.embedding_model_code.clone(),
                    input: effective_query.clone(),
                })
                .await
            {
                Ok(embedding) => context.query_embedding = Some(embedding),
                Err(_) if compiled.degrade_policy == "FAIL_FAST" => {
                    return Err(RagError::SearchRouteUnavailable);
                }
                Err(_) => {
                    let warning = json!({
                        "routeId": route.route_id,
                        "code": strategy.unavailable_code(),
                        "message": strategy.unavailable_message()
                    });
                    context.warnings.push(warning.clone());
                    return Ok(RagStageOutcome {
                        status: "SKIPPED".into(),
                        warnings: vec![warning],
                        strategy_id: None,
                    });
                }
            }
        }
        if strategy.should_skip_without_embedding() && context.query_embedding.is_none() {
            return Ok(RagStageOutcome {
                status: "SKIPPED".into(),
                warnings: Vec::new(),
                strategy_id: None,
            });
        }
        let corpus = context
            .route_corpora
            .get(&route.corpus_id)
            .ok_or(RagError::CorpusNotFound)?;
        let route_profile_filters =
            merge_search_filters(corpus, context.profile_filters, &route.filters)?;
        let (filters, source_version) =
            filters_and_version(corpus, &route_profile_filters, context.request, compiled)?;
        let query = RagRouteSearchQuery {
            corpus_id: route.corpus_id.clone(),
            route_id: route.route_id.clone(),
            retriever: strategy.storage_retriever.to_string(),
            query: effective_query.clone(),
            workspace_code: context.request.workspace_code.clone(),
            query_embedding: (strategy.needs_query_embedding())
                .then(|| {
                    context
                        .query_embedding
                        .as_ref()
                        .map(|result| result.embedding.clone())
                })
                .flatten(),
            query_embedding_model_code: (strategy.needs_query_embedding())
                .then(|| {
                    context
                        .query_embedding
                        .as_ref()
                        .map(|result| result.model_code.clone())
                })
                .flatten(),
            source_version,
            scopes: search_scopes(&context.request.access_context),
            filters,
            return_fields: context.request.context.return_fields.clone(),
            limit: route.top_k.max(context.request.limit).max(1) * 4,
        };
        let rows = match self.repository.search_route(query) {
            Ok(rows) => rows,
            Err(RagError::SearchRouteUnavailable) if compiled.degrade_policy != "FAIL_FAST" => {
                let warning = json!({
                    "routeId": route.route_id,
                    "code": strategy.unavailable_code(),
                    "message": strategy.unavailable_message()
                });
                context.warnings.push(warning.clone());
                return Ok(RagStageOutcome {
                    status: "SKIPPED".into(),
                    warnings: vec![warning],
                    strategy_id: None,
                });
            }
            Err(error) => return Err(error),
        };
        let mut route_hits = context
            .artifacts
            .get_optional_cloned::<Vec<RankedCandidate>>("hits.route")?
            .unwrap_or_default();
        route_hits.extend(rank_route_rows(
            rows,
            route,
            &strategy,
            &effective_query,
            context
                .query_embedding
                .as_ref()
                .map(|result| result.embedding.as_slice()),
        ));
        context.artifacts.put("hits.route", route_hits);
        Ok(RagStageOutcome::completed())
    }

    async fn execute_fuse_stage(
        &self,
        compiled: &CompiledRetrievalProfile,
        context: &mut RagRetrievalPipelineContext<'_>,
    ) -> Result<RagStageOutcome, RagError> {
        let route_hits = context
            .artifacts
            .get_cloned::<Vec<RankedCandidate>>("hits.route")?;
        let mut hits = fuse_and_dedup(compiled, route_hits);
        hits.sort_by(stable_hit_order);
        apply_topic_policy(compiled, &mut hits);
        let effective_query = effective_query(context)?;
        if !effective_query.trim().is_empty() && compiled.rerank_enabled && hits.len() > 1 {
            apply_rerank(
                &self.runtime,
                compiled,
                context.request,
                &effective_query,
                &mut hits,
            )
            .await?;
        }
        hits.truncate(context.request.limit);
        context.artifacts.put("hits.fused", hits);
        Ok(RagStageOutcome::completed())
    }

    async fn execute_context_stage(
        &self,
        compiled: &CompiledRetrievalProfile,
        context: &mut RagRetrievalPipelineContext<'_>,
        _node: &CompiledRagPipelineNode,
    ) -> Result<RagStageOutcome, RagError> {
        let mut hits = context
            .artifacts
            .get_cloned::<Vec<RankedCandidate>>("hits.deduped")?;
        self.expand_final_graph_hits(&mut hits, compiled, context.request, context.route_corpora)?;
        let context_pack = assemble_context_pack(
            &context.request.context.strategy,
            &hits,
            context.request.token_budget,
        );
        context.artifacts.put("hits.final", hits);
        context.artifacts.put("context.pack", context_pack.clone());
        Ok(RagStageOutcome {
            status: "COMPLETED".into(),
            warnings: Vec::new(),
            strategy_id: Some(context.request.context.strategy.clone()),
        })
    }

    fn has_blank_query_route(
        &self,
        compiled: &CompiledRetrievalProfile,
        context: &RagRetrievalPipelineContext<'_>,
    ) -> Result<bool, RagError> {
        for route in &compiled.routes {
            let strategy = retriever_strategy(&route.retriever)?;
            if !strategy.can_run_without_query() {
                continue;
            }
            if strategy.is_structured_filter() {
                let corpus = context
                    .route_corpora
                    .get(&route.corpus_id)
                    .ok_or(RagError::CorpusNotFound)?;
                if corpus
                    .storage_binding
                    .sqlite_table_for_segment("textUnit")
                    .is_none()
                {
                    continue;
                }
            }
            return Ok(true);
        }
        Ok(false)
    }

    fn expand_final_graph_hits(
        &self,
        hits: &mut [RankedCandidate],
        compiled: &CompiledRetrievalProfile,
        request: &RagSearchRequest,
        route_corpora: &BTreeMap<String, crate::domain::rag::RagCorpusRecord>,
    ) -> Result<(), RagError> {
        let Some(policy) = compiled.graph_expansion_policy.as_ref() else {
            return Ok(());
        };
        if !policy
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            return Ok(());
        }
        for hit in hits {
            let Some(corpus) = route_corpora.get(&hit.result.corpus_id) else {
                continue;
            };
            if !corpus.storage_binding.is_graph_backend() {
                continue;
            }
            let expanded = self
                .repository
                .expand_related_graph(RagGraphExpansionQuery {
                    corpus_id: hit.result.corpus_id.clone(),
                    source_code: hit.result.source_code.clone(),
                    source_version: hit.result.source_version.clone(),
                    target_code: hit.target_code.clone(),
                    scope_type: hit.scope_type.clone(),
                    scope_code: hit.scope_code.clone(),
                    workspace_code: request.workspace_code.clone(),
                    relationships: request.context.graph_expansion.relationships.clone(),
                    policy: policy.clone(),
                })?;
            if let Some(expanded) = expanded {
                let mut metadata = hit.result.metadata.as_object().cloned().unwrap_or_default();
                metadata.insert("relatedGraph".into(), expanded);
                hit.result.metadata = Value::Object(metadata);
            }
        }
        Ok(())
    }
}
