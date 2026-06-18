use std::{
    fs,
    future::Future,
    path::PathBuf,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{Connection, OptionalExtension};
use serde_json::{Value, json};
use tron_backend::{
    adapters::outbound::persistence::{
        GrafeoAdapterConfig, GrafeoGraphStore, GrafeoStoreCore, GraphBackedRagRepository,
        SqliteSkillRegistryRepository, sqlite::SqliteRagRepository,
    },
    application::{
        graph::{ScopeResolver, WorkspaceDetailProvider},
        rag::{
            DefaultRagStrategyRegistry, RagDocumentSourceCall, RagDocumentSourceRuntime,
            RagEmbeddingCall, RagEmbeddingResult, RagEmbeddingRuntime, RagIngestAccessContext,
            RagRerankCall, RagResolvedDocument, RagSearchContextRequest,
            RagSearchGraphExpansionRequest, RagSearchRequest, RagSearchRuntime,
            RagSearchTraceRequest, RagService,
        },
        role::{RoleRepository, RoleService},
        skill::{SkillRagIndexService, SkillRegistryService},
        user::{PasswordHasher, UserRepository, UserService},
        workspace::{WorkspaceRepository, WorkspaceService},
    },
    config::AppConfig,
    domain::{
        auth::{AuthError, OAuth2IdentityLink, RefreshSession},
        rag::RagError,
        role::{Role, RoleError},
        skill::{SkillRagIndexStatus, WorkspaceSkillListQuery},
        user::{ListUsersQuery, PagedUsers, User, UserError, UserStatistics},
        workspace::{
            Workspace, WorkspaceDetail, WorkspaceError, WorkspaceLineageItem, WorkspaceRoleAccess,
            WorkspaceStatus, WorkspaceUserAccess,
        },
    },
    shared::{id::SnowflakeId, sqlite_extensions::default_sqlite_extension_context},
};

type TestRagService =
    RagService<Arc<Mutex<GraphBackedRagRepository>>, DefaultRagStrategyRegistry, FakeRuntime>;

type TestSkillRagIndexService = SkillRagIndexService<
    Arc<Mutex<GraphBackedRagRepository>>,
    DefaultRagStrategyRegistry,
    FakeRuntime,
>;

#[tokio::test]
async fn indexes_skill_name_description_and_contains_edges() {
    let harness = TestHarness::new("skill-rag-index");
    harness.write_skill(
        "rag-reviewer",
        "Reviews RAG retrieval changes before release.",
        &["scripts/review.sh", "references/checklist.md"],
    );
    harness.skills.rescan_registry().unwrap();
    harness
        .skills
        .bind_workspace_skill("TRON_CORE", "rag-reviewer", true)
        .unwrap();

    let summary = harness.indexer.index_workspace("TRON_CORE").await.unwrap();

    assert_eq!(summary.indexed_count, 1);
    let row = harness.skill_text_unit("rag-reviewer").unwrap();
    assert_eq!(row["skill_name"], json!("rag-reviewer"));
    assert_eq!(
        row["skill_description"],
        json!("Reviews RAG retrieval changes before release.")
    );
    assert_eq!(row["status"], json!("ACTIVE"));
    assert_eq!(row["source_status"], json!("ACTIVE"));
    if let Some(vector_count) = harness.skill_vector_row_count() {
        assert_eq!(vector_count, 1);
    }

    let related_paths = harness.related_graph_paths("rag-reviewer").await;
    assert_eq!(
        related_paths,
        vec![
            "SKILL.md".to_string(),
            "references/checklist.md".to_string(),
            "scripts/review.sh".to_string(),
        ]
    );
}

#[tokio::test]
async fn disabling_or_unbinding_skill_deactivates_rag_sources() {
    let harness = TestHarness::new("skill-rag-disable");
    harness.write_skill(
        "rag-reviewer",
        "Reviews RAG retrieval changes before release.",
        &["scripts/review.sh"],
    );
    harness.skills.rescan_registry().unwrap();
    harness
        .skills
        .bind_workspace_skill("TRON_CORE", "rag-reviewer", true)
        .unwrap();
    harness.indexer.index_workspace("TRON_CORE").await.unwrap();

    harness
        .skills
        .bind_workspace_skill("TRON_CORE", "rag-reviewer", false)
        .unwrap();
    let disabled = harness.indexer.index_workspace("TRON_CORE").await.unwrap();

    assert_eq!(disabled.disabled_count, 1);
    assert_eq!(harness.search_skill_hits("rag-reviewer").await, 0);
    let state = harness
        .skills
        .get_workspace_skill("TRON_CORE", "rag-reviewer")
        .unwrap()
        .rag_index
        .unwrap();
    assert_eq!(state.vector_status, SkillRagIndexStatus::Disabled);
    assert_eq!(state.graph_status, SkillRagIndexStatus::Disabled);
    let row = harness.skill_text_unit("rag-reviewer").unwrap();
    assert_eq!(row["status"], json!("DISABLED"));
    assert_eq!(row["source_status"], json!("DISABLED"));

    harness
        .skills
        .bind_workspace_skill("TRON_CORE", "rag-reviewer", true)
        .unwrap();
    harness.indexer.index_workspace("TRON_CORE").await.unwrap();
    harness
        .skills
        .unbind_workspace_skill("TRON_CORE", "rag-reviewer")
        .unwrap();
    let unbound = harness.indexer.index_workspace("TRON_CORE").await.unwrap();

    assert_eq!(unbound.disabled_count, 1);
    assert_eq!(harness.search_skill_hits("rag-reviewer").await, 0);
}

#[tokio::test]
async fn reindexing_skill_replaces_stale_contains_edges() {
    let harness = TestHarness::new("skill-rag-reindex");
    harness.write_skill(
        "rag-reviewer",
        "Reviews RAG retrieval changes before release.",
        &["scripts/review.sh", "references/old.md"],
    );
    harness.skills.rescan_registry().unwrap();
    harness
        .skills
        .bind_workspace_skill("TRON_CORE", "rag-reviewer", true)
        .unwrap();
    harness.indexer.index_workspace("TRON_CORE").await.unwrap();

    fs::remove_file(harness.skill_path("rag-reviewer").join("references/old.md")).unwrap();
    harness.indexer.index_workspace("TRON_CORE").await.unwrap();

    let related_paths = harness.related_graph_paths("rag-reviewer").await;
    assert_eq!(
        related_paths,
        vec!["SKILL.md".to_string(), "scripts/review.sh".to_string()]
    );
}

#[tokio::test]
async fn failed_state_persists_and_refresh_retries_it() {
    let harness = TestHarness::new("skill-rag-retry");
    harness.write_skill(
        "rag-reviewer",
        "Reviews RAG retrieval changes before release.",
        &["scripts/review.sh"],
    );
    harness.skills.rescan_registry().unwrap();
    harness
        .skills
        .bind_workspace_skill("TRON_CORE", "rag-reviewer", true)
        .unwrap();
    fs::remove_file(harness.skill_path("rag-reviewer").join("SKILL.md")).unwrap();

    let failed = harness.indexer.index_workspace("TRON_CORE").await.unwrap();

    assert_eq!(failed.failed_count, 1);
    let listed = harness
        .skills
        .list_workspace_skills("TRON_CORE", WorkspaceSkillListQuery { search: None })
        .unwrap();
    let failed_state = listed[0].rag_index.as_ref().unwrap();
    assert_eq!(failed_state.vector_status, SkillRagIndexStatus::Failed);
    assert_eq!(failed_state.graph_status, SkillRagIndexStatus::Failed);

    harness.write_skill(
        "rag-reviewer",
        "Reviews RAG retrieval changes before release.",
        &["scripts/review.sh"],
    );
    let retried = harness.indexer.index_workspace("TRON_CORE").await.unwrap();

    assert_eq!(retried.indexed_count, 1);
    let recovered = harness
        .skills
        .get_workspace_skill("TRON_CORE", "rag-reviewer")
        .unwrap()
        .rag_index
        .unwrap();
    assert_eq!(recovered.vector_status, SkillRagIndexStatus::Indexed);
    assert_eq!(recovered.graph_status, SkillRagIndexStatus::Indexed);
}

struct TestHarness {
    database_url: String,
    registry_root: PathBuf,
    skills: SkillRegistryService,
    indexer: TestSkillRagIndexService,
    rag: TestRagService,
}

impl TestHarness {
    fn new(label: &str) -> Self {
        let database_url = temp_database_path(label).to_string_lossy().to_string();
        let graph_path = temp_database_path(&format!("{label}-graph"))
            .to_string_lossy()
            .to_string();
        let registry_root = temp_registry_root(label);
        fs::create_dir_all(registry_root.join(".agents/skills")).unwrap();
        let skills = SkillRegistryService::new(
            SqliteSkillRegistryRepository::connect(&database_url).unwrap(),
            build_workspace_service(),
            &registry_root,
        );
        let repository = build_rag_repository(&database_url, &graph_path);
        let runtime = FakeRuntime::new(vec![0.25; 4096]);
        let rag = RagService::new(repository, DefaultRagStrategyRegistry::default(), runtime);
        let indexer = SkillRagIndexService::new(skills.clone(), rag.clone());
        Self {
            database_url,
            registry_root,
            skills,
            indexer,
            rag,
        }
    }

    fn skill_path(&self, skill_name: &str) -> PathBuf {
        self.registry_root.join(".agents/skills").join(skill_name)
    }

    fn write_skill(&self, skill_name: &str, description: &str, files: &[&str]) {
        let skill_dir = self.skill_path(skill_name);
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            format!("---\ndescription: {description}\n---\n# {skill_name}\n"),
        )
        .unwrap();
        for file in files {
            let path = skill_dir.join(file);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, format!("{skill_name} {file}\n")).unwrap();
        }
    }

    fn skill_text_unit(&self, skill_name: &str) -> Option<Value> {
        let connection = open_test_connection(&self.database_url);
        connection
            .query_row(
                "SELECT skill_name, skill_description, status, source_status
                 FROM rag_skill_text_units
                 WHERE skill_name = ?1",
                [skill_name],
                |row| {
                    Ok(json!({
                        "skill_name": row.get::<_, String>(0)?,
                        "skill_description": row.get::<_, String>(1)?,
                        "status": row.get::<_, String>(2)?,
                        "source_status": row.get::<_, String>(3)?,
                    }))
                },
            )
            .optional()
            .unwrap()
    }

    fn skill_vector_row_count(&self) -> Option<usize> {
        let connection = open_test_connection(&self.database_url);
        if !table_exists(&connection, "rag_skill_text_units_vec_d4096") {
            return None;
        }
        Some(
            connection
                .query_row(
                    "SELECT count(*) FROM rag_skill_text_units_vec_d4096",
                    [],
                    |row| row.get::<_, usize>(0),
                )
                .unwrap(),
        )
    }

    async fn search_skill_hits(&self, skill_name: &str) -> usize {
        self.rag
            .search(search_request(
                "skill-graph-related-v1",
                skill_name,
                skill_name,
            ))
            .await
            .unwrap()
            .hits
            .len()
    }

    async fn related_graph_paths(&self, skill_name: &str) -> Vec<String> {
        let result = self
            .rag
            .search(search_request(
                "skill-graph-related-v1",
                skill_name,
                skill_name,
            ))
            .await
            .unwrap();
        let mut paths = result.hits[0].metadata["relatedGraph"]["edges"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|edge| edge["relationship"] == json!("CONTAINS"))
            .filter_map(|edge| edge["properties"]["path"].as_str().map(ToOwned::to_owned))
            .collect::<Vec<_>>();
        paths.sort_unstable();
        paths
    }
}

