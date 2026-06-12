impl<R, S> RagManagementService<R, S>
where
    R: RagConfigRepository + RagManagementRepository,
    S: RagStrategyRegistry,
{
    pub fn new(repository: R, strategy_registry: S) -> Self {
        Self {
            repository,
            strategy_registry,
        }
    }

    pub fn list_corpora(&self) -> Result<Vec<RagCorpusManagementListItem>, RagError> {
        let corpora = self.repository.list_corpora()?;
        let profiles = self
            .repository
            .list_profiles(RagProfileListQuery::default())?;
        Ok(corpora
            .into_iter()
            .map(|corpus| {
                let capabilities = capabilities_from_value(&corpus.capabilities);
                let metadata_groups = metadata_groups_from_value(&corpus.capabilities);
                let ingest_profile_id =
                    best_profile_reference(&profiles, RagProfileType::Ingest, &corpus.corpus_id)
                        .map(|profile| profile.profile_id.clone());
                let retrieval_profile_id =
                    best_profile_reference(&profiles, RagProfileType::Retrieval, &corpus.corpus_id)
                        .map(|profile| profile.profile_id.clone());
                RagCorpusManagementListItem {
                    corpus_id: corpus.corpus_id,
                    display_name: corpus.display_name,
                    description: corpus.description,
                    status: corpus.status.as_str().to_string(),
                    capabilities,
                    metadata_groups,
                    storage_summary: storage_summary(&corpus.storage_binding),
                    ingest_profile_id,
                    retrieval_profile_id,
                }
            })
            .collect())
    }

    pub fn get_corpus(&self, corpus_id: &str) -> Result<RagCorpusManagementView, RagError> {
        let managed = self
            .repository
            .get_managed_corpus(corpus_id)?
            .ok_or(RagError::CorpusNotFound)?;
        Ok(project_corpus_view(managed))
    }

    pub fn corpus_catalog(&self) -> RagCorpusCatalogView {
        RagCorpusCatalogView {
            storage_backends: vec!["SQLITE".into(), "GRAPH".into()],
            code_strategies: vec!["CORPUS_SOURCE_VERSION_SCOPE_HASH".into()],
            contract_fields: RagContractFieldCatalog {
                source: SOURCE_CONTRACT_FIELDS
                    .into_iter()
                    .map(|(logical_name, column)| RagContractFieldDescriptor {
                        logical_name: logical_name.to_string(),
                        column: column.to_string(),
                    })
                    .collect(),
                text_unit: TEXT_UNIT_CONTRACT_FIELDS
                    .into_iter()
                    .map(|(logical_name, column)| RagContractFieldDescriptor {
                        logical_name: logical_name.to_string(),
                        column: column.to_string(),
                    })
                    .collect(),
            },
            filter_field_types: vec!["string".into(), "integer".into(), "boolean".into()],
            filter_operators: vec![
                "eq".into(),
                "in".into(),
                "gt".into(),
                "gte".into(),
                "lt".into(),
                "lte".into(),
            ],
            capability_rules: vec![validation_issue(
                "RAG_STORAGE_GRAPH_BINDING_REQUIRED",
                "Graph enabled requires at least one non-diagnostics GRAPH target.",
                Some("storageBinding"),
            )],
            compatibility_rules: vec![validation_issue(
                "RAG_STORAGE_TOPOLOGY_UNSUPPORTED",
                "Phase 1 accepts SQLite embedded, SQLite split, or graph single-node projections only.",
                Some("storageBinding"),
            )],
        }
    }

    pub fn verify_corpus(
        &self,
        input: RagCorpusManagementInput,
    ) -> Result<RagCorpusVerifyView, RagError> {
        let existing = self.repository.get_corpus(&input.corpus_id)?;
        let (projection, errors) = verify_corpus_input(&input, existing.as_ref())?;
        Ok(RagCorpusVerifyView {
            valid: errors.is_empty(),
            errors,
            runtime_projection: projection,
        })
    }

    pub fn create_corpus(
        &self,
        input: RagCorpusManagementInput,
    ) -> Result<RagCorpusManagementView, RagError> {
        let existing = self.repository.get_corpus(&input.corpus_id)?;
        let (projection, errors) = verify_corpus_input(&input, existing.as_ref())?;
        if !errors.is_empty() {
            return Err(first_validation_error(&errors));
        }
        let record = build_corpus_record(&input, existing.as_ref(), &projection)?;
        self.repository.save_managed_corpus(&record)?;
        let managed = self
            .repository
            .get_managed_corpus(&record.corpus_id)?
            .ok_or(RagError::StorageFailure)?;
        Ok(project_corpus_view(managed))
    }

    pub fn update_corpus(
        &self,
        corpus_id: &str,
        mut input: RagCorpusManagementInput,
    ) -> Result<RagCorpusManagementView, RagError> {
        if input.corpus_id.trim().is_empty() {
            input.corpus_id = corpus_id.to_string();
        }
        if input.corpus_id != corpus_id {
            return Err(RagError::ManagementRequestInvalid);
        }
        self.create_corpus(input)
    }

    pub fn list_profiles(
        &self,
        profile_type: Option<&str>,
    ) -> Result<Vec<RagProfileManagementListItem>, RagError> {
        let query = RagProfileListQuery {
            profile_type: profile_type.map(parse_profile_type).transpose()?,
            ..RagProfileListQuery::default()
        };
        Ok(self
            .repository
            .list_profiles(query)?
            .into_iter()
            .map(|profile| RagProfileManagementListItem {
                profile_id: profile.profile_id,
                profile_type: profile.profile_type.as_str().to_string(),
                status: profile.status.as_str().to_string(),
                description: profile_description(&profile.config),
                primary_corpus_id: profile.primary_corpus_id,
                corpus_ids: profile.corpus_ids,
                required_source_version: profile.required_source_version,
            })
            .collect())
    }

    pub fn get_profile(&self, profile_id: &str) -> Result<RagProfileManagementView, RagError> {
        let profile = self
            .repository
            .get_profile(profile_id)?
            .ok_or(RagError::ProfileNotFound)?;
        Ok(project_profile_view(&profile))
    }

    pub fn profile_catalog(&self) -> Result<RagProfileCatalogView, RagError> {
        let corpus_records = self.repository.list_corpora()?;
        let corpora = corpus_records
            .iter()
            .map(|corpus| RagCorpusOptionView {
                corpus_id: corpus.corpus_id.clone(),
                display_name: corpus.display_name.clone(),
                status: corpus.status.as_str().to_string(),
                capabilities: capabilities_from_value(&corpus.capabilities),
                storage_summary: storage_summary(&corpus.storage_binding),
            })
            .collect();
        let retrieval_route_types = corpus_records
            .iter()
            .map(retrieval_route_type_options_for_corpus)
            .collect();
        let retrieval_strategies = corpus_records
            .iter()
            .flat_map(retrieval_strategy_options_for_corpus)
            .collect();
        Ok(RagProfileCatalogView {
            corpora,
            ingest_stage_definitions: DEFAULT_INGEST_STAGE_ORDER
                .into_iter()
                .map(ToOwned::to_owned)
                .collect(),
            ingest_stage_strategies: BTreeMap::from([
                ("validateSource".into(), vec!["RAG_VALIDATE_SOURCE".into()]),
                (
                    "preparePayload".into(),
                    vec![
                        "RAG_TEXT_CONTRACT_PAYLOAD_V1".into(),
                        "RAG_DOCUMENT_REF_CONTRACT_PAYLOAD_V1".into(),
                    ],
                ),
                (
                    "resolveSource".into(),
                    vec!["PROJECT_DOC_RESOLVE_SOURCE".into()],
                ),
                (
                    "parseDocument".into(),
                    vec!["RAG_PARSE_MARKDOWN_DOCUMENT".into()],
                ),
                ("splitText".into(), vec!["RAG_SPLIT_TEXT".into()]),
                (
                    "extractTree".into(),
                    vec!["RAG_MARKDOWN_TREE_EXTRACT_V1".into()],
                ),
                ("fulltextIndex".into(), vec!["RAG_FULLTEXT_INDEX".into()]),
                ("vectorIndex".into(), vec!["RAG_VECTOR_INDEX".into()]),
                (
                    "persistTree".into(),
                    vec!["RAG_PERSIST_TREE_GRAPH_V1".into()],
                ),
                ("persistSource".into(), vec!["RAG_PERSIST_SOURCE".into()]),
            ]),
            retrieval_route_types,
            retrieval_strategies,
            fusion_policy_types: vec!["SUM_SCORE".into(), "RRF".into()],
            dedup_policy_types: vec!["TEXT_UNIT".into(), "CANONICAL_TEXT_UNIT".into()],
            topic_policy_types: vec!["NONE".into(), "EMBEDDING_MMR_V1".into()],
            graph_expansion_policies: vec!["OUTGOING_DEPTH_1".into()],
            context_strategies: SUPPORTED_CONTEXT_STRATEGIES
                .into_iter()
                .map(ToOwned::to_owned)
                .collect(),
        })
    }

    pub fn pipeline_catalog(&self, profile_type: &str) -> Result<RagPipelineCatalogView, RagError> {
        Ok(pipeline_catalog(parse_profile_type(profile_type)?))
    }

    pub fn validate_filters(
        &self,
        input: RagFilterValidateInput,
    ) -> Result<RagFilterValidateView, RagError> {
        let corpus = self
            .repository
            .get_corpus(&input.corpus_id)?
            .ok_or(RagError::CorpusNotFound)?;
        let (normalized_filters, field_errors) = normalize_filter_object(&corpus, &input.filters);
        Ok(RagFilterValidateView {
            valid: field_errors.is_empty(),
            normalized_filters,
            field_errors,
        })
    }

    pub fn verify_profile(
        &self,
        input: RagProfileManagementInput,
    ) -> Result<RagProfileVerifyView, RagError> {
        let corpora = self.repository.list_corpora()?;
        let errors = validate_profile_input(&input, &corpora, &self.strategy_registry)?;
        Ok(RagProfileVerifyView {
            valid: errors.is_empty(),
            errors,
        })
    }

    pub fn create_profile(
        &self,
        input: RagProfileManagementInput,
    ) -> Result<RagProfileManagementView, RagError> {
        let corpora = self.repository.list_corpora()?;
        let errors = validate_profile_input(&input, &corpora, &self.strategy_registry)?;
        if !errors.is_empty() {
            return Err(first_validation_error(&errors));
        }
        let record = build_profile_record(&input, &corpora)?;
        self.repository.save_profile(&record)?;
        let saved = self
            .repository
            .get_profile(&record.profile_id)?
            .ok_or(RagError::StorageFailure)?;
        Ok(project_profile_view(&saved))
    }

    pub fn update_profile(
        &self,
        profile_id: &str,
        mut input: RagProfileManagementInput,
    ) -> Result<RagProfileManagementView, RagError> {
        if input.profile_id.trim().is_empty() {
            input.profile_id = profile_id.to_string();
        }
        if input.profile_id != profile_id {
            return Err(RagError::ManagementRequestInvalid);
        }
        self.create_profile(input)
    }
}

