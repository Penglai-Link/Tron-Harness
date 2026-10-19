use super::*;
use crate::application::rag::{RagEmbeddingBatchCall, RagEmbeddingCall, RagEmbeddingResult};
use eliteforge_agent_framework::EmbeddingVector;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Default)]
struct BatchRuntime {
    calls: Arc<Mutex<Vec<Vec<String>>>>,
    route_resolutions: Arc<Mutex<usize>>,
    requested_models: Arc<Mutex<Vec<Option<String>>>>,
    mode: &'static str,
    after_first: Option<Arc<dyn Fn() + Send + Sync>>,
    changed: Arc<AtomicBool>,
}
impl RagEmbeddingRuntime for BatchRuntime {
    fn embedding_batch_input_limit(
        &self,
        _: &str,
        _: Option<&str>,
    ) -> Result<Option<usize>, String> {
        Ok((self.mode == "provider-limit").then_some(4))
    }
    fn resolve_index_model(&self, _: &str, model: Option<&str>) -> Result<Option<String>, String> {
        if model == Some("PERSON_AI:ROUTE:EmbeddingAuto") {
            let mut count = self.route_resolutions.lock().unwrap();
            *count += 1;
            // A second route resolution would choose a different vector space.
            Ok(Some(if *count == 1 { "model" } else { "changed" }.into()))
        } else {
            Ok(model.map(str::to_owned))
        }
    }