fn build_rag_repository(
    database_url: &str,
    graph_path: &str,
) -> Arc<Mutex<GraphBackedRagRepository>> {
    let core = GrafeoStoreCore::connect(GrafeoAdapterConfig {
        storage_path: graph_path.to_string(),
    })
    .unwrap();
    let store = GrafeoGraphStore::new(core);
    Arc::new(Mutex::new(GraphBackedRagRepository::new(
        SqliteRagRepository::connect(database_url).unwrap(),
        Arc::new(store),
        Arc::new(ScopeResolver::with_workspace_provider(
            FakeWorkspaceProvider,
        )),
    )))
}

fn build_workspace_service() -> WorkspaceService {
    WorkspaceService::new(
        StaticWorkspaceRepository,
        UserService::new(
            EmptyUserRepository,
            NoopPasswordHasher,
            AppConfig::default().auth,
        ),
        RoleService::new(EmptyRoleRepository),
    )
}

fn search_request(profile_id: &str, query: &str, skill_name: &str) -> RagSearchRequest {
    RagSearchRequest {
        profile_id: profile_id.into(),
        workspace_code: "TRON_CORE".into(),
        query: query.into(),
        access_context: RagIngestAccessContext {
            user_code: "USR-AUTH-0001".into(),
            agent_code: "AGENT_CODEX".into(),
            workspace_code: "TRON_CORE".into(),
        },
        filters: json!({ "skillName": skill_name }),
        limit: 5,
        token_budget: 256,
        context: RagSearchContextRequest {
            strategy: "RESULT_LIST".into(),
            return_fields: vec!["skillName".into(), "skillDescription".into()],
            graph_expansion: RagSearchGraphExpansionRequest {
                relationships: vec!["CONTAINS".into()],
            },
        },
        trace: RagSearchTraceRequest { enabled: false },
    }
}

