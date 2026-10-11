//! Host-owned memory writes use the real MemoryService and SQLite/RAG repositories.
mod common;
#[path = "support/knowledge_base_fixture.rs"]
mod fixture;
use async_trait::async_trait;
use fixture::{Fixture, USER};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};
use tron_backend::{
    adapters::outbound::persistence::sqlite::SqliteDudeMemoryRepository,
    application::{
        dude_memory::*,
        memory::{MemoryRunContext, MemorySearchCommand},
        rag::RagSettings,
    },
    domain::dude_memory::*,
};

struct InterruptedRepository {
    inner: SqliteDudeMemoryRepository,
    interrupt: Arc<AtomicBool>,
}
impl DudeMemoryRepository for InterruptedRepository {
    fn list_batches(&mut self, u: &str, d: &str) -> Result<Vec<MemoryBatch>, DudeMemoryError> {
        self.inner.list_batches(u, d)
    }
    fn get_batch(&mut self, c: &str) -> Result<MemoryBatch, DudeMemoryError> {
        self.inner.get_batch(c)
    }
    fn cursor_versions(
        &mut self,
        u: &str,
        d: &str,
    ) -> Result<BTreeMap<String, String>, DudeMemoryError> {
        self.inner.cursor_versions(u, d)
    }
    fn save_batch(
        &mut self,
        p: Option<&MemoryBatch>,
        n: &MemoryBatch,
        c: bool,
    ) -> Result<(), DudeMemoryError> {
        if n.memory_receipts.len() == 2 && self.interrupt.swap(false, Ordering::SeqCst) {
            return Err(DudeMemoryError::Storage(
                "simulated restart after native memory write".into(),
            ));
        }
        self.inner.save_batch(p, n, c)
    }
    fn list_candidates(&mut self, u: &str) -> Result<Vec<SkillCandidate>, DudeMemoryError> {
        self.inner.list_candidates(u)
    }
    fn get_candidate(&mut self, c: &str) -> Result<SkillCandidate, DudeMemoryError> {
        self.inner.get_candidate(c)
    }
    fn save_candidate(
        &mut self,
        p: Option<&SkillCandidate>,
        n: &SkillCandidate,
    ) -> Result<(), DudeMemoryError> {
        self.inner.save_candidate(p, n)
    }
}
struct PlannedAgent {
    output: Value,
    calls: AtomicUsize,
}
#[async_trait]
impl DudeMemoryAgentRuntime for PlannedAgent {
    async fn execute(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: String,
    ) -> Result<String, DudeMemoryError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.output.to_string())
    }
}
fn source(workspace: &str, code: &str) -> MemorySource {
    MemorySource {
        source_code: code.into(),
        workspace_code: workspace.into(),
        version: "v1".into(),
        excerpt: "Confirmed user decision backed by completed work".into(),
        root_owner_dude_code: None,
        lineage_dude_codes: vec![],
        kind: MemorySourceKind::Conversation,
    }
}
fn proposal(workspace: &str, key: &str, kind: &str, scope: &str, source: &str) -> Value {
    json!({"workspaceCode":workspace,"semanticKey":key,"title":key,"text":format!("Stable {key} evidence"),"claimKind":kind,"scope":scope,"sourceCodes":[source],"conflictWithMemoryCode":null})
}
fn sessions() -> Arc<dyn DudeMemorySessionFactory> {
    Arc::new(CallbackDudeMemorySessionFactory(
        |_: String, _: String, batch: String, skill: String| -> MemorySessionFuture {
            Box::pin(async move { Ok(format!("author/{batch}/{skill}")) })
        },
    ))
}
async fn find_memory(
    f: &Fixture,
    workspace: &str,
    code: &str,
) -> tron_backend::application::memory::MemorySearchResponse {
    f.state
        .memory
        .search(
            MemoryRunContext::new(USER, workspace),
            MemorySearchCommand {
                query: None,
                mode: None,
                memory_code: Some(code.into()),
                scope: None,
                scope_code: None,
                memory_type: None,
                memory_status: None,
                review_status: Some(json!(["APPROVED", "PENDING"])),
                injectable: None,
                limit: Some(10),
                token_budget: Some(4000),
                trace: Some(false),
            },
        )
        .await
        .unwrap()
}
#[tokio::test]
async fn dude_memory_runner_resumes_real_memory_writes_without_replanning_or_cursor_loss() {
    let f = Fixture::new();
    f.state
        .rag
        .configure_corpus(
            "memory-card",
            RagSettings {
                vector: false,
                graph: false,
                ..Default::default()
            },
        )
        .unwrap();
    let interruption = Arc::new(AtomicBool::new(true));
    let service = DudeMemoryService::new(
        InterruptedRepository {
            inner: SqliteDudeMemoryRepository::connect(&f.state.config.database_url).unwrap(),
            interrupt: interruption,
        },
        f.state.skills.clone(),
        f.directory.path().join("staging"),
    );
    let sources = vec![
        source("TRON_CORE", "decision-a"),
        source("COMPLIANCE", "decision-b"),
    ];
    let batch = service
        .prepare_batch("memory-dude", USER, sources.clone())
        .unwrap()
        .unwrap();
    let agent = Arc::new(PlannedAgent {
        calls: AtomicUsize::new(0),
        output: json!({"summary":"Preserved evidence and scope","memories":[
        proposal("TRON_CORE","stable-fact","Fact","Space","decision-a"),
        proposal("COMPLIANCE","global-preference","UserDecision","User","decision-b"),
        proposal("COMPLIANCE","unconfirmed-inference","Inference","Space","decision-b")
    ],"skills":[]}),
    });
    let runner = DudeMemoryRunner::with_agent(
        service.clone(),
        f.state.memory.clone(),
        f.state.agent_framework.clone(),
        agent.clone(),
        sessions(),
    );
    assert!(
        runner
            .execute_batch(USER, &batch.batch_code, "batch-session", false)
            .await
            .is_err()
    );
    let interrupted = service.get_batch(USER, &batch.batch_code).unwrap();
    assert_eq!(interrupted.status, MemoryBatchStatus::Running);
    assert!(interrupted.consolidation_plan.is_some());
    assert_eq!(interrupted.memory_receipts.len(), 1);
    assert!(
        service
            .cursor_snapshot(USER, "memory-dude")
            .unwrap()
            .is_empty()
    );
    let first_code = interrupted.memory_receipts[0].memory_code.clone();
    assert_eq!(
        find_memory(&f, "TRON_CORE", &first_code).await.items.len(),
        1
    );
    // Reopen the durable repository: the second write exists but its receipt was not committed.
    let reopened = DudeMemoryService::new(
        SqliteDudeMemoryRepository::connect(&f.state.config.database_url).unwrap(),
        f.state.skills.clone(),
        f.directory.path().join("staging"),
    );
    let resumed = DudeMemoryRunner::with_agent(
        reopened.clone(),
        f.state.memory.clone(),
        f.state.agent_framework.clone(),
        agent.clone(),
        sessions(),
    );
    let done = resumed
        .execute_batch(USER, &batch.batch_code, "batch-session", false)
        .await
        .unwrap();
    assert_eq!(done.status, MemoryBatchStatus::Completed);
    assert_eq!(done.memory_receipts.len(), 3);
    assert_eq!(
        agent.calls.load(Ordering::SeqCst),
        1,
        "persisted plan must survive restart"
    );
    assert_eq!(done.memory_receipts[0].memory_code, first_code);
    assert_eq!(
        reopened.cursor_snapshot(USER, "memory-dude").unwrap().len(),
        2
    );
    assert!(
        reopened
            .prepare_batch("memory-dude", USER, sources)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        resumed
            .execute_batch(USER, &batch.batch_code, "batch-session", false)
            .await
            .unwrap(),
        done
    );
    assert_eq!(agent.calls.load(Ordering::SeqCst), 1);
    for receipt in &done.memory_receipts {
        let found = find_memory(&f, &receipt.workspace_code, &receipt.memory_code).await;
        assert_eq!(
            found.items.len(),
            1,
            "retries must reconcile the same durable memory identity"
        );
        let item = &found.items[0];
        if receipt.claim_kind == MemoryClaimKind::Inference {
            assert_eq!(item.review_status.as_deref(), Some("PENDING"));
            assert_eq!(item.injectable, Some(false));
        } else {
            assert_eq!(item.review_status.as_deref(), Some("APPROVED"));
            assert_eq!(item.injectable, Some(true));
        }
        assert_eq!(
            item.scope,
            if receipt.claim_kind == MemoryClaimKind::UserDecision {
                "USER"
            } else {
                "SPACE"
            }
        );
    }
    f.state.agent_framework.close().await;
}

