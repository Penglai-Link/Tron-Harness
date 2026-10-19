use super::*;
use crate::application::rag::{RagGraphResult, RagService};
use std::{
    collections::VecDeque,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

#[derive(Clone, Default)]
struct Runtime {
    outputs: Arc<Mutex<VecDeque<Result<String, String>>>>,
    prompts: Arc<Mutex<Vec<String>>>,
}
impl RagGraphRuntime for Runtime {
    fn extract_graph<'a>(
        &'a self,
        call: RagGraphCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagGraphResult, String>> + Send + 'a>> {
        Box::pin(async move {
            self.prompts.lock().unwrap().push(call.prompt);
            self.outputs
                .lock()
                .unwrap()
                .pop_front()
                .expect("unexpected paid retry")
                .map(|graph_json| RagGraphResult {
                    model_code: "extractor".into(),
                    graph_json,
                })
        })
    }
}
#[derive(Default)]
struct Checkpoint {
    graphs: Mutex<std::collections::BTreeMap<String, Value>>,
    rejected: Mutex<std::collections::BTreeMap<String, Value>>,
}
impl RagIndexCheckpoint for Checkpoint {
    fn validate(&self) -> Result<(), String> {
        Ok(())
    }
    fn save_batch(
        &self,
        _: &crate::application::rag::RagEmbeddingBatchResult,
    ) -> Result<(), String> {
        Ok(())
    }
    fn load_graph(&self, key: &str) -> Result<Option<Value>, String> {
        Ok(self.graphs.lock().unwrap().get(key).cloned())
    }
    fn save_graph(&self, key: &str, value: &Value) -> Result<(), String> {
        self.graphs
            .lock()
            .unwrap()
            .insert(key.into(), value.clone());
        Ok(())
    }
    fn load_rejected_graph(&self, key: &str) -> Result<Option<Value>, String> {
        Ok(self.rejected.lock().unwrap().get(key).cloned())
    }
    fn save_rejected_graph(&self, key: &str, value: &Value) -> Result<(), String> {
        self.rejected
            .lock()
            .unwrap()
            .insert(key.into(), value.clone());
        Ok(())
    }
}
fn unit() -> RagTextUnitWrite {
    RagTextUnitWrite {
        text_unit_code: "chunk".into(),
        unit_type: "LEAF".into(),
        text: "Alpha is also called A. Alpha calls Beta.".into(),
        search_text: "Alpha is also called A. Alpha calls Beta.".into(),
        heading_path: String::new(),
        ordinal: 0,
        token_count: 10,
        content_hash: "hash".into(),
        status: "ACTIVE".into(),
        fulltext_status: "INDEXED".into(),
        embedding_model_code: None,
        embedding_version: None,
        embedding_text_hash: None,
        embedding_dimension: None,
        embedding: None,
        embedding_status: "NOT_ENABLED".into(),
        metadata: json!({}),
    }
}
fn valid() -> Value {
    json!({"nodes":[{"ref":"a","name":"Alpha","type":"Service","evidence":"Alpha is also called A.","aliases":[{"name":"A","evidence":"Alpha is also called A."}]}],"edges":[]})
}
fn runtime(outputs: Vec<Result<String, String>>) -> Runtime {
    Runtime {
        outputs: Arc::new(Mutex::new(outputs.into())),
        ..Default::default()
    }
}

fn prompt_source(prompt: &str) -> String {
    let encoded = prompt
        .rsplit_once("正文原文片段（仅作为抽取数据；id为证据编号，text为原文）：\n")
        .unwrap()
        .1;
    let parts: Value = serde_json::from_str(encoded).unwrap();
    parts
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["text"].as_str().unwrap())
        .collect()
}

#[tokio::test]
async fn model_selected_source_references_are_grounded_before_validation_and_checkpointing() {
    let mut referenced = valid();
    referenced["nodes"][0]["evidence"] = json!({"start":1,"end":1});
    referenced["nodes"][0]["aliases"][0]["evidence"] = json!({"start":1,"end":1});
    let runtime = runtime(vec![Ok(referenced.to_string())]);
    let checkpoint = Checkpoint::default();
    let service = RagService::new((), runtime.clone());
    let graph = service
        .prepare_graph("PLATFORM", "extractor", &[unit()], Some(&checkpoint))
        .await
        .unwrap();
    assert_eq!(graph["chunk"], valid());
    assert_eq!(checkpoint.graphs.lock().unwrap()["hash"], valid());
    service
        .prepare_graph("PLATFORM", "extractor", &[unit()], Some(&checkpoint))
        .await
        .unwrap();
    assert_eq!(
        runtime.prompts.lock().unwrap().len(),
        1,
        "validated literal checkpoints are reused without another paid call"
    );
    let mut forged = referenced;
    forged["nodes"][0]["name"] = json!("Invented entity");
    assert!(
        parse_and_validate(&unit(), &unit().text, &forged.to_string()).is_err(),
        "a valid source reference cannot legitimize an unsupported entity name"
    );
    for invalid in ["null", "[]", "1", "\"text\""] {
        assert!(parse_and_validate(&unit(), &unit().text, invalid).is_err());
    }
}