fn table_exists(connection: &Connection, table_name: &str) -> bool {
    connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE name = ?1 LIMIT 1",
            [table_name],
            |_| Ok(()),
        )
        .optional()
        .unwrap()
        .is_some()
}

fn open_test_connection(database_url: &str) -> Connection {
    let connection = Connection::open(database_url).unwrap();
    default_sqlite_extension_context()
        .prepare_connection(&connection)
        .unwrap();
    connection
}

fn temp_database_path(label: &str) -> PathBuf {
    let nanos = unique_nanos();
    std::env::temp_dir().join(format!("{label}-{nanos}.sqlite"))
}

fn temp_registry_root(label: &str) -> PathBuf {
    let nanos = unique_nanos();
    std::env::temp_dir().join(format!("{label}-{nanos}"))
}

fn unique_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

#[derive(Clone)]
struct FakeRuntime {
    embedding: Vec<f32>,
    embedding_calls: Arc<AtomicUsize>,
}

impl FakeRuntime {
    fn new(embedding: Vec<f32>) -> Self {
        Self {
            embedding,
            embedding_calls: Arc::new(AtomicUsize::new(0)),
        }
    }
}

impl RagEmbeddingRuntime for FakeRuntime {
    fn embed<'a>(
        &'a self,
        _call: RagEmbeddingCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingResult, String>> + Send + 'a>> {
        Box::pin(async move {
            self.embedding_calls.fetch_add(1, Ordering::SeqCst);
            Ok(RagEmbeddingResult {
                model_code: "FAKE_EMBEDDING".into(),
                embedding: self.embedding.clone(),
            })
        })
    }
}

