//! Execute real repository/service paths while counting SQLite statements.
use super::framework_storage::SharedDatabase;
use super::*;
use crate::application::{
    app::{AppRepository, AppService},
    project::{ProjectRepository, ProjectService},
    prompt::PromptRepository,
    task::{TaskRepository, TaskService},
    workspace::WorkspaceService,
};
use crate::domain::task::TaskListQuery;
use rusqlite::params;
use std::sync::atomic::{AtomicUsize, Ordering};
static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
static LAST_CANDIDATE_SQL: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
static LAST_TASK_PAGE_SQL: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
static SELECTS: AtomicUsize = AtomicUsize::new(0);
fn trace(event: rusqlite::trace::TraceEvent<'_>) {
    if let rusqlite::trace::TraceEvent::Stmt(statement, _) = event {
        let sql = statement.sql();
        if sql.contains("page AS MATERIALIZED") {
            *LAST_TASK_PAGE_SQL.lock().unwrap() = Some(sql.to_string());
        }
        if sql.trim_start().starts_with("SELECT") && sql.contains("AS size_missing") {
            *LAST_CANDIDATE_SQL.lock().unwrap() = Some(sql.to_string());
        }
        if sql.trim_start().starts_with("SELECT") || sql.trim_start().starts_with("WITH") {
            SELECTS.fetch_add(1, Ordering::SeqCst);
        }
    }
}

fn assert_candidate_index(database: &SharedDatabase) {
    let sql = LAST_CANDIDATE_SQL
        .lock()
        .unwrap()
        .clone()
        .expect("claim must execute the candidate query");
    database
        .call(
            move |connection| {
                let mut statement = connection.prepare(&format!("EXPLAIN QUERY PLAN {sql}"))?;
                let plan = statement
                    .query_map(
                        params![1_i64, None::<String>, None::<i64>, 0_i64, 0_i64, 64_i64],
                        |row| row.get::<_, String>(3),
                    )?
                    .collect::<Result<Vec<_>, _>>()?;
                assert!(
                    plan.iter()
                        .any(|line| line.contains("idx_kb_ingestion_parse_order")),
                    "candidate priority index unused: {plan:?}"
                );
                assert!(
                    !plan.iter().any(|line| line.contains("TEMP B-TREE")),
                    "claim sorted the entire backlog: {plan:?}"
                );
                Ok::<_, rusqlite::Error>(())
            },
            || rusqlite::Error::InvalidQuery,
        )
        .unwrap();
}

fn load_summary(
    name: &str,
    rows: usize,
    workers: usize,
    mut times: Vec<f64>,
    elapsed: f64,
    selects: usize,
) {
    times.sort_by(f64::total_cmp);
    let percentile = |p: usize| times[((times.len() - 1) * p).div_ceil(100)];
    eprintln!(
        "{}",
        serde_json::json!({
            "suite":name,"rows":rows,"workers":workers,"requests":times.len(),"selects":selects,
            "p50Ms":percentile(50),"p95Ms":percentile(95),"maxMs":times.last(),
            "requestsPerSecond":times.len() as f64/elapsed
        })
    );
}