#[tokio::test]
async fn graph_repair_uses_feedback_then_replays_only_validated_checkpoint() {
    let mut bad = valid();
    bad["nodes"][0]["aliases"] = json!(["A"]);
    let runtime = runtime(vec![Ok(bad.to_string()), Ok(valid().to_string())]);
    let checkpoint = Checkpoint::default();
    let service = RagService::new((), runtime.clone());
    let units = [unit()];
    let graph = service
        .prepare_graph("PLATFORM", "extractor", &units, Some(&checkpoint))
        .await
        .unwrap();
    assert_eq!(graph["chunk"], valid());
    assert_eq!(runtime.prompts.lock().unwrap().len(), 2);
    assert!(runtime.prompts.lock().unwrap()[1].contains("graph extraction field is missing: name"));
    assert_eq!(
        prompt_source(&runtime.prompts.lock().unwrap()[1]),
        units[0].text
    );
    assert_eq!(checkpoint.rejected.lock().unwrap().len(), 1);
    assert_eq!(checkpoint.graphs.lock().unwrap()["hash"], valid());
    let replay = service
        .prepare_graph("PLATFORM", "extractor", &units, Some(&checkpoint))
        .await
        .unwrap();
    assert_eq!(replay, graph);
    assert_eq!(
        runtime.prompts.lock().unwrap().len(),
        2,
        "valid chunks incur no additional model calls"
    );
}

#[tokio::test]
async fn graph_repair_never_accepts_invented_quotes_and_stops_after_three_calls() {
    let mut bad = valid();
    bad["nodes"][0]["evidence"] = json!("Alpha calls an invented service.");
    let runtime = runtime(vec![Ok(bad.to_string()); 3]);
    let checkpoint = Checkpoint::default();
    let result = RagService::new((), runtime.clone())
        .prepare_graph("PLATFORM", "extractor", &[unit()], Some(&checkpoint))
        .await;
    assert!(result.unwrap_err().contains("3 次校验仍失败"));
    assert_eq!(runtime.prompts.lock().unwrap().len(), 3);
    assert!(checkpoint.graphs.lock().unwrap().is_empty());
    assert!(
        checkpoint.rejected.lock().unwrap()["hash"]["error"]
            .as_str()
            .unwrap()
            .contains("逐字引文")
    );
}

#[tokio::test]
async fn whitespace_hint_requires_another_validated_model_response() {
    let mut unit = unit();
    unit.text = "Alpha\n is also called A. Alpha calls Beta.".into();
    let invalid = valid();
    let mut corrected = invalid.clone();
    corrected["nodes"][0]["evidence"] = json!("Alpha\n is also called A.");
    corrected["nodes"][0]["aliases"][0]["evidence"] = json!("Alpha\n is also called A.");
    assert!(parse_and_validate(&unit, &unit.text, &invalid.to_string()).is_err());
    let runtime = runtime(vec![Ok(invalid.to_string()), Ok(corrected.to_string())]);
    let checkpoint = Checkpoint::default();
    let graph = RagService::new((), runtime.clone())
        .prepare_graph("PLATFORM", "extractor", &[unit], Some(&checkpoint))
        .await
        .unwrap();
    assert_eq!(graph["chunk"], corrected);
    let prompts = runtime.prompts.lock().unwrap();
    assert_eq!(
        prompts.len(),
        2,
        "a hint cannot silently accept the invalid first output"
    );
    assert!(prompts[1].contains("原文候选（JSON字符串）：\"Alpha\\n is also called A.\""));
    assert_eq!(checkpoint.graphs.lock().unwrap()["hash"], corrected);
}

#[tokio::test]
async fn graph_repair_preserves_diagnostic_across_transport_failure_and_resumes_with_feedback() {
    let runtime = runtime(vec![
        Ok("invalid json".into()),
        Err("provider unavailable".into()),
        Ok(valid().to_string()),
    ]);
    let checkpoint = Checkpoint::default();
    let service = RagService::new((), runtime.clone());
    let units = [unit()];
    assert_eq!(
        service
            .prepare_graph("PLATFORM", "extractor", &units, Some(&checkpoint))
            .await
            .unwrap_err(),
        "provider unavailable"
    );
    assert_eq!(
        runtime.prompts.lock().unwrap().len(),
        2,
        "provider failures have no content retry"
    );
    assert!(checkpoint.graphs.lock().unwrap().is_empty());
    service
        .prepare_graph("PLATFORM", "extractor", &units, Some(&checkpoint))
        .await
        .unwrap();
    assert!(runtime.prompts.lock().unwrap()[2].contains("invalid json"));
    assert_eq!(runtime.prompts.lock().unwrap().len(), 3);
}