impl RagSearchRuntime for FakeRuntime {
    fn embed<'a>(
        &'a self,
        call: RagEmbeddingCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingResult, String>> + Send + 'a>> {
        Box::pin(async move { <Self as RagEmbeddingRuntime>::embed(self, call).await })
    }

    fn rerank<'a>(
        &'a self,
        _call: RagRerankCall,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<(usize, f64)>, String>> + Send + 'a>> {
        Box::pin(async { Ok(vec![(0, 1.0)]) })
    }
}

impl RagDocumentSourceRuntime for FakeRuntime {
    fn resolve_project_doc_markdown<'a>(
        &'a self,
        _call: RagDocumentSourceCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagResolvedDocument, RagError>> + Send + 'a>> {
        Box::pin(async { Err(RagError::SourceResolveFailed) })
    }
}

#[derive(Debug, Clone, Copy)]
struct FakeWorkspaceProvider;

impl WorkspaceDetailProvider for FakeWorkspaceProvider {
    fn get_workspace(&self, workspace_code: &str) -> Result<WorkspaceDetail, WorkspaceError> {
        if workspace_code != "TRON_CORE" {
            return Err(WorkspaceError::NotFound);
        }
        Ok(workspace_detail())
    }
}

struct StaticWorkspaceRepository;

