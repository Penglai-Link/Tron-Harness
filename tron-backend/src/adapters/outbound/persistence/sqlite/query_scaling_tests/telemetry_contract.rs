use super::*;
use crate::{
    adapters::outbound::observability::TelemetryProjection,
    application::{observability::*, rag::RagConfigRepository},
};
use eliteforge_agent_core::{TelemetryMetricPoint, TelemetryRetention, TelemetrySpan};
use eliteforge_agent_official_plugin_storage_local::SqliteTelemetryStore;
use eliteforge_agent_test_support::storage_contracts::*;
use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
fn projection(f: &Fixture) -> Arc<TelemetryProjection> {
    let store = super::super::framework_storage::blocking(SqliteTelemetryStore::with_database(
        f.database.runtime(),
        TelemetryRetention {
            trace_max_age: Duration::from_secs(30 * 86400),
            metric_max_age: Duration::from_secs(30 * 86400),
            max_completed_traces: None,
        },
    ))
    .unwrap();
    Arc::new(TelemetryProjection {
        database: f.database.runtime(),
        store: Arc::new(store),
        instance_id: "performance".into(),
    })
}
fn query() -> TraceQuery {
    TraceQuery {
        filters: Default::default(),
        category: Some(TraceCategory::Page),
        sorts: Vec::new(),
        sort_by: Default::default(),
        sort_direction: Default::default(),
        workspace_code: "TRON_CORE".into(),
        session_code: None,
        trace_id: None,
        current: 1,
        size: 20,
    }
}
fn seed_telemetry(db: &SharedDatabase, start: usize, end: usize) {
    db.call(move|c|->rusqlite::Result<()> {
        let at=SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;let tx=c.transaction()?;
        for i in start..end {
            let id=format!("perf-{i:08}");let workspace=if i%2==0 {"TRON_CORE"}else{"other"};
            let attrs=BTreeMap::from([("tron.workspace.code".into(),workspace.into()),("tron.trace.page".into(),"true".into()),("url.path".into(),"/performance".into())]);
            tx.execute("INSERT INTO ef_telemetry_traces VALUES(?1,?2,?3,0,?4,0)",params![id,at-i as i64,at-i as i64+10,serde_json::to_string(&attrs).unwrap()])?;
            for (span_id,name,parent,attributes) in [
                ("root","page /performance",None,attrs.clone()),
                ("model","agent.model",Some("root".into()),BTreeMap::from([("gen_ai.usage.input_tokens".into(),"7".into()),("gen_ai.usage.output_tokens".into(),"3".into()),("payload".into(),"x".repeat(2048))]))
            ] {
                let span=TelemetrySpan {trace_id:id.clone(),span_id:span_id.into(),parent_span_id:parent,name:name.into(),started_at:at-i as i64,ended_at:at-i as i64+10,attributes,..Default::default()};
                tx.execute("INSERT INTO ef_telemetry_spans VALUES(?1,?2,?3,?4)",params![id,span_id,span.ended_at,serde_json::to_string(&span).unwrap()])?;
            }
            tx.execute("INSERT INTO ef_telemetry_trace_rollups(trace_id,span_count,model_count,input_tokens,output_tokens) VALUES(?1,2,1,7,3)",[&id])?;
            for j in 0..6 {
                let p=TelemetryMetricPoint {point_id:format!("point-{i:08}-{j}"),name:"performance".into(),timestamp:at-i as i64,resource:BTreeMap::from([("tron.workspace.code".into(),workspace.into())]),data:serde_json::json!({"asInt":"3"}),..Default::default()};
                tx.execute("INSERT INTO ef_telemetry_metric_points VALUES(?1,?2,?3,?4)",params![p.point_id,p.name,p.timestamp,serde_json::to_string(&p).unwrap()])?;
            }
        }
        tx.commit()
    },||rusqlite::Error::InvalidQuery).unwrap();
}
fn read(projection: &TelemetryProjection, kind: usize, rows: usize) {
    let q = query();
    let total = rows.div_ceil(2);
    match kind {
        0 => {
            let page = TraceRepository::query(projection, &q).unwrap();
            assert_eq!(page.total, total);
            assert_eq!(page.records.len(), total.min(20));
            assert!(
                page.records
                    .iter()
                    .all(|r| r.model_count == 1 && r.input_tokens == 7)
            );
        }
        1 => {
            let page = projection
                .scoped_metric_points(Some("TRON_CORE".into()), Some("performance".into()), 1, 20)
                .unwrap();
            assert_eq!(page.total, total * 6);
            assert_eq!(page.records.len(), (total * 6).min(20));
            assert!(
                page.records
                    .iter()
                    .all(|p| p.resource["tron.workspace.code"] == "TRON_CORE")
            );
        }
        2 => {
            let stats = projection.metrics(&q).unwrap();
            assert_eq!(stats.trace_count, total);
            assert_eq!(stats.failed_trace_count, 0);
            assert_eq!(stats.input_tokens, total as u64 * 7);
            assert_eq!(stats.p95_duration_ms, 10.);
        }
        _ => {
            let detail = projection.detail("TRON_CORE", "perf-00000000").unwrap();
            assert_eq!(detail.spans.len(), 2);
            assert_eq!(detail.trace.input_tokens, 7);
        }
    }
}
#[test]
fn observability_and_project_pages_have_constant_storage_budgets() {
    let _guard = TEST_LOCK.lock().unwrap();
    let f = fixture();
    let projection = projection(&f);
    let mut start = 0;
    for rows in [1, 100, 1000] {
        seed_telemetry(&f.database, start, rows);
        start = rows;
        for kind in 0..4 {
            let (_, selects) = measure(|| read(&projection, kind, rows));
            assert_query_budget("host telemetry projection", 0, selects, 2);
        }
        let page = projection
            .scoped_metric_points(
                Some("TRON_CORE".into()),
                Some("performance".into()),
                100_000,
                20,
            )
            .unwrap();
        assert!(page.records.is_empty());
        assert_eq!(page.total, rows.div_ceil(2) * 6);
    }
    seed(&f.database, 0, 40);
    let mut codes = Vec::new();
    for current in 1..=5 {
        let (page, selects) = measure(|| f.projects.page("TRON_CORE", current, 9).unwrap());
        assert_query_budget("native project page", 0, selects, 3);
        assert_eq!(page.total, 40);
        codes.extend(page.records.into_iter().map(|p| p.project_code));
    }
    assert_eq!(
        codes,
        (0..40)
            .rev()
            .map(|i| format!("SCALING-PROJECT-{i}"))
            .collect::<Vec<_>>()
    );
}
#[test]
fn rag_corpus_counts_batch_schema_and_statistics_without_per_corpus_queries() {
    let _guard = TEST_LOCK.lock().unwrap();
    let f = fixture();
    let repository = SqliteRagRepository::connect(&f.path).unwrap();
    let mut start = 0;
    for rows in [1, 40, 600] {
        f.database.call(move|c|->rusqlite::Result<()> {
            let tx=c.transaction()?;
            for i in start..rows {
                let table=format!("rag_contract_units_{i}");
                tx.execute_batch(&format!("CREATE TABLE {table}(id INTEGER PRIMARY KEY,source_code TEXT,source_version TEXT,scope_type TEXT,scope_code TEXT,status TEXT,metadata TEXT); CREATE VIRTUAL TABLE {table}_vec_d3 USING vec0(embedding float[3]); INSERT INTO {table} VALUES(1,'source','v1','workspace','w','ACTIVE','{{}}'),(2,'source','v1','workspace','w','ACTIVE','{{}}'),(3,'other','v1','workspace','w','ARCHIVED','{{}}'); INSERT INTO {table}_vec_d3(rowid,embedding) VALUES(1,'[1,2,3]');"))?;
                let binding=serde_json::json!({"mode":"TARGETED","bindings":{"textUnit":"units"},"targets":[{"id":"units","backend":"SQLITE","table":table}]});
                tx.execute("INSERT INTO rag_corpora VALUES(?1,?1,'enabled',NULL,'{}','{}','{}',?2,1,1)",params![format!("contract-{i}"),binding.to_string()])?;
            }
            tx.commit()
        },||rusqlite::Error::InvalidQuery).unwrap();
        start = rows;
        let (counts, selects) = measure(|| repository.list_corpora_with_counts().unwrap());
        assert_query_budget("RAG batch counts", 0, selects, 3);
        assert_eq!(counts.len(), rows + 2);
        let contract_counts = counts
            .iter()
            .filter(|(corpus, _)| corpus.corpus_id.starts_with("contract-"));
        assert_eq!(contract_counts.clone().count(), rows);
        assert!(contract_counts.into_iter().all(|(_, c)| c.source_count == 1
            && c.chunk_count == 2
            && c.vector_count == 1
            && c.rebuild_source_count == 1));
    }
}
#[test]
#[ignore = "10000 traces and 60000 metrics through host projections with 16 concurrent readers"]
fn host_telemetry_concurrency_load() {
    let _guard = TEST_LOCK.lock().unwrap();
    let f = fixture();
    let projection = projection(&f);
    seed_telemetry(&f.database, 0, 10_000);
    let before = SELECTS.load(Ordering::SeqCst);
    let barrier = Arc::new(std::sync::Barrier::new(17));
    let start = Instant::now();
    let samples = std::thread::scope(|scope| {
        let handles = (0..16)
            .map(|worker| {
                let projection = projection.clone();
                let barrier = barrier.clone();
                scope.spawn(move || {
                    barrier.wait();
                    (0..40)
                        .map(|round| {
                            let kind = (worker + round) % 4;
                            let started = Instant::now();
                            read(&projection, kind, 10_000);
                            (kind, started.elapsed().as_secs_f64() * 1000.)
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect::<Vec<_>>();
        barrier.wait();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect::<Vec<_>>()
    });
    let elapsed = start.elapsed().as_secs_f64();
    assert_query_budget(
        "host telemetry concurrent",
        before,
        SELECTS.load(Ordering::SeqCst),
        16 * 40 * 2,
    );
    for (kind, name) in [
        "host-trace-page",
        "host-metric-page",
        "host-trace-statistics",
        "host-trace-detail",
    ]
    .into_iter()
    .enumerate()
    {
        let load = ReadLoad {
            milliseconds: samples
                .iter()
                .filter(|(k, _)| *k == kind)
                .map(|(_, ms)| *ms)
                .collect(),
            elapsed_seconds: elapsed,
        };
        eprintln!("{}", load.report(name));
        load.assert_p95(name, 1000.);
    }
}