fn project_corpus_view(managed: RagCorpusManagementRecord) -> RagCorpusManagementView {
    let storage_binding = storage_binding_payload_from_record(&managed.record.storage_binding);
    let runtime_projection = RagRuntimeProjectionView {
        compatible: true,
        storage_binding: storage_binding.clone(),
        issues: Vec::new(),
    };
    RagCorpusManagementView {
        corpus_id: managed.record.corpus_id,
        display_name: managed.record.display_name,
        description: managed.record.description,
        status: managed.record.status.as_str().to_string(),
        capabilities: capabilities_from_value(&managed.record.capabilities),
        metadata_groups: metadata_groups_from_value(&managed.record.capabilities),
        filter_schema: managed.record.filter_schema,
        field_mapping: RagFieldMappingPayload {
            source: managed.record.source_field_mapping.clone(),
            text_unit: if managed.record.text_unit_field_mapping.is_empty() {
                derive_text_unit_mapping(&managed.record.field_mapping)
            } else {
                managed.record.text_unit_field_mapping.clone()
            },
        },
        storage_binding,
        runtime_projection,
    }
}

fn verify_corpus_input(
    input: &RagCorpusManagementInput,
    existing: Option<&RagCorpusRecord>,
) -> Result<(RagRuntimeProjectionView, Vec<RagValidationIssueView>), RagError> {
    let mut errors = Vec::new();
    if input.corpus_id.trim().is_empty() || input.display_name.trim().is_empty() {
        errors.push(validation_issue(
            "RAG_MANAGEMENT_REQUEST_INVALID",
            "corpusId and displayName are required.",
            None,
        ));
    }
    let projection = match validate_storage_binding(&input.storage_binding, &input.capabilities) {
        Ok(binding) => RagRuntimeProjectionView {
            compatible: true,
            storage_binding: storage_binding_payload_from_record(&binding),
            issues: Vec::new(),
        },
        Err(mut projection_errors) => {
            errors.append(&mut projection_errors);
            RagRuntimeProjectionView {
                compatible: false,
                storage_binding: existing
                    .map(|record| storage_binding_payload_from_record(&record.storage_binding))
                    .unwrap_or_else(|| input.storage_binding.clone()),
                issues: errors.clone(),
            }
        }
    };
    Ok((projection, errors))
}

fn build_corpus_record(
    input: &RagCorpusManagementInput,
    existing: Option<&RagCorpusRecord>,
    _projection: &RagRuntimeProjectionView,
) -> Result<RagCorpusRecord, RagError> {
    let storage_binding = validate_storage_binding(&input.storage_binding, &input.capabilities)
        .map_err(|_| RagError::StorageBindingInvalid)?;
    let metadata_groups = input
        .metadata_groups
        .clone()
        .or_else(|| existing.map(|record| metadata_groups_from_value(&record.capabilities)))
        .unwrap_or_default();
    Ok(RagCorpusRecord {
        corpus_id: input.corpus_id.clone(),
        display_name: input.display_name.clone(),
        status: parse_status(&input.status)?,
        description: input.description.clone(),
        capabilities: capabilities_value(&input.capabilities, &metadata_groups),
        filter_schema: input.filter_schema.clone(),
        field_mapping: merge_field_mappings(&input.field_mapping),
        source_field_mapping: input.field_mapping.source.clone(),
        text_unit_field_mapping: input.field_mapping.text_unit.clone(),
        storage_binding,
    })
}