impl WorkspaceRepository for StaticWorkspaceRepository {
    fn list_workspaces(&mut self) -> Result<Vec<Workspace>, WorkspaceError> {
        Ok(vec![Workspace {
            id: SnowflakeId::new(1),
            workspace_code: "TRON_CORE".into(),
            name: "Tron Core".into(),
            description: None,
            parent_workspace_code: None,
            status: WorkspaceStatus::Active,
            created_at: "2026-06-04T00:00:00Z".into(),
            updated_at: "2026-06-04T00:00:00Z".into(),
            archived_at: None,
        }])
    }

    fn next_id(&mut self) -> Result<i64, WorkspaceError> {
        Ok(2)
    }

    fn create_workspace(&mut self, _workspace: &Workspace) -> Result<(), WorkspaceError> {
        Ok(())
    }

    fn update_workspace(&mut self, _workspace: &Workspace) -> Result<(), WorkspaceError> {
        Ok(())
    }

    fn list_user_access(
        &mut self,
        _workspace_code: &str,
    ) -> Result<Vec<WorkspaceUserAccess>, WorkspaceError> {
        Ok(Vec::new())
    }

    fn replace_user_access(
        &mut self,
        _workspace_code: &str,
        _entries: &[WorkspaceUserAccess],
    ) -> Result<Vec<WorkspaceUserAccess>, WorkspaceError> {
        Ok(Vec::new())
    }

    fn list_role_access(
        &mut self,
        _workspace_code: &str,
    ) -> Result<Vec<WorkspaceRoleAccess>, WorkspaceError> {
        Ok(Vec::new())
    }

    fn replace_role_access(
        &mut self,
        _workspace_code: &str,
        _entries: &[WorkspaceRoleAccess],
    ) -> Result<Vec<WorkspaceRoleAccess>, WorkspaceError> {
        Ok(Vec::new())
    }

    fn archive_subtree(
        &mut self,
        _workspace_codes: &[String],
        _archived_at: &str,
    ) -> Result<usize, WorkspaceError> {
        Ok(0)
    }

    fn restore_subtree(
        &mut self,
        _workspace_codes: &[String],
        _restored_at: &str,
    ) -> Result<usize, WorkspaceError> {
        Ok(0)
    }
}

fn workspace_detail() -> WorkspaceDetail {
    WorkspaceDetail {
        id: "1".into(),
        workspace_code: "TRON_CORE".into(),
        name: "Tron Core".into(),
        description: None,
        parent_workspace_code: None,
        depth: 1,
        status: WorkspaceStatus::Active,
        path: vec![WorkspaceLineageItem {
            workspace_code: "TRON_CORE".into(),
            name: "Tron Core".into(),
            depth: 1,
        }],
        children: Vec::new(),
        direct_user_count: 0,
        role_grant_count: 0,
        effective_user_count: 0,
        created_at: "2026-06-04T00:00:00Z".into(),
        updated_at: "2026-06-04T00:00:00Z".into(),
        archived_at: None,
    }
}

struct EmptyUserRepository;