#[test]
#[ignore = "local load test: 10000 rows, concurrent lists and queue claims"]
fn large_data_concurrency_query_load() {
    use std::{
        sync::{Arc, Barrier},
        time::Instant,
    };
    let _guard = TEST_LOCK.lock().unwrap();
    let Fixture {
        _directory,
        path,
        database,
        tasks,
        projects,
        mut prompts,
    } = fixture();
    let mut start = 0;
    for size in [1, 1_000, 10_000] {
        seed(&database, start, size);
        start = size;
        let (page, task_queries) = measure(|| {
            tasks
                .list_tasks(TaskListQuery {
                    workspace_code: Some("TRON_CORE".into()),
                    size: Some(100),
                    ..Default::default()
                })
                .unwrap()
        });
        let reader = SqliteTaskRepository::connect(&path).unwrap().reader();
        let projection = reader
            .page(
                &TaskListQuery {
                    workspace_code: Some("TRON_CORE".into()),
                    ..Default::default()
                },
                Some((0, 100)),
            )
            .unwrap();
        assert_eq!(
            projection.tasks.len(),
            (size as usize).min(100),
            "unrelated task rows were decoded before pagination"
        );
        let sql = LAST_TASK_PAGE_SQL.lock().unwrap().clone().unwrap();
        database
            .call_read(
                move |c| {
                    let plan = c
                        .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))?
                        .query_map(params!["TRON_CORE", 100, 0], |r| r.get::<_, String>(3))?
                        .collect::<Result<Vec<_>, _>>()?;
                    assert!(
                        plan.iter().any(|p| p.contains("idx_tasks_list_scope")),
                        "page scope index unused: {plan:?}"
                    );
                    assert!(
                        !plan.iter().any(|p| p.contains("TEMP B-TREE FOR ORDER BY")),
                        "page sorted all tasks: {plan:?}"
                    );
                    Ok::<_, rusqlite::Error>(())
                },
                || rusqlite::Error::InvalidQuery,
            )
            .unwrap();
        assert_eq!(page.total as usize, size as usize);
        assert_eq!(page.records.len(), (size as usize).min(100));
        assert!(page.records.iter().all(
            |task| task.agent_codes == ["main-agent"] && task.app_codes == ["APP-TRON-CONSOLE"]
        ));
        let (bounded, page_queries) = measure(|| projects.page("TRON_CORE", 1, 100).unwrap());
        assert_eq!(bounded.total, size as usize);
        assert_eq!(bounded.records.len(), (size as usize).min(100));
        eliteforge_agent_test_support::storage_contracts::assert_query_budget(
            "project native page",
            0,
            page_queries,
            3,
        );
        let (page, project_queries) = measure(|| projects.list("TRON_CORE").unwrap());
        assert_eq!(page.len(), size as usize);
        assert!(
            page.iter()
                .all(|project| project.primary_app_code.as_deref() == Some("APP-TRON-CONSOLE"))
        );
        let (page, prompt_queries) = measure(|| prompts.list_prompts().unwrap());
        let selected = page
            .iter()
            .filter(|prompt| prompt.prompt_code.starts_with("scaling-"))
            .collect::<Vec<_>>();
        assert_eq!(selected.len(), size as usize);
        assert!(
            selected
                .iter()
                .all(|prompt| prompt.variables.len() == 1 && prompt.translations.len() == 1)
        );
        assert_eq!((task_queries, project_queries, prompt_queries), (4, 2, 3));
        eprintln!(
            "{}",
            serde_json::json!({"suite":"tron-list-scaling","rows":size,"selects":[task_queries,project_queries,prompt_queries]})
        );
    }
    let workers = 16;
    let rounds = 12;
    let repositories = (0..workers)
        .map(|_| SqlitePromptRepository::connect(&path).unwrap())
        .collect::<Vec<_>>();
    let barrier = Arc::new(Barrier::new(workers + 1));
    SELECTS.store(0, Ordering::SeqCst);
    let started = Instant::now();
    let times = std::thread::scope(|scope| {
        let handles = repositories
            .into_iter()
            .map(|mut prompts| {
                let tasks = tasks.clone();
                let projects = projects.clone();
                let barrier = barrier.clone();
                scope.spawn(move || {
                    barrier.wait();
                    let mut times = Vec::new();
                    for round in 0..rounds {
                        let now = Instant::now();
                        match round % 3 {
                            0 => {
                                let page = tasks
                                    .list_tasks(TaskListQuery {
                                        workspace_code: Some("TRON_CORE".into()),
                                        size: Some(100),
                                        ..Default::default()
                                    })
                                    .unwrap();
                                assert_eq!(page.total, 10_000);
                                assert_eq!(page.records.len(), 100);
                                assert!(
                                    page.records
                                        .iter()
                                        .all(|task| task.app_codes == ["APP-TRON-CONSOLE"])
                                );
                            }
                            1 => assert_eq!(projects.list("TRON_CORE").unwrap().len(), 10_000),
                            _ => assert_eq!(
                                prompts
                                    .list_prompts()
                                    .unwrap()
                                    .iter()
                                    .filter(|prompt| prompt.prompt_code.starts_with("scaling-"))
                                    .count(),
                                10_000
                            ),
                        }
                        times.push((round % 3, now.elapsed().as_secs_f64() * 1_000.0));
                    }
                    times
                })
            })
            .collect::<Vec<_>>();
        barrier.wait();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>()
    });
    let count = SELECTS.load(Ordering::SeqCst);
    assert_eq!(
        count,
        workers * rounds / 3 * (4 + 2 + 3),
        "concurrent list queries amplified"
    );
    for (kind, name) in [
        (0, "tron-task-concurrent"),
        (1, "tron-project-concurrent"),
        (2, "tron-prompt-concurrent"),
    ] {
        let mut samples = times
            .iter()
            .filter(|(actual, _)| *actual == kind)
            .map(|(_, time)| *time)
            .collect::<Vec<_>>();
        samples.sort_by(f64::total_cmp);
        let p95 = samples[((samples.len() - 1) * 95).div_ceil(100)];
        assert!(p95 < 1000.0, "{name} P95 exceeded 1 second: {p95} ms");
        load_summary(
            name,
            10_000,
            workers,
            times
                .iter()
                .filter(|(actual, _)| *actual == kind)
                .map(|(_, time)| *time)
                .collect(),
            started.elapsed().as_secs_f64(),
            workers * rounds / 3 * [4, 2, 3][kind],
        );
    }
    load_summary(
        "tron-list-concurrent",
        10_000,
        workers,
        times.into_iter().map(|(_, time)| time).collect(),
        started.elapsed().as_secs_f64(),
        count,
    );

    let (paged, selects) = measure(|| {
        let barrier = Arc::new(Barrier::new(17));
        let started = Instant::now();
        let times = std::thread::scope(|scope| {
            let handles = (0..16)
                .map(|worker| {
                    let barrier = barrier.clone();
                    let projects = projects.clone();
                    scope.spawn(move || {
                        barrier.wait();
                        (0..4)
                            .map(|round| {
                                let now = Instant::now();
                                let page = projects
                                    .page("TRON_CORE", worker * 4 + round + 1, 100)
                                    .unwrap();
                                assert_eq!(page.total, 10_000);
                                assert_eq!(page.records.len(), 100);
                                now.elapsed().as_secs_f64() * 1000.
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
        eliteforge_agent_test_support::storage_contracts::ReadLoad {
            milliseconds: times,
            elapsed_seconds: started.elapsed().as_secs_f64(),
        }
    });
    paged.assert_p95("project bounded page", 1000.);
    eliteforge_agent_test_support::storage_contracts::assert_query_budget(
        "project concurrent pages",
        0,
        selects,
        64 * 3,
    );
    eprintln!("{}", paged.report("tron-project-page-concurrent"));
    use crate::application::knowledge_base::ingestion::*;
    use sha2::{Digest, Sha256};
    for size in [1_000, 10_000] {
        database.call(move |c| {
            let tx = c.transaction()?;
            tx.execute("DELETE FROM knowledge_base_ingestion_tasks",[])?;
            tx.execute("DELETE FROM knowledge_base_documents",[])?;
            tx.execute("INSERT OR IGNORE INTO knowledge_bases(id,owner_user_code,payload_json) VALUES('scaling','owner','{}')",[])?;
            {
                let mut documents = tx.prepare("INSERT INTO knowledge_base_documents VALUES(?1,'scaling',?2,0)")?;
                let mut tasks = tx.prepare("INSERT INTO knowledge_base_ingestion_tasks(id,document_id,knowledge_base_id,input_config_hash,state,stage,attempt,lease_until_ms,retry_after_ms,payload_json) VALUES(?1,?2,'scaling',?3,'QUEUED','Parsing',0,0,0,?4)")?;
                for index in 0..size {
                    let document = format!("load-document-{index}");
                    let config = serde_json::json!({"tier":"standard"});
                    let hash = format!("{:x}",Sha256::digest(format!("resource-{index}:1:{config}").as_bytes()));
                    let task = IngestionTask::queued(format!("load-task-{index}"),"scaling".into(),document.clone(),"owner".into(),"workspace".into(),0,1,hash,config);
                    documents.execute(params![document,serde_json::json!({"id":document,"resourceCode":format!("resource-{index}"),"resourceRevision":1,"revision":0,"status":"READY"}).to_string()])?;
                    tasks.execute(params![task.id,task.document_id,task.input_config_hash,serde_json::to_string(&task).unwrap()])?;
                }
            }
            tx.commit()
        },||rusqlite::Error::InvalidQuery).unwrap();
        let queues = (0..workers)
            .map(|_| SqliteKnowledgeIngestionQueue::connect(&path).unwrap())
            .collect::<Vec<_>>();
        let barrier = Arc::new(Barrier::new(workers + 1));
        SELECTS.store(0, Ordering::SeqCst);
        let started = Instant::now();
        let results = std::thread::scope(|scope| {
            let handles = queues
                .into_iter()
                .enumerate()
                .map(|(worker, queue)| {
                    let barrier = barrier.clone();
                    scope.spawn(move || {
                        barrier.wait();
                        let mut results = Vec::new();
                        for _ in 0..32 {
                            let now = Instant::now();
                            let task = queue
                                .claim(IngestionClaim {
                                    task_id: None,
                                    stage: IngestionStage::Parsing,
                                    owner: format!("worker-{worker}"),
                                    now_ms: 1,
                                    lease_ms: 60_000,
                                    global_capacity: 512,
                                    embedding_resource_group: None,
                                    parse_endpoints: (0..8)
                                        .map(|node| ParseEndpointCapacity {
                                            endpoint_id: format!("node-{node}"),
                                            capacity: 64,
                                            resource_group: format!("node-{node}"),
                                            resource_group_capacity: 64,
                                            weight: 1,
                                            supported_tiers: vec![],
                                        })
                                        .collect(),
                                })
                                .unwrap()
                                .expect("a free slot must be claimed");
                            results.push((task.id, now.elapsed().as_secs_f64() * 1_000.0));
                        }
                        results
                    })
                })
                .collect::<Vec<_>>();
            barrier.wait();
            handles
                .into_iter()
                .flat_map(|handle| handle.join().unwrap())
                .collect::<Vec<_>>()
        });
        let elapsed = started.elapsed().as_secs_f64();
        let count = SELECTS.load(Ordering::SeqCst);
        assert_candidate_index(&database);
        assert_eq!(
            count,
            workers * 32 * 3,
            "queue claim queries amplified with backlog"
        );
        assert_eq!(
            results
                .iter()
                .map(|(id, _)| id)
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            512,
            "tasks were claimed twice"
        );
        assert_eq!(
            results
                .iter()
                .map(|(id, _)| id.clone())
                .collect::<std::collections::BTreeSet<_>>(),
            (0..512).map(|index| format!("load-task-{index}")).collect(),
            "FIFO changed under contention"
        );
        database.call(|c| {
            let running:usize = c.query_row("SELECT count(*) FROM knowledge_base_ingestion_tasks WHERE state='RUNNING'",[],|r|r.get(0))?;
            assert_eq!(running,512);
            let maximum:usize = c.query_row("SELECT max(n) FROM (SELECT count(*) n FROM knowledge_base_ingestion_tasks WHERE state='RUNNING' GROUP BY endpoint_id)",[],|r|r.get(0))?;
            assert_eq!(maximum,64,"node capacity exceeded");
            Ok::<_,rusqlite::Error>(())
        },||rusqlite::Error::InvalidQuery).unwrap();
        let mut latencies = results.iter().map(|(_, time)| *time).collect::<Vec<_>>();
        latencies.sort_by(f64::total_cmp);
        let p95 = latencies[((latencies.len() - 1) * 95).div_ceil(100)];
        assert!(p95 < 1_000.0, "local queue P95 exceeded 1 s: {p95} ms");
        load_summary(
            "tron-queue-concurrent",
            size,
            workers,
            results.into_iter().map(|(_, time)| time).collect(),
            elapsed,
            count,
        );
    }
}

fn measure<T>(operation: impl FnOnce() -> T) -> (T, usize) {
    SELECTS.store(0, Ordering::SeqCst);
    let result = operation();
    (result, SELECTS.load(Ordering::SeqCst))
}

#[test]
fn task_read_handle_overlaps_a_writer_without_leaking_uncommitted_relations() {
    use std::{sync::mpsc, time::Duration};
    let _guard = TEST_LOCK.lock().unwrap();
    let Fixture {
        _directory,
        path,
        database,
        ..
    } = fixture();
    seed(&database, 0, 1);
    let reader = SqliteTaskRepository::connect(&path).unwrap().reader();
    let (ready_tx, ready_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let writing = database.clone();
    let writer = std::thread::spawn(move || {
        writing.call(
            move |connection| {
                let tx = connection.transaction()?;
                tx.execute(
                    "UPDATE tasks SET title='committed title' WHERE task_code='SCALING-TASK-0'",
                    [],
                )?;
                tx.execute(
                    "DELETE FROM task_app_links WHERE task_code='SCALING-TASK-0'",
                    [],
                )?;
                ready_tx.send(()).unwrap();
                release_rx.recv_timeout(Duration::from_secs(10)).unwrap();
                tx.commit()
            },
            || rusqlite::Error::InvalidQuery,
        )
    });
    ready_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let (result_tx, result_rx) = mpsc::channel();
    let reading = reader.clone();
    let read = std::thread::spawn(move || {
        result_tx
            .send((
                reading.list_tasks().unwrap(),
                reading.relations(&["SCALING-TASK-0".into()]).unwrap(),
            ))
            .unwrap();
    });
    let result = result_rx.recv_timeout(Duration::from_secs(2));
    release_tx.send(()).unwrap();
    writer.join().unwrap().unwrap();
    read.join().unwrap();
    let (tasks, relations) = result.expect("task reads queued behind an uncommitted writer");
    assert_eq!(tasks[0].title, "SCALING-TASK-0");
    assert_eq!(relations["SCALING-TASK-0"].apps.len(), 1);
    assert_eq!(reader.list_tasks().unwrap()[0].title, "committed title");
    assert!(
        reader.relations(&["SCALING-TASK-0".into()]).unwrap()["SCALING-TASK-0"]
            .apps
            .is_empty()
    );
}
struct Fixture {
    _directory: tempfile::TempDir,
    path: String,
    database: SharedDatabase,
    tasks: TaskService,
    projects: ProjectService,
    prompts: SqlitePromptRepository,
}
fn fixture() -> Fixture {
    let directory = tempfile::tempdir().unwrap();
    let path = directory
        .path()
        .join("scaling.sqlite")
        .to_string_lossy()
        .into_owned();
    let database = SharedDatabase::open(&path).unwrap();
    let workspaces = WorkspaceService::new(SqliteWorkspaceRepository::connect(&path).unwrap());
    let apps = AppService::new(
        SqliteAppRepository::connect(&path).unwrap(),
        workspaces.clone(),
    );
    let tasks = TaskService::new(
        SqliteTaskRepository::connect(&path).unwrap(),
        workspaces.clone(),
        apps.clone(),
    );
    let projects = ProjectService::new(
        SqliteProjectRepository::connect(&path).unwrap(),
        workspaces,
        apps,
    );
    let prompts = SqlitePromptRepository::connect(&path).unwrap();
    database.call(|c| c.execute_batch("INSERT INTO workspaces(id,workspace_code,name,status,created_at,updated_at) VALUES(1,'TRON_CORE','Scaling workspace','active',1,1); INSERT INTO apps(id,app_code,workspace_code,app_name,local_directory,runtime_role,status,created_at,updated_at) VALUES(1,'APP-TRON-CONSOLE','TRON_CORE','Scaling app','/tmp','frontend','active',1,1);"),||rusqlite::Error::InvalidQuery).unwrap();
    // Install tracing on the writer and every round-robin WAL reader.
    database
        .call(
            |c| {
                c.trace_v2(
                    rusqlite::trace::TraceEventCodes::SQLITE_TRACE_STMT,
                    Some(trace),
                );
                Ok::<_, rusqlite::Error>(())
            },
            || rusqlite::Error::InvalidQuery,
        )
        .unwrap();
    for _ in 0..8 {
        database
            .call_read(
                |c| {
                    c.trace_v2(
                        rusqlite::trace::TraceEventCodes::SQLITE_TRACE_STMT,
                        Some(trace),
                    );
                    Ok::<_, rusqlite::Error>(())
                },
                || rusqlite::Error::InvalidQuery,
            )
            .unwrap();
    }
    Fixture {
        _directory: directory,
        path,
        database,
        tasks,
        projects,
        prompts,
    }
}
fn seed(database: &SharedDatabase, start: i64, end: i64) {
    database.call(move |c| {
            let tx = c.transaction()?;
            for i in start..end {
                let task = format!("SCALING-TASK-{i}");
                let project = format!("SCALING-PROJECT-{i}");
                let prompt = format!("scaling-prompt-{i}");
                let id = 900_000_000_000_000_000_i64+i;
                tx.execute("INSERT INTO tasks(id,task_code,workspace_code,title,flow_code,status,priority,task_type,source_type,created_at,updated_at) VALUES(?1,?2,'TRON_CORE',?2,'default','todo','Medium','General','Internal',1,1)",params![id,task])?;
                tx.execute("INSERT INTO task_agent_links(id,task_code,agent_code,linked_at) VALUES(?1,?2,'main-agent',1)",params![id,task])?;
                tx.execute("INSERT INTO task_app_links(id,task_code,app_code,linked_at) VALUES(?1,?2,'APP-TRON-CONSOLE',1)",params![id,task])?;
                tx.execute("INSERT INTO projects(id,project_code,workspace_code,name,status,created_at,updated_at) VALUES(?1,?2,'TRON_CORE',?2,'ACTIVE',1,1)",params![id,project])?;
                tx.execute("INSERT INTO project_app_links VALUES(?1,?2,'APP-TRON-CONSOLE',0,1)",params![id,project])?;
                tx.execute("INSERT INTO prompts(id,prompt_code,display_name,category,prompt_body,enabled,created_at,updated_at) VALUES(?1,?2,?2,'TEMPLATE','body',1,1,1)",params![id,prompt])?;
                tx.execute("INSERT INTO prompt_variables(prompt_code,variable_name,description,position) VALUES(?1,'name','Name',0)",[&prompt])?;
                tx.execute("INSERT INTO prompt_translations(prompt_code,locale,display_name,prompt_body,updated_at) VALUES(?1,'en-US','English','body',1)",[&prompt])?;
            }
            tx.commit()
        },||rusqlite::Error::InvalidQuery).unwrap();
}
#[test]
fn list_queries_stay_constant_as_tasks_projects_and_prompts_grow() {
    let _guard = TEST_LOCK.lock().unwrap();
    let Fixture {
        _directory,
        path,
        database,
        tasks,
        projects,
        mut prompts,
    } = fixture();
    let mut counts = Vec::new();
    for (start, end) in [(0, 1), (1, 40)] {
        seed(&database, start, end);
        let (task_page, task_queries) = measure(|| {
            tasks
                .list_tasks(TaskListQuery {
                    workspace_code: Some("TRON_CORE".into()),
                    size: Some(100),
                    ..Default::default()
                })
                .unwrap()
        });
        let selected = task_page
            .records
            .iter()
            .filter(|task| task.task_code.starts_with("SCALING-"))
            .collect::<Vec<_>>();
        assert_eq!(selected.len(), end as usize);
        assert!(selected.iter().all(
            |task| task.agent_codes == ["main-agent"] && task.app_codes == ["APP-TRON-CONSOLE"]
        ));
        let (project_page, project_queries) = measure(|| projects.list("TRON_CORE").unwrap());
        let selected = project_page
            .iter()
            .filter(|project| project.project_code.starts_with("SCALING-"))
            .collect::<Vec<_>>();
        assert_eq!(selected.len(), end as usize);
        assert!(
            selected
                .iter()
                .all(|project| project.primary_app_code.as_deref() == Some("APP-TRON-CONSOLE"))
        );
        let (prompt_page, prompt_queries) = measure(|| prompts.list_prompts().unwrap());
        let selected = prompt_page
            .iter()
            .filter(|prompt| prompt.prompt_code.starts_with("scaling-"))
            .collect::<Vec<_>>();
        assert_eq!(selected.len(), end as usize);
        assert!(
            selected
                .iter()
                .all(|prompt| prompt.variables.len() == 1 && prompt.translations.len() == 1)
        );
        counts.push((task_queries, project_queries, prompt_queries));
    }
    assert_eq!(
        counts[0], counts[1],
        "list SELECTs grew with row count: {counts:?}"
    );
    assert!(
        counts[1].0 <= 8 && counts[1].1 <= 5 && counts[1].2 == 3,
        "unexpected query budget: {counts:?}"
    );
    eprintln!("SELECTs for 1/40 rows (tasks,projects,prompts): {counts:?}");
    let expected = (0..40)
        .map(|index| format!("SCALING-TASK-{index}"))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut actual = Vec::new();
    for current in 1..=4 {
        let page = tasks
            .list_tasks(TaskListQuery {
                workspace_code: Some("TRON_CORE".into()),
                current: Some(current),
                size: Some(13),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(page.total, 40);
        assert!(page.records.iter().all(
            |task| task.agent_codes == ["main-agent"] && task.app_codes == ["APP-TRON-CONSOLE"]
        ));
        actual.extend(page.records.into_iter().map(|task| task.task_code));
    }
    assert_eq!(actual, expected);
    // More IDs than SQLite's legacy 999-variable limit, including missing IDs.
    let codes = (0..1200)
        .map(|i| format!("SCALING-TASK-{i}"))
        .collect::<Vec<_>>();
    let mut repository = SqliteTaskRepository::connect(&path).unwrap();
    let (relations, count) = measure(|| repository.relations(&codes).unwrap());
    assert_eq!(relations.len(), 40);
    assert_eq!(count, 2);
    let mut app_repository = SqliteAppRepository::connect(&path).unwrap();
    let mut app_codes = vec!["missing".to_owned(); 1200];
    app_codes.push("APP-TRON-CONSOLE".into());
    assert_eq!(app_repository.apps_by_codes(&app_codes).unwrap().len(), 1);
    let mut project_repository = SqliteProjectRepository::connect(&path).unwrap();
    assert!(project_repository.list_links_many(&[]).unwrap().is_empty());
    use crate::application::navigation::{
        DEFAULT_NAVIGATION_MENU_TREE_ITEMS, NavigationRepository,
    };
    let mut navigation = SqliteNavigationRepository::connect(&path).unwrap();
    let defaults = DEFAULT_NAVIGATION_MENU_TREE_ITEMS;
    let mut navigation_counts = Vec::new();
    for items in [&defaults[..1], defaults] {
        database
            .call(
                |c| {
                    // Delete leaves first: parent_item_id uses ON DELETE RESTRICT.
                    while c.execute("DELETE FROM navigation_menu_tree_items WHERE id NOT IN (SELECT parent_item_id FROM navigation_menu_tree_items WHERE parent_item_id IS NOT NULL)", [])? > 0 {}
                    Ok::<_, rusqlite::Error>(())
                },
                || rusqlite::Error::InvalidQuery,
            )
            .unwrap();
        let (restored, count) = measure(|| navigation.restore_default_menu_trees(items).unwrap());
        assert_eq!(restored, items.len() as i64);
        navigation_counts.push(count);
    }
    assert_eq!(
        navigation_counts[0], navigation_counts[1],
        "navigation restore lookups grew per item"
    );
    let restored = navigation.list_items("ADVANCED").unwrap();
    assert_eq!(restored.len(), defaults.len());
    for expected in defaults {
        let item = restored
            .iter()
            .find(|item| item.menu.menu_code == expected.menu_code)
            .unwrap();
        assert_eq!(item.parent_menu_code.as_deref(), expected.parent_menu_code);
    }
    let navigation = crate::application::navigation::NavigationService::new(
        SqliteNavigationRepository::connect(&path).unwrap(),
        16,
    );
    let (restored, service_queries) = measure(|| navigation.restore_default_menu_trees().unwrap());
    assert_eq!(restored.restored_item_count, 0);
    assert!(
        service_queries <= 12,
        "service restored menus with per-item reads: {service_queries}"
    );
    eprintln!("navigation restore service SELECTs: {service_queries}");
    use crate::application::knowledge_base::ingestion::*;
    use sha2::{Digest, Sha256};
    let queue = SqliteKnowledgeIngestionQueue::connect(&path).unwrap();
    let mut claim_counts = Vec::new();
    for size in [1, 40] {
        database.call(move |c| {
            c.execute("DELETE FROM knowledge_base_ingestion_tasks",[])?;
            c.execute("DELETE FROM knowledge_base_documents",[])?;
            c.execute("INSERT OR IGNORE INTO knowledge_bases(id,owner_user_code,payload_json) VALUES('scaling','owner','{}')",[])?;
            for i in 0..size {
                let id=format!("document-{i}");
                let doc=serde_json::json!({"id":id,"resourceCode":format!("resource-{i}"),"resourceRevision":1,"revision":0,"status":"READY"});
                c.execute("INSERT INTO knowledge_base_documents VALUES(?1,'scaling',?2,0)",params![id,doc.to_string()])?;
            }
            Ok::<_,rusqlite::Error>(())
        },||rusqlite::Error::InvalidQuery).unwrap();
        for i in 0..size {
            let config = serde_json::json!({"tier":"standard"});
            let hash = format!(
                "{:x}",
                Sha256::digest(format!("resource-{i}:1:{config}").as_bytes())
            );
            queue
                .enqueue(IngestionTask::queued(
                    format!("task-{i}"),
                    "scaling".into(),
                    format!("document-{i}"),
                    "owner".into(),
                    "workspace".into(),
                    0,
                    1,
                    hash,
                    config,
                ))
                .unwrap();
        }
        let parse_endpoints = (0..size)
            .map(|i| ParseEndpointCapacity {
                endpoint_id: format!("node-{i}"),
                capacity: 1,
                resource_group: format!("node-{i}"),
                resource_group_capacity: 1,
                weight: 1,
                supported_tiers: vec![],
            })
            .collect();
        let (claimed, count) = measure(|| {
            queue
                .claim(IngestionClaim {
                    task_id: None,
                    stage: IngestionStage::Parsing,
                    owner: "worker".into(),
                    now_ms: 1,
                    lease_ms: 100,
                    global_capacity: 100,
                    embedding_resource_group: None,
                    parse_endpoints,
                })
                .unwrap()
        });
        assert!(claimed.is_some());
        claim_counts.push(count);
        assert_candidate_index(&database);
    }
    assert_eq!(
        claim_counts[0], claim_counts[1],
        "claim queries grew with tasks/nodes"
    );
    eprintln!(
        "navigation restore SELECTs (1/all items): {navigation_counts:?}; queue claim SELECTs (1/40 tasks and nodes): {claim_counts:?}"
    );
}

#[test]
fn task_sql_pages_preserve_filters_off_page_progress_and_unicode_search() {
    let _guard = TEST_LOCK.lock().unwrap();
    let f = fixture();
    seed(&f.database, 0, 40);
    f.database.call(|c| c.execute_batch("UPDATE tasks SET status='NOT_STARTED',flow_code='DEFAULT'; UPDATE tasks SET parent_task_code='SCALING-TASK-0',progress_value=20 WHERE task_code='SCALING-TASK-1';
        UPDATE tasks SET parent_task_code='SCALING-TASK-0',progress_value=80 WHERE task_code='SCALING-TASK-2';
        UPDATE tasks SET parent_task_code='SCALING-TASK-1',progress_value=60 WHERE task_code='SCALING-TASK-3';
        UPDATE tasks SET parent_task_code='SCALING-TASK-0',progress_value=100,archived_at=2 WHERE task_code='SCALING-TASK-4';
        UPDATE tasks SET parent_task_code='SCALING-TASK-4',progress_value=100 WHERE task_code='SCALING-TASK-5';
        UPDATE tasks SET title='Ä中文', description='needle', execution_result='execution evidence', priority='High' WHERE task_code='SCALING-TASK-0';"), || rusqlite::Error::InvalidQuery).unwrap();
    let base = TaskListQuery {
        workspace_code: Some("TRON_CORE".into()),
        ..Default::default()
    };
    let queries = vec![
        base.clone(),
        TaskListQuery {
            search: Some("ä中文 NEEDLE".into()),
            ..base.clone()
        },
        TaskListQuery {
            search: Some("execution evidence".into()),
            ..base.clone()
        },
        TaskListQuery {
            parent_task_code: Some(" __root__ ".into()),
            ..base.clone()
        },
        TaskListQuery {
            parent_task_code: Some("SCALING-TASK-0".into()),
            ..base.clone()
        },
        TaskListQuery {
            priority: Some(crate::domain::task::TaskPriority::High),
            ..base.clone()
        },
        TaskListQuery {
            include_archived: Some(true),
            ..base.clone()
        },
        TaskListQuery {
            assignee_user_code: Some("unknown".into()),
            ..base.clone()
        },
        TaskListQuery {
            status: Some("unknown".into()),
            ..base.clone()
        },
    ];
    for (query, expected_count) in queries.into_iter().zip([39, 1, 1, 35, 2, 1, 40, 0, 0]) {
        // Kanban uses the original in-memory projection, providing an independent
        // oracle for filtering, progress, ordering of links and all list fields.
        let mut expected = f
            .tasks
            .kanban(query.clone())
            .unwrap()
            .into_iter()
            .flat_map(|c| c.tasks)
            .collect::<Vec<_>>();
        assert_eq!(expected.len(), expected_count);
        expected.sort_by(|a, b| a.task_code.cmp(&b.task_code));
        let mut actual = Vec::new();
        for current in 1..=expected.len().div_ceil(3) + 1 {
            let page = f
                .tasks
                .list_tasks(TaskListQuery {
                    current: Some(current),
                    size: Some(3),
                    ..query.clone()
                })
                .unwrap();
            assert_eq!(page.total, expected.len());
            actual.extend(page.records);
        }
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            serde_json::to_value(expected).unwrap()
        );
    }
    let reader = SqliteTaskRepository::connect(&f.path).unwrap().reader();
    let selected = reader.page(&base, Some((0, 1))).unwrap();
    assert_eq!(selected.tasks.len(), 4);
    let root = f
        .tasks
        .list_tasks(TaskListQuery {
            size: Some(1),
            ..base.clone()
        })
        .unwrap();
    assert_eq!(root.records[0].progress, 70);
    assert_eq!(root.records[0].subtask_count, 2);
    let empty = f
        .tasks
        .list_tasks(TaskListQuery {
            current: Some(usize::MAX),
            size: Some(100),
            ..base
        })
        .unwrap();
    assert_eq!(empty.total, 39);
    assert!(empty.records.is_empty());
    f.database.call(|c| c.execute_batch("UPDATE tasks SET owner_dude_code='dude-a', workspace_code=NULL WHERE task_code='SCALING-TASK-9'"), ||rusqlite::Error::InvalidQuery).unwrap();
    let owned = f
        .tasks
        .list_tasks(TaskListQuery {
            owner_dude_code: Some(" dude-a ".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(owned.total, 1);
    assert_eq!(owned.records[0].task_code, "SCALING-TASK-9");
}

#[test]
fn task_and_project_services_read_committed_state_while_writer_is_held() {
    use std::{sync::mpsc, time::Duration};
    let _guard = TEST_LOCK.lock().unwrap();
    let f = fixture();
    seed(&f.database, 0, 3);
    f.database.call(|c|c.execute_batch("DELETE FROM project_app_links WHERE project_code='SCALING-PROJECT-1'; UPDATE projects SET status='ARCHIVED',archived_at=2 WHERE project_code='SCALING-PROJECT-2';"),||rusqlite::Error::InvalidQuery).unwrap();
    let initial = f.projects.list("TRON_CORE").unwrap();
    assert_eq!(initial.len(), 2);
    assert_eq!(initial[0].project_code, "SCALING-PROJECT-1");
    assert!(initial[0].app_codes.is_empty());
    let (ready_tx, ready_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let db = f.database.clone();
    let writer = std::thread::spawn(move || {
        db.call(move |c| {
        let tx=c.transaction()?;
        tx.execute_batch("UPDATE tasks SET title='new title' WHERE task_code='SCALING-TASK-0'; DELETE FROM task_app_links WHERE task_code='SCALING-TASK-0'; UPDATE projects SET name='new name' WHERE project_code='SCALING-PROJECT-0';")?;
        ready_tx.send(()).unwrap();release_rx.recv_timeout(Duration::from_secs(10)).unwrap();tx.commit()
    },||rusqlite::Error::InvalidQuery)
    });
    ready_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let tasks = f.tasks.clone();
    let projects = f.projects.clone();
    let (result_tx, result_rx) = mpsc::channel();
    let read = std::thread::spawn(move || {
        let page = tasks
            .list_tasks(TaskListQuery {
                workspace_code: Some("TRON_CORE".into()),
                ..Default::default()
            })
            .unwrap();
        result_tx
            .send((page, projects.list("TRON_CORE").unwrap()))
            .unwrap();
    });
    let result = result_rx.recv_timeout(Duration::from_secs(2));
    release_tx.send(()).unwrap();
    writer.join().unwrap().unwrap();
    read.join().unwrap();
    let (tasks, projects) =
        result.expect("list services waited for the writer or service mutation mutex");
    assert_eq!(tasks.records[0].title, "SCALING-TASK-0");
    assert_eq!(tasks.records[0].app_codes, ["APP-TRON-CONSOLE"]);
    assert_eq!(projects[1].name, "SCALING-PROJECT-0");
    let after = f
        .tasks
        .list_tasks(TaskListQuery {
            workspace_code: Some("TRON_CORE".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(after.records[0].title, "new title");
    assert!(after.records[0].app_codes.is_empty());
    assert_eq!(f.projects.list("TRON_CORE").unwrap()[1].name, "new name");
}

mod telemetry_contract;