    fn embedding_fingerprint(
        &self,
        _: &str,
        _: Option<&str>,
        _: usize,
    ) -> Result<Option<String>, String> {
        Ok(Some(
            if self.changed.load(Ordering::SeqCst) {
                "changed"
            } else {
                "original"
            }
            .into(),
        ))
    }
    fn embed<'a>(
        &'a self,
        _: RagEmbeddingCall,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<RagEmbeddingResult, String>> + Send + 'a>,
    > {
        Box::pin(async { panic!("ingestion must not call the single-input adapter") })
    }
    fn embed_batch<'a>(
        &'a self,
        call: RagEmbeddingBatchCall,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<RagEmbeddingBatchResult, String>> + Send + 'a>,
    > {
        Box::pin(async move {
            self.requested_models
                .lock()
                .unwrap()
                .push(call.model_code.clone());
            let mut calls = self.calls.lock().unwrap();
            calls.push(call.inputs.clone());
            if calls.len() == 1 {
                if let Some(update) = &self.after_first {
                    update();
                }
            }
            if self.mode == "fail_second" && calls.len() == 2 {
                return Err("outage".into());
            }
            if self.mode == "fingerprint" {
                self.changed.store(true, Ordering::SeqCst);
            }
            let mut vectors: Vec<_> = call
                .inputs
                .iter()
                .enumerate()
                .map(|(index, input)| EmbeddingVector {
                    index,
                    values: vec![input.parse::<f32>().unwrap_or(index as f32); call.dimensions],
                })
                .rev()
                .collect();
            match self.mode {
                "missing" => {
                    vectors.pop();
                }
                "duplicate" => {
                    vectors[0].index = vectors[1].index;
                }
                "out_of_range" => {
                    vectors[0].index = call.inputs.len();
                }
                "dimension" => {
                    vectors[0].values.pop();
                }
                "nan" => {
                    vectors[0].values[0] = f32::NAN;
                }
                _ => {}
            }
            Ok(RagEmbeddingBatchResult {
                model_code: if self.mode == "model" {
                    "changed"
                } else {
                    "model"
                }
                .into(),
                provider_model: Some(
                    if self.mode == "provider" && calls.len() > 1 {
                        "changed"
                    } else {
                        "provider"
                    }
                    .into(),
                ),
                vectors,
            })
        })
    }
}
#[derive(Default)]
struct Checkpoint {
    saved: Mutex<RagPreparedVectors>,
    batches: Mutex<Vec<usize>>,
    cancelled: AtomicBool,
    fail_save: bool,
}
impl RagIndexCheckpoint for Checkpoint {
    fn validate(&self) -> Result<(), String> {
        if self.cancelled.load(Ordering::SeqCst) {
            Err("cancelled".into())
        } else {
            Ok(())
        }
    }
    fn save_batch(&self, result: &RagEmbeddingBatchResult) -> Result<(), String> {
        if self.fail_save {
            return Err("storage failure".into());
        }
        self.batches.lock().unwrap().push(result.vectors.len());
        let mut saved = self.saved.lock().unwrap();
        saved.fingerprint = Some("original".into());
        saved.model_code = Some(result.model_code.clone());
        saved.provider_model = result.provider_model.clone();
        saved
            .vectors
            .extend(result.vectors.iter().map(|vector| vector.values.clone()));
        Ok(())
    }
}
fn inputs(count: usize) -> Vec<String> {
    (0..count).map(|index| index.to_string()).collect()
}
#[tokio::test]
async fn embedding_batch_100_inputs_use_four_calls_and_preserve_response_indices() {
    let runtime = BatchRuntime::default();
    let checkpoint = Checkpoint::default();
    let rag = RagService::new((), runtime.clone());
    let result = rag
        .prepare_vectors(
            "PLATFORM",
            Some("model"),
            &inputs(100),
            2,
            Default::default(),
            Some(&checkpoint),
        )
        .await
        .unwrap();
    assert_eq!(
        runtime
            .calls
            .lock()
            .unwrap()
            .iter()
            .map(Vec::len)
            .collect::<Vec<_>>(),
        [32, 32, 32, 4]
    );
    assert_eq!(*checkpoint.batches.lock().unwrap(), [32, 32, 32, 4]);
    for (index, vector) in result.vectors.iter().enumerate() {
        assert_eq!(*vector, vec![index as f32; 2]);
    }
}
#[tokio::test]
async fn embedding_batch_limits_include_utf8_tokens_exact_boundary_and_oversize() {
    let runtime = BatchRuntime::default();
    let rag =
        RagService::new((), runtime.clone()).with_embedding_batch_limits(RagEmbeddingBatchLimits {
            max_inputs: 3,
            max_tokens: 6,
        });
    rag.prepare_vectors(
        "PLATFORM",
        Some("model"),
        &["中".into(), "文".into(), "a".into(), "b".into()],
        2,
        Default::default(),
        None,
    )
    .await
    .unwrap();
    assert_eq!(
        *runtime.calls.lock().unwrap(),
        [vec!["中", "文"], vec!["a", "b"]]
    );
    runtime.calls.lock().unwrap().clear();
    assert!(
        rag.prepare_vectors(
            "PLATFORM",
            Some("model"),
            &["中文字".into()],
            2,
            Default::default(),
            None
        )
        .await
        .is_err()
    );
    assert!(runtime.calls.lock().unwrap().is_empty());
    for limits in [
        RagEmbeddingBatchLimits {
            max_inputs: 0,
            max_tokens: 6,
        },
        RagEmbeddingBatchLimits {
            max_inputs: 3,
            max_tokens: 0,
        },
    ] {
        assert!(
            RagService::new((), runtime.clone())
                .with_embedding_batch_limits(limits)
                .prepare_vectors(
                    "PLATFORM",
                    Some("model"),
                    &inputs(1),
                    2,
                    Default::default(),
                    None
                )
                .await
                .is_err()
        );
    }
}
#[tokio::test]
async fn embedding_batch_retry_skips_all_durable_completed_inputs() {
    let runtime = BatchRuntime {
        mode: "fail_second",
        ..Default::default()
    };
    let checkpoint = Checkpoint::default();
    let rag =
        RagService::new((), runtime.clone()).with_embedding_batch_limits(RagEmbeddingBatchLimits {
            max_inputs: 2,
            max_tokens: 100,
        });
    assert!(
        rag.prepare_vectors(
            "PLATFORM",
            Some("model"),
            &inputs(5),
            2,
            Default::default(),
            Some(&checkpoint)
        )
        .await
        .is_err()
    );
    let completed = checkpoint.saved.lock().unwrap().clone();
    assert_eq!(completed.vectors.len(), 2);
    let result = rag
        .prepare_vectors(
            "PLATFORM",
            Some("model"),
            &inputs(5),
            2,
            completed,
            Some(&checkpoint),
        )
        .await
        .unwrap();
    assert_eq!(
        *runtime.calls.lock().unwrap(),
        [vec!["0", "1"], vec!["2", "3"], vec!["2", "3"], vec!["4"]]
    );
    assert_eq!(result.vectors.len(), 5);
    assert_eq!(*checkpoint.batches.lock().unwrap(), [2, 2, 1]);
}
#[tokio::test]
async fn embedding_batch_invalid_responses_never_advance_checkpoint() {
    for mode in [
        "missing",
        "duplicate",
        "out_of_range",
        "dimension",
        "nan",
        "model",
        "fingerprint",
    ] {
        let runtime = BatchRuntime {
            mode,
            ..Default::default()
        };
        let checkpoint = Checkpoint::default();
        assert!(
            RagService::new((), runtime)
                .prepare_vectors(
                    "PLATFORM",
                    Some("model"),
                    &inputs(2),
                    2,
                    Default::default(),
                    Some(&checkpoint)
                )
                .await
                .is_err(),
            "{mode}"
        );
        assert!(
            checkpoint.saved.lock().unwrap().vectors.is_empty(),
            "{mode}"
        );
    }
    let checkpoint = Checkpoint {
        fail_save: true,
        ..Default::default()
    };
    assert!(
        RagService::new((), BatchRuntime::default())
            .prepare_vectors(
                "PLATFORM",
                Some("model"),
                &inputs(2),
                2,
                Default::default(),
                Some(&checkpoint)
            )
            .await
            .is_err()
    );
    assert!(checkpoint.saved.lock().unwrap().vectors.is_empty());
}
#[tokio::test]
async fn embedding_batch_provider_drift_and_cancelled_resume_cannot_commit() {
    let runtime = BatchRuntime {
        mode: "provider",
        ..Default::default()
    };
    let checkpoint = Checkpoint::default();
    let rag =
        RagService::new((), runtime.clone()).with_embedding_batch_limits(RagEmbeddingBatchLimits {
            max_inputs: 2,
            max_tokens: 100,
        });
    assert!(
        rag.prepare_vectors(
            "PLATFORM",
            Some("model"),
            &inputs(4),
            2,
            Default::default(),
            Some(&checkpoint)
        )
        .await
        .is_err()
    );
    assert_eq!(checkpoint.saved.lock().unwrap().vectors.len(), 2);
    checkpoint.cancelled.store(true, Ordering::SeqCst);
    let completed = checkpoint.saved.lock().unwrap().clone();
    assert!(
        rag.prepare_vectors(
            "PLATFORM",
            Some("model"),
            &inputs(4),
            2,
            completed,
            Some(&checkpoint)
        )
        .await
        .is_err()
    );
    assert_eq!(runtime.calls.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn embedding_batch_settings_hot_update_and_resume_preserve_completed_vectors() {
    use crate::{
        adapters::outbound::config::YamlSystemConfigStore,
        application::system_config::{
            BackendRuntimeConfig, SystemConfigChange, SystemConfigService,
        },
        config::AppConfig,
    };
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("application.yaml");
    std::fs::write(
        &path,
        "modelInvocation:\n  embedding:\n    maxBatchInputs: 3\n    maxBatchTokens: 100\n",
    )
    .unwrap();
    let active = BackendRuntimeConfig::new(AppConfig::from_yaml_file(&path).unwrap());
    let settings = SystemConfigService::new(Arc::new(YamlSystemConfigStore::new(path)))
        .with_runtime_config(active.clone());
    let runtime = BatchRuntime {
        mode: "fail_second",
        after_first: Some(Arc::new(move || {
            let doc = settings.get().unwrap();
            settings
                .update(
                    &doc.revision,
                    vec![
                        SystemConfigChange {
                            path: "modelInvocation.embedding.maxBatchInputs".into(),
                            value: 2.into(),
                        },
                        SystemConfigChange {
                            path: "modelInvocation.embedding.maxBatchTokens".into(),
                            value: 1.into(),
                        },
                    ],
                    false,
                )
                .unwrap();
        })),
        ..Default::default()
    };
    let rag = RagService::new((), runtime.clone()).with_runtime_config(active);
    let checkpoint = Checkpoint::default();
    assert!(
        rag.prepare_vectors(
            "PLATFORM",
            Some("model"),
            &inputs(6),
            2,
            Default::default(),
            Some(&checkpoint)
        )
        .await
        .is_err()
    );
    let saved = checkpoint.saved.lock().unwrap().clone();
    assert_eq!(saved.vectors.len(), 3);
    let result = rag
        .prepare_vectors(
            "PLATFORM",
            Some("model"),
            &inputs(6),
            2,
            saved,
            Some(&checkpoint),
        )
        .await
        .unwrap();
    assert_eq!(result.vectors.len(), 6);
    assert_eq!(
        runtime
            .calls
            .lock()
            .unwrap()
            .iter()
            .map(Vec::len)
            .collect::<Vec<_>>(),
        vec![3, 1, 1, 1, 1]
    );
    assert_eq!(runtime.calls.lock().unwrap()[2], vec!["3"]);
}

#[tokio::test]
async fn automatic_embedding_resolves_once_and_resume_keeps_published_space() {
    let runtime = BatchRuntime {
        mode: "fail_second",
        ..Default::default()
    };
    let checkpoint = Checkpoint::default();
    let rag =
        RagService::new((), runtime.clone()).with_embedding_batch_limits(RagEmbeddingBatchLimits {
            max_inputs: 2,
            max_tokens: 100,
        });
    assert!(
        rag.prepare_vectors(
            "PLATFORM",
            Some("PERSON_AI:ROUTE:EmbeddingAuto"),
            &inputs(5),
            2,
            Default::default(),
            Some(&checkpoint)
        )
        .await
        .is_err()
    );
    let completed = checkpoint.saved.lock().unwrap().clone();
    assert_eq!(completed.model_code.as_deref(), Some("model"));
    let result = rag
        .prepare_vectors(
            "PLATFORM",
            Some("PERSON_AI:ROUTE:EmbeddingAuto"),
            &inputs(5),
            2,
            completed,
            Some(&checkpoint),
        )
        .await
        .unwrap();
    assert_eq!(result.vectors.len(), 5);
    assert_eq!(*runtime.route_resolutions.lock().unwrap(), 1);
    assert!(
        runtime
            .requested_models
            .lock()
            .unwrap()
            .iter()
            .all(|code| code.as_deref() == Some("model"))
    );
}

#[tokio::test]
async fn provider_batch_limit_is_applied_before_dispatch_without_discarding_checkpoints() {
    let runtime = BatchRuntime {
        mode: "provider-limit",
        ..Default::default()
    };
    let rag = RagService::new((), runtime.clone());
    let checkpoint = Checkpoint::default();
    let result = rag
        .prepare_vectors(
            "PLATFORM",
            Some("model"),
            &inputs(10),
            2,
            Default::default(),
            Some(&checkpoint),
        )
        .await
        .unwrap();
    assert_eq!(
        runtime
            .calls
            .lock()
            .unwrap()
            .iter()
            .map(Vec::len)
            .collect::<Vec<_>>(),
        [4, 4, 2]
    );
    assert_eq!(result.vectors.len(), 10);
    assert_eq!(*checkpoint.batches.lock().unwrap(), [4, 4, 2]);
}

#[test]
fn structured_graph_preserves_cross_chunk_declarations_without_fabricating_quotes() {
    use crate::application::rag::{RagTextUnitWrite, graph::build_graph_generation};
    use crate::domain::rag::graph::{GraphPartition, RagGraphGeneration, RagGraphNodeKind};
    use serde_json::json;
    fn unit(code: &str, text: &str, ordinal: usize) -> RagTextUnitWrite {
        RagTextUnitWrite {
            text_unit_code: code.into(),
            unit_type: "LEAF".into(),
            text: text.into(),
            search_text: text.into(),
            heading_path: String::new(),
            ordinal,
            token_count: 10,
            content_hash: code.into(),
            status: "ACTIVE".into(),
            fulltext_status: "INDEXED".into(),
            embedding_model_code: None,
            embedding_version: None,
            embedding_text_hash: None,
            embedding_dimension: None,
            embedding: None,
            embedding_status: "NOT_ENABLED".into(),
            metadata: json!({"location":{"lineStart":ordinal+1}}),
        }
    }
    let units = vec![
        unit("first", "Alpha is the registry skill.", 0),
        unit("second", "Files: SKILL.md", 1),
        unit(
            "third",
            "Other original instructions without a named entity.",
            2,
        ),
    ];
    let structured = json!({"nodes":[{"ref":"skill","name":"Alpha","type":"Skill","evidence":"Alpha"},{"ref":"file","name":"SKILL.md","type":"SkillFile","evidence":"SKILL.md"}],"edges":[{"from":"skill","to":"file","relationship":"CONTAINS","evidence":"SKILL.md"}]});
    let evidence = structured_evidence(&units, &structured).unwrap();
    assert_eq!(evidence["second"]["references"]["skill"], "first");
    assert!(
        evidence["second"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|node| node["ref"] != "skill")
    );
    assert!(evidence["third"]["nodes"].as_array().unwrap().is_empty());
    let generation = RagGraphGeneration {
        partition: GraphPartition {
            corpus_id: "skill".into(),
            scope_type: "SPACE".into(),
            scope_code: "PLATFORM".into(),
        },
        id: "generation".into(),
        source_code: "one-protected-source".into(),
        source_version: "v1".into(),
        task_id: "task".into(),
        nodes: vec![],
        edges: vec![],
    };
    let (facts, projections) =
        build_graph_generation(generation.clone(), &units, &evidence).unwrap();
    assert_eq!(projections.len(), 2);
    let relation = facts.edges.iter().find(|edge| !edge.structural).unwrap();
    assert_eq!(relation.support.chunk_code, "second");
    assert_eq!(relation.support.quote, "SKILL.md");
    let declaration = facts
        .nodes
        .iter()
        .find(|node| node.kind == RagGraphNodeKind::EntityContribution && node.name == "Alpha")
        .unwrap();
    assert_eq!(declaration.properties["chunkCode"], "first");
    assert_eq!(declaration.properties["description"], "Alpha");
    let mut foreign = evidence.clone();
    foreign["second"]["references"]["skill"] = json!("foreign-source-chunk");
    assert!(build_graph_generation(generation.clone(), &units, &foreign).is_err());
    let mut invented = evidence;
    invented["first"]["nodes"][0]["name"] = json!("Invented");
    assert!(build_graph_generation(generation, &units, &invented).is_err());
}