impl UserRepository for EmptyUserRepository {
    fn list_users(&mut self, query: ListUsersQuery) -> Result<PagedUsers, UserError> {
        Ok(PagedUsers {
            current: query.current.unwrap_or(1),
            size: query.size.unwrap_or(100),
            total: 0,
            records: Vec::new(),
        })
    }

    fn get_user(&mut self, _user_code: &str) -> Result<User, UserError> {
        Err(UserError::NotFound)
    }

    fn statistics(&mut self) -> Result<UserStatistics, UserError> {
        Ok(UserStatistics {
            total_users: 0,
            active_users: 0,
            role_assignments: 0,
            space_assignments: 0,
        })
    }

    fn next_id(&mut self) -> Result<i64, UserError> {
        Ok(1)
    }

    fn next_user_code(&mut self, _username: &str) -> Result<String, UserError> {
        Ok("USR-EMPTY".into())
    }

    fn user_code_exists(
        &mut self,
        _user_code: &str,
        _except_user_code: Option<&str>,
    ) -> Result<bool, UserError> {
        Ok(false)
    }

    fn username_exists(
        &mut self,
        _username: &str,
        _except_user_code: Option<&str>,
    ) -> Result<bool, UserError> {
        Ok(false)
    }

    fn email_exists(
        &mut self,
        _email: &str,
        _except_user_code: Option<&str>,
    ) -> Result<bool, UserError> {
        Ok(false)
    }

    fn insert_user(&mut self, _user: &User) -> Result<(), UserError> {
        Ok(())
    }

    fn update_user(&mut self, _user: &User) -> Result<(), UserError> {
        Ok(())
    }

    fn replace_role_codes(&mut self, _user: &User) -> Result<(), UserError> {
        Ok(())
    }

    fn replace_space_codes(&mut self, _user: &User) -> Result<(), UserError> {
        Ok(())
    }

    fn record_password_reset(&mut self, _user: &User, _reset_id: i64) -> Result<(), UserError> {
        Ok(())
    }

    fn find_user_by_login(&mut self, _login: &str) -> Result<Option<User>, AuthError> {
        Ok(None)
    }

    fn find_active_user_by_email(&mut self, _email: &str) -> Result<Option<User>, AuthError> {
        Ok(None)
    }

    fn find_user_by_provider_subject(
        &mut self,
        _provider_code: &str,
        _provider_subject: &str,
    ) -> Result<Option<User>, AuthError> {
        Ok(None)
    }

    fn create_refresh_session(&mut self, _session: &RefreshSession) -> Result<(), AuthError> {
        Ok(())
    }

    fn find_refresh_session(
        &mut self,
        _session_code: &str,
    ) -> Result<Option<RefreshSession>, AuthError> {
        Ok(None)
    }

    fn replace_refresh_session(&mut self, _session: &RefreshSession) -> Result<(), AuthError> {
        Ok(())
    }

    fn revoke_refresh_session(
        &mut self,
        _session_code: &str,
        _revoked_at: i64,
    ) -> Result<(), AuthError> {
        Ok(())
    }

    fn upsert_oauth_identity(&mut self, _identity: &OAuth2IdentityLink) -> Result<(), AuthError> {
        Ok(())
    }
}

struct NoopPasswordHasher;

impl PasswordHasher for NoopPasswordHasher {
    fn hash_password(&self, value: &str) -> Result<String, UserError> {
        Ok(value.to_string())
    }

    fn verify_password(&self, _value: &str, _hash: &str) -> Result<bool, AuthError> {
        Ok(true)
    }
}

struct EmptyRoleRepository;

impl RoleRepository for EmptyRoleRepository {
    fn list_roles(&mut self) -> Result<Vec<Role>, RoleError> {
        Ok(Vec::new())
    }

    fn next_id(&mut self) -> Result<i64, RoleError> {
        Ok(1)
    }

    fn replace_roles(&mut self, _roles: &[Role]) -> Result<Vec<Role>, RoleError> {
        Ok(Vec::new())
    }
}