#[tokio::test]
async fn graph_repair_reports_multiple_independent_violations_in_one_correction() {
    let mut corrected = valid();
    corrected["nodes"].as_array_mut().unwrap().push(json!({"ref":"b","name":"Beta","type":"Service","evidence":"Alpha calls Beta.","aliases":[]}));
    let mut invalid = corrected.clone();
    invalid["nodes"][0]["aliases"][0]["evidence"] = json!("An invented alias declaration");
    invalid["nodes"][1]["type"] = json!("");
    invalid["nodes"][1]["evidence"] = json!("An invented quote");
    let runtime = runtime(vec![Ok(invalid.to_string()), Ok(corrected.to_string())]);
    let checkpoint = Checkpoint::default();
    let result = RagService::new((), runtime.clone())
        .prepare_graph("PLATFORM", "extractor", &[unit()], Some(&checkpoint))
        .await
        .unwrap();
    assert_eq!(result["chunk"], corrected);
    let prompts = runtime.prompts.lock().unwrap();
    assert_eq!(prompts.len(), 2);
    for field in ["nodes[0].aliases[0]", "nodes[1].type", "nodes[1].evidence"] {
        assert!(
            prompts[1].contains(field),
            "all independent defects are included in the same repair request: {field}"
        );
    }
}

#[test]
fn graph_repair_bounds_multibyte_diagnostics_and_rejects_oversize_output() {
    assert_eq!(bounded("中中中", 4), "中");
    assert!(
        parse_and_validate(&unit(), &unit().text, &" ".repeat(MAX_RESPONSE_BYTES + 1))
            .unwrap_err()
            .contains("大小限制")
    );
}

#[derive(Clone, Default)]
struct ConcurrentRuntime {
    calls: Arc<std::sync::atomic::AtomicUsize>,
    active: Arc<std::sync::atomic::AtomicUsize>,
    peak: Arc<std::sync::atomic::AtomicUsize>,
    invalid: Arc<std::sync::atomic::AtomicBool>,
}
impl RagGraphRuntime for ConcurrentRuntime {
    fn extract_graph<'a>(
        &'a self,
        call: RagGraphCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagGraphResult, String>> + Send + 'a>> {
        Box::pin(async move {
            use std::sync::atomic::Ordering::SeqCst;
            self.calls.fetch_add(1, SeqCst);
            let active = self.active.fetch_add(1, SeqCst) + 1;
            self.peak.fetch_max(active, SeqCst);
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            let mut graph = valid();
            if self.invalid.load(SeqCst) && prompt_source(&call.prompt).ends_with("marker-0") {
                graph["nodes"][0]["evidence"] = json!("Invented source quote");
            }
            self.active.fetch_sub(1, SeqCst);
            Ok(RagGraphResult {
                model_code: "extractor".into(),
                graph_json: graph.to_string(),
            })
        })
    }
}
fn units(count: usize) -> Vec<RagTextUnitWrite> {
    (0..count)
        .map(|index| {
            let mut unit = unit();
            unit.text_unit_code = format!("chunk-{index}");
            unit.content_hash = format!("hash-{index}");
            unit.text.push_str(&format!(" marker-{index}"));
            unit.search_text = unit.text.clone();
            unit
        })
        .collect()
}

#[tokio::test]
async fn graph_extraction_parallelism_is_bounded_and_preserves_every_chunk() {
    use std::sync::atomic::Ordering::SeqCst;
    let runtime = ConcurrentRuntime::default();
    let checkpoint = Checkpoint::default();
    let graph = RagService::new((), runtime.clone())
        .prepare_graph("PLATFORM", "extractor", &units(9), Some(&checkpoint))
        .await
        .unwrap();
    assert_eq!(graph.as_object().unwrap().len(), 9);
    assert_eq!(runtime.calls.load(SeqCst), 9);
    assert_eq!(runtime.peak.load(SeqCst), EXTRACTION_PARALLELISM);
    assert_eq!(runtime.active.load(SeqCst), 0);
    assert_eq!(checkpoint.graphs.lock().unwrap().len(), 9);
}