struct AuthorAgent {
    calls: Mutex<Vec<(String, String)>>,
    fail_author: AtomicBool,
}
#[async_trait]
impl DudeMemoryAgentRuntime for AuthorAgent {
    async fn execute(
        &self,
        _: &str,
        _: &str,
        session: &str,
        prompt: String,
    ) -> Result<String, DudeMemoryError> {
        self.calls
            .lock()
            .unwrap()
            .push((session.into(), prompt.clone()));
        if session == "memory-batch-session" {
            return Ok(json!({"summary":"Verified normalization experience","memories":[],"skills":[{"skillName":"normalize-labels","action":"Create","rationale":"Repeated successful normalization backed by executable tool outcome","sourceCodes":["verified-tool"]}]}).to_string());
        }
        assert!(
            prompt.contains("CREATOR_EXACT_VERSION_GUIDANCE"),
            "authoring must load actual installed creator instructions"
        );
        if self.fail_author.swap(false, Ordering::SeqCst) {
            return Err(DudeMemoryError::Storage("simulated author restart".into()));
        }
        Ok(json!({"files":[
            {"path":"SKILL.md","content":"---\nname: normalize-labels\ndescription: Normalize repeated spaces in imported labels.\n---\nUse scripts/normalize.py to preserve label words while collapsing whitespace.\n","encoding":"Utf8"},
            {"path":"scripts/normalize.py","content":"def normalize(value):\n    return ' '.join(value.split())\n","encoding":"Utf8"},
            {"path":"scripts/check.py","content":"from normalize import normalize\nassert normalize(' a  b ') == 'a b'\nassert normalize('') == ''\nprint('behavior verified')\n","encoding":"Utf8"}
        ],"behaviorChecks":[{"name":"normalization-behavior","program":"python3","args":["scripts/check.py"]}]}).to_string())
    }
}
#[tokio::test]
async fn dude_memory_authoring_uses_independent_persistent_session_and_resumes_after_batch_completion()
 {
    use tron_backend::{
        adapters::outbound::persistence::sqlite::SqliteSkillRegistryRepository,
        application::skill::SkillRegistryService,
    };
    let f = Fixture::new();
    let registry = f.directory.path().join("isolated-skill-registry");
    let skills = SkillRegistryService::new(
        SqliteSkillRegistryRepository::connect(&f.state.config.database_url).unwrap(),
        registry.clone(),
    );
    let creator = registry.join(".agents/skills/skill-creator");
    std::fs::create_dir_all(&creator).unwrap();
    std::fs::write(creator.join("SKILL.md"),"---\nname: skill-creator\ndescription: Create reusable skills from evidence.\nmetadata:\n  version: '7.8.9'\n---\nCREATOR_EXACT_VERSION_GUIDANCE\nPreserve intent and validate actual behavior.\n").unwrap();
    skills.rescan_registry().unwrap();
    let service = DudeMemoryService::new(
        SqliteDudeMemoryRepository::connect(&f.state.config.database_url).unwrap(),
        skills,
        f.directory.path().join("author-stages"),
    );
    let mut verified = source("TRON_CORE", "verified-tool");
    verified.kind = MemorySourceKind::TaskResult;
    let batch = service
        .prepare_batch("memory-dude", USER, vec![verified])
        .unwrap()
        .unwrap();
    let agent = Arc::new(AuthorAgent {
        calls: Mutex::new(Vec::new()),
        fail_author: AtomicBool::new(true),
    });
    let runner = DudeMemoryRunner::with_agent(
        service.clone(),
        f.state.memory.clone(),
        f.state.agent_framework.clone(),
        agent.clone(),
        sessions(),
    );
    let done = runner
        .execute_batch(USER, &batch.batch_code, "memory-batch-session", false)
        .await
        .unwrap();
    assert_eq!(
        done.status,
        MemoryBatchStatus::Completed,
        "author restart must not block incremental memory completion"
    );
    let first = service.list_candidates(USER).unwrap().pop().unwrap();
    assert_eq!(first.status, SkillCandidateStatus::Authoring);
    assert_ne!(first.author_session_code, "memory-batch-session");
    runner
        .resume_candidates("memory-dude", USER, false)
        .await
        .unwrap();
    let pending = service.get_candidate(USER, &first.candidate_code).unwrap();
    assert_eq!(pending.status, SkillCandidateStatus::PendingApproval);
    assert_eq!(pending.author_session_code, first.author_session_code);
    assert_eq!(pending.creator.as_ref().unwrap().version, "7.8.9");
    assert!(pending.validation.as_ref().unwrap().behavior_passed);
    assert!(
        pending
            .validation
            .as_ref()
            .unwrap()
            .checks
            .iter()
            .any(|c| c.evidence.contains("behavior verified"))
    );
    assert!(
        !registry.join(".agents/skills/normalize-labels").exists(),
        "manual approval is the default"
    );
    let calls = agent.calls.lock().unwrap();
    assert_eq!(calls.len(), 3);
    assert_eq!(
        calls[1].0, calls[2].0,
        "author revision/resume uses its own durable session"
    );
    assert_ne!(calls[0].0, calls[1].0);
    drop(calls);
    f.state.agent_framework.close().await;
}