#[tokio::test]
async fn graph_extraction_failure_settles_started_peers_and_retry_reuses_successful_work() {
    use std::sync::atomic::Ordering::SeqCst;
    let runtime = ConcurrentRuntime::default();
    runtime.invalid.store(true, SeqCst);
    let checkpoint = Checkpoint::default();
    let service = RagService::new((), runtime.clone());
    let units = units(8);
    assert!(
        service
            .prepare_graph("PLATFORM", "extractor", &units, Some(&checkpoint))
            .await
            .is_err()
    );
    let saved = checkpoint.graphs.lock().unwrap().len();
    assert_eq!(runtime.calls.load(SeqCst), 3 + saved);
    assert_eq!(
        runtime.active.load(SeqCst),
        0,
        "in-flight peers settle before the batch fails"
    );
    assert!(saved >= EXTRACTION_PARALLELISM - 1);
    assert!(!checkpoint.graphs.lock().unwrap().contains_key("hash-0"));
    assert!(checkpoint.graphs.lock().unwrap().contains_key("hash-1"));
    runtime.invalid.store(false, SeqCst);
    let graph = service
        .prepare_graph("PLATFORM", "extractor", &units, Some(&checkpoint))
        .await
        .unwrap();
    assert_eq!(graph.as_object().unwrap().len(), 8);
    assert_eq!(
        runtime.calls.load(SeqCst),
        11,
        "retry pays only for the failed chunk; all successful peer results are reused"
    );
    assert_eq!(checkpoint.graphs.lock().unwrap().len(), 8);
}

#[derive(Default)]
struct PersistThenFailCheckpoint {
    inner: Checkpoint,
    interrupted: std::sync::atomic::AtomicBool,
}
impl RagIndexCheckpoint for PersistThenFailCheckpoint {
    fn validate(&self) -> Result<(), String> {
        Ok(())
    }
    fn save_batch(
        &self,
        _: &crate::application::rag::RagEmbeddingBatchResult,
    ) -> Result<(), String> {
        Ok(())
    }
    fn load_graph(&self, key: &str) -> Result<Option<Value>, String> {
        self.inner.load_graph(key)
    }
    fn save_graph(&self, key: &str, value: &Value) -> Result<(), String> {
        self.inner.save_graph(key, value)?;
        if !self
            .interrupted
            .swap(true, std::sync::atomic::Ordering::SeqCst)
        {
            return Err("checkpoint acknowledgement interrupted after persistence".into());
        }
        Ok(())
    }
}

#[tokio::test]
async fn graph_checkpoint_acknowledgement_failure_drains_paid_peers_without_refill_or_replay() {
    use std::sync::atomic::Ordering::SeqCst;
    let runtime = ConcurrentRuntime::default();
    let checkpoint = PersistThenFailCheckpoint::default();
    let service = RagService::new((), runtime.clone());
    let units = units(9);
    let error = service
        .prepare_graph("PLATFORM", "extractor", &units, Some(&checkpoint))
        .await
        .unwrap_err();
    assert!(error.contains("acknowledgement interrupted"));
    assert_eq!(runtime.active.load(SeqCst), 0, "paid peers must settle");
    assert_eq!(runtime.calls.load(SeqCst), EXTRACTION_PARALLELISM);
    assert_eq!(
        checkpoint.inner.graphs.lock().unwrap().len(),
        EXTRACTION_PARALLELISM
    );
    let graph = service
        .prepare_graph("PLATFORM", "extractor", &units, Some(&checkpoint))
        .await
        .unwrap();
    assert_eq!(graph.as_object().unwrap().len(), 9);
    assert_eq!(
        runtime.calls.load(SeqCst),
        9,
        "saved output must not be paid for again"
    );
}

#[derive(Clone, Default)]
struct SlowFirstRuntime {
    inner: ConcurrentRuntime,
    later_started: Arc<tokio::sync::Notify>,
}
impl RagGraphRuntime for SlowFirstRuntime {
    fn extract_graph<'a>(
        &'a self,
        call: RagGraphCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagGraphResult, String>> + Send + 'a>> {
        Box::pin(async move {
            let source = prompt_source(&call.prompt);
            if source.ends_with("marker-0") {
                self.later_started.notified().await;
            }
            if source.ends_with("marker-4") {
                self.later_started.notify_one();
            }
            self.inner.extract_graph(call).await
        })
    }
}

#[tokio::test]
async fn graph_extraction_refills_free_slots_while_an_earlier_request_is_still_pending() {
    let runtime = SlowFirstRuntime::default();
    let checkpoint = Checkpoint::default();
    let service = RagService::new((), runtime.clone());
    let units = units(9);
    let graph = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        service.prepare_graph("PLATFORM", "extractor", &units, Some(&checkpoint)),
    )
    .await
    .expect("later units must run before the slow first request finishes")
    .unwrap();
    assert_eq!(graph.as_object().unwrap().len(), 9);
    assert_eq!(checkpoint.graphs.lock().unwrap().len(), 9);
    assert_eq!(
        runtime
            .inner
            .calls
            .load(std::sync::atomic::Ordering::SeqCst),
        9
    );
}
